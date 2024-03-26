use std::borrow::Cow;
use std::marker::PhantomData;

#[cfg(feature = "prometheus")]
use metrics_exporter_prometheus::PrometheusHandle;

use crate::{set_prefix, GenericMetricLayer, MakeDefaultHandle, Traffic};

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

#[derive(Default, Clone)]
/// Determines how endpoints are reported.
pub enum EndpointLabel {
    /// The reported endpoint label is always the fully qualified uri path that has been requested.
    Exact,
    /// The reported endpoint label is determined by first trying to extract and return [`axum::extract::MatchedPath`],
    /// and if that fails (typically on [nested routes]) it falls back to [`EndpointLabel::Exact`] behavior. This is
    /// the default option.
    ///
    /// [nested routes]: https://docs.rs/axum/latest/axum/extract/struct.MatchedPath.html#matched-path-in-nested-routers
    #[default]
    MatchedPath,
    /// Same as [`EndpointLabel::MatchedPath`], but instead of falling back to the exact uri called, it's given to a user-defined
    /// fallback function, that is expected to produce a String, which is then reported to Prometheus.
    MatchedPathWithFallbackFn(for<'f> fn(&'f str) -> String),
}

/// A builder for [`GenericMetricLayer`] that enables further customizations.
///
/// Most of the example code uses [`PrometheusMetricLayerBuilder`], which is only a type alias
/// specialized for Prometheus.
///
/// ## Example
/// ```rust,no_run
/// use axum_prometheus::PrometheusMetricLayerBuilder;
///
/// let (metric_layer, metric_handle) = PrometheusMetricLayerBuilder::new()
///     .with_ignore_patterns(&["/metrics", "/sensitive"])
///     .with_group_patterns_as("/foo", &["/foo/:bar", "/foo/:bar/:baz"])
///     .with_group_patterns_as("/bar", &["/auth/*path"])
///     .with_default_metrics()
///     .build_pair();
/// ```
#[derive(Clone, Default)]
pub struct MetricLayerBuilder<'a, T, M, S: MetricBuilderState> {
    pub(crate) traffic: Traffic<'a>,
    pub(crate) metric_handle: Option<T>,
    pub(crate) metric_prefix: Option<String>,
    pub(crate) enable_body_size: bool,
    pub(crate) no_initialize_metrics: bool,
    pub(crate) _marker: PhantomData<(S, M)>,
}

impl<'a, T, M, S> MetricLayerBuilder<'a, T, M, S>
where
    S: MetricBuilderState,
{
    /// Skip reporting a specific route pattern.
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
    /// when reporting to the external provider.
    ///
    /// Supports the same features as `axum`'s Router.
    ///
    ///  _Note that ignore patterns always checked before any other group pattern rule is applied
    /// and it short-circuits if a certain route is ignored._
    pub fn with_ignore_pattern(mut self, ignore_pattern: &'a str) -> Self {
        self.traffic.with_ignore_pattern(ignore_pattern);
        self
    }

    /// Skip reporting a collection of route patterns.
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
    /// [`with_ignore_pattern`]: crate::MetricLayerBuilder::with_ignore_pattern
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

    /// Determine how endpoints are reported. For more information, see [`EndpointLabel`].
    ///
    /// [`EndpointLabel`]: crate::EndpointLabel
    pub fn with_endpoint_label_type(mut self, endpoint_label: EndpointLabel) -> Self {
        self.traffic.with_endpoint_label_type(endpoint_label);
        self
    }

    /// Enable response body size tracking.
    ///
    /// #### Note:
    /// This may introduce some performance overhead.
    pub fn enable_response_body_size(mut self, enable: bool) -> Self {
        self.enable_body_size = enable;
        self
    }

    /// By default, all metrics are initialized via `metrics::describe_*` macros, setting descriptions and units.
    ///
    /// This function disables this initialization.
    pub fn no_initialize_metrics(mut self) -> Self {
        self.no_initialize_metrics = true;
        self
    }
}

impl<'a, T, M> MetricLayerBuilder<'a, T, M, LayerOnly> {
    /// Initialize the builder.
    pub fn new() -> MetricLayerBuilder<'a, T, M, LayerOnly> {
        MetricLayerBuilder {
            _marker: PhantomData,
            traffic: Traffic::new(),
            metric_handle: None,
            no_initialize_metrics: false,
            metric_prefix: None,
            enable_body_size: false,
        }
    }

    /// Use a prefix for the metrics instead of `axum`. This will use the following
    /// metric names:
    ///  - `{prefix}_http_requests_total`
    ///  - `{prefix}_http_requests_pending`
    ///  - `{prefix}_http_requests_duration_seconds`
    ///
    /// ..and will also use `{prefix}_http_response_body_size`, if response body size tracking is enabled.
    ///
    /// This method will take precedence over environment variables.
    ///
    /// ## Note
    ///
    /// This function inherently changes the metric names, beware to use the appropriate names.
    /// There're functions in the `utils` module to get them at runtime.
    ///
    /// [`utils`]: crate::utils
    pub fn with_prefix(mut self, prefix: impl Into<Cow<'a, str>>) -> Self {
        self.metric_prefix = Some(prefix.into().into_owned());
        self
    }
}
impl<'a, T, M> MetricLayerBuilder<'a, T, M, LayerOnly>
where
    M: MakeDefaultHandle<Out = T>,
{
    /// Finalize the builder and get the previously registered metric handle out of it.
    pub fn build(self) -> GenericMetricLayer<'a, T, M> {
        GenericMetricLayer::from_builder(self)
    }
}

impl<'a, T, M> MetricLayerBuilder<'a, T, M, LayerOnly>
where
    M: MakeDefaultHandle<Out = T> + Default,
{
    /// Attach the default exporter handle to the builder. This is similar to
    /// initializing with [`GenericMetricLayer::pair`].
    ///
    /// After calling this function you can finalize with the [`build_pair`] method, and
    /// can no longer call [`build`].
    ///
    /// [`build`]: crate::MetricLayerBuilder::build
    /// [`build_pair`]: crate::MetricLayerBuilder::build_pair
    pub fn with_default_metrics(self) -> MetricLayerBuilder<'a, T, M, Paired> {
        let mut builder = MetricLayerBuilder::<'_, _, _, Paired>::from_layer_only(self);
        builder.metric_handle = Some(M::make_default_handle(M::default()));
        builder
    }
}
impl<'a, T, M> MetricLayerBuilder<'a, T, M, LayerOnly> {
    /// Attach a custom built exporter handle to the builder that's returned from the passed
    /// in closure.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use axum_prometheus::{
    ///        PrometheusMetricLayerBuilder, AXUM_HTTP_REQUESTS_DURATION_SECONDS, utils::SECONDS_DURATION_BUCKETS,
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
    /// [`build`]: crate::MetricLayerBuilder::build
    /// [`build_pair`]: crate::MetricLayerBuilder::build_pair
    pub fn with_metrics_from_fn(
        self,
        f: impl FnOnce() -> T,
    ) -> MetricLayerBuilder<'a, T, M, Paired> {
        let mut builder = MetricLayerBuilder::<'_, _, _, Paired>::from_layer_only(self);
        builder.metric_handle = Some(f());
        builder
    }
}

impl<'a, T, M> MetricLayerBuilder<'a, T, M, Paired> {
    pub(crate) fn from_layer_only(layer_only: MetricLayerBuilder<'a, T, M, LayerOnly>) -> Self {
        if let Some(prefix) = layer_only.metric_prefix.as_ref() {
            set_prefix(prefix);
        }
        if !layer_only.no_initialize_metrics {
            describe_metrics(layer_only.enable_body_size);
        }
        MetricLayerBuilder {
            _marker: PhantomData,
            traffic: layer_only.traffic,
            metric_handle: layer_only.metric_handle,
            no_initialize_metrics: layer_only.no_initialize_metrics,
            metric_prefix: layer_only.metric_prefix,
            enable_body_size: layer_only.enable_body_size,
        }
    }
}

impl<'a, T, M> MetricLayerBuilder<'a, T, M, Paired>
where
    M: MakeDefaultHandle<Out = T> + Default,
{
    /// Finalize the builder and get out the [`GenericMetricLayer`] and the
    /// exporter handle out of it as a tuple.
    pub fn build_pair(self) -> (GenericMetricLayer<'a, T, M>, T) {
        GenericMetricLayer::pair_from_builder(self)
    }
}

#[cfg(feature = "prometheus")]
/// A builder for [`crate::PrometheusMetricLayer`] that enables further customizations.
pub type PrometheusMetricLayerBuilder<'a, S> =
    MetricLayerBuilder<'a, PrometheusHandle, crate::Handle, S>;

fn describe_metrics(enable_body_size: bool) {
    metrics::describe_counter!(
        crate::utils::requests_total_name(),
        metrics::Unit::Count,
        "The number of times a HTTP request was processed."
    );
    metrics::describe_gauge!(
        crate::utils::requests_pending_name(),
        metrics::Unit::Count,
        "The number of currently in-flight requests."
    );
    metrics::describe_histogram!(
        crate::utils::requests_duration_name(),
        metrics::Unit::Seconds,
        "The distribution of HTTP response times."
    );
    if enable_body_size {
        metrics::describe_histogram!(
            crate::utils::response_body_size_name(),
            metrics::Unit::Count,
            "The distribution of HTTP response body sizes."
        );
    }
}
