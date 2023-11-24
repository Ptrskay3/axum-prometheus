use http::{Request, Response};
use http_body_util::Full;
use tower::BoxError;

pub async fn echo(req: Request<Full<()>>) -> Result<Response<Full<()>>, BoxError> {
    Ok(Response::new(req.into_body()))
}
