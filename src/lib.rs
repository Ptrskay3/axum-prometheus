#![allow(clippy::module_name_repetitions, clippy::unreadable_literal)]

mod body;
mod future;
mod layer;
mod lifecycle;
mod service;
mod utils;

/// Identifies the gauge used for the requests pending metric.
pub const AXUM_HTTP_REQUESTS_PENDING: &str = "axum_http_requests_pending";

/// Identifies the histogram/summary used for request latency.
pub const AXUM_HTTP_REQUEST_DURATION_SECONDS: &str = "axum_http_request_duration_seconds";

/// Identifies the counter used for requests total.
pub const AXUM_HTTP_REQUESTS_TOTAL: &str = "axum_http_requests_total";

pub use layer::Metric;
pub use layer::PrometheusMetricLayer;
pub use lifecycle::layer::LifeCycleLayer;
pub use utils::SECONDS_DURATION_BUCKETS;

pub use metrics;
pub use metrics_exporter_prometheus;
