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

/// Identifies the counter used for total requests failed.
pub const AXUM_HTTP_REQUESTS_FAILED: &str = "axum_http_requests_failed";

use std::time::Instant;

pub use layer::Metric;
pub use layer::PrometheusMetricLayer;
pub use lifecycle::layer::LifeCycleLayer;
use lifecycle::Callbacks;
use metrics::decrement_gauge;
use metrics::histogram;
use metrics::increment_counter;
use metrics::increment_gauge;
use tower_http::classify::ClassifiedResponse;
use tower_http::classify::SharedClassifier;
use tower_http::classify::StatusInRangeAsFailures;
pub use utils::SECONDS_DURATION_BUCKETS;

pub use metrics;
pub use metrics_exporter_prometheus;
use utils::as_label;

#[derive(Clone, Debug, Copy)]
pub struct Traffic {}

impl Traffic {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Debug, Clone)]
pub struct MetricsData {
    pub status: u16,
    pub endpoint: String,
    pub start: Instant,
    pub method: &'static str,
}

impl<FailureClass> Callbacks<FailureClass> for Traffic {
    type Data = MetricsData;

    fn prepare<B>(&mut self, request: &http::Request<B>) -> Self::Data {
        let now = std::time::Instant::now();
        let endpoint = request.uri().path().to_owned();
        let status = 0u16;
        let method = as_label(request.method());

        let labels = [
            ("method", method.to_owned()),
            ("endpoint", endpoint.clone()),
        ];
        increment_counter!(AXUM_HTTP_REQUESTS_TOTAL, &labels);
        increment_gauge!(AXUM_HTTP_REQUESTS_PENDING, 1.0, &labels);

        MetricsData {
            status,
            endpoint,
            start: now,
            method,
        }
    }

    fn on_response<B>(
        &mut self,
        _res: &http::Response<B>,
        _classifier: ClassifiedResponse<FailureClass, ()>,
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
            AXUM_HTTP_REQUEST_DURATION_SECONDS,
            duration_seconds,
            &[
                ("method", data.method.to_string()),
                ("status", data.status.to_string()),
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
            ("endpoint", data.endpoint.clone()),
        ];
        decrement_gauge!(AXUM_HTTP_REQUESTS_PENDING, 1.0, &labels);
        increment_counter!(AXUM_HTTP_REQUESTS_FAILED, &labels);
    }
}

pub struct HttpClassifier {
    classifier: StatusInRangeAsFailures,
}

impl HttpClassifier {
    pub fn new() -> Self {
        Self {
            classifier: StatusInRangeAsFailures::new(400..=599),
        }
    }

    pub fn into_make_classifier(self) -> SharedClassifier<StatusInRangeAsFailures> {
        self.classifier.into_make_classifier()
    }
}
