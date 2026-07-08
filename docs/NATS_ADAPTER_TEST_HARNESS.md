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

## Adapter Boundaries

Phase 7 adapters must preserve the existing runtime boundaries:

```text
Resource Events stay in Resource streams.
ActionJournal records stay in ActionJournal streams.
ReactionJournal records stay in ReactionJournal streams.
View documents stay in ViewStore keys.
```

No NATS subject or key encoder is introduced in Phase 7.1. The first adapter MR that persists records to NATS must add contract tests for subject/key escaping before relying on that encoding.

## ActionJournal KV Keys

The Phase 7.4 NATS ActionJournal adapter stores records in a dedicated KV bucket, separate from Resource Event streams.

ActionJournal stream keys use:

```text
action.<action-id-byte-length>.<percent-encoded-action-id>
```

Only ASCII letters, digits, `_`, and `-` remain unescaped in the encoded token. All other bytes are encoded as uppercase `%XX`, so dots and NATS wildcards cannot change the KV key token structure.

## ViewStore KV Keys

The Phase 7.5 NATS ViewStore adapter stores each `ViewDocument` in a dedicated KV bucket, separate from Resource Event streams and Journal streams.

View document keys use:

```text
view.<view-type-byte-length>.<percent-encoded-view-type>.<view-id-byte-length>.<percent-encoded-view-id>
```

Index-prefix queries scan current KV documents and derive index membership from each document's latest payload. This preserves overwrite semantics without storing separate stale index entries.
