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
    body: HttpContent,
) -> Result<HttpResponse, Error> {
    let rt = emit::runtime::internal();

    let res = sender
        .send_request(metrics, {
            use emit::{Ctxt as _, Props as _};

            let body = {
                #[cfg(all(feature = "tls", feature = "gzip"))]
                {
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
                .header("content-length", body.len())
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
    metrics: Arc<InternalMetrics>,
    version: HttpVersion,
    uri: HttpUri,
    headers: Vec<(String, String)>,
    request: Box<dyn Fn(PreEncoded) -> HttpContent + Send + Sync>,
    response: Box<
        dyn Fn(HttpResponse) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, Error>> + Send>>
            + Send
            + Sync,
    >,
    sender: Mutex<Option<HttpSender>>,
}

pub(crate) struct HttpResponse {
    res: hyper::Response<body::Incoming>,
}

impl HttpConnection {
    pub fn http1<F: Future<Output = Result<Vec<u8>, Error>> + Send + 'static>(
        metrics: Arc<InternalMetrics>,
        url: impl AsRef<str>,
        headers: impl Into<Vec<(String, String)>>,
        request: impl Fn(PreEncoded) -> HttpContent + Send + Sync + 'static,
        response: impl Fn(HttpResponse) -> F + Send + Sync + 'static,
    ) -> Result<Self, Error> {
        Self::new(HttpVersion::Http1, metrics, url, headers, request, response)
    }

    pub fn http2<F: Future<Output = Result<Vec<u8>, Error>> + Send + 'static>(
        metrics: Arc<InternalMetrics>,
        url: impl AsRef<str>,
        headers: impl Into<Vec<(String, String)>>,
        request: impl Fn(PreEncoded) -> HttpContent + Send + Sync + 'static,
        response: impl Fn(HttpResponse) -> F + Send + Sync + 'static,
    ) -> Result<Self, Error> {
        Self::new(HttpVersion::Http2, metrics, url, headers, request, response)
    }

    fn new<F: Future<Output = Result<Vec<u8>, Error>> + Send + 'static>(
        version: HttpVersion,
        metrics: Arc<InternalMetrics>,
        url: impl AsRef<str>,
        headers: impl Into<Vec<(String, String)>>,
        request: impl Fn(PreEncoded) -> HttpContent + Send + Sync + 'static,
        response: impl Fn(HttpResponse) -> F + Send + Sync + 'static,
    ) -> Result<Self, Error> {
        let url = url.as_ref();

        Ok(HttpConnection {
            uri: HttpUri(
                url.parse()
                    .map_err(|e| Error::new(format_args!("failed to parse {url}"), e))?,
            ),
            version,
            request: Box::new(request),
            response: Box::new(move |res| Box::pin(response(res))),
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

    pub async fn send(&self, body: PreEncoded) -> Result<Vec<u8>, Error> {
        let mut sender = match self.poison() {
            Some(sender) => sender,
            None => connect(&self.metrics, self.version, &self.uri).await?,
        };

        let res = send_request(
            &self.metrics,
            &mut sender,
            &self.uri,
            self.headers.iter().map(|(k, v)| (&**k, &**v)),
            (self.request)(body),
        )
        .await?;

        self.unpoison(sender);

        (self.response)(res).await
    }
}

#[derive(Debug, Clone, Copy)]
enum HttpVersion {
    Http1,
    Http2,
}

enum HttpSender {
    Http1(http1::SendRequest<HttpContent>),
    Http2(http2::SendRequest<HttpContent>),
}

impl HttpSender {
    async fn send_request(
        &mut self,
        metrics: &InternalMetrics,
        req: Request<HttpContent>,
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

pub(crate) struct HttpContent {
    header: Option<HttpContentHeader>,
    payload: Option<HttpContentPayload>,
    content_type: &'static str,
    content_encoding: Option<&'static str>,
}

impl HttpContent {
    pub fn raw(payload: PreEncoded) -> Self {
        HttpContent {
            header: None,
            content_type: match Encoding::of(&payload) {
                Encoding::Proto => "application/x-protobuf",
                Encoding::Json => "application/json",
            },
            content_encoding: None,
            payload: Some(HttpContentPayload::PreEncoded(payload)),
        }
    }

    pub fn grpc(payload: PreEncoded) -> Self {
        HttpContent {
            header: Some(HttpContentHeader::Grpc(GrpcHeader {
                compressed: false,
                msg_len: payload.len() as u32,
            })),
            content_type: match Encoding::of(&payload) {
                Encoding::Proto => "application/grpc+proto",
                Encoding::Json => "application/grpc+json",
            },
            content_encoding: None,
            payload: Some(HttpContentPayload::PreEncoded(payload)),
        }
    }

    #[cfg(feature = "gzip")]
    fn gzip(self) -> Result<Self, Error> {
        let payload = self.payload.expect("attempt to compress in-flight request");

        let mut enc = flate2::write::GzEncoder::new(
            Vec::with_capacity(payload.len()),
            flate2::Compression::fast(),
        );

        let mut payload = payload.into_cursor();
        loop {
            let chunk = payload.chunk();
            if chunk.len() == 0 {
                break;
            }

            enc.write_all(chunk)
                .map_err(|e| Error::new("failed to compress a chunk of bytes", e))?;
            payload.advance(chunk.len());
        }

        let buf = enc
            .finish()
            .map_err(|e| Error::new("failed to finalize compression", e))?;

        Ok(HttpContent {
            content_type: self.content_type,
            content_encoding: Some("gzip"),
            header: match self.header {
                Some(HttpContentHeader::Grpc(_)) => Some(HttpContentHeader::Grpc(GrpcHeader {
                    compressed: true,
                    msg_len: buf.len() as u32,
                })),
                None => None,
            },
            payload: Some(HttpContentPayload::Bytes(buf.into_boxed_slice())),
        })
    }

    fn len(&self) -> usize {
        self.header.as_ref().map(|header| header.len()).unwrap_or(0)
            + self
                .payload
                .as_ref()
                .map(|payload| payload.len())
                .unwrap_or(0)
    }
}

impl Body for HttpContent {
    type Data = HttpContentCursor;

    type Error = std::convert::Infallible;

    fn poll_frame(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let unpinned = self.get_mut();

        if let Some(header) = unpinned.header.take() {
            return Poll::Ready(Some(Ok(Frame::data(header.into_cursor()))));
        }

        let Some(payload) = unpinned.payload.take() else {
            return Poll::Ready(None);
        };

        Poll::Ready(Some(Ok(Frame::data(payload.into_cursor()))))
    }

    fn is_end_stream(&self) -> bool {
        match (&self.header, &self.payload) {
            (Some(_), _) | (_, Some(_)) => false,
            _ => true,
        }
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint::with_exact(self.len() as u64)
    }
}

enum HttpContentHeader {
    Grpc(GrpcHeader),
}

enum HttpContentPayload {
    PreEncoded(PreEncoded),
    #[cfg(feature = "gzip")]
    Bytes(Box<[u8]>),
}

impl HttpContentHeader {
    fn len(&self) -> usize {
        match self {
            HttpContentHeader::Grpc(header) => header.len(),
        }
    }

    fn into_cursor(self) -> HttpContentCursor {
        match self {
            HttpContentHeader::Grpc(header) => HttpContentCursor::SmallBytes(header.into_cursor()),
        }
    }
}

impl HttpContentPayload {
    fn len(&self) -> usize {
        match self {
            HttpContentPayload::PreEncoded(payload) => payload.len(),
            HttpContentPayload::Bytes(payload) => payload.len(),
        }
    }

    fn into_cursor(self) -> HttpContentCursor {
        match self {
            HttpContentPayload::PreEncoded(payload) => {
                HttpContentCursor::PreEncoded(payload.into_cursor())
            }
            HttpContentPayload::Bytes(payload) => HttpContentCursor::Bytes(Cursor::new(payload)),
        }
    }
}

struct GrpcHeader {
    compressed: bool,
    msg_len: u32,
}

impl GrpcHeader {
    fn len(&self) -> usize {
        5
    }

    fn into_cursor(self) -> Cursor<[u8; 5]> {
        let compressed = if self.compressed { 1 } else { 0 };
        let len = self.msg_len.to_be_bytes();

        Cursor::new([compressed, len[0], len[1], len[2], len[3]])
    }
}

pub(crate) enum HttpContentCursor {
    PreEncoded(PreEncodedCursor),
    #[cfg(feature = "gzip")]
    Bytes(Cursor<Box<[u8]>>),
    SmallBytes(Cursor<[u8; 5]>),
}

impl Buf for HttpContentCursor {
    fn remaining(&self) -> usize {
        match self {
            HttpContentCursor::PreEncoded(buf) => buf.remaining(),
            #[cfg(feature = "gzip")]
            HttpContentCursor::Bytes(buf) => buf.remaining(),
            HttpContentCursor::SmallBytes(buf) => buf.remaining(),
        }
    }

    fn chunk(&self) -> &[u8] {
        match self {
            HttpContentCursor::PreEncoded(buf) => buf.chunk(),
            #[cfg(feature = "gzip")]
            HttpContentCursor::Bytes(buf) => buf.chunk(),
            HttpContentCursor::SmallBytes(buf) => buf.chunk(),
        }
    }

    fn advance(&mut self, cnt: usize) {
        match self {
            HttpContentCursor::PreEncoded(buf) => buf.advance(cnt),
            #[cfg(feature = "gzip")]
            HttpContentCursor::Bytes(buf) => buf.advance(cnt),
            HttpContentCursor::SmallBytes(buf) => buf.advance(cnt),
        }
    }
}

impl HttpResponse {
    pub fn http_status(&self) -> u16 {
        self.res.status().as_u16()
    }

    pub async fn stream_payload(
        mut self,
        mut body: impl FnMut(&[u8]),
        mut trailer: impl FnMut(&str, &str),
    ) -> Result<(), Error> {
        struct BufNext<'a, B, T>(&'a mut body::Incoming, &'a mut B, &'a mut T);

        impl<'a, B: FnMut(&[u8]), T: FnMut(&str, &str)> Future for BufNext<'a, B, T> {
            type Output = Result<bool, Error>;

            fn poll(self: Pin<&mut Self>, ctx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
                let BufNext(incoming, body, trailer) = unsafe { Pin::get_unchecked_mut(self) };

                match Pin::new(incoming).poll_frame(ctx) {
                    Poll::Ready(Some(Ok(frame))) => {
                        if let Some(frame) = frame.data_ref() {
                            (body)(frame);
                        }

                        if let Some(trailers) = frame.trailers_ref() {
                            for (k, v) in trailers {
                                let k = k.as_str();

                                if let Ok(v) = v.to_str() {
                                    (trailer)(k, v)
                                }
                            }
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

        while BufNext(frame, &mut body, &mut trailer).await? {}

        Ok(())
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
