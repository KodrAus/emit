use std::{
    error, fmt,
    pin::Pin,
    sync::Mutex,
    task::{Context, Poll},
};

use bytes::Buf;
use hyper::{
    body::{Body, Frame, SizeHint},
    client::conn::{http1, http1::SendRequest},
    Method, Request, Uri,
};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;

use crate::data::{PreEncoded, PreEncodedCursor};

pub(crate) struct Error(Box<dyn error::Error + Send + Sync>);

impl Error {
    fn new(e: impl error::Error + Send + Sync + 'static) -> Self {
        Error(Box::new(e))
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.0.source()
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

pub(crate) struct HttpConnection {
    uri: Uri,
    sender: Mutex<Option<SendRequest<HttpBody>>>,
}

impl HttpConnection {
    pub fn new(url: &str) -> Result<Self, Error> {
        Ok(HttpConnection {
            uri: url.parse().map_err(Error::new)?,
            sender: Mutex::new(None),
        })
    }

    fn poison(&self) -> Option<SendRequest<HttpBody>> {
        self.sender.lock().unwrap().take()
    }

    fn unpoison(&self, sender: SendRequest<HttpBody>) {
        *self.sender.lock().unwrap() = Some(sender);
    }

    pub async fn send(&self, body: PreEncoded) -> Result<(), Error> {
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

        send_request(&mut sender, &self.uri, body).await?;

        self.unpoison(sender);

        Ok(())
    }
}

async fn send_request(
    sender: &mut SendRequest<HttpBody>,
    uri: &Uri,
    body: PreEncoded,
) -> Result<(), Error> {
    sender
        .send_request(
            Request::builder()
                .uri(uri)
                .method(Method::POST)
                .header("host", uri.authority().unwrap().as_str())
                .header(
                    "content-type",
                    match body {
                        PreEncoded::Proto(_) => "application/x-protobuf",
                    },
                )
                .body(HttpBody(Some(body.into_cursor())))
                .map_err(Error::new)?,
        )
        .await
        .map_err(Error::new)?;

    Ok(())
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
