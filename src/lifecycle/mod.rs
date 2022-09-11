use bytes::Buf;
use http::{HeaderMap, Request, Response};
use tower_http::classify::ClassifiedResponse;

mod body;
mod future;
pub mod layer;
mod service;

pub trait Callbacks<FailureClass>: Sized {
    type Data;

    fn prepare<B>(&mut self, request: &Request<B>) -> Self::Data;

    #[inline]
    fn on_response<B>(
        &mut self,
        _response: &Response<B>,
        _classification: ClassifiedResponse<FailureClass, ()>,
        _data: &mut Self::Data,
    ) {
    }

    #[inline]
    fn on_body_chunk<B: Buf>(&self, _check: &B, _data: &Self::Data) {}

    #[inline]
    fn on_eos(
        self,
        _trailers: Option<&HeaderMap>,
        _classification: Result<(), FailureClass>,
        _data: Self::Data,
    ) {
    }

    fn on_failure(
        self,
        _failed_at: FailedAt,
        _failure_classification: FailureClass,
        _data: Self::Data,
    ) {
    }
}

/// Enum used to specify where an error was encountered.
#[derive(Debug)]
pub enum FailedAt {
    /// Generating the response failed.
    Response,
    /// Generating the response body failed.
    Body,
    /// Generating the response trailers failed.
    Trailers,
}
