//! A Prometheus middleware to collect HTTP metrics for Axum applications.
//!
//! `axum-prometheus` relies on [`metrics_exporter_prometheus`] as a backed to interact with Prometheus.
//!
//! ## Metrics
//!
//! By default three HTTP metrics are tracked
//! - `axum_http_requests_total` (labels: endpoint, method): the total number of HTTP requests handled (counter)
//! - `axum_http_requests_duration_seconds` (labels: endpoint, method, status): the request duration for all HTTP requests handled (histogram)
//! - `axum_http_requests_pending` (labels: endpoint, method): the number of currently in-flight requests (gauge)
//!
//! Note that in the future request size metric is also planned to be implemented.
//!
//! ## Usage
//!
//! Add `axum-prometheus` to your `Cargo.toml`.
//! ```not_rust
//! [dependencies]
//! axum-prometheus = "0.1.0"
//! ```
//!
//! Then you instantiate the prometheus middleware:
//! ```rust,no_run
//! use std::{net::SocketAddr, time::Duration};
//! use axum::{routing::get, Router};
//! use axum_prometheus::PrometheusMetricLayer;
//!
//! #[tokio::main]
//! async fn main() {
//!     let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();
//!     let app = Router::new()
//!         .route("/fast", get(|| async {}))
//!         .route(
//!             "/slow",
//!             get(|| async {
//!                 tokio::time::sleep(Duration::from_secs(1)).await;
//!             }),
//!         )
//!         .route("/metrics", get(|| async move { metric_handle.render() }))
//!         .layer(prometheus_layer);
//!
//!     let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
//!     axum::Server::bind(&addr)
//!         .serve(app.into_make_service())
//!         .await
//!         .unwrap();
//! }
//! ```
//!
//! Note that the `/metrics` endpoint is not automatically exposed, so you need to add that as a route manually.
//! Calling the `/metrics` endpoint will expose your metrics:
//! ```not_rust
//! axum_http_requests_total{method="GET",endpoint="/metrics"} 5
//! axum_http_requests_pending{method="GET",endpoint="/metrics"} 1
//! axum_http_requests_duration_seconds_bucket{method="GET",status="200",endpoint="/metrics",le="0.005"} 4
//! axum_http_requests_duration_seconds_bucket{method="GET",status="200",endpoint="/metrics",le="0.01"} 4
//! axum_http_requests_duration_seconds_bucket{method="GET",status="200",endpoint="/metrics",le="0.025"} 4
//! axum_http_requests_duration_seconds_bucket{method="GET",status="200",endpoint="/metrics",le="0.05"} 4
//! axum_http_requests_duration_seconds_bucket{method="GET",status="200",endpoint="/metrics",le="0.1"} 4
//! axum_http_requests_duration_seconds_bucket{method="GET",status="200",endpoint="/metrics",le="0.25"} 4
//! axum_http_requests_duration_seconds_bucket{method="GET",status="200",endpoint="/metrics",le="0.5"} 4
//! axum_http_requests_duration_seconds_bucket{method="GET",status="200",endpoint="/metrics",le="1"} 4
//! axum_http_requests_duration_seconds_bucket{method="GET",status="200",endpoint="/metrics",le="2.5"} 4
//! axum_http_requests_duration_seconds_bucket{method="GET",status="200",endpoint="/metrics",le="5"} 4
//! axum_http_requests_duration_seconds_bucket{method="GET",status="200",endpoint="/metrics",le="10"} 4
//! axum_http_requests_duration_seconds_bucket{method="GET",status="200",endpoint="/metrics",le="+Inf"} 4
//! axum_http_requests_duration_seconds_sum{method="GET",status="200",endpoint="/metrics"} 0.001997171
//! axum_http_requests_duration_seconds_count{method="GET",status="200",endpoint="/metrics"} 4
//! ```
//!
//! This crate is similar to (and takes inspiration from) [`actix-web-prom`](https://github.com/nlopes/actix-web-prom) and [`rocket_prometheus`](https://github.com/sd2k/rocket_prometheus),
//! and also builds on top of davidpdrsn's [earlier work with LifeCycleHooks](https://github.com/tower-rs/tower-http/pull/96) in `tower-http`.

#![allow(clippy::module_name_repetitions, clippy::unreadable_literal)]

/// Identifies the gauge used for the requests pending metric.
pub const AXUM_HTTP_REQUESTS_PENDING: &str = "axum_http_requests_pending";

/// Identifies the histogram/summary used for request latency.
pub const AXUM_HTTP_REQUESTS_DURATION_SECONDS: &str = "axum_http_requests_duration_seconds";

/// Identifies the counter used for requests total.
pub const AXUM_HTTP_REQUESTS_TOTAL: &str = "axum_http_requests_total";

use std::time::Instant;

pub mod lifecycle;
mod utils;

use lifecycle::layer::LifeCycleLayer;
use lifecycle::{service::LifeCycle, Callbacks};
use metrics::{decrement_gauge, histogram, increment_counter, increment_gauge};
use tower::Layer;
use tower_http::classify::{ClassifiedResponse, SharedClassifier, StatusInRangeAsFailures};
pub use utils::SECONDS_DURATION_BUCKETS;

use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};

pub use metrics;
pub use metrics_exporter_prometheus;

/// A marker struct that implements the [`lifecycle::Callbacks`] trait.
#[derive(Clone, Default)]
pub struct Traffic;

impl Traffic {
    pub fn new() -> Self {
        Self
    }
}

/// Struct used for storing and calculating information about the current request.
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
        let method = utils::as_label(request.method());

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
        _cls: ClassifiedResponse<FailureClass, ()>,
        data: &mut Self::Data,
    ) {
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
            AXUM_HTTP_REQUESTS_DURATION_SECONDS,
            duration_seconds,
            &[
                ("method", data.method.to_string()),
                ("status", res.status().as_u16().to_string()),
                ("endpoint", data.endpoint.to_string()),
            ]
        );
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
    /// - [`AXUM_HTTP_REQUESTS_TOTAL`]
    /// - [`AXUM_HTTP_REQUESTS_DURATION_SECONDS`].
    ///
    /// In terms of setup, the most important one is [`AXUM_HTTP_REQUESTS_DURATION_SECONDS`], which is a histogram metric
    /// used for request latency. You may set customized buckets tailored for your used case here.
    ///
    /// # Example
    /// ```
    /// use axum::{routing::get, Router};
    /// use axum_prometheus::{AXUM_HTTP_REQUESTS_DURATION_SECONDS, SECONDS_DURATION_BUCKETS, PrometheusMetricLayer};
    /// use metrics_exporter_prometheus::{Matcher, PrometheusBuilder};
    /// use std::net::SocketAddr;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let metric_layer = PrometheusMetricLayer::new();
    ///     // This is the default if you use `PrometheusMetricLayer::pair`.
    ///     let metric_handle = PrometheusBuilder::new()
    ///        .set_buckets_for_metric(
    ///            Matcher::Full(AXUM_HTTP_REQUESTS_DURATION_SECONDS.to_string()),
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

    /// Crate a new tower middleware and a default global Prometheus exporter with sensible defaults.
    ///
    /// # Example
    /// ```
    /// use axum::{routing::get, Router};
    /// use axum_prometheus::{PrometheusMetricLayer};
    /// use std::net::SocketAddr;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let (metric_layer, metric_handle) = PrometheusMetricLayer::pair();
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
    pub fn pair() -> (Self, PrometheusHandle) {
        let handle = PrometheusBuilder::new()
            .set_buckets_for_metric(
                Matcher::Full(AXUM_HTTP_REQUESTS_DURATION_SECONDS.to_string()),
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
