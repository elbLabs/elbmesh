#![cfg(feature = "nats-tests")]

use std::{fs, path::Path, time::Duration};

use async_nats::{
    header::{
        NATS_BATCH_COMMIT, NATS_BATCH_COMMIT_FINAL, NATS_BATCH_ID, NATS_BATCH_SEQUENCE,
        NATS_EXPECTED_LAST_SUBJECT_SEQUENCE, NATS_MESSAGE_ID, NATS_REQUIRED_API_LEVEL,
    },
    jetstream::{
        publish::PublishAck,
        stream::{Config, RetentionPolicy, StorageType},
    },
    HeaderMap,
};

const DUPLICATE_WINDOW: Duration = Duration::from_secs(120);

#[test]
fn async_nats_compiles_with_cumulative_2_10_through_2_14_fields() {
    let config = Config::default();
    let ack = PublishAck::default();

    let _server_2_10 = &config.metadata;
    let _server_2_11 = &config.pause_until;
    let _server_2_12 = config.allow_atomic_publish;
    let _server_2_14 = (config.allow_batch_publish, &ack.batch_id, ack.batch_size);
}

#[test]
fn native_stream_configs_use_separate_file_backed_limits_streams() {
    let contracts = [
        ("ELBMESH_RESOURCES", "elbmesh.resources.>", true),
        ("ELBMESH_ACTIONS", "elbmesh.actions.>", false),
        ("ELBMESH_OPERATIONS", "elbmesh.operations.>", false),
        ("ELBMESH_REACTIONS", "elbmesh.reactions.>", false),
    ];

    for (name, subject, atomic) in contracts {
        let config = native_stream_config(name, subject, atomic);

        assert_eq!(config.name, name);
        assert_eq!(config.subjects, [subject]);
        assert_eq!(config.retention, RetentionPolicy::Limits);
        assert_eq!(config.storage, StorageType::File);
        assert_eq!(config.num_replicas, 1);
        assert_eq!(config.duplicate_window, DUPLICATE_WINDOW);
        assert_eq!(config.allow_atomic_publish, atomic);
        assert!(
            !config.allow_batch_publish,
            "non-atomic fast ingest is outside the Elbmesh protocol"
        );
    }
}

#[test]
fn resource_batch_headers_keep_transport_and_aggregate_sequences_distinct() {
    let mut headers = HeaderMap::new();
    headers.insert(NATS_MESSAGE_ID, "event-002");
    headers.insert(NATS_BATCH_ID, "action-123");
    headers.insert(NATS_BATCH_SEQUENCE, "2");
    headers.insert(NATS_BATCH_COMMIT, NATS_BATCH_COMMIT_FINAL);
    headers.insert(NATS_REQUIRED_API_LEVEL, "4");
    headers.insert(NATS_EXPECTED_LAST_SUBJECT_SEQUENCE, "41");
    headers.insert("Elbmesh-Aggregate-Sequence", "8");
    headers.insert("Elbmesh-Message-Type", "offer_title_updated");
    headers.insert("Elbmesh-Message-Version", "1");
    headers.insert("Elbmesh-Resource-Type", "offer");
    headers.insert("Elbmesh-Resource-Id", "offer-123");
    headers.insert("Elbmesh-Stream-Type", "resource");
    headers.insert("Elbmesh-Correlation-Id", "correlation-123");
    headers.insert("Elbmesh-Causation-Id", "action-123");
    headers.insert("Elbmesh-Action-Id", "action-123");
    headers.insert("Elbmesh-Actor-Id", "actor-123");
    headers.insert("Elbmesh-Occurred-At", "2026-07-19T12:00:00Z");
    headers.insert("Elbmesh-Schema-Id", "event.offer_title_updated.v1");
    headers.insert("Elbmesh-Schema-Version", "1");
    headers.insert("Content-Type", "application/json");

    assert_eq!(header(&headers, NATS_MESSAGE_ID.as_ref()), "event-002");
    assert_eq!(header(&headers, NATS_BATCH_ID.as_ref()), "action-123");
    assert_eq!(header(&headers, NATS_BATCH_SEQUENCE.as_ref()), "2");
    assert_eq!(header(&headers, NATS_BATCH_COMMIT.as_ref()), "1");
    assert_eq!(
        header(&headers, NATS_EXPECTED_LAST_SUBJECT_SEQUENCE.as_ref()),
        "41"
    );
    assert_eq!(
        header(&headers, "Elbmesh-Aggregate-Sequence")
            .parse::<u64>()
            .expect("parse aggregate-local sequence"),
        8
    );
    for (name, expected) in [
        ("Elbmesh-Message-Type", "offer_title_updated"),
        ("Elbmesh-Message-Version", "1"),
        ("Elbmesh-Resource-Type", "offer"),
        ("Elbmesh-Resource-Id", "offer-123"),
        ("Elbmesh-Stream-Type", "resource"),
        ("Elbmesh-Correlation-Id", "correlation-123"),
        ("Elbmesh-Causation-Id", "action-123"),
        ("Elbmesh-Action-Id", "action-123"),
        ("Elbmesh-Actor-Id", "actor-123"),
        ("Elbmesh-Occurred-At", "2026-07-19T12:00:00Z"),
        ("Elbmesh-Schema-Id", "event.offer_title_updated.v1"),
        ("Elbmesh-Schema-Version", "1"),
        ("Content-Type", "application/json"),
    ] {
        assert_eq!(header(&headers, name), expected);
    }
}

#[test]
fn first_atomic_batch_message_parses_as_sequence_one_handshake() {
    let mut headers = HeaderMap::new();
    headers.insert(NATS_BATCH_ID, "action-123");
    headers.insert(NATS_BATCH_SEQUENCE, "1");
    headers.insert(NATS_REQUIRED_API_LEVEL, "4");

    assert_eq!(header(&headers, NATS_BATCH_ID.as_ref()), "action-123");
    assert_eq!(header(&headers, NATS_BATCH_SEQUENCE.as_ref()), "1");
    assert!(headers.get(NATS_BATCH_COMMIT).is_none());

    let successful_handshake_ack = b"";
    assert!(successful_handshake_ack.is_empty());
}

#[test]
fn final_atomic_batch_ack_parses_batch_identity_size_and_global_sequence() {
    let ack: PublishAck = serde_json::from_str(
        r#"{
            "stream": "ELBMESH_RESOURCES",
            "seq": 43,
            "duplicate": false,
            "batch": "action-123",
            "count": 3
        }"#,
    )
    .expect("parse NATS 2.14 atomic batch publish acknowledgement");

    assert_eq!(ack.stream, "ELBMESH_RESOURCES");
    assert_eq!(ack.sequence, 43);
    assert_eq!(ack.batch_id.as_deref(), Some("action-123"));
    assert_eq!(ack.batch_size, Some(3));
    assert!(!ack.duplicate);
}

#[test]
fn duplicate_message_rejection_ack_parses_stable_server_error() {
    let ack: serde_json::Value = serde_json::from_str(
        r#"{
            "error": {
                "code": 400,
                "err_code": 10201,
                "description": "batch publish contains duplicate message id"
            }
        }"#,
    )
    .expect("parse duplicate-message error acknowledgement");

    assert_eq!(ack["error"]["code"], 400);
    assert_eq!(ack["error"]["err_code"], 10201);
    assert_eq!(
        ack["error"]["description"],
        "batch publish contains duplicate message id"
    );
}

#[test]
fn storage_adr_defines_native_stream_subject_header_and_cursor_contracts() {
    let adr = workspace_file("docs/adr/0005-nats-streams-and-message-metadata.md");

    assert_contains_all(
        &adr,
        &[
            "ELBMESH_RESOURCES",
            "ELBMESH_ACTIONS",
            "ELBMESH_OPERATIONS",
            "ELBMESH_REACTIONS",
            "elbmesh.resources.>",
            "elbmesh.actions.>",
            "elbmesh.operations.>",
            "elbmesh.reactions.>",
            "Limits",
            "File",
            "replica",
            "2 minutes",
            "Nats-Msg-Id",
            "Elbmesh-Aggregate-Sequence",
            "Elbmesh-Message-Type",
            "Elbmesh-Message-Version",
            "Elbmesh-Resource-Type",
            "Elbmesh-Resource-Id",
            "Elbmesh-Stream-Type",
            "Elbmesh-Correlation-Id",
            "Elbmesh-Causation-Id",
            "Elbmesh-Action-Id",
            "Elbmesh-Actor-Id",
            "Elbmesh-Occurred-At",
            "Elbmesh-Schema-Id",
            "Elbmesh-Schema-Version",
            "Nats-Expected-Last-Subject-Sequence",
            "UTF-8 byte length",
            "uppercase `%XX`",
            "ELBMESH_REACTION_<encoded-reaction-type>",
            "ELBMESH_PROJECTION_<encoded-projection-type>",
            "ack floor",
            "checkpoint",
        ],
    );
}

#[test]
fn storage_adr_defines_atomic_batch_handshake_limits_and_reconciliation() {
    let adr = workspace_file("docs/adr/0005-nats-streams-and-message-metadata.md");

    assert_contains_all(
        &adr,
        &[
            "Nats-Batch-Id",
            "Nats-Batch-Sequence",
            "Nats-Batch-Commit",
            "Nats-Required-Api-Level",
            "zero-byte",
            "first message",
            "publish acknowledgement",
            "batch",
            "count",
            "1,000",
            "10 seconds",
            "abandon",
            "10201",
            "duplicate",
            "lost acknowledgement",
            "read-back reconciliation",
        ],
    );
}

fn native_stream_config(name: &str, subject: &str, atomic: bool) -> Config {
    Config {
        name: name.to_string(),
        subjects: vec![subject.to_string()],
        retention: RetentionPolicy::Limits,
        storage: StorageType::File,
        num_replicas: 1,
        duplicate_window: DUPLICATE_WINDOW,
        allow_atomic_publish: atomic,
        allow_batch_publish: false,
        ..Default::default()
    }
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> &'a str {
    headers
        .get(name)
        .unwrap_or_else(|| panic!("missing protocol header {name}"))
        .as_str()
}

fn workspace_file(relative_path: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative_path);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read workspace file '{}': {error}", path.display()))
}

fn assert_contains_all(document: &str, expected: &[&str]) {
    let missing: Vec<_> = expected
        .iter()
        .copied()
        .filter(|marker| !document.contains(marker))
        .collect();

    assert!(
        missing.is_empty(),
        "storage ADR is missing protocol commitments: {}",
        missing.join(", ")
    );
}
