# Restate Adapter Test Harness

Restate support is feature-gated; default tests require no live runtime.

## Features

| Feature | Purpose |
| --- | --- |
| `restate-adapter` | Compile Restate adapter code |
| `restate-tests` | Enable integration tests and include `restate-adapter` |

Without `ELBMESH_RESTATE_URL`, live tests are optional and do not connect. Use `-- --nocapture` so their explicit `ELBMESH_RESTATE_URL is not set; skipping Restate integration test` output makes clear that the live contract was not executed. An optional skip is not live proof.

## Commands

Default and harness-only checks:

```bash
cargo test --all
cargo test -p elbmesh-core --features restate-tests --test restate_harness
```

Optional local unavailable check:

```bash
env -u ELBMESH_RESTATE_URL cargo test -p elbmesh-core --features restate-tests --test operation_journal -- --nocapture
```

This command stays green for infrastructure-free development but prints the skip for the unavailable Restate-backed contract. That contract was not executed.

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

## Required CI Mode

The dedicated `Live Restate` job starts the Compose `restate` service and sets the ingress, admin, and service-advertise values. It fails before execution when `ELBMESH_RESTATE_URL` is empty or when listing `restate_live_*` tests in `operation_journal` yields zero tests. It then runs the complete binary without a filter:

```bash
cargo test -p elbmesh-core --features restate-tests --test operation_journal
```

No failure is allowed to continue. The required `Rust CI` aggregate also fails unless `Live Restate` succeeds, so only a successful provisioned run is publication-readiness evidence. Actual execution and required-check status can be proven only by the GitHub Actions run after the branch is pushed.

## Boundary

The harness starts the Rust SDK endpoint, registers it through the Admin API, and drives `RestateOperationJournal` through ingress. The virtual object is `ElbmeshOperationJournal`, keyed by `operation_id`.

Resource Events remain in Resource streams; OperationJournal state remains in Restate; provider diagnostics and generated visibility artifacts remain outside Resource Events.
