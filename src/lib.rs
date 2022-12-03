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
//! For more elaborate use-cases, see the builder-example that leverages [`PrometheusMetricLayerBuilder`].
//!
//! Add `axum-prometheus` to your `Cargo.toml`.
//! ```not_rust
//! [dependencies]
//! axum-prometheus = "0.2.0"
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

/// Identifies the gauge used for the requests pending metric.
pub const AXUM_HTTP_REQUESTS_PENDING: &str = "axum_http_requests_pending";

/// Identifies the histogram/summary used for request latency.
pub const AXUM_HTTP_REQUESTS_DURATION_SECONDS: &str = "axum_http_requests_duration_seconds";

/// Identifies the counter used for requests total.
pub const AXUM_HTTP_REQUESTS_TOTAL: &str = "axum_http_requests_total";

use std::collections::HashMap;
use std::marker::PhantomData;
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
pub struct Traffic<'a> {
    ignore_patterns: matchit::Router<()>,
    group_patterns: HashMap<&'a str, matchit::Router<()>>,
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
            self.ignore_patterns
                .insert(*pattern, ())
                .expect("good route specs");
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
        let endpoint = request.uri().path();
        if self.ignores(endpoint) {
            return None;
        }
        let endpoint = self.apply_group_pattern(endpoint).to_owned();
        let method = utils::as_label(request.method());

        let labels = [
            ("method", method.to_owned()),
            ("endpoint", endpoint.clone()),
        ];
        increment_counter!(AXUM_HTTP_REQUESTS_TOTAL, &labels);
        increment_gauge!(AXUM_HTTP_REQUESTS_PENDING, 1.0, &labels);

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
}

/// The tower middleware layer for recording http metrics with Prometheus.
#[derive(Clone)]
pub struct PrometheusMetricLayer<'a> {
    pub(crate) inner_layer: LifeCycleLayer<SharedClassifier<StatusInRangeAsFailures>, Traffic<'a>>,
}

#[doc(hidden)]
mod sealed {
    use super::{LayerOnly, Paired};
    pub trait Sealed {}
    impl Sealed for LayerOnly {}
    impl Sealed for Paired {}
}
pub trait MetricBuilderState: sealed::Sealed {}

pub enum Paired {}
pub enum LayerOnly {}
impl MetricBuilderState for Paired {}
impl MetricBuilderState for LayerOnly {}

/// A builder for [`PrometheusMetricLayer`] that enables further customizations,
/// such as ignoring or masking routes and defining customized [`PrometheusHandle`]s.
///
/// ## Example
/// ```rust,no_run
/// use axum_prometheus::PrometheusMetricLayerBuilder;
///
/// let (metric_layer, metric_handle) = PrometheusMetricLayerBuilder::new()
///     .with_ignore_patterns(&["/metrics", "/sensitive"])
///     .with_group_patterns_as("/foo", &["/foo/:bar", "/foo/:bar/:baz"])
///     .with_group_patterns_as("/foo", &["/auth/*path"])
///     .with_default_metrics()
///     .build_pair();
/// ```
#[derive(Clone, Default)]
pub struct PrometheusMetricLayerBuilder<'a, S: MetricBuilderState> {
    pub(crate) traffic: Traffic<'a>,
    pub(crate) metric_handle: Option<PrometheusHandle>,
    pub(crate) _marker: PhantomData<S>,
}

impl<'a, S> PrometheusMetricLayerBuilder<'a, S>
where
    S: MetricBuilderState,
{
    /// Skip reporting a specific route pattern to Prometheus.
    ///
    /// In the following example
    /// ```rust
    /// use axum_prometheus::PrometheusMetricLayerBuilder;
    ///
    /// let metric_layer = PrometheusMetricLayerBuilder::new()
    ///     .with_ignore_pattern("/metrics")
    ///     .build();
    /// ```
    /// any request that's URI path matches "/metrics" will be skipped altogether
    /// when reporting to Prometheus.
    ///
    /// Supports the same features as `axum`'s Router.
    ///
    ///  _Note that ignore patterns always checked before any other group pattern rule is applied
    /// and it short-circuits if a certain route is ignored._
    pub fn with_ignore_pattern(mut self, ignore_pattern: &'a str) -> Self {
        self.traffic.with_ignore_pattern(ignore_pattern);
        self
    }

    /// Skip reporting a collection of route patterns to Prometheus.
    ///
    /// Equivalent with calling [`with_ignore_pattern`] repeatedly.
    ///
    /// ```rust
    /// use axum_prometheus::PrometheusMetricLayerBuilder;
    ///
    /// let metric_layer = PrometheusMetricLayerBuilder::new()
    ///     .with_ignore_patterns(&["/foo", "/bar/:baz"])
    ///     .build();
    /// ```
    ///
    /// Supports the same features as `axum`'s Router.
    ///
    ///  _Note that ignore patterns always checked before any other group pattern rule is applied
    /// and it short-circuits if a certain route is ignored._
    ///
    /// [`with_ignore_pattern`]: crate::PrometheusMetricLayerBuilder::with_ignore_pattern
    pub fn with_ignore_patterns(mut self, ignore_patterns: &'a [&'a str]) -> Self {
        self.traffic.with_ignore_patterns(ignore_patterns);
        self
    }

    /// Group matching route patterns and report them under the given (arbitrary) endpoint.
    ///
    /// This feature is commonly useful for parametrized routes. Let's say you have these two routes:
    ///  - `/foo/:bar`
    ///  - `/foo/:bar/:baz`
    ///
    /// By default every unique request URL path gets reported with different endpoint label.
    /// This feature allows you to report these under a custom endpoint, for instance `/foo`:
    ///
    /// ```rust
    /// use axum_prometheus::PrometheusMetricLayerBuilder;
    ///
    /// let metric_layer = PrometheusMetricLayerBuilder::new()
    ///     // the choice of "/foo" is arbitrary
    ///     .with_group_patterns_as("/foo", &["/foo/:bar", "foo/:bar/:baz"])
    ///     .build();
    /// ```
    pub fn with_group_patterns_as(
        mut self,
        group_pattern: &'a str,
        patterns: &'a [&'a str],
    ) -> Self {
        self.traffic.with_group_patterns_as(group_pattern, patterns);
        self
    }
}

impl<'a> PrometheusMetricLayerBuilder<'a, LayerOnly> {
    /// Initialize the builder.
    pub fn new() -> PrometheusMetricLayerBuilder<'a, LayerOnly> {
        PrometheusMetricLayerBuilder {
            _marker: PhantomData,
            traffic: Traffic::new(),
            metric_handle: None,
        }
    }

    /// Attach the default [`PrometheusHandle`] to the builder. This is similar to
    /// initializing with [`PrometheusMetricLayer::pair`].
    ///
    /// After calling this function you can finalize with the [`build_pair`] method, and
    /// can no longer call [`build`].
    ///
    /// [`build`]: crate::PrometheusMetricLayerBuilder::build
    /// [`build_pair`]: crate::PrometheusMetricLayerBuilder::build_pair
    pub fn with_default_metrics(mut self) -> PrometheusMetricLayerBuilder<'a, Paired> {
        self.metric_handle = Some(PrometheusMetricLayer::make_default_handle());
        PrometheusMetricLayerBuilder::<'_, Paired>::from_layer_only(self)
    }

    /// Attach a custom [`PrometheusHandle`] to the builder that's returned from the passed
    /// in closure.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use axum_prometheus::{
    ///        PrometheusMetricLayerBuilder, AXUM_HTTP_REQUESTS_DURATION_SECONDS, SECONDS_DURATION_BUCKETS,
    /// };
    /// use metrics_exporter_prometheus::{Matcher, PrometheusBuilder};
    ///
    /// let (metric_layer, metric_handle) = PrometheusMetricLayerBuilder::new()
    ///     .with_metrics_from_fn(|| {
    ///         PrometheusBuilder::new()
    ///             .set_buckets_for_metric(
    ///                 Matcher::Full(AXUM_HTTP_REQUESTS_DURATION_SECONDS.to_string()),
    ///                 SECONDS_DURATION_BUCKETS,
    ///             )
    ///             .unwrap()
    ///             .install_recorder()
    ///             .unwrap()
    ///     })
    ///     .build_pair();
    /// ```
    /// After calling this function you can finalize with the [`build_pair`] method, and
    /// can no longer call [`build`].
    ///
    /// [`build`]: crate::PrometheusMetricLayerBuilder::build
    /// [`build_pair`]: crate::PrometheusMetricLayerBuilder::build_pair
    pub fn with_metrics_from_fn(
        mut self,
        f: impl FnOnce() -> PrometheusHandle,
    ) -> PrometheusMetricLayerBuilder<'a, Paired> {
        self.metric_handle = Some(f());
        PrometheusMetricLayerBuilder::<'_, Paired>::from_layer_only(self)
    }

    /// Finalize the builder and get the [`PrometheusMetricLayer`] out of it.
    pub fn build(self) -> PrometheusMetricLayer<'a> {
        PrometheusMetricLayer::from_builder(self)
    }
}

impl<'a> PrometheusMetricLayerBuilder<'a, Paired> {
    pub(crate) fn from_layer_only(layer_only: PrometheusMetricLayerBuilder<'a, LayerOnly>) -> Self {
        PrometheusMetricLayerBuilder {
            _marker: PhantomData,
            traffic: layer_only.traffic,
            metric_handle: layer_only.metric_handle,
        }
    }
    /// Finalize the builder and get out the [`PrometheusMetricLayer`] and the
    /// [`PrometheusHandle`] out of it as a tuple.
    pub fn build_pair(self) -> (PrometheusMetricLayer<'a>, PrometheusHandle) {
        PrometheusMetricLayer::pair_from_builder(self)
    }
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
