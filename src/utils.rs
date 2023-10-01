use http::Method;

use crate::{
    AXUM_HTTP_REQUESTS_DURATION_SECONDS, AXUM_HTTP_REQUESTS_PENDING, AXUM_HTTP_REQUESTS_TOTAL,
    AXUM_HTTP_RESPONSE_BODY_SIZE, PREFIXED_HTTP_REQUESTS_DURATION_SECONDS,
    PREFIXED_HTTP_REQUESTS_PENDING, PREFIXED_HTTP_REQUESTS_TOTAL, PREFIXED_HTTP_RESPONSE_BODY_SIZE,
};

/// Standard HTTP request duration buckets measured in seconds. The default buckets are tailored to broadly
/// measure the response time of a network service. Most likely, however, you will be required to define
/// buckets customized to your use case.
pub const SECONDS_DURATION_BUCKETS: &[f64; 11] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

pub(super) const fn as_label(method: &Method) -> &'static str {
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

/// The name of the requests total metric. By default, it's the same as [`AXUM_HTTP_REQUESTS_TOTAL`], but
/// can be changed via the [`with_prefix`] function.
///
/// [`with_prefix`]: crate::MetricLayerBuilder::with_prefix
pub fn requests_total_name() -> &'static str {
    PREFIXED_HTTP_REQUESTS_TOTAL
        .get()
        .map_or(AXUM_HTTP_REQUESTS_TOTAL, |s| s.as_str())
}

/// The name of the requests duration metric. By default, it's the same as [`AXUM_HTTP_REQUESTS_DURATION_SECONDS`], but
/// can be changed via the [`with_prefix`] function.
///
/// [`with_prefix`]: crate::MetricLayerBuilder::with_prefix
pub fn requests_duration_name() -> &'static str {
    PREFIXED_HTTP_REQUESTS_DURATION_SECONDS
        .get()
        .map_or(AXUM_HTTP_REQUESTS_DURATION_SECONDS, |s| s.as_str())
}

/// The name of the requests pending metric. By default, it's the same as [`AXUM_HTTP_REQUESTS_PENDING`], but
/// can be changed via the [`with_prefix`] function.
///
/// [`with_prefix`]: crate::MetricLayerBuilder::with_prefix
pub fn requests_pending_name() -> &'static str {
    PREFIXED_HTTP_REQUESTS_PENDING
        .get()
        .map_or(AXUM_HTTP_REQUESTS_PENDING, |s| s.as_str())
}

/// The name of the response body size metric. By default, it's the same as [`AXUM_HTTP_RESPONSE_BODY_SIZE`], but
/// can be changed via the [`with_prefix`] function.
///
/// [`with_prefix`]: crate::MetricLayerBuilder::with_prefix
pub fn response_body_size_name() -> &'static str {
    PREFIXED_HTTP_RESPONSE_BODY_SIZE
        .get()
        .map_or(AXUM_HTTP_RESPONSE_BODY_SIZE, |s| s.as_str())
}
