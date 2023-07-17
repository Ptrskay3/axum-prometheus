//! Run with
//!
//! ```not_rust
//! cd examples && cargo run -p exporter-tcp-example
//! ```

use axum::{routing::get, Router};
use axum_prometheus::{GenericMetricLayer, MakeDefaultHandle};
use metrics_exporter_tcp::TcpBuilder;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// A marker struct for the custom exporter. Must implement `Clone`, because calling `.layer(..)` requires it.
#[derive(Clone)]
struct TcpExporter;

// In order to use this with `axum_prometheus`, we must implement `MakeDefaultHandle`.
impl MakeDefaultHandle for TcpExporter {
    type Out = Self;

    // For `TcpRecorder`, we don't really need anything to keep, so we can just use this empty struct
    // after the TcpRecorder has been built and installed.
    fn make_default_handle() -> Self::Out {
        let builder = TcpBuilder::new();
        builder.install().expect("failed to install TCP exporter");
        TcpExporter
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "exporter-tcp-example=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let (metric_layer, _metric_handle) = GenericMetricLayer::<'_, TcpExporter>::pair();
    let app = Router::new()
        .route("/foo", get(|| async {}))
        .route("/bar", get(|| async {}))
        .layer(metric_layer);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
