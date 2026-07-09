#[cfg(feature = "restate-tests")]
mod support;

#[cfg(not(feature = "restate-tests"))]
#[test]
fn restate_harness_is_disabled_without_feature() {
    assert!(!cfg!(feature = "restate-tests"));
}

#[cfg(feature = "restate-tests")]
#[test]
fn restate_harness_uses_documented_env_names() {
    assert_eq!(support::restate::RESTATE_URL_ENV, "ELBMESH_RESTATE_URL");
    assert_eq!(
        support::restate::RESTATE_ADMIN_URL_ENV,
        "ELBMESH_RESTATE_ADMIN_URL"
    );
    assert_eq!(
        support::restate::RESTATE_SERVICE_ADVERTISE_HOST_ENV,
        "ELBMESH_RESTATE_SERVICE_ADVERTISE_HOST"
    );
}

#[cfg(feature = "restate-tests")]
#[test]
fn restate_harness_documents_docker_compose_defaults() {
    assert_eq!(support::restate::RESTATE_DOCKER_SERVICE, "restate");
    assert_eq!(
        support::restate::RESTATE_DOCKER_URL,
        "http://127.0.0.1:8080"
    );
    assert_eq!(
        support::restate::RESTATE_DOCKER_ADMIN_URL,
        "http://127.0.0.1:9070"
    );
}

#[cfg(feature = "restate-tests")]
#[test]
fn restate_harness_reports_skip_when_url_is_missing() {
    let skip = support::restate::RestateHarnessConfig::from_lookup(|_| None)
        .expect_err("missing Restate URL should skip Restate integration tests");

    assert_eq!(
        skip.reason(),
        "ELBMESH_RESTATE_URL is not set; skipping Restate integration test"
    );
}

#[cfg(feature = "restate-tests")]
#[test]
fn restate_harness_exposes_real_env_loader() {
    match support::restate::RestateHarnessConfig::from_env() {
        Ok(config) => assert!(!config.url().trim().is_empty()),
        Err(skip) => assert_eq!(
            skip.reason(),
            "ELBMESH_RESTATE_URL is not set; skipping Restate integration test"
        ),
    }
}

#[cfg(feature = "restate-tests")]
#[test]
fn restate_harness_reads_url_when_available() {
    let config = support::restate::RestateHarnessConfig::from_lookup(|key| {
        (key == support::restate::RESTATE_URL_ENV).then(|| "http://127.0.0.1:8080".to_string())
    })
    .expect("configured Restate URL should build harness config");

    assert_eq!(config.url(), "http://127.0.0.1:8080");
    assert_eq!(config.admin_url(), "http://127.0.0.1:9070");
    assert_eq!(config.service_advertise_host(), "127.0.0.1");
    assert_eq!(config.service_bind_host(), "0.0.0.0");
}

#[cfg(feature = "restate-tests")]
#[test]
fn restate_harness_reads_optional_admin_and_service_hosts() {
    let config = support::restate::RestateHarnessConfig::from_lookup(|key| match key {
        support::restate::RESTATE_URL_ENV => Some("http://127.0.0.1:8080/".to_string()),
        support::restate::RESTATE_ADMIN_URL_ENV => Some("http://127.0.0.1:9070/".to_string()),
        support::restate::RESTATE_SERVICE_ADVERTISE_HOST_ENV => {
            Some("host.docker.internal".to_string())
        }
        support::restate::RESTATE_SERVICE_BIND_HOST_ENV => Some("0.0.0.0".to_string()),
        _ => None,
    })
    .expect("configured Restate harness should build config");

    assert_eq!(config.url(), "http://127.0.0.1:8080");
    assert_eq!(config.admin_url(), "http://127.0.0.1:9070");
    assert_eq!(config.service_advertise_host(), "host.docker.internal");
    assert_eq!(config.service_bind_host(), "0.0.0.0");
}
