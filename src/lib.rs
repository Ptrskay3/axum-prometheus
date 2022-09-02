#![allow(clippy::module_name_repetitions, clippy::unreadable_literal)]

use http::Method;

mod body;
mod future;
mod layer;
mod service;
mod utils;

/// Identifies the gauge used for the requests pending metric.
pub const AXUM_HTTP_REQUESTS_PENDING: &str = "axum_http_requests_pending";

/// Identifies the histogram/summary used for request latency.
pub const AXUM_HTTP_REQUEST_DURATION_SECONDS: &str = "axum_http_request_duration_seconds";

/// Identifies the counter used for requests total.
pub const AXUM_HTTP_REQUESTS_TOTAL: &str = "axum_http_requests_total";

pub(crate) const fn as_label(method: &Method) -> &'static str {
    match *method {
        Method::OPTIONS => "OPTIONS",
        Method::GET => "GET",
        Method::POST => "POST",
        Method::PUT => "PUT",
        Method::DELETE => "DELETE",
        Method::HEAD => "HEAD",
        Method::TRACE => "TRACE",
        Method::CONNECT => "CONNECT",
        Method::PATCH => "PATCH",
        _ => "",
    }
}

pub use layer::Metric;
pub use layer::PrometheusMetricLayer;
pub use utils::SECONDS_DURATION_BUCKETS;

pub use metrics;
pub use metrics_exporter_prometheus;
