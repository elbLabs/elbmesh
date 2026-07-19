# NATS Adapter Test Harness

NATS support is feature-gated; default tests require no broker.

## Features

| Feature | Purpose |
| --- | --- |
| `nats-adapter` | Compile NATS adapter code |
| `nats-tests` | Enable NATS integration tests and include `nats-adapter` |

Without `ELBMESH_NATS_URL`, gated tests are optional and do not connect. Use `-- --nocapture` so their explicit `ELBMESH_NATS_URL is not set; skipping NATS integration test` output makes clear that the live contract was not executed. An optional skip is not live proof.

## Commands

Default and harness-only checks:

```bash
cargo test --all
cargo test -p elbmesh-core --features nats-tests --test nats_harness
```

Optional local unavailable check:

```bash
env -u ELBMESH_NATS_URL cargo test -p elbmesh-core --features nats-tests --test event_store_contract -- --nocapture
```

This command stays green for infrastructure-free development but prints a skip for every unavailable NATS-backed contract. Those contracts were not executed.

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

## Required CI Mode

The dedicated `Live NATS` job starts the Compose `nats` service and sets `ELBMESH_NATS_URL`. It fails before execution when that URL is empty or when listing `nats_event_store_*` tests in `event_store_contract` yields zero tests. It then runs the complete binary without a filter:

```bash
cargo test -p elbmesh-core --features nats-tests --test event_store_contract
```

No failure is allowed to continue. The required `Rust CI` aggregate also fails unless `Live NATS` succeeds, so only a successful provisioned run is publication-readiness evidence. Actual execution and required-check status can be proven only by the GitHub Actions run after the branch is pushed.

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
