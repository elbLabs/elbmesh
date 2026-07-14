# Phased Delivery Plan

This plan groups implementation into phases. The Orchestrator uses it to create GitHub Issue task cards and PRs/MRs. Implementation agents should not work outside the active phase unless the Orchestrator explicitly updates this plan.

## Operating Rule

```text
Plan phase -> create GitHub Issue -> write tests first -> publish red commit and draft PR/MR -> implement -> publish green commit -> review -> mark ready -> human merge -> update issue and plan.
```

Every Issue/PR pair must be small enough to review and large enough to prove one useful behavior.

## Global Quality Gates

Every phase and MR must satisfy these gates:

```text
Tests are written before implementation.
All behavior has tests.
All public/runtime errors are named errors.
Domain Action errors implement ActionFailure where relevant.
No raw String errors at framework boundaries unless wrapped in a named error.
No anyhow in core framework public boundaries.
Rust formatting passes.
Clippy passes with warnings denied once configured.
Docs are updated when architecture, workflow, or vocabulary changes.
No unplanned refactors or future-proof abstractions.
Abstractions exist only to protect a boundary, support an adapter, or remove real duplication.
```

Recommended verification commands:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

Until Clippy is configured cleanly, the MR must state the current limitation and still run `cargo test --all`.

## Review, Test, And Visualization Cadence

After every two implementation phases, run a review and demonstrability checkpoint before continuing into the next pair of phases.

Checkpoint cadence:

```text
After Phase 2: review typed core + execution journals through Phase 2.5.
After Phase 4: review manifest skeleton + reference flow fixture.
After Phase 6: review reactions + views.
After Phase 8: review NATS + Restate/external operations.
After Phase 10: review generation + CLI/agentic tooling.
```

Each checkpoint must produce or update:

```text
technical debt register
runtime or architecture flow diagrams
test coverage matrix
visual/demo run that a human can inspect
decision list for ambiguities before the next phases
```

The checkpoint is not a substitute for per-MR review. It is a higher-level review that asks whether the system is understandable, recoverable, testable, and demonstrable before adding more surface area.

## GitHub Issue Labels

Phase labels:

```text
phase:0-rails
phase:1-core
phase:2-journals
phase:2.5-visibility
phase:3-manifest
phase:4-reference-flow
phase:5-reactions
phase:6-views
phase:7-nats
phase:8-external-restate
phase:9-generation-docs
phase:10-cli-agentic
```

Status labels:

```text
status:planned
status:tests-needed
status:tests-ready
status:implementation
status:review
status:blocked
status:decision-needed
status:merged
```

Agent and quality labels:

```text
agent:orchestrator
agent:test-writer
agent:implementer
agent:reviewer
needs:docs
needs:adr
needs:architecture-check
needs:named-errors
needs:human-decision
type:docs
```

## Phase 0: Repository And Team Rails

Goal: make the repo safe for agentic development.

Status: mostly started.

Deliverables:

```text
docs/GOAL.md
docs/GLOSSARY.md
docs/DEVELOPMENT_WORKFLOW.md
docs/AGENT_SKILLS.md
docs/PHASED_DELIVERY_PLAN.md
ADRs 0001-0014
.opencode/skills/*/SKILL.md
workspace and elbmesh-core crate
```

Exit criteria:

```text
The Orchestrator can create phase-scoped task cards.
Implementation agents know the required docs and quality gates.
PR Publisher can automatically open a linked draft PR and mark it ready after review without merging.
`elbmesh-reviewer` reports final PR merge readiness after explicit review criteria pass; a human performs the merge.
Current tests pass.
```

## Phase 1: Typed Core Hardening

Goal: make the in-memory Resource/Action/Event runtime robust before infrastructure adapters.

Scope:

```text
Resource trait
Action trait
Event trait
Apply<Event>
Handle<Action>
ActionExecutor
ActionContext
InMemoryEventStore
ActionScenario
typed Action errors
message metadata
Receipts
```

MR candidates:

```text
MR 1.1: record_applied multi-event behavior and tests
MR 1.2: replay multiple historical Events and tests
MR 1.3: wrong-resource Event rejection and tests
MR 1.4: receipt metadata and emitted event assertions
MR 1.5: core error audit, replacing ad-hoc strings with named errors where needed
```

Exit criteria:

```text
Core runtime behavior is covered by scenario tests.
Every public core failure path uses named errors.
No NATS, Restate, Reactions, Views, or External Operations are introduced yet.
```

## Phase 2: Execution Journals

Goal: separate Resource Events from execution/audit/recovery records.

Scope:

```text
ActionJournal trait
InMemoryActionJournal
ActionCalled
ActionCompleted
ActionRejected
ActionFailed
journal metadata
ActionExecutor journal integration
```

MR candidates:

```text
MR 2.1: ActionJournal contract tests and in-memory implementation
MR 2.2: successful Action journals called/completed records
MR 2.3: rejected Action journals called/rejected records and no Resource Event
MR 2.4: failed runtime Action journals failed record with named error classification
```

Exit criteria:

```text
Resource event streams remain clean.
Action execution is auditable through journals.
Journals have named record types and typed errors.
```

## Phase 2.5: Runtime Visibility, Debt Review, And Demonstrability

Goal: make the typed core and execution journal runtime explainable before adding manifests, reference flows, infrastructure adapters, or durable external operations.

Scope:

```text
current runtime flow model
technical debt register
failure mode matrix
logical ExecutionTrace model
visualization/test strategy
review checkpoint for Phases 1 and 2
GitHub Issue sequencing for later phases
```

MR candidates:

```text
MR 2.5.1: document runtime visibility model and two-phase review cadence
MR 2.5.2: document technical debt register and failure mode matrix
MR 2.5.3: trace-model test plan for logical visibility before implementation
MR 2.5.4: demonstrability plan for Offer-based examples and human review runs
```

Exit criteria:

```text
The current runtime can be explained as success, rejection, and failure timelines.
Known debt is visible before NATS, Restate, or generation work begins.
The plan distinguishes logical framework visibility from later NATS/Restate overlays.
Every subsequent pair of phases has a review/test/visualization checkpoint.
No NATS, Restate, code generation, or CLI implementation is introduced in this phase.
```

## Phase 3: Manifest Skeleton And Architecture Checks

Goal: define the architecture-as-code contract before generation.

Scope:

```text
ArchitectureManifest
ResourceDefinition
ComponentDefinition
ActionDefinition
EventDefinition
ReactionDefinition
ViewDefinition
QueryDefinition
ExternalOperationDefinition
Manifest validation errors
```

MR candidates:

```text
MR 3.1: manifest structs and schema-version fields
MR 3.2: validation for one Action target Resource and one Event owner Resource
MR 3.3: validation for Reaction graph cycle rejection
MR 3.4: validation for declared External Operations
MR 3.5: first architecture-check report format
```

Exit criteria:

```text
Architecture rules are testable against a manifest fixture.
Validation failures use named errors.
No code generation yet.
```

## Phase 4: Reference Flow Fixture

Goal: create a generated-like Offer-to-Invoice fixture that proves framework ergonomics.

Scope:

```text
Offer
Sales Order
Order Confirmation
Invoice
Create Offer
Accept Offer
Create Sales Order
Create Order Confirmation
Create Invoice
corresponding Events
typed errors
manual manifest fixture
```

MR candidates:

```text
MR 4.1: Offer Resource with Create/Accept behavior
MR 4.2: Sales Order Resource with Create behavior
MR 4.3: Order Confirmation Resource with Create behavior
MR 4.4: Invoice Resource with Create behavior
MR 4.5: manifest fixture for the reference flow
```

Exit criteria:

```text
Each Resource has scenario tests.
The fixture looks like generated code but remains handwritten.
No cross-Resource mutation is performed directly.
```

## Phase 5: Reactions And Workflow Graphs

Goal: implement typed Event-to-Action Reactions as the workflow execution primitive.

Scope:

```text
ReactionDefinition
ReactionRuntime
ReactionJournal
Event subscription matching
pure Reaction conditions
deterministic downstream action IDs
cycle rejection from manifest validation
```

MR candidates:

```text
MR 5.1: ReactionJournal contract and in-memory implementation
MR 5.2: one Event triggers one Action through ActionExecutor
MR 5.3: one Event triggers multiple Reactions
MR 5.4: deterministic action IDs make Reaction retries idempotent
MR 5.5: connect Offer Accepted -> Sales Order Created flow
```

Exit criteria:

```text
Workflows are visible as Action -> Event -> Reaction -> Action graphs.
Reactions do not mutate Resources directly.
Failures stay in journals unless explicitly modelled as domain Events.
```

## Phase 6: Views And Queries

Goal: support materialized read models and declared query capabilities.

Scope:

```text
ViewStore trait
InMemoryViewStore
Project<Event>
projection checkpoints
get_by_id
list_by_index_prefix
View indexes
```

MR candidates:

```text
MR 6.1: ViewStore contract and in-memory implementation
MR 6.2: projection contract for one Resource Event source
MR 6.3: projection from multiple Resource types
MR 6.4: declared all index and list_by_index_prefix query
MR 6.5: Document Flow Status or Receivables View in the reference flow
```

Exit criteria:

```text
Views are rebuildable from Events.
Queries target declared Views and indexes.
No live external enrichment yet.
```

## Phase 7: NATS Adapters

Goal: replace in-memory ports with NATS-backed adapters while preserving contracts.

Scope:

```text
NATS JetStream EventStore
NATS-backed ActionJournal
NATS-backed OperationJournal later
NATS-backed ReactionJournal
NATS KV ViewStore
NATS Object Store for large payloads later
```

MR candidates:

```text
MR 7.1: NATS test harness and adapter feature flags
MR 7.2: JetStream EventStore contract tests
MR 7.3: expected version handling on NATS
MR 7.4: ActionJournal on NATS
MR 7.5: KV ViewStore contract tests
```

Exit criteria:

```text
NATS adapters pass the same contract tests as in-memory adapters.
NATS metadata contains required fields.
Resource replay remains deterministic.
```

## Phase 8: External Operations And Restate

Goal: solve durable external side effects without exposing execution mechanics as domain concepts.

Scope:

```text
ExternalOperation trait
OperationJournal
idempotency keys
mock LexOffice API
Restate execution adapter
external success then Event append failure recovery
```

MR candidates:

```text
MR 8.1: OperationJournal contract and in-memory implementation
MR 8.2: declared ExternalOperation metadata and validation
MR 8.3: mock LexOffice Create Invoice operation with idempotency
MR 8.4: ActionExecutor integration for ExternalOperation through context
MR 8.5: Restate retry proves external call not repeated after append failure
```

Exit criteria:

```text
External API succeeds and append fails once test passes.
External API is not called twice.
Resource Event is recorded exactly once.
Provider details stay in OperationJournal/Object Store, not Resource Events.
```

## Phase 9: Generation And Capability Docs

Goal: connect manifest, schemas, bindings, docs, and agent metadata.

Scope:

```text
manifest fixtures
JSON Schema contracts
generated Rust DTO/binding stubs
RESOURCE_CAPABILITIES.md
resource-capabilities.json
manifest hash
generator version
```

MR candidates:

```text
MR 9.1: manifest fixture to capability JSON
MR 9.2: capability Markdown generation
MR 9.3: schema ID/version checks
MR 9.4: generated binding stub shape
MR 9.5: docs drift check for generated outputs
```

Exit criteria:

```text
Human and machine-readable capability docs are generated from the same source.
Generated artifacts include manifest hash and generator version.
Agents can inspect available Resources, Actions, Events, Reactions, Views, and Queries.
```

## Phase 10: CLI And Agentic Tooling

Goal: make architecture changes guided and checkable.

Scope:

```text
elbmesh check-architecture
elbmesh explain-flow
add-action later
add-event later
add-projection later
skill generation/checking
```

MR candidates:

```text
MR 10.1: check-architecture reads manifest and reports named validation errors
MR 10.2: explain-flow traverses Action/Event/Reaction/View graph
MR 10.3: skill consistency check against docs/AGENT_SKILLS.md
MR 10.4: generated skill packaging proof
```

Exit criteria:

```text
Agents can run architecture checks before claiming completion.
Agents can explain an Action or Event flow without source-code guessing.
```

## Orchestrator Issue/PR Queue Rules

The Orchestrator may run multiple implementation agents only when GitHub Issues are independent.

Parallel-safe examples:

```text
one agent writes ActionJournal contract tests
one agent writes Manifest validation tests
one agent updates docs for an accepted process decision
```

Not parallel-safe examples:

```text
two agents editing ActionExecutor behavior
one agent changing traits while another implements an adapter against old traits
one agent refactoring errors while another adds new errors in the same module
```

The Orchestrator must keep the GitHub queue current with:

```text
phase
issue
pull request
owner agent
dependencies
status
verification result
review result
merge result
```
