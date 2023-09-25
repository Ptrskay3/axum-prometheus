//! Run with
//!
//! ```not_rust
//! cd examples && cargo run -p simple-example
//! ```

use axum::{routing::get, Router};
use std::{net::SocketAddr, time::Duration};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "simple-example=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let (mut prometheus_layer, metric_handle) = axum_prometheus::PrometheusMetricLayer::pair();
    prometheus_layer.enable_response_body_size();
    let app = Router::new()
        .route("/fast", get(|| async { "Hello" }))
        .route(
            "/slow",
            get(|| async {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }),
        )
        .route("/metrics", get(|| async move { metric_handle.render() }))
        .layer(prometheus_layer);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
