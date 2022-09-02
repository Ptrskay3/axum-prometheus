#![allow(clippy::must_use_candidate)]

use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use tower::Layer;

use crate::{
    service::PrometheusMetricService, AXUM_HTTP_REQUEST_DURATION_SECONDS, SECONDS_DURATION_BUCKETS,
};

/// The tower middleware layer for recording http metrics with Prometheus.
#[derive(Clone)]
pub struct PrometheusMetricLayer {
    pub metric: Metric,
}

impl PrometheusMetricLayer {
    /// Construct a new [`PrometheusMetricLayer`] from a [`Metric`].
    pub fn new(metric: Metric) -> Self {
        Self { metric }
    }

    /// Construct the default [`PrometheusMetricLayer`] with `SECONDS_DURATION_BUCKETS` for
    /// HTTP latency histogram. This will call [`metrics_exporter_prometheus::PrometheusBuilder::install_recorder`].
    pub fn pair() -> (Self, Metric) {
        let handle = PrometheusBuilder::new()
            .set_buckets_for_metric(
                Matcher::Full(AXUM_HTTP_REQUEST_DURATION_SECONDS.to_string()),
                SECONDS_DURATION_BUCKETS,
            )
            .unwrap()
            .install_recorder()
            .unwrap();
        let metric = Metric::new(handle);
        let layer = Self::new(metric.clone());
        (layer, metric)
    }

    /// Build a custom Prometheus setup by calling the passed in closure.
    ///
    ///  __Make sure to call [`metrics_exporter_prometheus::PrometheusBuilder::install_recorder`].__
    ///
    /// # Example
    /// ```
    /// use axum::{routing::get, Router};
    /// use axum_prometheus::{AXUM_HTTP_REQUEST_DURATION_SECONDS, PrometheusMetricLayer};
    /// use metrics_exporter_prometheus::{Matcher, PrometheusBuilder};
    /// use std::net::SocketAddr;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let (metric_layer, metric_handle) = PrometheusMetricLayer::pair_from_fn(|| {
    ///         PrometheusBuilder::new()
    ///             .set_buckets_for_metric(
    ///                 Matcher::Full(AXUM_HTTP_REQUEST_DURATION_SECONDS.to_string()),
    ///                 &[0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0],
    ///             )
    ///             .unwrap()
    ///             .install_recorder()
    ///             .unwrap()
    ///     });
    ///
    ///     let app = Router::new()
    ///       .route("/fast", get(|| async {}))
    ///       .route(
    ///           "/slow",
    ///           get(|| async {
    ///               tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    ///           }),
    ///       )
    ///       .route("/metrics", get(|| async move { metric_handle.render() }))
    ///       .layer(metric_layer);
    ///
    ///    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    ///    let server = axum::Server::bind(&addr)
    ///        .serve(app.into_make_service());
    ///    // and to actually run the server:
    ///    // server.await.unwrap();
    /// }
    /// ```

    pub fn pair_from_fn<F>(f: F) -> (Self, Metric)
    where
        F: FnOnce() -> PrometheusHandle,
    {
        let metric = Metric::from_fn(f);
        let layer = Self::new(metric.clone());
        (layer, metric)
    }
}

impl<S> Layer<S> for PrometheusMetricLayer {
    type Service = PrometheusMetricService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        PrometheusMetricService { inner }
    }
}

/// A wrapper over [`metrics_exporter_prometheus::PrometheusHandle`].
#[derive(Clone)]
pub struct Metric {
    handle: PrometheusHandle,
}

impl Metric {
    pub fn new(handle: PrometheusHandle) -> Self {
        Self { handle }
    }

    pub fn from_fn<F>(f: F) -> Self
    where
        F: FnOnce() -> PrometheusHandle,
    {
        Self { handle: f() }
    }

    pub fn render(&self) -> String {
        self.handle.render()
    }
}
