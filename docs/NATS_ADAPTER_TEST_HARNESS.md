# NATS Adapter Test Harness

NATS support is feature-gated; default tests require no broker.

The Compose service pins the exact `nats:2.14.3-alpine` image and starts it with JetStream enabled. The native stream protocol is defined by [ADR 0005](adr/0005-nats-streams-and-message-metadata.md); changing the server version or protocol fields requires updating that decision and its contracts together.

## Features

| Feature | Purpose |
| --- | --- |
| `nats-adapter` | Compile NATS adapter code |
| `nats-tests` | Enable NATS integration tests and include `nats-adapter` |

The optional `async-nats` dependency disables default features. Its explicit features include JetStream/KV support and cumulative `server_2_10`, `server_2_11`, `server_2_12`, and `server_2_14` contracts so atomic publish configuration and NATS 2.14 batch acknowledgements remain compile-checked.

Without `ELBMESH_NATS_URL`, gated tests are optional and do not connect. Use `-- --nocapture` so their explicit `ELBMESH_NATS_URL is not set; skipping NATS integration test` output makes clear that the live contract was not executed. An optional skip is not live proof.

## Commands

Default and harness-only checks:

```bash
cargo test --all
cargo test -p elbmesh-core --features nats-tests --test nats_harness
cargo test -p elbmesh-core --features nats-tests --test nats_native_stream_protocol
```

Optional local unavailable check:

```bash
env -u ELBMESH_NATS_URL cargo test -p elbmesh-core --features nats-tests --test event_store_contract -- --nocapture
```

This command stays green for infrastructure-free development but prints a skip for every unavailable NATS-backed contract. Those contracts were not executed.

Live local contracts:

```bash
docker compose up -d nats
ELBMESH_NATS_URL=nats://127.0.0.1:4222 cargo test -p elbmesh-core --features nats-tests --test nats_harness
ELBMESH_NATS_URL=nats://127.0.0.1:4222 cargo test -p elbmesh-core --features nats-tests --test nats_native_stream_protocol
ELBMESH_NATS_URL=nats://127.0.0.1:4222 cargo test -p elbmesh-core --features nats-tests --test event_store_contract
ELBMESH_NATS_URL=nats://127.0.0.1:4222 cargo test -p elbmesh-core --features nats-tests --test action_journal
ELBMESH_NATS_URL=nats://127.0.0.1:4222 cargo test -p elbmesh-core --features nats-tests --test operation_journal
ELBMESH_NATS_URL=nats://127.0.0.1:4222 cargo test -p elbmesh-core --features nats-tests --test reaction_journal
ELBMESH_NATS_URL=nats://127.0.0.1:4222 cargo test -p elbmesh-core --features nats-tests --test view_store
docker compose down
```

A configured but unreachable URL is a failure; an unset URL is a deliberate skip. The live harness verifies both the exact server version and the JetStream capability reported by the connected server. The native protocol binary compile-checks stream fields, NATS/application headers, and acknowledgement decoding; it does not replace the live EventStore and journal adapter contracts.

## Required CI Mode

The dedicated `Live NATS` job starts the Compose `nats` service and sets `ELBMESH_NATS_URL`. It fails before execution when that URL is empty or when listing `nats_event_store_*` tests in `event_store_contract` yields zero tests. It then runs the complete binary without a filter:

```bash
cargo test -p elbmesh-core --features nats-tests --test event_store_contract
```

No failure is allowed to continue. The required `Rust CI` aggregate also fails unless `Live NATS` succeeds, so only a successful provisioned run is publication-readiness evidence. Actual execution and required-check status can be proven only by the GitHub Actions run after the branch is pushed.

## Storage Boundaries

The accepted target boundary is native JetStream messages in separate Resource Event, ActionJournal, OperationJournal, and ReactionJournal streams. View documents, View indexes, and projection checkpoints remain KV. Only Resource Events become Resource replay input.

This protocol-foundation slice does not replace the existing KV-backed EventStore or journal adapters. Their live contracts remain in place until the follow-on native adapter issues; ADR 0005 governs the subjects, headers, atomic append, sequence, deduplication, durable consumer, and reconciliation behavior those replacements must implement.

## Current KV Key Formats

| Store | Key |
| --- | --- |
| Resource Event stream | `resource.<type-length>.<type>.<id-length>.<id>` |
| ActionJournal | `action.<id-length>.<id>` |
| OperationJournal | `operation.<id-length>.<id>` |
| ReactionJournal | `reaction.<id-length>.<id>` |
| ViewStore | `view.<type-length>.<type>.<id-length>.<id>` |

Tokens leave ASCII letters, digits, `_`, and `-` unescaped; every other byte uses uppercase `%XX`. Length prefixes plus encoding prevent dots and NATS wildcards from changing token structure.

View index queries scan current documents and derive membership from each latest payload, so overwrites cannot leave observable stale index entries.
