//! This example uses the `BaseMetricLayer`, which only emits metrics using the `metrics` crate's macros,
//! and the global exporter/recorder is fully up to user to initialize and configure.
//!
//! Run with
//!
//! ```not_rust
//! cd examples && cargo run -p base-metric-layer-example
//! ```
//!
use axum::{routing::get, Router};
use axum_prometheus::{metrics_exporter_prometheus::PrometheusBuilder, BaseMetricLayer};
use std::{net::SocketAddr, time::Duration};

#[tokio::main]
async fn main() {
    // Initialize the recorder as you like. This example uses push gateway mode instead of a http listener.
    // To use this, don't forget to enable the "push-gateway" feature in `axum-prometheus`.
    PrometheusBuilder::new()
        .with_push_gateway(
            "http://127.0.0.1:9091/metrics/job/example",
            Duration::from_secs(10),
            None,
            None,
        )
        .expect("push gateway endpoint should be valid")
        .install()
        .expect("failed to install Prometheus recorder");

    let app = Router::<()>::new()
        .route("/fast", get(|| async {}))
        .route(
            "/slow",
            get(|| async {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }),
        )
        // Only need to add this layer at the end.
        .layer(BaseMetricLayer::new());
    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 3000)))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap()
}
