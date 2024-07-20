# Changelog

All notable changes to this project will be documented in this file.

# [Unreleased]

# [0.7.0] - 2024-07-20

### Changed

- `MakeDefaultHandle::make_default_handle` now takes `self` as argument. This allows custom implementor structs to hold non-static data. [\#49]
- Change the default initialization of `PrometheusHandle` to prevent unbounded memory growth of histograms. [\#52]
- Bump `metrics` to `0.23`, `metrics-exporter-prometheus` to `0.15`. [\#52]
- Document MSRV as 1.70 currently. [\#52]

### Added

- `GenericMetricLayer::pair_from` to initialize from a concrete struct. `GenericMetricLayer::pair` now requires that the handle type implements `Default`. [\#49]
- `BaseMetricLayer` that serves a more lightweight alternative to `GenericMetricLayer`. [\#56]

# [0.6.1] - 2024-01-23

- Disabled the `"push-gateway"` feature in `metrics-exporter-prometheus` by default, and added a way to enable it via 
  the same name under `axum_prometheus`. This change ensures that this crate can still be built without openssl support, see [here](https://github.com/Ptrskay3/axum-prometheus/issues/42). [\#44]
- Update examples to `metrics-exporter-prometheus` to `0.13` and `metrics` to `0.22`. [\#43]


# [0.6.0] - 2024-01-22

- Update `metrics-exporter-prometheus` to `0.13` and `metrics` to `0.22`. [\#39]

# [0.5.0] - 2023-11-27

### Added

- Support for response body size metric, which can be turned on via `PrometheusMetricLayerBuilder::enable_response_body_size`. [\#33]
- All metrics now are initialized via `metrics::describe_*` function by default, but can be turned off with `PrometheusMetricLayerBuilder::no_initialize_metrics`. [\#33]
- Compatibility with `http-body = "1.0"` and`axum = "0.7"`. [\#36]

### Changed

- The lower-level Lifecycle API has changed: separated the `OnBodyChunk` trait, which is ran when a response body chunk has been generated. [#\33]

# [0.4.0] - 2023-07-24

### Added

- Support for different exporters than Prometheus. Developers now allowed to use their own exporter - as long as it's using the `metrics.rs` ecosystem. This is meant to be a non-breaking change - if you're using Prometheus, you shouldn't notice any changes in the public API. If you do however, please file an issue! [\#28]
- An example showcasing `StatsD` exporter [\#28]
- Simple snapshot tests [\#28]
- Utility functions to get metric names at runtime [\#28]

### Fixed

- Previous attempts to fix `PrometheusMetricBuilder::with_prefix` in 0.3.4 were not complete, this is now fully addressed. [\#28]

# [0.3.4] - 2023-07-16

### Fixed

- `PrometheusMetricBuilder::with_prefix` is now properly setting the metric prefix, and the metric handle also takes that prefix into account.
  Previously the metric initialization incorrectly ignored the prefix, which caused the requests duration histogram to use `quantile` instead of `le` labels.

# [0.3.3] - 2023-05-02

- Update `metrics-exporter-prometheus` to `0.12` and `metrics` to `0.21`.

# [0.3.2] - 2023-03-25

### Added

- The status code of the response is now captured in the total requests counter metric.

# [0.3.1] - 2023-02-16

### Added

- `with_prefix` to `PrometheusMetricLayerBuilder`, which can be used to rename the default prefix (`axum`) for all metrics. This is especially useful when
  working with cargo workspaces that has more than one `axum_prometheus` instance (since environment variables don't work there).

## [0.3.0] - 2023-01-04

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

[unreleased]: https://github.com/Ptrskay3/axum-prometheus/compare/release/0.7.0..master
[0.2.0]: https://github.com/Ptrskay3/axum-prometheus/compare/9fb600d7d9ac2e6d38e6399119fc7ba7f25d5fe0...756dc67bf2baae2de406e012bdaa2334ce0fcdcb
[0.3.0]: https://github.com/Ptrskay3/axum-prometheus/compare/axum-0.6...release/0.3
[0.3.1]: https://github.com/Ptrskay3/axum-prometheus/compare/release/0.3...release/0.3.1
[0.3.2]: https://github.com/Ptrskay3/axum-prometheus/compare/release/0.3.1...release/0.3.2
[0.3.3]: https://github.com/Ptrskay3/axum-prometheus/compare/release/0.3.2...release/0.3.3
[0.3.4]: https://github.com/Ptrskay3/axum-prometheus/compare/release/0.3.3...release/0.3.4
[0.4.0]: https://github.com/Ptrskay3/axum-prometheus/compare/release/0.3.4...release/0.4.0
[0.5.0]: https://github.com/Ptrskay3/axum-prometheus/compare/release/0.4.0...release/0.5.0
[0.6.0]: https://github.com/Ptrskay3/axum-prometheus/compare/release/0.5.0...release/0.6.0
[0.6.1]: https://github.com/Ptrskay3/axum-prometheus/compare/release/0.6.0...release/0.6.1
[0.7.0]: https://github.com/Ptrskay3/axum-prometheus/compare/release/0.6.1...release/0.7.0
[\#28]: https://github.com/Ptrskay3/axum-prometheus/pull/28
