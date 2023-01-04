# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added

- Routing patterns can be ignored, and grouped together when reporting to Prometheus.
- Endpoint label behavior can be altered with the new `EndpointLabel` enum.
- Added a new builder `PrometheusMetricLayerBuilder` to easily customize these.

  ```rust
  let (prometheus_layer, metric_handle) = PrometheusMetricLayerBuilder::new()
      // ignore reporting requests that match "/foo" or "/sensitive"
      .with_ignore_patterns(&["/foo", "/sensitive"])
      // if the any of the second argument matches, report them at the `/bar` endpoint
      .with_group_patterns_as("/bar", &["/foo/:bar", "/foo/:bar/:baz"])
      // use `axum::extract::MatchedPath`, and if that fails, use the exact requested URI
      .with_endpoint_label_type(EndpointLabel::MatchedPath)
      .with_default_metrics()
      .build_pair();
  ```

- A [builder-example](examples/builder-example/) and an [endpoint-type-example](examples/endpoint-type-example/).

- The metric names can be changed by setting some environmental variables at compile time. It is best to set these in the `config.toml` (note this is not the same file as `Cargo.toml`):
  ```toml
  [env]
  AXUM_HTTP_REQUESTS_TOTAL = "my_app_requests_total"
  AXUM_HTTP_REQUESTS_DURATION_SECONDS = "my_app_requests_duration_seconds"
  AXUM_HTTP_REQUESTS_PENDING = "my_app_requests_pending"
  ```

## [0.2.0] - 2022-10-25

### Added

- Compatibility with `axum-core = "0.3"` and thus `axum = "0.6"`.

## 0.1.0

First version.

[unreleased]: https://github.com/Ptrskay3/axum-prometheus/compare/master...feat/custom-patterns
[0.2.0]: https://github.com/Ptrskay3/axum-prometheus/compare/axum-0.6...HEAD
