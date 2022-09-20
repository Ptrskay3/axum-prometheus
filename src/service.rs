// use std::task::{Context, Poll};

// use axum::extract::MatchedPath;
// use http::{Request, Response};
// use tower::Service;

// use crate::{body::ResponseBody, future::ResponseFuture, utils::as_label};

// #[derive(Clone)]
// pub struct PrometheusMetricService<S> {
//     pub inner: S,
// }

// impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for PrometheusMetricService<S>
// where
//     S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone,
//     ResBody: Send,
//     ReqBody: Send,
// {
//     type Response = Response<ResponseBody<ResBody>>;
//     type Error = S::Error;
//     type Future = ResponseFuture<S::Future>;

//     #[inline]
//     fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         self.inner.poll_ready(cx)
//     }

//     fn call(&mut self, request: Request<ReqBody>) -> Self::Future {
//         let method = as_label(request.method());
//         let path = if let Some(matched_path) = request.extensions().get::<MatchedPath>() {
//             matched_path.as_str().to_owned()
//         } else {
//             request.uri().path().to_owned()
//         };
//         ResponseFuture::new(self.inner.call(request), method, path)
//     }
// }
