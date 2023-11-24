mod common;
use common::echo;
use http::Request;
use http_body_util::Full;
use tower::{ServiceBuilder, ServiceExt};
use tower_service::Service;

#[tokio::test]
async fn metric_handle_rendered_correctly() {
    let (layer, handle) = axum_prometheus::PrometheusMetricLayer::pair();

    let mut service = ServiceBuilder::new().layer(layer).service_fn(echo);

    let req = Request::builder().body(Full::default()).unwrap();
    let _res = service.ready().await.unwrap().call(req).await.unwrap();
    insta::with_settings!({
            filters => vec![
                (
                    r"\b[-+]?[0-9]*\.?[0-9]+\b\\naxum_http_requests_duration_seconds_count",
                    "",
                )
            ]
        },
    {

        insta::assert_yaml_snapshot!(handle.render());
    }
    );
}
