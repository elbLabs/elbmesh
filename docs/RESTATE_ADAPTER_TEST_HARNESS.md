# Restate Adapter Test Harness

Restate live tests are feature-gated so default test runs do not require a local Restate runtime.

## Feature Flags

```text
restate-adapter: compiles Restate adapter code.
restate-tests: enables Restate integration tests and includes restate-adapter.
```

Default `cargo test --all` must keep passing without Restate.

## Running Tests

Run the default suite:

```bash
cargo test --all
```

Run Restate-gated harness tests without a live runtime:

```bash
cargo test -p elbmesh-core --features restate-tests --test restate_harness
```

Live Restate tests must call the shared test harness before connecting. If `ELBMESH_RESTATE_URL` is not set, the test reports the skip and returns without failing.

Example environment:

```text
ELBMESH_RESTATE_URL=http://127.0.0.1:8080
ELBMESH_RESTATE_ADMIN_URL=http://127.0.0.1:9070
ELBMESH_RESTATE_SERVICE_ADVERTISE_HOST=host.docker.internal
```

`ELBMESH_RESTATE_URL` is the Restate ingress URL. `ELBMESH_RESTATE_ADMIN_URL` defaults to `http://127.0.0.1:9070`. `ELBMESH_RESTATE_SERVICE_ADVERTISE_HOST` defaults to `127.0.0.1`; set it to `host.docker.internal` when Restate runs in Docker and tests run on the host. The compose service maps `host.docker.internal` through Docker's `host-gateway` for Linux compatibility.

## Docker-Backed Local Restate

Start a local Restate server:

```bash
docker compose up -d restate
```

If another Restate server already uses admin port `9070`, override the host admin port:

```bash
ELBMESH_RESTATE_ADMIN_PORT=9071 docker compose up -d restate
```

Run the live adapter tests against that server:

```bash
ELBMESH_RESTATE_URL=http://127.0.0.1:8080 \
ELBMESH_RESTATE_ADMIN_URL=http://127.0.0.1:9070 \
ELBMESH_RESTATE_SERVICE_ADVERTISE_HOST=host.docker.internal \
cargo test -p elbmesh-core --features restate-tests --test operation_journal restate_live_operation_journal_appends_called_and_completed_records

ELBMESH_RESTATE_URL=http://127.0.0.1:8080 \
ELBMESH_RESTATE_ADMIN_URL=http://127.0.0.1:9070 \
ELBMESH_RESTATE_SERVICE_ADVERTISE_HOST=host.docker.internal \
cargo test -p elbmesh-core --features restate-tests --test action_context_external_operation live_restate_operation_journal_retry_after_append_failure_reuses_completed_external_operation
```

When using `ELBMESH_RESTATE_ADMIN_PORT=9071`, set `ELBMESH_RESTATE_ADMIN_URL=http://127.0.0.1:9071` in the test commands.

Stop the local server:

```bash
docker compose down
```

These live commands should fail if Docker is not running or if `ELBMESH_RESTATE_URL` points at no Restate ingress. A feature-gated test run without `ELBMESH_RESTATE_URL` still skips live adapter work by design.

## Adapter Boundaries

The live harness starts the elbmesh Rust SDK endpoint in-process, registers it with Restate Admin API, then drives `RestateOperationJournal` through Restate ingress.

Runtime lanes remain separate:

```text
Resource Events stay in Resource streams.
OperationJournal records stay in the Restate OperationJournal object state.
Provider diagnostics stay out of Resource Events.
Generated visibility artifacts are not written as Resource Events.
```

The Restate virtual object is named `ElbmeshOperationJournal` and is keyed by `operation_id`.
