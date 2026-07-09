# NATS Adapter Test Harness

Phase 7 NATS work is feature-gated so default test runs do not require a local NATS server.

## Feature Flags

```text
nats-adapter: compiles NATS adapter code when adapter code exists.
nats-tests: enables NATS integration tests and includes nats-adapter.
```

Default `cargo test --all` must keep passing without NATS.

## Running Tests

Run the default suite:

```bash
cargo test --all
```

Run NATS-gated harness tests:

```bash
cargo test -p elbmesh-core --features nats-tests --test nats_harness
```

Future NATS integration tests must call the shared test harness before connecting. If `ELBMESH_NATS_URL` is not set, the test should report the skip and return without failing.

Example environment:

```text
ELBMESH_NATS_URL=nats://127.0.0.1:4222
```

## Docker-Backed Local NATS

Start a local NATS server with JetStream enabled:

```bash
docker compose up -d nats
```

Run the live adapter contract tests against that server:

```bash
ELBMESH_NATS_URL=nats://127.0.0.1:4222 cargo test -p elbmesh-core --features nats-tests --test event_store_contract
ELBMESH_NATS_URL=nats://127.0.0.1:4222 cargo test -p elbmesh-core --features nats-tests --test action_journal
ELBMESH_NATS_URL=nats://127.0.0.1:4222 cargo test -p elbmesh-core --features nats-tests --test operation_journal
ELBMESH_NATS_URL=nats://127.0.0.1:4222 cargo test -p elbmesh-core --features nats-tests --test view_store
```

Stop the local server:

```bash
docker compose down
```

These live commands should fail if Docker is not running or if `ELBMESH_NATS_URL` points at no broker. A feature-gated test run without `ELBMESH_NATS_URL` still skips live adapter work by design so default development does not require NATS.

## Adapter Boundaries

Phase 7 adapters must preserve the existing runtime boundaries:

```text
Resource Events stay in Resource streams.
ActionJournal records stay in ActionJournal streams.
OperationJournal records stay in OperationJournal streams.
ReactionJournal records stay in ReactionJournal streams.
View documents stay in ViewStore keys.
```

No NATS subject or key encoder is introduced in Phase 7.1. The first adapter MR that persists records to NATS must add contract tests for subject/key escaping before relying on that encoding.

## Resource EventStore KV Keys

The NATS EventStore adapter stores each Resource stream as one JSON stream document in a dedicated KV bucket, separate from Journal streams and ViewStore keys.

Resource Event stream keys use:

```text
resource.<resource-type-byte-length>.<percent-encoded-resource-type>.<resource-id-byte-length>.<percent-encoded-resource-id>
```

Only ASCII letters, digits, `_`, and `-` remain unescaped in encoded tokens. All other bytes are encoded as uppercase `%XX`, so dots and NATS wildcards cannot change the KV key token structure.

## ActionJournal KV Keys

The Phase 7.4 NATS ActionJournal adapter stores records in a dedicated KV bucket, separate from Resource Event streams.

ActionJournal stream keys use:

```text
action.<action-id-byte-length>.<percent-encoded-action-id>
```

Only ASCII letters, digits, `_`, and `-` remain unescaped in the encoded token. All other bytes are encoded as uppercase `%XX`, so dots and NATS wildcards cannot change the KV key token structure.

## OperationJournal KV Keys

The NATS OperationJournal adapter stores records in a dedicated KV bucket, separate from Resource Event streams, ActionJournal streams, ReactionJournal streams, and ViewStore keys.

OperationJournal stream keys use:

```text
operation.<operation-id-byte-length>.<percent-encoded-operation-id>
```

Only ASCII letters, digits, `_`, and `-` remain unescaped in the encoded token. All other bytes are encoded as uppercase `%XX`, so dots and NATS wildcards cannot change the KV key token structure.

## ViewStore KV Keys

The Phase 7.5 NATS ViewStore adapter stores each `ViewDocument` in a dedicated KV bucket, separate from Resource Event streams and Journal streams.

View document keys use:

```text
view.<view-type-byte-length>.<percent-encoded-view-type>.<view-id-byte-length>.<percent-encoded-view-id>
```

Index-prefix queries scan current KV documents and derive index membership from each document's latest payload. This preserves overwrite semantics without storing separate stale index entries.
