use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Instant,
};

use http::HeaderMap;
use http_body::Body;
use metrics::{decrement_gauge, histogram};
use pin_project::{pin_project, pinned_drop};

use crate::{AXUM_HTTP_REQUESTS_PENDING, AXUM_HTTP_REQUEST_DURATION_SECONDS};

#[pin_project(PinnedDrop)]
pub struct ResponseBody<B> {
    #[pin]
    inner: B,
    start: Instant,
    method: &'static str,
    status: u16,
    endpoint: String,
}

impl<B> ResponseBody<B> {
    pub(crate) fn new(
        inner: B,
        start: Instant,
        method: &'static str,
        status: u16,
        endpoint: String,
    ) -> Self {
        Self {
            inner,
            start,
            method,
            status,
            endpoint,
        }
    }
}

impl<B: Body> Body for ResponseBody<B> {
    type Data = B::Data;
    type Error = B::Error;

    #[inline]
    fn poll_data(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Data, Self::Error>>> {
        self.project().inner.poll_data(cx)
    }

    #[inline]
    fn poll_trailers(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Option<HeaderMap>, Self::Error>> {
        self.project().inner.poll_trailers(cx)
    }

    #[inline]
    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    #[inline]
    fn size_hint(&self) -> http_body::SizeHint {
        self.inner.size_hint()
    }
}

#[pinned_drop]
impl<B> PinnedDrop for ResponseBody<B> {
    fn drop(self: Pin<&mut Self>) {
        let duration_seconds = self.start.elapsed().as_secs_f64();

        decrement_gauge!(
            AXUM_HTTP_REQUESTS_PENDING,
            1.0,
            &[
                ("method", self.method.to_string()),
                ("endpoint", self.endpoint.to_string())
            ]
        );
        histogram!(
            AXUM_HTTP_REQUEST_DURATION_SECONDS,
            duration_seconds,
            &[
                ("method", self.method.to_string()),
                ("status", self.status.to_string()),
                ("endpoint", self.endpoint.to_string()),
            ]
        );
    }
}
