//!A middleware to collect HTTP metrics for Axum applications.
//!
//! `axum-prometheus` relies on [`metrics.rs`](https://metrics.rs/) and its ecosystem to collect and export metrics - for instance for Prometheus, `metrics_exporter_prometheus` is used as a backend to interact with Prometheus.
//!
//! ## Metrics
//!
//! By default three HTTP metrics are tracked
//! - `axum_http_requests_total` (labels: endpoint, method, status): the total number of HTTP requests handled (counter)
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
//! ..or optionally use [`PrometheusMetricLayerBuilder::with_prefix`] function.
//!
//! ## Usage
//!
//! For more elaborate use-cases, see the builder-example that leverages [`PrometheusMetricLayerBuilder`].
//!
//! Add `axum-prometheus` to your `Cargo.toml`.
//! ```not_rust
//! [dependencies]
//! axum-prometheus = "0.4.0"
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
//! axum_http_requests_total{method="GET",endpoint="/metrics",status="200"} 5
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
//! ## Using a different exporter than Prometheus
//!
//! This crate may be used with other exporters than Prometheus. First, disable the default features:
//!
//! ```toml
//! axum-prometheus = { version = "0.4.0", default-features = false }
//! ```
//!
//! Then implement the `MakeDefaultHandle` for the provider you'd like to use. For `StatsD`:
//!
//! ```rust,ignore
//! use metrics_exporter_statsd::StatsdBuilder;
//! use axum_prometheus::{MakeDefaultHandle, GenericMetricLayer};
//!
//! // A marker struct for the custom StatsD exporter.
//! struct Recorder;
//!
//! // In order to use this with `axum_prometheus`, we must implement `MakeDefaultHandle`.
//! impl MakeDefaultHandle for Recorder {
//!     type Out = ();
//!
//!     fn make_default_handle() -> Self::Out {
//!         // The regular setup for StatsD..
//!         let recorder = StatsdBuilder::from("127.0.0.1", 8125)
//!             .with_queue_size(5000)
//!             .with_buffer_size(1024)
//!             .build(Some("prefix"))
//!             .expect("Could not create StatsdRecorder");
//!
//!         metrics::set_boxed_recorder(Box::new(recorder)).unwrap();
//!         // We don't need to return anything meaningful from here (unlike PrometheusHandle)
//!         // Let's just return an empty tuple.
//!         ()
//!     }
//! }
//!
//! fn main() {
//!     // ...
//!     // Use `GenericMetricLayer` instead of `PrometheusMetricLayer`.
//!     let (metric_layer, _handle) = GenericMetricLayer::<'_, _, Recorder>::pair();
//!     // ...
//!
//! }
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

#[doc(hidden)]
pub static PREFIXED_HTTP_REQUESTS_TOTAL: OnceCell<String> = OnceCell::new();
#[doc(hidden)]
pub static PREFIXED_HTTP_REQUESTS_DURATION_SECONDS: OnceCell<String> = OnceCell::new();
#[doc(hidden)]
pub static PREFIXED_HTTP_REQUESTS_PENDING: OnceCell<String> = OnceCell::new();

use std::borrow::Cow;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::time::Instant;

mod builder;
pub mod lifecycle;
pub mod utils;
use axum::extract::MatchedPath;
pub use builder::EndpointLabel;
pub use builder::MetricLayerBuilder;
#[cfg(feature = "prometheus")]
pub use builder::PrometheusMetricLayerBuilder;
use builder::{LayerOnly, Paired};
use lifecycle::layer::LifeCycleLayer;
use lifecycle::{service::LifeCycle, Callbacks};
use metrics::{decrement_gauge, histogram, increment_counter, increment_gauge};
use once_cell::sync::OnceCell;
use tower::Layer;
use tower_http::classify::{ClassifiedResponse, SharedClassifier, StatusInRangeAsFailures};

#[cfg(feature = "prometheus")]
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};

pub use metrics;
#[cfg(feature = "prometheus")]
pub use metrics_exporter_prometheus;

/// Use a prefix for the metrics instead of `axum`. This will use the following
/// metric names:
///  - `{prefix}_http_requests_total`
///  - `{prefix}_http_requests_pending`
///  - `{prefix}_http_requests_duration_seconds`
///
/// Note that this will take precedence over environment variables, and can only
/// be called once. Attempts to call this a second time will panic.
fn set_prefix(prefix: impl AsRef<str>) {
    PREFIXED_HTTP_REQUESTS_TOTAL
        .set(format!("{}_http_requests_total", prefix.as_ref()))
        .expect("the prefix has already been set, and can only be set once.");

    PREFIXED_HTTP_REQUESTS_DURATION_SECONDS
        .set(format!(
            "{}_http_requests_duration_seconds",
            prefix.as_ref()
        ))
        .expect("the prefix has already been set, and can only be set once.");
    PREFIXED_HTTP_REQUESTS_PENDING
        .set(format!("{}_http_requests_pending", prefix.as_ref()))
        .expect("the prefix has already been set, and can only be set once.");
}

/// A marker struct that implements the [`lifecycle::Callbacks`] trait.
#[derive(Clone, Default)]
pub struct Traffic<'a> {
    ignore_patterns: matchit::Router<()>,
    group_patterns: HashMap<&'a str, matchit::Router<()>>,
    endpoint_label: EndpointLabel,
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
    pub body_size: usize,
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

        let requests_pending = PREFIXED_HTTP_REQUESTS_PENDING
            .get()
            .map_or(AXUM_HTTP_REQUESTS_PENDING, |s| s.as_str());
        increment_gauge!(
            requests_pending,
            1.0,
            &[
                ("method", method.to_owned()),
                ("endpoint", endpoint.clone()),
            ]
        );

        Some(MetricsData {
            endpoint,
            start: now,
            method,
            body_size: 0,
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

            let requests_pending = PREFIXED_HTTP_REQUESTS_PENDING
                .get()
                .map_or(AXUM_HTTP_REQUESTS_PENDING, |s| s.as_str());
            decrement_gauge!(
                requests_pending,
                1.0,
                &[
                    ("method", data.method.to_string()),
                    ("endpoint", data.endpoint.to_string()),
                ]
            );

            let labels = [
                ("method", data.method.to_string()),
                ("status", res.status().as_u16().to_string()),
                ("endpoint", data.endpoint.to_string()),
            ];

            let requests_total = PREFIXED_HTTP_REQUESTS_TOTAL
                .get()
                .map_or(AXUM_HTTP_REQUESTS_TOTAL, |s| s.as_str());
            increment_counter!(requests_total, &labels);

            let requests_duration = PREFIXED_HTTP_REQUESTS_DURATION_SECONDS
                .get()
                .map_or(AXUM_HTTP_REQUESTS_DURATION_SECONDS, |s| s.as_str());
            histogram!(requests_duration, duration_seconds, &labels);
        }
    }
}

/// The tower middleware layer for recording http metrics with different exporters.
pub struct GenericMetricLayer<'a, T, M> {
    pub(crate) inner_layer:
        LifeCycleLayer<SharedClassifier<StatusInRangeAsFailures>, Traffic<'a>, ()>,
    _marker: PhantomData<(T, M)>,
}

// We don't require that `T` nor `M` is `Clone`, since none of them is actually contained in this type.
impl<'a, T, M> std::clone::Clone for GenericMetricLayer<'a, T, M> {
    fn clone(&self) -> Self {
        GenericMetricLayer {
            inner_layer: self.inner_layer.clone(),
            _marker: self._marker.clone(),
        }
    }
}

impl<'a, T, M> GenericMetricLayer<'a, T, M>
where
    M: MakeDefaultHandle<Out = T>,
{
    /// Create a new tower middleware that can be used to track metrics.
    ///
    /// By default, this __will not__ "install" the exporter which sets it as the
    /// global recorder for all `metrics` calls.
    /// If you're using Prometheus, here you can use [`metrics_exporter_prometheus::PrometheusBuilder`]
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
    /// use axum_prometheus::{AXUM_HTTP_REQUESTS_DURATION_SECONDS, utils::SECONDS_DURATION_BUCKETS, PrometheusMetricLayer};
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
        let inner_layer = LifeCycleLayer::new(make_classifier, Traffic::new(), ());
        Self {
            inner_layer,
            _marker: PhantomData,
        }
    }

    pub(crate) fn from_builder(builder: MetricLayerBuilder<'a, T, M, LayerOnly>) -> Self {
        let make_classifier =
            StatusInRangeAsFailures::new_for_client_and_server_errors().into_make_classifier();
        let inner_layer = LifeCycleLayer::new(make_classifier, builder.traffic, ());
        Self {
            inner_layer,
            _marker: PhantomData,
        }
    }

    pub(crate) fn pair_from_builder(builder: MetricLayerBuilder<'a, T, M, Paired>) -> (Self, T) {
        let make_classifier =
            StatusInRangeAsFailures::new_for_client_and_server_errors().into_make_classifier();
        let inner_layer = LifeCycleLayer::new(make_classifier, builder.traffic, ());

        (
            Self {
                inner_layer,
                _marker: PhantomData,
            },
            builder.metric_handle.unwrap_or_else(M::make_default_handle),
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
    pub fn pair() -> (Self, T) {
        (Self::new(), M::make_default_handle())
    }
}

impl<'a, T, M> Default for GenericMetricLayer<'a, T, M>
where
    M: MakeDefaultHandle<Out = T>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, S, T, M> Layer<S> for GenericMetricLayer<'a, T, M> {
    type Service = LifeCycle<S, SharedClassifier<StatusInRangeAsFailures>, Traffic<'a>, ()>;

    fn layer(&self, inner: S) -> Self::Service {
        self.inner_layer.layer(inner)
    }
}

/// The trait that allows to use a metrics exporter in `GenericMetricLayer`.
pub trait MakeDefaultHandle {
    /// The type of the metrics handle to return from [`MetricLayerBuilder`].
    type Out;

    /// The function that defines how to initialize a metric exporter by default, if none were provided.
    ///
    /// # Example
    ///
    /// ```rust, no_run
    /// use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle, Matcher};
    /// use axum_prometheus::{utils::{SECONDS_DURATION_BUCKETS, requests_duration_name}, MakeDefaultHandle, GenericMetricLayer};
    ///
    /// // A wrapper struct to work around Rust's orphan rules.
    /// pub struct Handle(pub PrometheusHandle);
    ///
    /// impl MakeDefaultHandle for Handle {
    ///     type Out = PrometheusHandle;
    ///
    ///     fn make_default_handle() -> Self::Out {
    ///         PrometheusBuilder::new()
    ///             .set_buckets_for_metric(
    ///                 Matcher::Full(requests_duration_name().to_string()),
    ///                 SECONDS_DURATION_BUCKETS,
    ///             )
    ///             .unwrap()
    ///             .install_recorder()
    ///             .unwrap()
    ///     }
    /// }
    /// ```
    /// and then, to use it:
    /// ```rust,ignore
    /// let (layer, handle) =  GenericMetricLayer::<'_, _, Handle>::pair();
    /// ```
    fn make_default_handle() -> Self::Out;
}

/// The default handle for the Prometheus exporter.
#[cfg(feature = "prometheus")]
#[derive(Clone)]
pub struct Handle(pub PrometheusHandle);

#[cfg(feature = "prometheus")]
impl MakeDefaultHandle for Handle {
    type Out = PrometheusHandle;

    fn make_default_handle() -> Self::Out {
        PrometheusBuilder::new()
            .set_buckets_for_metric(
                Matcher::Full(
                    PREFIXED_HTTP_REQUESTS_DURATION_SECONDS
                        .get()
                        .map_or(AXUM_HTTP_REQUESTS_DURATION_SECONDS, |s| s.as_str())
                        .to_string(),
                ),
                utils::SECONDS_DURATION_BUCKETS,
            )
            .unwrap()
            .install_recorder()
            .unwrap()
    }
}

#[cfg(feature = "prometheus")]
/// The tower middleware layer for recording http metrics with Prometheus.
pub type PrometheusMetricLayer<'a> = GenericMetricLayer<'a, PrometheusHandle, Handle>;
