use http::{Request, Response};
use hyper::Body;
use tower::BoxError;

pub async fn echo(req: Request<Body>) -> Result<Response<Body>, BoxError> {
    Ok(Response::new(req.into_body()))
}
