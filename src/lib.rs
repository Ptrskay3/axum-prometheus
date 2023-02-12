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
//! ### Renaming Metrics
//!  
//! These metrics can be renamed by specifying environmental variables at compile time:
//! - `AXUM_HTTP_REQUESTS_TOTAL`
//! - `AXUM_HTTP_REQUESTS_DURATION_SECONDS`
//! - `AXUM_HTTP_REQUESTS_PENDING`
//!
//! These environmental variables can be set in your `.cargo/config.toml` since Cargo 1.56:
//! ```toml
//! [env]
//! AXUM_HTTP_REQUESTS_TOTAL = "my_app_requests_total"
//! AXUM_HTTP_REQUESTS_DURATION_SECONDS = "my_app_requests_duration_seconds"
//! AXUM_HTTP_REQUESTS_PENDING = "my_app_requests_pending"
//! ```
//!
//! ## Usage
//!
//! For more elaborate use-cases, see the builder-example that leverages [`PrometheusMetricLayerBuilder`].
//!
//! Add `axum-prometheus` to your `Cargo.toml`.
//! ```not_rust
//! [dependencies]
//! axum-prometheus = "0.3.0"
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
//!
//! [`PrometheusMetricLayerBuilder`]: crate::PrometheusMetricLayerBuilder

#![allow(clippy::module_name_repetitions, clippy::unreadable_literal)]

/// Identifies the gauge used for the requests pending metric. Defaults to
/// `axum_http_requests_pending`, but can be changed by setting the `AXUM_HTTP_REQUESTS_PENDING`
/// env at compile time.
pub const AXUM_HTTP_REQUESTS_PENDING: &str = match option_env!("AXUM_HTTP_REQUESTS_PENDING") {
    Some(n) => n,
    None => "axum_http_requests_pending",
};

/// Identifies the histogram/summary used for request latency. Defaults to `axum_http_requests_duration_seconds`,
/// but can be changed by setting the `AXUM_HTTP_REQUESTS_DURATION_SECONDS` env at compile time.
pub const AXUM_HTTP_REQUESTS_DURATION_SECONDS: &str =
    match option_env!("AXUM_HTTP_REQUESTS_DURATION_SECONDS") {
        Some(n) => n,
        None => "axum_http_requests_duration_seconds",
    };

/// Identifies the counter used for requests total. Defaults to `axum_http_requests_total`,
/// but can be changed by setting the `AXUM_HTTP_REQUESTS_TOTAL` env at compile time.
pub const AXUM_HTTP_REQUESTS_TOTAL: &str = match option_env!("AXUM_HTTP_REQUESTS_TOTAL") {
    Some(n) => n,
    None => "axum_http_requests_total",
};

use std::borrow::Cow;
use std::collections::HashMap;
use std::time::Instant;

mod builder;
pub mod lifecycle;
mod utils;
use axum::extract::MatchedPath;
pub use builder::EndpointLabel;
pub use builder::PrometheusMetricLayerBuilder;
use builder::{LayerOnly, Paired};
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
pub struct Traffic<'a> {
    ignore_patterns: matchit::Router<()>,
    group_patterns: HashMap<&'a str, matchit::Router<()>>,
    endpoint_label: EndpointLabel,
    metric_prefix: Option<String>,
}

impl<'a> Traffic<'a> {
    pub(crate) fn new() -> Self {
        Traffic::default()
    }

    pub(crate) fn with_ignore_pattern(&mut self, ignore_pattern: &'a str) {
        self.ignore_patterns
            .insert(ignore_pattern, ())
            .expect("good route specs");
    }

    pub(crate) fn with_ignore_patterns(&mut self, ignore_patterns: &'a [&'a str]) {
        for pattern in ignore_patterns {
            self.with_ignore_pattern(pattern);
        }
    }

    pub(crate) fn with_group_patterns_as(&mut self, group_pattern: &'a str, patterns: &'a [&str]) {
        self.group_patterns
            .entry(group_pattern)
            .and_modify(|router| {
                for pattern in patterns {
                    router.insert(*pattern, ()).expect("good route specs");
                }
            })
            .or_insert_with(|| {
                let mut inner_router = matchit::Router::new();
                for pattern in patterns {
                    inner_router.insert(*pattern, ()).expect("good route specs");
                }
                inner_router
            });
    }

    pub(crate) fn ignores(&self, path: &str) -> bool {
        self.ignore_patterns.at(path).is_ok()
    }

    pub(crate) fn apply_group_pattern(&self, path: &'a str) -> &'a str {
        self.group_patterns
            .iter()
            .find_map(|(&group, router)| router.at(path).ok().and(Some(group)))
            .unwrap_or(path)
    }

    pub(crate) fn with_endpoint_label_type(&mut self, endpoint_label: EndpointLabel) {
        self.endpoint_label = endpoint_label;
    }
}

/// Struct used for storing and calculating information about the current request.
#[derive(Debug, Clone)]
pub struct MetricsData {
    pub endpoint: String,
    pub start: Instant,
    pub method: &'static str,
}

impl<'a, FailureClass> Callbacks<FailureClass> for Traffic<'a> {
    type Data = Option<MetricsData>;

    fn prepare<B>(&mut self, request: &http::Request<B>) -> Self::Data {
        let now = std::time::Instant::now();
        let exact_endpoint = request.uri().path();
        if self.ignores(exact_endpoint) {
            return None;
        }
        let endpoint = match self.endpoint_label {
            EndpointLabel::Exact => Cow::from(exact_endpoint),
            EndpointLabel::MatchedPath => Cow::from(
                request
                    .extensions()
                    .get::<MatchedPath>()
                    .map_or(exact_endpoint, MatchedPath::as_str),
            ),
            EndpointLabel::MatchedPathWithFallbackFn(fallback_fn) => {
                if let Some(mp) = request
                    .extensions()
                    .get::<MatchedPath>()
                    .map(MatchedPath::as_str)
                {
                    Cow::from(mp)
                } else {
                    Cow::from(fallback_fn(exact_endpoint))
                }
            }
        };
        let endpoint = self.apply_group_pattern(&endpoint).to_owned();
        let method = utils::as_label(request.method());

        let labels = [
            ("method", method.to_owned()),
            ("endpoint", endpoint.clone()),
        ];

        if let Some(prefix) = self.metric_prefix.as_ref() {
            increment_counter!(format!("{prefix}_http_requests_total"), &labels);
            increment_gauge!(format!("{prefix}_http_requests_pending"), 1.0, &labels);
        } else {
            increment_counter!(AXUM_HTTP_REQUESTS_TOTAL, &labels);
            increment_gauge!(AXUM_HTTP_REQUESTS_PENDING, 1.0, &labels);
        }

        Some(MetricsData {
            endpoint,
            start: now,
            method,
        })
    }

    fn on_response<B>(
        &mut self,
        res: &http::Response<B>,
        _cls: ClassifiedResponse<FailureClass, ()>,
        data: &mut Self::Data,
    ) {
        if let Some(data) = data {
            let duration_seconds = data.start.elapsed().as_secs_f64();

            let gauge_labels = [
                ("method", data.method.to_string()),
                ("endpoint", data.endpoint.to_string()),
            ];

            let histogram_labels = [
                ("method", data.method.to_string()),
                ("status", res.status().as_u16().to_string()),
                ("endpoint", data.endpoint.to_string()),
            ];

            if let Some(prefix) = self.metric_prefix.as_ref() {
                decrement_gauge!(
                    format!("{prefix}_http_requests_pending"),
                    1.0,
                    &gauge_labels
                );
                histogram!(
                    format!("{prefix}_http_requests_duration_seconds"),
                    duration_seconds,
                    &histogram_labels,
                );
            } else {
                decrement_gauge!(AXUM_HTTP_REQUESTS_PENDING, 1.0, &gauge_labels);
                histogram!(
                    AXUM_HTTP_REQUESTS_DURATION_SECONDS,
                    duration_seconds,
                    &histogram_labels,
                );
            }
        }
    }
}

/// The tower middleware layer for recording http metrics with Prometheus.
#[derive(Clone)]
pub struct PrometheusMetricLayer<'a> {
    pub(crate) inner_layer: LifeCycleLayer<SharedClassifier<StatusInRangeAsFailures>, Traffic<'a>>,
}

impl<'a> PrometheusMetricLayer<'a> {
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

    pub(crate) fn from_builder(builder: PrometheusMetricLayerBuilder<'a, LayerOnly>) -> Self {
        let make_classifier =
            StatusInRangeAsFailures::new_for_client_and_server_errors().into_make_classifier();
        let inner_layer = LifeCycleLayer::new(make_classifier, builder.traffic);
        Self { inner_layer }
    }

    pub(crate) fn pair_from_builder(
        builder: PrometheusMetricLayerBuilder<'a, Paired>,
    ) -> (Self, PrometheusHandle) {
        let make_classifier =
            StatusInRangeAsFailures::new_for_client_and_server_errors().into_make_classifier();
        let inner_layer = LifeCycleLayer::new(make_classifier, builder.traffic);

        (
            Self { inner_layer },
            builder
                .metric_handle
                .unwrap_or_else(Self::make_default_handle),
        )
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
        (Self::new(), Self::make_default_handle())
    }

    pub(crate) fn make_default_handle() -> PrometheusHandle {
        PrometheusBuilder::new()
            .set_buckets_for_metric(
                Matcher::Full(AXUM_HTTP_REQUESTS_DURATION_SECONDS.to_string()),
                SECONDS_DURATION_BUCKETS,
            )
            .unwrap()
            .install_recorder()
            .unwrap()
    }
}

impl<'a> Default for PrometheusMetricLayer<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, S> Layer<S> for PrometheusMetricLayer<'a> {
    type Service = LifeCycle<S, SharedClassifier<StatusInRangeAsFailures>, Traffic<'a>>;

    fn layer(&self, inner: S) -> Self::Service {
        self.inner_layer.layer(inner)
    }
}
