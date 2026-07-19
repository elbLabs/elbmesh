use std::{fs, path::Path};

#[cfg(feature = "nats-tests")]
const PINNED_NATS_SERVER_VERSION: &str = "2.14.3";
const PINNED_NATS_DOCKER_IMAGE: &str = "nats:2.14.3-alpine";

#[cfg(feature = "nats-tests")]
mod support;

#[cfg(not(feature = "nats-tests"))]
#[test]
fn nats_harness_is_disabled_without_feature() {
    assert!(!cfg!(feature = "nats-adapter"));
}

#[test]
fn nats_harness_pins_exact_2_14_server_with_jetstream_enabled() {
    let compose = workspace_file("docker-compose.yml");

    assert!(
        compose.contains(&format!("image: {PINNED_NATS_DOCKER_IMAGE}")),
        "NATS harness must pin exact image {PINNED_NATS_DOCKER_IMAGE}"
    );
    assert!(
        compose.contains("command: [\"-js\", \"-m\", \"8222\"]"),
        "NATS harness must start JetStream explicitly"
    );
}

#[test]
fn async_nats_disables_defaults_and_enables_cumulative_server_contracts() {
    let manifest = workspace_file("crates/elbmesh-core/Cargo.toml");
    let dependency = manifest
        .lines()
        .find(|line| line.trim_start().starts_with("async-nats ="))
        .expect("elbmesh-core must declare async-nats");

    assert!(
        dependency.contains("default-features = false"),
        "async-nats defaults must remain disabled: {dependency}"
    );
    for required_feature in ["server_2_10", "server_2_11", "server_2_12", "server_2_14"] {
        assert!(
            dependency.contains(&format!("\"{required_feature}\"")),
            "async-nats must enable cumulative {required_feature} support: {dependency}"
        );
    }
}

#[cfg(feature = "nats-tests")]
#[test]
fn nats_harness_uses_documented_url_env_name() {
    assert_eq!(support::nats::NATS_URL_ENV, "ELBMESH_NATS_URL");
}

#[cfg(feature = "nats-tests")]
#[test]
fn nats_harness_documents_docker_compose_defaults() {
    assert_eq!(support::nats::NATS_DOCKER_SERVICE, "nats");
    assert_eq!(support::nats::NATS_DOCKER_URL, "nats://127.0.0.1:4222");
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

#[cfg(feature = "nats-tests")]
#[tokio::test]
async fn live_nats_harness_reports_exact_pinned_version_and_jetstream() {
    let config = match support::nats::NatsHarnessConfig::from_env() {
        Ok(config) => config,
        Err(skip) => {
            eprintln!("{}", skip.reason());
            return;
        }
    };

    let client = async_nats::connect(config.url())
        .await
        .expect("connect to configured NATS harness");
    let server = client.server_info();

    assert_eq!(server.version, PINNED_NATS_SERVER_VERSION);
    assert!(server.jetstream, "configured NATS must enable JetStream");
}

fn workspace_file(relative_path: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative_path);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read workspace file '{}': {error}", path.display()))
}
