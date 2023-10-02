//! Request lifecycle hooks that can be used to further customize how and what callbacks to run
//! on events.
//!
//! `axum-prometheus` is built on top of lifecycle hooks. Using this module allows you to customize
//! behavior even more.
use bytes::Buf;
use http::{HeaderMap, Request, Response};
use tower_http::classify::ClassifiedResponse;

mod body;
mod future;
pub mod layer;
pub mod service;

/// Trait that defines callbacks for [`LifeCycle`] to call.
///
/// [`LifeCycle`]: service::LifeCycle
pub trait Callbacks<FailureClass>: Sized {
    /// Additional data to attach to callbacks.
    type Data;

    /// Create an instance of `Self::Data` from the request.
    ///
    /// This method is called immediately after the request is received by [`Service::call`].
    ///
    /// The value returned here will be passed to the other methods in this trait.
    ///
    /// [`Service::call`]: tower::Service::call
    fn prepare<B>(&mut self, request: &Request<B>) -> Self::Data;

    /// Perform some action when a response has been generated.
    ///
    /// This method is called when the inner [`Service`]'s response future
    /// completes with `Ok(response)`, regardless if the response is classified
    /// as a success or a failure.
    ///
    /// If the response is the start of a stream (as determined by the
    /// classifier passed to [`LifeCycle::new`] or [`LifeCycleLayer::new`]) then
    /// `classification` will be [`ClassifiedResponse::RequiresEos(())`],
    /// otherwise it will be [`ClassifiedResponse::Ready`].
    ///
    /// The default implementation does nothing and returns immediately.
    ///
    /// [`ClassifiedResponse::RequiresEos(())`]: tower_http::classify::ClassifiedResponse::RequiresEos
    /// [`Service`]: tower::Service
    /// [`LifeCycle::new`]: service::LifeCycle::new
    /// [`LifeCycleLayer::new`]: layer::LifeCycleLayer::new
    #[inline]
    fn on_response<B>(
        &mut self,
        _response: &Response<B>,
        _classification: ClassifiedResponse<FailureClass, ()>,
        _data: &mut Self::Data,
    ) {
    }

    /// Perform some action when a stream has ended.
    ///
    /// This is called when [`Body::poll_trailers`] completes with
    /// `Ok(trailers)` regardless if the trailers are classified as a failure.
    ///
    /// A stream that ends successfully will trigger two callbacks.
    /// [`on_response`] will be called once the response has been generated and
    /// the stream has started and [`on_eos`] will be called once the stream has
    /// ended.
    ///
    /// If the trailers were classified as a success then `classification` will
    /// be `Ok(())` otherwise `Err(failure_class)`.
    ///
    /// The default implementation does nothing and returns immediately.
    ///
    /// [`on_response`]: Callbacks::on_response
    /// [`on_eos`]: Callbacks::on_eos
    /// [`Body::poll_trailers`]: http_body::Body::poll_trailers
    #[inline]
    fn on_eos(
        self,
        _trailers: Option<&HeaderMap>,
        _classification: Result<(), FailureClass>,
        _data: Self::Data,
    ) {
    }

    /// Perform some action when an error has been encountered.
    ///
    /// This method is only called in the following scenarios:
    ///
    /// - The inner [`Service`]'s response future resolves to an error.
    /// - [`Body::poll_data`] returns an error.
    /// - [`Body::poll_trailers`] returns an error.
    ///
    /// That means this method is _not_ called if a response is classified as a
    /// failure (then [`on_response`] is called) or an end-of-stream is
    /// classified as a failure (then [`on_eos`] is called).
    ///
    /// `failed_at` specifies where the error happened.
    ///
    /// The default implementation does nothing and returns immediately.
    ///
    /// [`Service`]: tower::Service
    /// [`on_response`]: Callbacks::on_response
    /// [`on_eos`]: Callbacks::on_eos
    /// [`Service::call`]: tower::Service::call
    /// [`Body::poll_data`]: http_body::Body::poll_data
    /// [`Body::poll_trailers`]: http_body::Body::poll_trailers
    fn on_failure(
        self,
        _failed_at: FailedAt,
        _failure_classification: FailureClass,
        _data: &mut Self::Data,
    ) {
    }
}

/// A trait that allows to hook into [`http_body::Body::poll_data`]'s lifecycle.
pub trait OnBodyChunk<B: Buf> {
    type Data;

    /// Perform some action when a response body chunk has been generated.
    ///
    /// This is called when [`Body::poll_data`] completes with `Some(Ok(chunk))`
    /// regardless if the chunk is empty or not.
    ///
    /// The default implementation does nothing and returns immediately.
    ///
    /// [`Body::poll_data`]: http_body::Body::poll_data
    #[inline]
    fn call(&mut self, _body: &B, _exact_body_size: Option<u64>, _data: &mut Self::Data) {}
}

/// A trait that allows to hook into [`http_body::Body::poll_data`]'s lifecycle.
pub trait OnExactBodySize {
    type Data;

    /// Perform some action when a response body's exact size is known ahead of time (that is,
    /// [`http_body::SizeHint::exact`] returns `Some(size)`).
    ///
    /// This is called when [`Body::poll_data`] completes with `Some(_)`
    /// regardless if the inner chunk is errored or not. It's called before `OnBodyChunk::call`.
    ///
    /// The default implementation does nothing and returns immediately.
    #[inline]
    fn call(&mut self, _size: u64, _data: &mut Self::Data) {}
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
