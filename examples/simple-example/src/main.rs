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
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "simple-example=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let (prometheus_layer, metric_handle) = axum_prometheus::PrometheusMetricLayer::pair();
    let app = Router::new()
        .route("/fast", get(|| async {}))
        .route(
            "/slow",
            get(|| async {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }),
        )
        .route("/metrics", get(|| async move { metric_handle.render() }))
        .route(
            "/bad",
            get(|| async { axum::http::StatusCode::BAD_REQUEST }),
        )
        .layer(prometheus_layer);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
