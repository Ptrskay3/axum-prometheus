use tower::Layer;

use super::service::LifeCycle;

#[derive(Debug, Clone)]
pub struct LifeCycleLayer<MC, Callbacks> {
    make_classifier: MC,
    callbacks: Callbacks,
}

impl<MC, Callbacks> LifeCycleLayer<MC, Callbacks> {
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
