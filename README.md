# Axum-Prometheus

<div align="center">
<a href="https://github.com/ptrskay3/axum-prometheus/actions/workflows/base.yml">
<img src="https://github.com/ptrskay3/axum-prometheus/actions/workflows/base.yml/badge.svg" />
</a>
<a href="https://crates.io/crates/axum-prometheus">
<img src="https://img.shields.io/crates/v/axum-prometheus.svg" />
</a>
<a href="https://docs.rs/axum-prometheus">
<img src="https://docs.rs/axum-prometheus/badge.svg" />
</a>
</div>

A middleware to collect HTTP metrics for Axum applications.

`axum-prometheus` relies on [`metrics.rs`](https://metrics.rs/) and its ecosystem to collect and export metrics - for instance for Prometheus, `metrics_exporter_prometheus` is used as a backend to interact with Prometheus.

## Metrics

By default three HTTP metrics are tracked

- `axum_http_requests_total` (labels: endpoint, method, status): the total number of HTTP requests handled (counter)
- `axum_http_requests_duration_seconds` (labels: endpoint, method, status): the request duration for all HTTP requests handled (histogram)
- `axum_http_requests_pending` (labels: endpoint, method): the number of currently in-flight requests (gauge)

This crate also allows to track response body sizes as a histogram — see `PrometheusMetricLayerBuilder::enable_response_body_size`.

### Renaming Metrics

These metrics can be renamed by specifying environmental variables at compile time:

- `AXUM_HTTP_REQUESTS_TOTAL`
- `AXUM_HTTP_REQUESTS_DURATION_SECONDS`
- `AXUM_HTTP_REQUESTS_PENDING`
- `AXUM_HTTP_RESPONSE_BODY_SIZE` (if body size tracking is enabled)

These environmental variables can be set in your `.cargo/config.toml` since Cargo 1.56:

```toml
[env]
AXUM_HTTP_REQUESTS_TOTAL = "my_app_requests_total"
AXUM_HTTP_REQUESTS_DURATION_SECONDS = "my_app_requests_duration_seconds"
AXUM_HTTP_REQUESTS_PENDING = "my_app_requests_pending"
AXUM_HTTP_RESPONSE_BODY_SIZE = "my_app_response_body_size"
```

..or optionally use `PrometheusMetricLayerBuilder::with_prefix` function.

### Compatibility

| Axum Version | Crate Version |
| ------------ | ------------- |
| `0.5`        | `0.1`         |
| `0.6`        | `0.2`, `0.3`, `0.4`  |
| `0.7`        | `0.5`, `0.6`         |

## Usage

For more elaborate use-cases, see the [`builder example`](examples/builder-example/).

Add `axum-prometheus` to your `Cargo.toml`.

```toml
[dependencies]
axum-prometheus = "0.6.1"
```

Then you instantiate the prometheus middleware:

```rust
use std::{net::SocketAddr, time::Duration};
use axum::{routing::get, Router};
use axum_prometheus::PrometheusMetricLayer;

#[tokio::main]
async fn main() {
    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();
    let app = Router::<()>::new()
        .route("/fast", get(|| async {}))
        .route(
            "/slow",
            get(|| async {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }),
        )
        .route("/metrics", get(|| async move { metric_handle.render() }))
        .layer(prometheus_layer);


    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 3000)))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

Note that the `/metrics` endpoint is not automatically exposed, so you need to add that as a route manually.
Calling the `/metrics` endpoint will expose your metrics:

```not_rust
axum_http_requests_total{method="GET",endpoint="/metrics",status="200"} 5
axum_http_requests_pending{method="GET",endpoint="/metrics"} 1
axum_http_requests_duration_seconds_bucket{method="GET",status="200",endpoint="/metrics",le="0.005"} 4
axum_http_requests_duration_seconds_bucket{method="GET",status="200",endpoint="/metrics",le="0.01"} 4
axum_http_requests_duration_seconds_bucket{method="GET",status="200",endpoint="/metrics",le="0.025"} 4
axum_http_requests_duration_seconds_bucket{method="GET",status="200",endpoint="/metrics",le="0.05"} 4
axum_http_requests_duration_seconds_bucket{method="GET",status="200",endpoint="/metrics",le="0.1"} 4
axum_http_requests_duration_seconds_bucket{method="GET",status="200",endpoint="/metrics",le="0.25"} 4
axum_http_requests_duration_seconds_bucket{method="GET",status="200",endpoint="/metrics",le="0.5"} 4
axum_http_requests_duration_seconds_bucket{method="GET",status="200",endpoint="/metrics",le="1"} 4
axum_http_requests_duration_seconds_bucket{method="GET",status="200",endpoint="/metrics",le="2.5"} 4
axum_http_requests_duration_seconds_bucket{method="GET",status="200",endpoint="/metrics",le="5"} 4
axum_http_requests_duration_seconds_bucket{method="GET",status="200",endpoint="/metrics",le="10"} 4
axum_http_requests_duration_seconds_bucket{method="GET",status="200",endpoint="/metrics",le="+Inf"} 4
axum_http_requests_duration_seconds_sum{method="GET",status="200",endpoint="/metrics"} 0.001997171
axum_http_requests_duration_seconds_count{method="GET",status="200",endpoint="/metrics"} 4
```

Let's note that since `metrics-exporter-prometheus = "0.13"` that crate [introduced](https://github.com/metrics-rs/metrics/commit/d817f5c6f4909eeafbd9ff9ceadbf29302169bfa) the `push-gateway` default feature, that
requires openssl support. The `axum_prometheus` crate __does not__ rely on, nor enable this feature by default — if you need it, 
you may enable it through the `"push-gateway"` feature in `axum_prometheus`.

## Using a different exporter than Prometheus

This crate may be used with other exporters than Prometheus. First, disable the default features:

```toml
axum-prometheus = { version = "0.6.1", default-features = false }
```

Then implement the `MakeDefaultHandle` for the provider you'd like to use. For `StatsD`:

```rust
use metrics_exporter_statsd::StatsdBuilder;
use axum_prometheus::{MakeDefaultHandle, GenericMetricLayer};

// A marker struct for the custom StatsD exporter.
struct Recorder;

// In order to use this with `axum_prometheus`, we must implement `MakeDefaultHandle`.
impl MakeDefaultHandle for Recorder {
    // We don't need to return anything meaningful from here (unlike PrometheusHandle)
    // Let's just return an empty tuple.
    type Out = ();

    fn make_default_handle() -> Self::Out {
        // The regular setup for StatsD..
        let recorder = StatsdBuilder::from("127.0.0.1", 8125)
            .with_queue_size(5000)
            .with_buffer_size(1024)
            .build(Some("prefix"))
            .expect("Could not create StatsdRecorder");

        metrics::set_boxed_recorder(Box::new(recorder)).unwrap();
    }
}

fn main() {
    // ...
    // Use `GenericMetricLayer` instead of `PrometheusMetricLayer`.
    let (metric_layer, _handle) = GenericMetricLayer::<'_, _, Recorder>::pair();
    // ...

}
```

---

This crate is similar to (and takes inspiration from) [`actix-web-prom`](https://github.com/nlopes/actix-web-prom) and [`rocket_prometheus`](https://github.com/sd2k/rocket_prometheus),
and also builds on top of davidpdrsn's [earlier work with LifeCycleHooks](https://github.com/tower-rs/tower-http/pull/96) in `tower-http`.
