use std::task::{Context, Poll};

use http::{Request, Response};
use http_body::Body;
use tower::Service;
use tower_http::classify::MakeClassifier;

use super::{
    body::ResponseBody, future::ResponseFuture, layer::LifeCycleLayer, Callbacks, OnBodyChunk,
    OnExactBodySize,
};

#[derive(Clone, Debug)]
pub struct LifeCycle<S, MC, Callbacks, OnBodyChunk, OnExactBodySize> {
    pub(super) inner: S,
    pub(super) make_classifier: MC,
    pub(super) callbacks: Callbacks,
    pub(super) on_body_chunk: OnBodyChunk,
    pub(super) on_exact_body_size: OnExactBodySize,
}

impl<S, MC, Callbacks, OnBodyChunk, OnExactBodySize>
    LifeCycle<S, MC, Callbacks, OnBodyChunk, OnExactBodySize>
{
    pub fn new(
        inner: S,
        make_classifier: MC,
        callbacks: Callbacks,
        on_body_chunk: OnBodyChunk,
        on_exact_body_size: OnExactBodySize,
    ) -> Self {
        Self {
            inner,
            make_classifier,
            callbacks,
            on_body_chunk,
            on_exact_body_size,
        }
    }

    pub fn layer(
        make_classifier: MC,
        callbacks: Callbacks,
        on_body_chunk: OnBodyChunk,
        on_exact_body_size: OnExactBodySize,
    ) -> LifeCycleLayer<MC, Callbacks, OnBodyChunk, OnExactBodySize> {
        LifeCycleLayer::new(
            make_classifier,
            callbacks,
            on_body_chunk,
            on_exact_body_size,
        )
    }

    /// Gets a reference to the underlying service.
    pub fn get_ref(&self) -> &S {
        &self.inner
    }

    /// Gets a mutable reference to the underlying service.
    pub fn get_mut(&mut self) -> &mut S {
        &mut self.inner
    }

    /// Consumes `self`, returning the underlying service.
    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S, MC, ReqBody, ResBody, CallbacksT, OnBodyChunkT, OnExactBodySizeT> Service<Request<ReqBody>>
    for LifeCycle<S, MC, CallbacksT, OnBodyChunkT, OnExactBodySizeT>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
    ResBody: Body,
    MC: MakeClassifier,
    CallbacksT: Callbacks<MC::FailureClass> + Clone,
    S::Error: std::fmt::Display + 'static,
    OnBodyChunkT: OnBodyChunk<ResBody::Data, Data = CallbacksT::Data> + Clone,
    OnExactBodySizeT: OnExactBodySize<Data = CallbacksT::Data> + Clone,
    CallbacksT::Data: Clone,
{
    type Response = Response<
        ResponseBody<
            ResBody,
            MC::ClassifyEos,
            CallbacksT,
            OnBodyChunkT,
            OnExactBodySizeT,
            CallbacksT::Data,
        >,
    >;
    type Error = S::Error;
    type Future = ResponseFuture<
        S::Future,
        MC::Classifier,
        CallbacksT,
        OnBodyChunkT,
        OnExactBodySizeT,
        CallbacksT::Data,
    >;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let callbacks_data = self.callbacks.prepare(&req);

        let classifier = self.make_classifier.make_classifier(&req);

        ResponseFuture {
            inner: self.inner.call(req),
            classifier: Some(classifier),
            callbacks: Some(self.callbacks.clone()),
            callbacks_data: Some(callbacks_data),
            on_body_chunk: Some(self.on_body_chunk.clone()),
            on_exact_body_size: Some(self.on_exact_body_size.clone()),
        }
    }
}
