//! Run with
//!
//! ```not_rust
//! cd examples && cargo run -p exporter-statsd-example
//! ```

use axum::{routing::get, Router};
use axum_prometheus::{GenericMetricLayer, MakeDefaultHandle};
use metrics_exporter_statsd::StatsdBuilder;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// A marker struct for the custom StatsD exporter.
struct Recorder;

// In order to use this with `axum_prometheus`, we must implement `MakeDefaultHandle`.
impl MakeDefaultHandle for Recorder {
    type Out = ();

    fn make_default_handle() -> Self::Out {
        // The regular setup for StatsD..
        let recorder = StatsdBuilder::from("127.0.0.1", 8125)
            .with_queue_size(5000)
            .with_buffer_size(1024)
            .build(Some("prefix"))
            .expect("Could not create StatsdRecorder");

        metrics::set_boxed_recorder(Box::new(recorder)).unwrap();
        // We don't need to return anything meaningful from here (unlike PrometheusHandle)
        // Let's just return an empty tuple.
        ()
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "exporter-statsd-example=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Use `GenericMetricLayer` instead of `PrometheusMetricLayer`.
    let (metric_layer, _) = GenericMetricLayer::<'_, _, Recorder>::pair();
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
