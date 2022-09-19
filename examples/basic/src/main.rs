//! Run with
//!
//! ```not_rust
//! cd examples && cargo run -p simple-example
//! ```

use axum::{routing::get, Router};
use axum_prometheus::{
    LifeCycleLayer, Traffic, AXUM_HTTP_REQUEST_DURATION_SECONDS, SECONDS_DURATION_BUCKETS,
};
use std::{net::SocketAddr, time::Duration};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "simple-example=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let metric_handle = PrometheusBuilder::new()
        .set_buckets_for_metric(
            Matcher::Full(AXUM_HTTP_REQUEST_DURATION_SECONDS.to_string()),
            SECONDS_DURATION_BUCKETS,
        )
        .unwrap()
        .install_recorder()
        .unwrap();

    let mc = axum_prometheus::HttpClassifier::new().into_make_classifier();
    let lifecycle_layer = axum_prometheus::LifeCycleLayer::new(mc, Traffic::new());
    let app = Router::new()
        .route("/fast", get(|| async {}))
        .route(
            "/slow",
            get(|| async {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }),
        )
        .route("/metrics", get(|| async move { metric_handle.render() }))
        .layer(lifecycle_layer);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
