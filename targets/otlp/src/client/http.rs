use std::{
    fmt,
    future::Future,
    io::{Cursor, Write},
    pin::Pin,
    sync::{Arc, Mutex},
    task::{self, Context, Poll},
};

use bytes::Buf;
use emit::well_known::{KEY_SPAN_ID, KEY_TRACE_ID};
use hyper::{
    body::{self, Body, Frame, SizeHint},
    client::conn::{http1, http2},
    Method, Request, Uri,
};

use crate::{
    client::Encoding,
    data::{PreEncoded, PreEncodedCursor},
    internal_metrics::InternalMetrics,
    Error,
};

async fn connect(
    metrics: &InternalMetrics,
    version: HttpVersion,
    uri: &HttpUri,
) -> Result<HttpSender, Error> {
    let io = tokio::net::TcpStream::connect((uri.host(), uri.port()))
        .await
        .map_err(|e| {
            metrics.otlp_http_conn_failed.increment();

            Error::new("failed to connect TCP stream", e)
        })?;

    metrics.otlp_http_conn_established.increment();

    if uri.is_https() {
        #[cfg(feature = "tls")]
        {
            let io = tls_handshake(metrics, io, uri).await?;

            http_handshake(metrics, version, io).await
        }
        #[cfg(not(feature = "tls"))]
        {
            return Err(Error::new("https support requires the `tls` Cargo feature"));
        }
    } else {
        http_handshake(metrics, version, io).await
    }
}

async fn tls_handshake(
    metrics: &InternalMetrics,
    io: tokio::net::TcpStream,
    uri: &HttpUri,
) -> Result<tokio_rustls::client::TlsStream<tokio::net::TcpStream>, Error> {
    use tokio_rustls::{rustls, TlsConnector};

    let domain = uri.host().to_owned().try_into().map_err(|e| {
        metrics.otlp_http_conn_tls_failed.increment();

        Error::new(format_args!("could not extract a DNS name from {uri}"), e)
    })?;

    let tls = {
        let mut root_store = rustls::RootCertStore::empty();

        for cert in rustls_native_certs::load_native_certs().map_err(|e| {
            metrics.otlp_http_conn_tls_failed.increment();
            Error::new("failed to load native certificates", e)
        })? {
            let _ = root_store.add(cert);
        }

        Arc::new(
            rustls::ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth(),
        )
    };

    let conn = TlsConnector::from(tls);

    let io = conn.connect(domain, io).await.map_err(|e| {
        metrics.otlp_http_conn_tls_failed.increment();

        Error::new("failed to connect TLS stream", e)
    })?;

    metrics.otlp_http_conn_tls_handshake.increment();

    Ok(io)
}

async fn http_handshake(
    metrics: &InternalMetrics,
    version: HttpVersion,
    io: impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Sync + Unpin + 'static,
) -> Result<HttpSender, Error> {
    match version {
        HttpVersion::Http1 => http1_handshake(metrics, io).await,
        HttpVersion::Http2 => http2_handshake(metrics, io).await,
    }
}

async fn http1_handshake(
    metrics: &InternalMetrics,
    io: impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Sync + Unpin + 'static,
) -> Result<HttpSender, Error> {
    let (sender, conn) = http1::handshake(HttpIo(io)).await.map_err(|e| {
        metrics.otlp_http_conn_failed.increment();

        Error::new("failed to perform HTTP1 handshake", e)
    })?;

    tokio::task::spawn(async move {
        let _ = conn.await;
    });

    Ok(HttpSender::Http1(sender))
}

async fn http2_handshake(
    metrics: &InternalMetrics,
    io: impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Sync + Unpin + 'static,
) -> Result<HttpSender, Error> {
    let (sender, conn) = http2::handshake(TokioAmbientExecutor, HttpIo(io))
        .await
        .map_err(|e| {
            metrics.otlp_http_conn_failed.increment();

            Error::new("failed to perform HTTP2 handshake", e)
        })?;

    tokio::task::spawn(async move {
        let _ = conn.await;
    });

    Ok(HttpSender::Http2(sender))
}

async fn send_request(
    metrics: &InternalMetrics,
    sender: &mut HttpSender,
    uri: &HttpUri,
    headers: impl Iterator<Item = (&str, &str)>,
    body: HttpBody,
) -> Result<HttpResponse, Error> {
    let rt = emit::runtime::internal();

    let res = sender
        .send_request(metrics, {
            use emit::{Ctxt as _, Props as _};

            let body = {
                #[cfg(all(feature = "tls", feature = "gzip"))]
                {
                    // TODO: This is happening at the wrong level
                    // gzip should be done _before_ framing
                    if !uri.is_https() {
                        body.gzip()?
                    } else {
                        body
                    }
                }
                #[cfg(all(not(feature = "tls"), feature = "gzip"))]
                {
                    body.gzip()?
                }
                #[cfg(not(feature = "gzip"))]
                {
                    body
                }
            };

            let mut req = Request::builder()
                .uri(&uri.0)
                .method(Method::POST)
                .header("host", uri.authority())
                .header("content-length", body.content_length)
                .header("content-type", body.content_type);

            if let Some(content_encoding) = body.content_encoding {
                req = req.header("content-encoding", content_encoding);
            }

            for (k, v) in headers {
                req = req.header(k, v);
            }

            // Propagate traceparent for the batch
            let (trace_id, span_id) = rt.ctxt().with_current(|props| {
                (
                    props.pull::<emit::trace::TraceId, _>(KEY_TRACE_ID),
                    props.pull::<emit::trace::SpanId, _>(KEY_SPAN_ID),
                )
            });

            req = if let (Some(trace_id), Some(span_id)) = (trace_id, span_id) {
                req.header("traceparent", format!("00-{trace_id}-{span_id}-00"))
            } else {
                req
            };

            req.body(body).map_err(|e| {
                metrics.otlp_http_request_failed.increment();

                Error::new("failed to stream HTTP body", e)
            })?
        })
        .await?;

    Ok(res)
}

pub(crate) struct HttpConnection {
    version: HttpVersion,
    uri: HttpUri,
    headers: Vec<(String, String)>,
    body: fn(PreEncoded) -> HttpBody,
    sender: Mutex<Option<HttpSender>>,
    metrics: Arc<InternalMetrics>,
}

pub(crate) struct HttpResponse {
    res: hyper::Response<body::Incoming>,
}

impl HttpConnection {
    pub fn http1(
        metrics: Arc<InternalMetrics>,
        url: impl AsRef<str>,
        headers: impl Into<Vec<(String, String)>>,
        body: fn(PreEncoded) -> HttpBody,
    ) -> Result<Self, Error> {
        Self::new(HttpVersion::Http1, metrics, url, headers, body)
    }

    pub fn http2(
        metrics: Arc<InternalMetrics>,
        url: impl AsRef<str>,
        headers: impl Into<Vec<(String, String)>>,
        body: fn(PreEncoded) -> HttpBody,
    ) -> Result<Self, Error> {
        Self::new(HttpVersion::Http2, metrics, url, headers, body)
    }

    fn new(
        version: HttpVersion,
        metrics: Arc<InternalMetrics>,
        url: impl AsRef<str>,
        headers: impl Into<Vec<(String, String)>>,
        body: fn(PreEncoded) -> HttpBody,
    ) -> Result<Self, Error> {
        let url = url.as_ref();

        Ok(HttpConnection {
            uri: HttpUri(
                url.parse()
                    .map_err(|e| Error::new(format_args!("failed to parse {url}"), e))?,
            ),
            version,
            body,
            headers: headers.into(),
            sender: Mutex::new(None),
            metrics,
        })
    }

    fn poison(&self) -> Option<HttpSender> {
        self.sender.lock().unwrap().take()
    }

    fn unpoison(&self, sender: HttpSender) {
        *self.sender.lock().unwrap() = Some(sender);
    }

    pub async fn send(&self, body: PreEncoded) -> Result<HttpResponse, Error> {
        let mut sender = match self.poison() {
            Some(sender) => sender,
            None => connect(&self.metrics, self.version, &self.uri).await?,
        };

        let res = send_request(
            &self.metrics,
            &mut sender,
            &self.uri,
            self.headers.iter().map(|(k, v)| (&**k, &**v)),
            (self.body)(body),
        )
        .await?;

        self.unpoison(sender);

        Ok(res)
    }
}

#[derive(Debug, Clone, Copy)]
enum HttpVersion {
    Http1,
    Http2,
}

enum HttpSender {
    Http1(http1::SendRequest<HttpBody>),
    Http2(http2::SendRequest<HttpBody>),
}

impl HttpSender {
    async fn send_request(
        &mut self,
        metrics: &InternalMetrics,
        req: Request<HttpBody>,
    ) -> Result<HttpResponse, Error> {
        let res = match self {
            HttpSender::Http1(sender) => sender.send_request(req).await,
            HttpSender::Http2(sender) => sender.send_request(req).await,
        }
        .map_err(|e| {
            metrics.otlp_http_request_failed.increment();

            Error::new("failed to send HTTP request", e)
        })?;

        metrics.otlp_http_request_sent.increment();

        Ok(HttpResponse { res })
    }
}

struct HttpUri(Uri);

impl fmt::Display for HttpUri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl HttpUri {
    fn is_https(&self) -> bool {
        self.0.scheme().unwrap() == &hyper::http::uri::Scheme::HTTPS
    }

    fn host(&self) -> &str {
        self.0.host().unwrap()
    }

    fn authority(&self) -> &str {
        self.0.authority().unwrap().as_str()
    }

    fn port(&self) -> u16 {
        self.0.port_u16().unwrap_or(80)
    }
}

pub(crate) struct HttpBody {
    payload: HttpBodyPayload,
    content_length: usize,
    content_type: &'static str,
    content_encoding: Option<&'static str>,
}

enum HttpBodyPayload {
    Raw(Option<PreEncodedCursor>),
    Grpc {
        header: Option<Cursor<[u8; 5]>>,
        payload: Option<PreEncodedCursor>,
    },
    #[cfg(feature = "gzip")]
    Gzip(Option<Cursor<Box<[u8]>>>),
}

pub(crate) enum HttpBodyData {
    GrpcHeader(Cursor<[u8; 5]>),
    Payload(PreEncodedCursor),
    #[cfg(feature = "gzip")]
    Gzip(Cursor<Box<[u8]>>),
}

impl Buf for HttpBodyData {
    fn remaining(&self) -> usize {
        match self {
            HttpBodyData::Payload(buf) => buf.remaining(),
            HttpBodyData::GrpcHeader(buf) => buf.remaining(),
            #[cfg(feature = "gzip")]
            HttpBodyData::Gzip(buf) => buf.remaining(),
        }
    }

    fn chunk(&self) -> &[u8] {
        match self {
            HttpBodyData::Payload(buf) => buf.chunk(),
            HttpBodyData::GrpcHeader(buf) => buf.chunk(),
            #[cfg(feature = "gzip")]
            HttpBodyData::Gzip(buf) => buf.chunk(),
        }
    }

    fn advance(&mut self, cnt: usize) {
        match self {
            HttpBodyData::Payload(buf) => buf.advance(cnt),
            HttpBodyData::GrpcHeader(buf) => buf.advance(cnt),
            #[cfg(feature = "gzip")]
            HttpBodyData::Gzip(buf) => buf.advance(cnt),
        }
    }
}

impl HttpBody {
    pub fn raw(payload: PreEncoded) -> Self {
        let encoding = Encoding::of(&payload);

        let payload = HttpBodyPayload::Raw(Some(payload.into_cursor()));

        HttpBody {
            content_type: match encoding {
                Encoding::Proto => "application/x-protobuf",
                Encoding::Json => "application/json",
            },
            content_encoding: None,
            content_length: payload.remaining(),
            payload,
        }
    }

    pub fn grpc_framed(payload: PreEncoded) -> Self {
        let encoding = Encoding::of(&payload);

        let payload = HttpBodyPayload::Grpc {
            header: Some({
                let compressed = 0u8;
                let len = (payload.len() as u32).to_be_bytes();

                Cursor::new([compressed, len[0], len[1], len[2], len[3]])
            }),
            payload: Some(payload.into_cursor()),
        };

        HttpBody {
            content_type: match encoding {
                Encoding::Proto => "application/grpc+proto",
                Encoding::Json => "application/grpc+json",
            },
            content_encoding: None,
            content_length: payload.remaining(),
            payload,
        }
    }

    #[cfg(feature = "gzip")]
    fn gzip(mut self) -> Result<Self, Error> {
        let mut enc = flate2::write::GzEncoder::new(
            Vec::with_capacity(self.payload.remaining()),
            flate2::Compression::fast(),
        );

        // Read the gzipped content
        while let Some(mut data) = self.payload.next_chunk() {
            loop {
                let chunk = data.chunk();
                if chunk.len() == 0 {
                    break;
                }

                enc.write_all(chunk)
                    .map_err(|e| Error::new("failed to compress a chunk of bytes", e))?;
                data.advance(chunk.len());
            }
        }

        let buf = enc
            .finish()
            .map_err(|e| Error::new("failed to finalize compression", e))?;

        let payload = HttpBodyPayload::Gzip(Some(Cursor::new(buf.into_boxed_slice())));

        Ok(HttpBody {
            content_type: self.content_type,
            content_encoding: Some("gzip"),
            content_length: payload.remaining(),
            payload,
        })
    }
}

impl HttpBodyPayload {
    fn next_chunk(&mut self) -> Option<HttpBodyData> {
        match self {
            HttpBodyPayload::Raw(ref mut payload) => payload.take().map(HttpBodyData::Payload),
            HttpBodyPayload::Grpc {
                ref mut header,
                ref mut payload,
            } => header
                .take()
                .map(HttpBodyData::GrpcHeader)
                .or_else(|| payload.take().map(HttpBodyData::Payload)),
            #[cfg(feature = "gzip")]
            HttpBodyPayload::Gzip(ref mut payload) => payload.take().map(HttpBodyData::Gzip),
        }
    }

    fn remaining(&self) -> usize {
        match self {
            HttpBodyPayload::Raw(ref payload) => payload
                .as_ref()
                .map(|payload| payload.remaining())
                .unwrap_or(0),
            HttpBodyPayload::Grpc {
                ref header,
                ref payload,
            } => {
                header
                    .as_ref()
                    .map(|header| header.remaining())
                    .unwrap_or(0)
                    + payload
                        .as_ref()
                        .map(|payload| payload.remaining())
                        .unwrap_or(0)
            }
            #[cfg(feature = "gzip")]
            HttpBodyPayload::Gzip(ref payload) => payload
                .as_ref()
                .map(|payload| payload.remaining())
                .unwrap_or(0),
        }
    }
}

impl Body for HttpBody {
    type Data = HttpBodyData;

    type Error = std::convert::Infallible;

    fn poll_frame(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        if let Some(buf) = self.get_mut().payload.next_chunk() {
            Poll::Ready(Some(Ok(Frame::data(buf))))
        } else {
            Poll::Ready(None)
        }
    }

    fn is_end_stream(&self) -> bool {
        match self.payload {
            HttpBodyPayload::Raw(None) => true,
            HttpBodyPayload::Grpc {
                header: None,
                payload: None,
            } => true,
            HttpBodyPayload::Raw(_) => false,
            HttpBodyPayload::Grpc { .. } => false,
            #[cfg(feature = "gzip")]
            HttpBodyPayload::Gzip(None) => true,
            #[cfg(feature = "gzip")]
            HttpBodyPayload::Gzip(_) => false,
        }
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint::with_exact(self.payload.remaining() as u64)
    }
}

impl HttpResponse {
    pub fn status(&self) -> u16 {
        self.res.status().as_u16()
    }

    pub async fn read_to_vec(mut self) -> Result<Vec<u8>, Error> {
        struct BufNext<'a>(&'a mut body::Incoming, &'a mut Vec<u8>);

        impl<'a> Future for BufNext<'a> {
            type Output = Result<bool, Error>;

            fn poll(
                mut self: Pin<&mut Self>,
                ctx: &mut task::Context<'_>,
            ) -> task::Poll<Self::Output> {
                match Pin::new(&mut self.0).poll_frame(ctx) {
                    Poll::Ready(Some(Ok(frame))) => {
                        if let Some(frame) = frame.data_ref() {
                            self.1.extend_from_slice(frame);
                        }

                        if let Some(trailers) = frame.trailers_ref() {
                            todo!()
                        }

                        Poll::Ready(Ok(true))
                    }
                    Poll::Ready(None) => Poll::Ready(Ok(false)),
                    Poll::Ready(Some(Err(e))) => {
                        Poll::Ready(Err(Error::new("failed to read HTTP response body", e)))
                    }
                    Poll::Pending => Poll::Pending,
                }
            }
        }

        let frame = self.res.body_mut();
        let mut body = Vec::new();

        while BufNext(frame, &mut body).await? {}

        Ok(body)
    }
}

struct HttpIo<T>(T);

impl<T: tokio::io::AsyncRead> hyper::rt::Read for HttpIo<T> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut buf: hyper::rt::ReadBufCursor<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let io = unsafe { self.map_unchecked_mut(|io| &mut io.0) };

        let mut read_buf = tokio::io::ReadBuf::uninit(unsafe { buf.as_mut() });

        match tokio::io::AsyncRead::poll_read(io, cx, &mut read_buf) {
            Poll::Ready(Ok(())) => {
                let read = read_buf.filled().len();
                unsafe { buf.advance(read) };

                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T: tokio::io::AsyncWrite> hyper::rt::Write for HttpIo<T> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        let io = unsafe { self.map_unchecked_mut(|io| &mut io.0) };

        tokio::io::AsyncWrite::poll_write(io, cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        let io = unsafe { self.map_unchecked_mut(|io| &mut io.0) };

        tokio::io::AsyncWrite::poll_flush(io, cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let io = unsafe { self.map_unchecked_mut(|io| &mut io.0) };

        tokio::io::AsyncWrite::poll_shutdown(io, cx)
    }
}

#[derive(Clone, Copy)]
struct TokioAmbientExecutor;

impl<F: Future + Send + 'static> hyper::rt::Executor<F> for TokioAmbientExecutor
where
    F::Output: Send + 'static,
{
    fn execute(&self, fut: F) {
        tokio::spawn(fut);
    }
}
