#![allow(clippy::module_name_repetitions, clippy::unreadable_literal)]

pub mod lifecycle;
mod utils;

/// Identifies the gauge used for the requests pending metric.
pub const AXUM_HTTP_REQUESTS_PENDING: &str = "axum_http_requests_pending";

/// Identifies the histogram/summary used for request latency.
pub const AXUM_HTTP_REQUEST_DURATION_SECONDS: &str = "axum_http_request_duration_seconds";

/// Identifies the counter used for requests total.
pub const AXUM_HTTP_REQUESTS_TOTAL: &str = "axum_http_requests_total";

/// Identifies the counter used for total requests failed.
pub const AXUM_HTTP_REQUESTS_FAILED: &str = "axum_http_requests_failed";

use std::time::Instant;

use lifecycle::layer::LifeCycleLayer;
use lifecycle::{service::LifeCycle, Callbacks};
use metrics::{decrement_gauge, histogram, increment_counter, increment_gauge};
use tower::Layer;
use tower_http::classify::{ClassifiedResponse, SharedClassifier, StatusInRangeAsFailures};
pub use utils::SECONDS_DURATION_BUCKETS;

use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};

pub use metrics;
pub use metrics_exporter_prometheus;
use utils::as_label;

/// A marker struct that implements the [`axum_prometheus::lifecycle::Callback`] trait.
#[derive(Clone, Default)]
pub struct Traffic;

impl Traffic {
    pub fn new() -> Self {
        Self
    }
}

/// The data that's used for storing and calculating information about the current request.
#[derive(Debug, Clone)]
pub struct MetricsData {
    pub endpoint: String,
    pub start: Instant,
    pub method: &'static str,
}

impl<FailureClass> Callbacks<FailureClass> for Traffic {
    type Data = MetricsData;

    fn prepare<B>(&mut self, request: &http::Request<B>) -> Self::Data {
        let now = std::time::Instant::now();
        let endpoint = request.uri().path().to_owned();
        let method = as_label(request.method());

        let labels = [
            ("method", method.to_owned()),
            ("endpoint", endpoint.clone()),
        ];
        increment_counter!(AXUM_HTTP_REQUESTS_TOTAL, &labels);
        increment_gauge!(AXUM_HTTP_REQUESTS_PENDING, 1.0, &labels);

        MetricsData {
            endpoint,
            start: now,
            method,
        }
    }

    fn on_response<B>(
        &mut self,
        res: &http::Response<B>,
        cls: ClassifiedResponse<FailureClass, ()>,
        data: &mut Self::Data,
    ) {
        let success = matches!(cls, ClassifiedResponse::Ready(Ok(_)));

        let duration_seconds = data.start.elapsed().as_secs_f64();

        decrement_gauge!(
            AXUM_HTTP_REQUESTS_PENDING,
            1.0,
            &[
                ("method", data.method.to_string()),
                ("endpoint", data.endpoint.to_string())
            ]
        );
        histogram!(
            AXUM_HTTP_REQUEST_DURATION_SECONDS,
            duration_seconds,
            &[
                ("method", data.method.to_string()),
                ("status", res.status().as_u16().to_string()),
                ("success", success.to_string()),
                ("endpoint", data.endpoint.to_string()),
            ]
        );
    }

    fn on_failure(
        self,
        _failed_at: lifecycle::FailedAt,
        _failure_classification: FailureClass,
        data: Self::Data,
    ) {
        let labels = [
            ("method", data.method.to_owned()),
            ("success", "false".into()),
            ("endpoint", data.endpoint),
        ];
        decrement_gauge!(AXUM_HTTP_REQUESTS_PENDING, 1.0, &labels);
        increment_counter!(AXUM_HTTP_REQUESTS_FAILED, &labels);
    }
}

/// The tower middleware layer for recording http metrics with Prometheus.
#[derive(Clone)]
pub struct PrometheusMetricLayer {
    pub(crate) inner_layer: LifeCycleLayer<SharedClassifier<StatusInRangeAsFailures>, Traffic>,
}

impl PrometheusMetricLayer {
    /// Create a new tower middleware that can be used to track metrics with Prometheus.
    ///
    /// By default, this __will not__ "install" the exporter which sets it as the
    /// global recorder for all `metrics` calls. Instead, here you can use the [`metrics_exporter_prometheus::PrometheusBuilder`]
    /// to build your own customized metrics exporter.
    ///
    /// This middleware is using the following constants for identifying different HTTP metrics:
    ///
    /// - [`AXUM_HTTP_REQUESTS_PENDING`]
    /// - [`AXUM_HTTP_REQUESTS_FAILED`]
    /// - [`AXUM_HTTP_REQUESTS_TOTAL`]
    /// - [`AXUM_HTTP_REQUEST_DURATION_SECONDS`].
    ///
    /// In terms of setup, the most important one is [`AXUM_HTTP_REQUEST_DURATION_SECONDS`], which is a histogram metric
    /// used for request latency. You may set customized buckets tailored for your used case here.
    ///
    /// # Example
    /// ```
    /// use axum::{routing::get, Router};
    /// use axum_prometheus::{AXUM_HTTP_REQUEST_DURATION_SECONDS, SECONDS_DURATION_BUCKETS
    /// , PrometheusMetricLayer};
    /// use metrics_exporter_prometheus::{Matcher, PrometheusBuilder};
    /// use std::net::SocketAddr;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let metric_layer = PrometheusMetricLayer::new();
    ///     // This is the default if you use `PrometheusMetricLayer::pair`.
    ///     let metric_handle = PrometheusBuilder::new()
    ///        .set_buckets_for_metric(
    ///            Matcher::Full(AXUM_HTTP_REQUEST_DURATION_SECONDS.to_string()),
    ///            SECONDS_DURATION_BUCKETS,
    ///        )
    ///        .unwrap()
    ///        .install_recorder()
    ///        .unwrap();
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
    pub fn new() -> Self {
        let make_classifier =
            StatusInRangeAsFailures::new_for_client_and_server_errors().into_make_classifier();
        let inner_layer = LifeCycleLayer::new(make_classifier, Traffic::new());
        Self { inner_layer }
    }

    pub fn pair() -> (Self, PrometheusHandle) {
        let handle = PrometheusBuilder::new()
            .set_buckets_for_metric(
                Matcher::Full(AXUM_HTTP_REQUEST_DURATION_SECONDS.to_string()),
                SECONDS_DURATION_BUCKETS,
            )
            .unwrap()
            .install_recorder()
            .unwrap();

        (Self::new(), handle)
    }
}

impl Default for PrometheusMetricLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Layer<S> for PrometheusMetricLayer {
    type Service = LifeCycle<S, SharedClassifier<StatusInRangeAsFailures>, Traffic>;

    fn layer(&self, inner: S) -> Self::Service {
        self.inner_layer.layer(inner)
    }
}
