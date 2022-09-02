/// Standard HTTP request duration buckets measured in seconds. The default buckets are tailored to broadly
/// measure the response time of a network service. Most likely, however, you will be required to define
/// buckets customized to your use case.
pub const SECONDS_DURATION_BUCKETS: &[f64; 11] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];
