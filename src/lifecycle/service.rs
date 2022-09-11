use std::task::{Context, Poll};

use http::{Request, Response};
use http_body::Body;
use tower::Service;
use tower_http::classify::MakeClassifier;

use super::{body::ResponseBody, future::ResponseFuture, layer::LifeCycleLayer, Callbacks};

#[derive(Clone, Debug)]
pub struct LifeCycle<S, MC, Callbacks> {
    pub(super) inner: S,
    pub(super) make_classifier: MC,
    pub(super) callbacks: Callbacks,
}

impl<S, MC, Callbacks> LifeCycle<S, MC, Callbacks> {
    pub fn new(inner: S, make_classifier: MC, callbacks: Callbacks) -> Self {
        Self {
            inner,
            make_classifier,
            callbacks,
        }
    }

    pub fn layer(make_classifier: MC, callbacks: Callbacks) -> LifeCycleLayer<MC, Callbacks> {
        LifeCycleLayer::new(make_classifier, callbacks)
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

impl<S, MC, ReqBody, ResBody, CallbacksT> Service<Request<ReqBody>> for LifeCycle<S, MC, CallbacksT>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
    ResBody: Body,
    MC: MakeClassifier,
    CallbacksT: Callbacks<MC::FailureClass> + Clone,
    S::Error: std::fmt::Display + 'static,
{
    type Response = Response<ResponseBody<ResBody, MC::ClassifyEos, CallbacksT, CallbacksT::Data>>;
    type Error = S::Error;
    type Future = ResponseFuture<S::Future, MC::Classifier, CallbacksT, CallbacksT::Data>;

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
        }
    }
}
