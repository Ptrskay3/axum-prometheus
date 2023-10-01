use tower::Layer;

use super::service::LifeCycle;

/// [`Layer`] for adding callbacks to the lifecycle of request.
///
/// See the [module docs](crate::lifecycle) for more details.
///
/// [`Layer`]: tower::Layer
#[derive(Debug, Clone)]
pub struct LifeCycleLayer<MC, Callbacks, OnBodyChunk, OnExactBodySize> {
    pub(super) make_classifier: MC,
    pub(super) callbacks: Callbacks,
    pub(super) on_body_chunk: OnBodyChunk,
    pub(super) on_exact_body_size: OnExactBodySize,
}

impl<MC, Callbacks, OnBodyChunk, OnExactBodySize>
    LifeCycleLayer<MC, Callbacks, OnBodyChunk, OnExactBodySize>
{
    /// Create a new `LifeCycleLayer`.
    pub fn new(
        make_classifier: MC,
        callbacks: Callbacks,
        on_body_chunk: OnBodyChunk,
        on_exact_body_size: OnExactBodySize,
    ) -> Self {
        LifeCycleLayer {
            make_classifier,
            callbacks,
            on_body_chunk,
            on_exact_body_size,
        }
    }

    pub(crate) fn on_body_chunk(&mut self, on_body_chunk: OnBodyChunk) {
        self.on_body_chunk = on_body_chunk;
    }
    pub(crate) fn on_exact_body_size(&mut self, on_exact_body_size: OnExactBodySize) {
        self.on_exact_body_size = on_exact_body_size;
    }
}

impl<S, MC, Callbacks, OnBodyChunk, OnExactBodySize> Layer<S>
    for LifeCycleLayer<MC, Callbacks, OnBodyChunk, OnExactBodySize>
where
    MC: Clone,
    Callbacks: Clone,
    OnBodyChunk: Clone,
    OnExactBodySize: Clone,
{
    type Service = LifeCycle<S, MC, Callbacks, OnBodyChunk, OnExactBodySize>;

    fn layer(&self, inner: S) -> Self::Service {
        LifeCycle {
            inner,
            make_classifier: self.make_classifier.clone(),
            callbacks: self.callbacks.clone(),
            on_body_chunk: self.on_body_chunk.clone(),
            on_exact_body_size: self.on_exact_body_size.clone(),
        }
    }
}
