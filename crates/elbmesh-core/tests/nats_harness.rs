#[cfg(feature = "nats-tests")]
mod support;

#[cfg(not(feature = "nats-tests"))]
#[test]
fn nats_harness_is_disabled_without_feature() {
    assert!(!cfg!(feature = "nats-adapter"));
}

#[cfg(feature = "nats-tests")]
#[test]
fn nats_harness_uses_documented_url_env_name() {
    assert_eq!(support::nats::NATS_URL_ENV, "ELBMESH_NATS_URL");
}

#[cfg(feature = "nats-tests")]
#[test]
fn nats_harness_reports_skip_when_url_is_missing() {
    let skip = support::nats::NatsHarnessConfig::from_lookup(|_| None)
        .expect_err("missing NATS URL should skip NATS integration tests");

    assert_eq!(
        skip.reason(),
        "ELBMESH_NATS_URL is not set; skipping NATS integration test"
    );
}

#[cfg(feature = "nats-tests")]
#[test]
fn nats_harness_exposes_real_env_loader() {
    match support::nats::NatsHarnessConfig::from_env() {
        Ok(config) => assert!(!config.url().trim().is_empty()),
        Err(skip) => assert_eq!(
            skip.reason(),
            "ELBMESH_NATS_URL is not set; skipping NATS integration test"
        ),
    }
}

#[cfg(feature = "nats-tests")]
#[test]
fn nats_harness_reads_url_when_available() {
    let config = support::nats::NatsHarnessConfig::from_lookup(|key| {
        (key == support::nats::NATS_URL_ENV).then(|| "nats://127.0.0.1:4222".to_string())
    })
    .expect("configured NATS URL should build harness config");

    assert_eq!(config.url(), "nats://127.0.0.1:4222");
}
