# Restate Adapter Test Harness

Restate support is feature-gated; default tests require no live runtime.

## Features

| Feature | Purpose |
| --- | --- |
| `restate-adapter` | Compile Restate adapter code |
| `restate-tests` | Enable integration tests and include `restate-adapter` |

Live tests skip when `ELBMESH_RESTATE_URL` is unset.

## Commands

Default and harness-only checks:

```bash
cargo test --all
cargo test -p elbmesh-core --features restate-tests --test restate_harness
```

Start Restate and run live contracts:

```bash
docker compose up -d restate

ELBMESH_RESTATE_URL=http://127.0.0.1:8080 \
ELBMESH_RESTATE_ADMIN_URL=http://127.0.0.1:9070 \
ELBMESH_RESTATE_SERVICE_ADVERTISE_HOST=host.docker.internal \
cargo test -p elbmesh-core --features restate-tests --test operation_journal restate_live_operation_journal_appends_called_and_completed_records

ELBMESH_RESTATE_URL=http://127.0.0.1:8080 \
ELBMESH_RESTATE_ADMIN_URL=http://127.0.0.1:9070 \
ELBMESH_RESTATE_SERVICE_ADVERTISE_HOST=host.docker.internal \
cargo test -p elbmesh-core --features restate-tests --test action_context_external_operation live_restate_operation_journal_retry_after_append_failure_reuses_completed_external_operation

docker compose down
```

`ELBMESH_RESTATE_ADMIN_URL` defaults to `http://127.0.0.1:9070`. If that port is occupied, start with `ELBMESH_RESTATE_ADMIN_PORT=9071` and use the matching admin URL. Use `host.docker.internal` when Restate runs in Docker and tests run on the host.

A configured but unreachable runtime is a failure; an unset ingress URL is a deliberate skip.

## Boundary

The harness starts the Rust SDK endpoint, registers it through the Admin API, and drives `RestateOperationJournal` through ingress. The virtual object is `ElbmeshOperationJournal`, keyed by `operation_id`.

Resource Events remain in Resource streams; OperationJournal state remains in Restate; provider diagnostics and generated visibility artifacts remain outside Resource Events.
