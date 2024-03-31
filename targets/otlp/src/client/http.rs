use std::{
    future::Future,
    io::Cursor,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{self, Context, Poll},
};

use bytes::Buf;
use emit::well_known::{KEY_SPAN_ID, KEY_TRACE_ID};
use hyper::{
    body::{self, Body, Frame, SizeHint},
    client::conn::{http1, http1::SendRequest},
    Method, Request, Uri,
};

use crate::{
    client::Encoding,
    data::{PreEncoded, PreEncodedCursor},
    internal_metrics::InternalMetrics,
    Error,
};

async fn connect(metrics: &InternalMetrics, uri: &Uri) -> Result<SendRequest<HttpBody>, Error> {
    let io = tokio::net::TcpStream::connect((uri.host().unwrap(), uri.port_u16().unwrap_or(80)))
        .await
        .map_err(|e| {
            metrics.otlp_http_conn_failed.increment();

            Error::new(e)
        })?;

    metrics.otlp_http_conn_established.increment();

    if uri.scheme().unwrap() == &hyper::http::uri::Scheme::HTTPS {
        #[cfg(feature = "tls")]
        {
            let io = tls(metrics, io, uri).await?;

            spawn(metrics, io).await
        }
        #[cfg(not(feature = "tls"))]
        {
            return Err(Error::new("https support requires the `tls` Cargo feature"));
        }
    } else {
        spawn(metrics, io).await
    }
}

async fn tls(
    metrics: &InternalMetrics,
    io: tokio::net::TcpStream,
    uri: &Uri,
) -> Result<tokio_rustls::client::TlsStream<tokio::net::TcpStream>, Error> {
    use tokio_rustls::{rustls, TlsConnector};

    let domain = uri.host().unwrap().to_owned().try_into().map_err(|e| {
        metrics.otlp_http_conn_tls_failed.increment();

        Error::new(e)
    })?;

    let tls = {
        let mut root_store = rustls::RootCertStore::empty();

        for cert in rustls_native_certs::load_native_certs().map_err(|e| {
            metrics.otlp_http_conn_tls_failed.increment();
            Error::new(e)
        })? {
            root_store.add(cert).map_err(|e| {
                metrics.otlp_http_conn_tls_failed.increment();

                Error::new(e)
            })?;
        }

        Arc::new(
            rustls::ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth(),
        )
    };

    let conn = TlsConnector::from(tls);

    conn.connect(domain, io).await.map_err(|e| {
        metrics.otlp_http_conn_tls_failed.increment();

        Error::new(e)
    })
}

async fn spawn(
    metrics: &InternalMetrics,
    io: impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Sync + Unpin + 'static,
) -> Result<SendRequest<HttpBody>, Error> {
    let (sender, conn) = http1::handshake(HttpIo(io)).await.map_err(|e| {
        metrics.otlp_http_conn_failed.increment();

        Error::new(e)
    })?;

    tokio::task::spawn(async move {
        let _ = conn.await;
    });

    Ok(sender)
}

async fn send_request(
    metrics: &InternalMetrics,
    sender: &mut SendRequest<HttpBody>,
    uri: &Uri,
    headers: impl Iterator<Item = (&str, &str)>,
    body: HttpBody,
) -> Result<hyper::Response<body::Incoming>, Error> {
    let rt = emit::runtime::internal();

    let res = sender
        .send_request({
            use emit::{Ctxt as _, Props as _};

            let mut req = Request::builder()
                .uri(uri)
                .method(Method::POST)
                .header("host", uri.authority().unwrap().as_str())
                .header("content-type", body.content_type);

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

                Error::new(e)
            })?
        })
        .await
        .map_err(|e| {
            metrics.otlp_http_request_failed.increment();

            Error::new(e)
        })?;

    metrics.otlp_http_request_sent.increment();

    Ok(res)
}

pub(crate) struct HttpConnection {
    uri: Uri,
    headers: Vec<(String, String)>,
    body: fn(PreEncoded) -> HttpBody,
    sender: Mutex<Option<SendRequest<HttpBody>>>,
    metrics: Arc<InternalMetrics>,
}

pub(crate) struct HttpResponse {
    res: hyper::Response<body::Incoming>,
}

impl HttpConnection {
    pub fn new(
        metrics: Arc<InternalMetrics>,
        url: impl AsRef<str>,
        headers: impl Into<Vec<(String, String)>>,
        body: fn(PreEncoded) -> HttpBody,
    ) -> Result<Self, Error> {
        Ok(HttpConnection {
            uri: url.as_ref().parse().map_err(Error::new)?,
            body,
            headers: headers.into(),
            sender: Mutex::new(None),
            metrics,
        })
    }

    fn poison(&self) -> Option<SendRequest<HttpBody>> {
        self.sender.lock().unwrap().take()
    }

    fn unpoison(&self, sender: SendRequest<HttpBody>) {
        *self.sender.lock().unwrap() = Some(sender);
    }

    pub async fn send(&self, body: PreEncoded) -> Result<HttpResponse, Error> {
        let mut sender = match self.poison() {
            Some(sender) => sender,
            None => connect(&self.metrics, &self.uri).await?,
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

        Ok(HttpResponse { res })
    }
}

pub(crate) struct HttpBody {
    payload: HttpBodyPayload,
    content_type: &'static str,
}

enum HttpBodyPayload {
    Raw(Option<PreEncodedCursor>),
    Grpc {
        header: Option<Cursor<[u8; 5]>>,
        payload: Option<PreEncodedCursor>,
    },
}

pub(crate) enum HttpBodyData {
    GrpcHeader(Cursor<[u8; 5]>),
    Payload(PreEncodedCursor),
}

impl Buf for HttpBodyData {
    fn remaining(&self) -> usize {
        match self {
            HttpBodyData::Payload(buf) => buf.remaining(),
            HttpBodyData::GrpcHeader(buf) => buf.remaining(),
        }
    }

    fn chunk(&self) -> &[u8] {
        match self {
            HttpBodyData::Payload(buf) => buf.chunk(),
            HttpBodyData::GrpcHeader(buf) => buf.chunk(),
        }
    }

    fn advance(&mut self, cnt: usize) {
        match self {
            HttpBodyData::Payload(buf) => buf.advance(cnt),
            HttpBodyData::GrpcHeader(buf) => buf.advance(cnt),
        }
    }
}

impl HttpBody {
    pub fn raw(payload: PreEncoded) -> Self {
        let encoding = Encoding::of(&payload);

        HttpBody {
            content_type: match encoding {
                Encoding::Proto => "application/x-protobuf",
                Encoding::Json => "application/json",
            },
            payload: HttpBodyPayload::Raw(Some(payload.into_cursor())),
        }
    }

    pub fn grpc(payload: PreEncoded) -> Self {
        let compressed = 0u8;
        let len = (payload.len() as u32).to_be_bytes();

        let header = [compressed, len[0], len[1], len[2], len[3]];

        let encoding = Encoding::of(&payload);

        HttpBody {
            content_type: match encoding {
                Encoding::Proto => "application/grpc+proto",
                Encoding::Json => "application/grpc+json",
            },
            payload: HttpBodyPayload::Grpc {
                header: Some(Cursor::new(header)),
                payload: Some(payload.into_cursor()),
            },
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
        let next = match self.get_mut().payload {
            HttpBodyPayload::Raw(ref mut payload) => payload.take().map(HttpBodyData::Payload),
            HttpBodyPayload::Grpc {
                ref mut header,
                ref mut payload,
            } => header
                .take()
                .map(HttpBodyData::GrpcHeader)
                .or_else(|| payload.take().map(HttpBodyData::Payload)),
        };

        if let Some(buf) = next {
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
            _ => false,
        }
    }

    fn size_hint(&self) -> SizeHint {
        match self.payload {
            HttpBodyPayload::Raw(ref payload) => SizeHint::with_exact(
                payload
                    .as_ref()
                    .map(|payload| payload.remaining())
                    .unwrap_or(0) as u64,
            ),
            HttpBodyPayload::Grpc {
                ref header,
                ref payload,
            } => SizeHint::with_exact(
                (header
                    .as_ref()
                    .map(|header| header.remaining())
                    .unwrap_or(0)
                    + payload
                        .as_ref()
                        .map(|payload| payload.remaining())
                        .unwrap_or(0)) as u64,
            ),
        }
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
                    Poll::Ready(Some(Err(e))) => Poll::Ready(Err(Error::new(e))),
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
