//! Run with
//!
//! ```not_rust
//! cd examples && cargo run -p exporter-statsd-example
//! ```

use axum::{routing::get, Router};
use axum_prometheus::{metrics, GenericMetricLayer, MakeDefaultHandle};
use metrics_exporter_statsd::StatsdBuilder;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

struct Recorder<'a> {
    host: &'a str,
    port: u16,
    queue_size: usize,
    buffer_size: usize,
    prefix: Option<&'a str>,
}

// In order to use this with `axum_prometheus`, we must implement `MakeDefaultHandle`.
impl<'a> MakeDefaultHandle for Recorder<'a> {
    // We don't need to return anything meaningful from here (unlike PrometheusHandle)
    // Let's just return an empty tuple.
    type Out = ();

    fn make_default_handle(self) -> Self::Out {
        // The regular setup for StatsD..
        let recorder = StatsdBuilder::from(self.host, self.port)
            .with_queue_size(self.queue_size)
            .with_buffer_size(self.buffer_size)
            .build(self.prefix)
            .expect("Could not create StatsDRecorder");

        metrics::set_global_recorder(recorder).unwrap();
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "exporter_statsd_example=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Use `GenericMetricLayer` instead of `PrometheusMetricLayer`.
    // By using `pair_from_init`, you can inject any values into the recorder.
    // `GenericMetricLayer::pair` is only callable if the recorder struct implements Default.
    let (metric_layer, _) = GenericMetricLayer::pair_from_init(Recorder {
        host: "127.0.0.1",
        port: 8125,
        queue_size: 5000,
        buffer_size: 1024,
        prefix: Some("prefix"),
    });
    let app = Router::new()
        .route("/foo", get(|| async {}))
        .route("/bar", get(|| async {}))
        .layer(metric_layer);

    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 3000)))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
