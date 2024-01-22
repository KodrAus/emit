use std::{
    future::Future,
    pin::Pin,
    sync::Mutex,
    task::{self, Context, Poll},
};

use bytes::Buf;
use hyper::{
    body::{self, Body, Frame, SizeHint},
    client::conn::{http1, http1::SendRequest},
    Method, Request, Uri,
};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;

use crate::{
    data::{PreEncoded, PreEncodedCursor},
    Error,
};

pub(crate) struct HttpConnection {
    uri: Uri,
    headers: Vec<(String, String)>,
    sender: Mutex<Option<SendRequest<HttpBody>>>,
}

pub(crate) struct HttpResponse {
    res: hyper::Response<body::Incoming>,
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

impl HttpConnection {
    pub fn new(
        url: impl AsRef<str>,
        headers: impl Into<Vec<(String, String)>>,
    ) -> Result<Self, Error> {
        Ok(HttpConnection {
            uri: url.as_ref().parse().map_err(Error::new)?,
            headers: headers.into(),
            sender: Mutex::new(None),
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
            None => {
                let io = TokioIo::new(
                    TcpStream::connect((
                        self.uri.host().unwrap(),
                        self.uri.port_u16().unwrap_or(80),
                    ))
                    .await
                    .map_err(Error::new)?,
                );

                let (sender, conn) = http1::handshake(io).await.map_err(Error::new)?;

                tokio::task::spawn(async move {
                    let _ = conn.await;
                });

                sender
            }
        };

        let res = send_request(
            &mut sender,
            &self.uri,
            self.headers.iter().map(|(k, v)| (&**k, &**v)),
            body,
        )
        .await?;

        self.unpoison(sender);

        Ok(HttpResponse { res })
    }
}

async fn send_request(
    sender: &mut SendRequest<HttpBody>,
    uri: &Uri,
    headers: impl Iterator<Item = (&str, &str)>,
    body: PreEncoded,
) -> Result<hyper::Response<body::Incoming>, Error> {
    let rt = emit::runtime::internal();

    let res = sender
        .send_request({
            use emit::{Ctxt as _, Props as _};

            let mut req = Request::builder()
                .uri(uri)
                .method(Method::POST)
                .header("host", uri.authority().unwrap().as_str())
                .header(
                    "content-type",
                    match body {
                        PreEncoded::Proto(_) => "application/x-protobuf",
                    },
                );

            for (k, v) in headers {
                req = req.header(k, v);
            }

            // Propagate traceparent for the batch
            let mut trace_id = None;
            let mut span_id = None;

            rt.ctxt().with_current(|props| {
                trace_id = props.pull::<emit::TraceId>();
                span_id = props.pull::<emit::SpanId>();
            });

            req = if let (Some(trace_id), Some(span_id)) = (trace_id, span_id) {
                req.header("traceparent", format!("00-{trace_id}-{span_id}-00"))
            } else {
                req
            };

            req.body(HttpBody(Some(body.into_cursor())))
                .map_err(Error::new)?
        })
        .await
        .map_err(Error::new)?;

    Ok(res)
}

pub(crate) struct HttpBody(Option<PreEncodedCursor>);

impl Body for HttpBody {
    type Data = PreEncodedCursor;

    type Error = std::convert::Infallible;

    fn poll_frame(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        if let Some(buf) = self.get_mut().0.take() {
            Poll::Ready(Some(Ok(Frame::data(buf))))
        } else {
            Poll::Ready(None)
        }
    }

    fn is_end_stream(&self) -> bool {
        self.0.is_none()
    }

    fn size_hint(&self) -> SizeHint {
        if let Some(ref buf) = self.0 {
            SizeHint::with_exact(buf.remaining() as u64)
        } else {
            SizeHint::with_exact(0)
        }
    }
}
