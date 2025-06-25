//! Run with
//!
//! ```not_rust
//! cd examples && cargo run -p endpoint-type-example
//! ```

use axum::{routing::get, Router};
use axum_prometheus::{EndpointLabel, PrometheusMetricLayerBuilder};
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "endpoint_type_example=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let (prometheus_layer, metric_handle) = PrometheusMetricLayerBuilder::new()
        .with_endpoint_label_type(EndpointLabel::MatchedPathWithFallbackFn(|path| {
            format!("{path}_changed")
        }))
        .with_default_metrics()
        .build_pair();

    let app = Router::new()
        .route(
            "/foo/{bar}",
            get(|| async {
                tracing::debug!("calling /foo/{{bar}}");
            }),
        )
        .nest(
            "/baz",
            Router::new().route(
                "/qux/{a}",
                get(|| async {
                    // Calling `/baz/qux/2`, this'll show up as `endpoint="/baz/qux/2_changed` because of the fallback function.
                    tracing::debug!("calling /baz/qux/{{a}}");
                }),
            ),
        )
        .route("/metrics", get(|| async move { metric_handle.render() }))
        .layer(prometheus_layer);

    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 3000)))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
