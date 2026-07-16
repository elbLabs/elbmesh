# NATS Adapter Test Harness

NATS support is feature-gated; default tests require no broker.

## Features

| Feature | Purpose |
| --- | --- |
| `nats-adapter` | Compile NATS adapter code |
| `nats-tests` | Enable NATS integration tests and include `nats-adapter` |

Without `ELBMESH_NATS_URL`, gated tests report a skip instead of connecting.

## Commands

Default and harness-only checks:

```bash
cargo test --all
cargo test -p elbmesh-core --features nats-tests --test nats_harness
```

Live local contracts:

```bash
docker compose up -d nats
ELBMESH_NATS_URL=nats://127.0.0.1:4222 cargo test -p elbmesh-core --features nats-tests --test event_store_contract
ELBMESH_NATS_URL=nats://127.0.0.1:4222 cargo test -p elbmesh-core --features nats-tests --test action_journal
ELBMESH_NATS_URL=nats://127.0.0.1:4222 cargo test -p elbmesh-core --features nats-tests --test operation_journal
ELBMESH_NATS_URL=nats://127.0.0.1:4222 cargo test -p elbmesh-core --features nats-tests --test reaction_journal
ELBMESH_NATS_URL=nats://127.0.0.1:4222 cargo test -p elbmesh-core --features nats-tests --test view_store
docker compose down
```

A configured but unreachable URL is a failure; an unset URL is a deliberate skip.

## Storage Boundaries

Resource Events, ActionJournal, OperationJournal, ReactionJournal, and ViewStore documents use separate KV buckets. Journals and Views never become Resource replay input.

## Key Formats

| Store | Key |
| --- | --- |
| Resource Event stream | `resource.<type-length>.<type>.<id-length>.<id>` |
| ActionJournal | `action.<id-length>.<id>` |
| OperationJournal | `operation.<id-length>.<id>` |
| ReactionJournal | `reaction.<id-length>.<id>` |
| ViewStore | `view.<type-length>.<type>.<id-length>.<id>` |

Tokens leave ASCII letters, digits, `_`, and `-` unescaped; every other byte uses uppercase `%XX`. Length prefixes plus encoding prevent dots and NATS wildcards from changing token structure.

View index queries scan current documents and derive membership from each latest payload, so overwrites cannot leave observable stale index entries.
