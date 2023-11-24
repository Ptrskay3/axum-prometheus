use bytes::Bytes;
use http::{Request, Response};
use http_body_util::BodyExt;
use tower::BoxError;

pub async fn echo(req: Request<BoxBody>) -> Result<Response<BoxBody>, BoxError> {
    Ok(Response::new(req.into_body()))
}

pub type BoxBody = http_body_util::combinators::UnsyncBoxBody<Bytes, BoxError>;

#[derive(Debug)]
pub struct Body(BoxBody);

impl Body {
    pub(crate) fn new<B>(body: B) -> Self
    where
        B: http_body::Body<Data = Bytes> + Send + 'static,
        B::Error: Into<BoxError>,
    {
        Self(body.map_err(Into::into).boxed_unsync())
    }

    pub(crate) fn empty() -> Self {
        Self::new(http_body_util::Empty::new())
    }
}

impl Default for Body {
    fn default() -> Self {
        Self::empty()
    }
}
