// use std::pin::Pin;
// use std::task::{Context, Poll};
// use std::time::Instant;

// use std::future::Future;

// use futures::ready;
// use http::Response;
// use metrics::{increment_counter, increment_gauge};

// use crate::body::ResponseBody;
// use crate::{AXUM_HTTP_REQUESTS_PENDING, AXUM_HTTP_REQUESTS_TOTAL};

// #[pin_project::pin_project]
// pub struct ResponseFuture<F> {
//     #[pin]
//     inner: F,
//     start: Instant,
//     method: &'static str,
//     endpoint: String,
// }

// impl<F> ResponseFuture<F> {
//     pub fn new(inner: F, method: &'static str, endpoint: String) -> Self {
//         let labels = [
//             ("method", method.to_owned()),
//             ("endpoint", endpoint.clone()),
//         ];
//         increment_counter!(AXUM_HTTP_REQUESTS_TOTAL, &labels);
//         increment_gauge!(AXUM_HTTP_REQUESTS_PENDING, 1.0, &labels);

//         Self {
//             inner,
//             start: Instant::now(),
//             method,
//             endpoint,
//         }
//     }
// }

// impl<F, B, E> Future for ResponseFuture<F>
// where
//     F: Future<Output = Result<Response<B>, E>>,
// {
//     type Output = Result<Response<ResponseBody<B>>, E>;

//     fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//         let this = self.project();
//         let res = ready!(this.inner.poll(cx))?;
//         let status_code = res.status().as_u16();
//         Poll::Ready(Ok(res.map(|b| {
//             ResponseBody::new(
//                 b,
//                 *this.start,
//                 this.method,
//                 status_code,
//                 this.endpoint.clone(),
//             )
//         })))
//     }
// }
