use tower::Layer;

use super::service::LifeCycle;

/// [`Layer`] for adding callbacks to the lifecycle of request.
///
/// See the [module docs](crate::lifecycle) for more details.
///
/// [`Layer`]: tower::Layer
#[derive(Debug, Clone)]
pub struct LifeCycleLayer<MC, Callbacks, OnBodyChunk> {
    pub(super) make_classifier: MC,
    pub(super) callbacks: Callbacks,
    pub(super) on_body_chunk: OnBodyChunk,
}

impl<MC, Callbacks, OnBodyChunk> LifeCycleLayer<MC, Callbacks, OnBodyChunk> {
    /// Create a new `LifeCycleLayer`.
    pub fn new(make_classifier: MC, callbacks: Callbacks, on_body_chunk: OnBodyChunk) -> Self {
        LifeCycleLayer {
            make_classifier,
            callbacks,
            on_body_chunk,
        }
    }
}

impl<S, MC, Callbacks, OnBodyChunk> Layer<S> for LifeCycleLayer<MC, Callbacks, OnBodyChunk>
where
    MC: Clone,
    Callbacks: Clone,
    OnBodyChunk: Clone,
{
    type Service = LifeCycle<S, MC, Callbacks, OnBodyChunk>;

    fn layer(&self, inner: S) -> Self::Service {
        LifeCycle {
            inner,
            make_classifier: self.make_classifier.clone(),
            callbacks: self.callbacks.clone(),
            on_body_chunk: self.on_body_chunk.clone(),
        }
    }
}
