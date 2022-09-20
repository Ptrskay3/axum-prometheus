use tower::Layer;

use super::service::LifeCycle;

/// [`Layer`] for adding callbacks to the lifecycle of request.
///
/// See the [module docs](crate::lifecycle) for more details.
///
/// [`Layer`]: tower::Layer
#[derive(Debug, Clone)]
pub struct LifeCycleLayer<MC, Callbacks> {
    pub(super) make_classifier: MC,
    pub(super) callbacks: Callbacks,
}

impl<MC, Callbacks> LifeCycleLayer<MC, Callbacks> {
    /// Create a new `LifeCycleLayer`.
    pub fn new(make_classifier: MC, callbacks: Callbacks) -> Self {
        LifeCycleLayer {
            make_classifier,
            callbacks,
        }
    }
}

impl<S, MC, Callbacks> Layer<S> for LifeCycleLayer<MC, Callbacks>
where
    MC: Clone,
    Callbacks: Clone,
{
    type Service = LifeCycle<S, MC, Callbacks>;

    fn layer(&self, inner: S) -> Self::Service {
        LifeCycle {
            inner,
            make_classifier: self.make_classifier.clone(),
            callbacks: self.callbacks.clone(),
        }
    }
}
