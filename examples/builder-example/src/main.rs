//! Run with
//!
//! ```not_rust
//! cd examples && cargo run -p builder-example
//! ```

use axum::{routing::get, Router};
use axum_prometheus::{
    metrics_exporter_prometheus::{Matcher, PrometheusBuilder},
    utils::SECONDS_DURATION_BUCKETS,
    PrometheusMetricLayerBuilder, AXUM_HTTP_REQUESTS_DURATION_SECONDS,
};
use std::{net::SocketAddr, time::Duration};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "builder_example=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let (prometheus_layer, metric_handle) = PrometheusMetricLayerBuilder::new()
        .with_prefix("builder-example")
        // ignore reporting requests that match "/metrics"
        .with_ignore_pattern("/metrics")
        // if the any of the second argument matches, report them at the `/foo` endpoint
        .with_group_patterns_as("/foo", &["/foo/:bar", "/foo/:bar/:baz"])
        // build a custom PrometheusHandle
        .with_metrics_from_fn(|| {
            PrometheusBuilder::new()
                .set_buckets_for_metric(
                    Matcher::Full(AXUM_HTTP_REQUESTS_DURATION_SECONDS.to_string()),
                    SECONDS_DURATION_BUCKETS,
                )
                .unwrap()
                .install_recorder()
                .unwrap()
        })
        .build_pair();

    let app = Router::new()
        .route(
            "/foo/:bar",
            get(|| async {
                tracing::debug!("calling /foo/:bar");
            }),
        )
        .route(
            "/foo/:bar/:baz",
            get(|| async {
                tracing::debug!("calling /foo/:bar/:baz");
            }),
        )
        .route(
            "/fast",
            get(|| async {
                tracing::debug!("calling /fast");
            }),
        )
        .route(
            "/slow",
            get(|| async {
                tracing::debug!("calling /slow");
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
