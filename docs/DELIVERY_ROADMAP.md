# Delivery Roadmap

This roadmap organizes Elbmesh delivery by capability and dependency. GitHub Issues, including their explicit `Depends on` and `Blocks` relationships, are the delivery source of truth. This document explains direction and candidate work; it is not a second queue and does not authorize work without an issue.

## Operating Rule

```text
Choose an unblocked GitHub Issue
-> write focused tests first
-> publish a separate red commit and linked draft pull request
-> implement the smallest coherent change
-> publish a separate green commit
-> complete independent review and required CI
-> mark ready for human review
-> human reviews and merges
```

Delivery uses red, green, and review **stages**. These are evidence-producing steps for one issue, not roadmap gates.

## Source Of Truth And Dependency Order

- The GitHub Issue is the task card and records acceptance criteria, non-goals, quality gates, and architecture context.
- Explicit issue dependencies determine ordering. An issue may start only when its `Depends on` issues are resolved or the dependency is explicitly changed.
- A pull request links or closes exactly the issue whose accepted scope it implements unless the Orchestrator records an explicit split.
- This roadmap supplies capability context and candidates. If it disagrees with an expanded issue task card, ADR, or recorded human decision, stop and resolve the conflict rather than guessing.
- Independent issues may run concurrently only when their edit surfaces and architecture contracts do not conflict.

## Active Issue Statuses

Exactly two issue status labels are active:

```text
status:implementation
status:review
```

`status:implementation` remains on the issue throughout test authoring, red publication, implementation, green publication, agent review, and any rework. The Publisher sets or keeps it after accepted red publication. The Publisher changes it to `status:review` only while marking the pull request ready, after a no-blocker Reviewer report and all required CI evidence pass. GitHub merged/closed state records completion; there is no completion status label.

## Global Quality Gates

Every implementation issue must preserve tests-before-implementation and pass:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

Additional gates:

```text
Changed behavior has focused tests.
Accepted tests and fixtures remain immutable during implementation.
Public/runtime failures use named error types.
Domain Action errors implement ActionFailure where relevant.
Resource/Action/Event/Reaction/View boundaries remain explicit.
Resource Events remain separate from execution journals and provider diagnostics.
Architecture, vocabulary, workflow, and generated documentation stay synchronized.
Red tests and green implementation/docs retain separate commit provenance.
No unplanned refactor or speculative abstraction is included.
Only a human may merge; no agent enables auto-merge.
```

## Capability And Milestone Checkpoints

A review checkpoint is triggered when a coherent capability milestone becomes demonstrable, when a dependency boundary is about to change, or when accumulated debt could invalidate the next capability. Checkpoints are not scheduled by a fixed count of delivered roadmap areas.

Each checkpoint should answer:

```text
Can a human understand the current runtime or architecture flow?
Can behavior be demonstrated without reading source code?
Do tests cover important success, rejection, failure, and recovery paths?
Which debt or ambiguity affects the next dependent capability?
Do adapter and tooling observations match the logical model?
```

Useful checkpoint artifacts include a flow diagram or timeline, failure-mode matrix, test-coverage matrix, technical-debt register, human-readable demonstration run, and decision list. Existing checkpoint documents retain their original names and text as historical records.

## Capability Dependency Map

| Capability | Depends On | Useful Scope | Milestone Evidence |
| --- | --- | --- | --- |
| Repository and agent delivery rails | Goal, glossary, ADRs | Issue templates, role boundaries, tests-first publication, CI, human-only merge | One issue completes red/green/review publication with auditable evidence |
| Typed Resource runtime | Delivery rails | `Resource`, `Action`, `Event`, `Apply`, `Handle`, `ActionExecutor`, `ActionContext`, in-memory store, typed errors, receipts | Given/when/then scenarios prove replay, rejection, append, metadata, and version behavior |
| Execution journals and runtime visibility | Typed Resource runtime | Action/Operation/Reaction journals, logical traces, failure modes, idempotency debt | Domain facts and execution records are visibly separate under success and failure |
| Architecture manifest and checks | Stable vocabulary and typed runtime contracts | Resource, Component, Action, Event, Reaction, View, Query, Policy, and External Operation definitions and validation | Invalid ownership, undeclared operations, and invalid graphs fail with named errors |
| Reference business flow | Typed runtime and manifest contracts | Offer, Sales Order, Order Confirmation, Invoice, typed behavior and manifest fixture | Each Resource has scenarios and cross-Resource progress uses Events and Reactions |
| Reaction runtime | Events, Action execution, journal identity | Typed Event-to-Action subscriptions, deterministic downstream action IDs, dispatch, retry | Reactions invoke Actions without direct Resource mutation and retries are auditable |
| Views and Queries | Stable Resource Events and Reaction flow | ViewStore, projections, declared indexes and query primitives | Views rebuild from Resource Events and Queries target declared Views/indexes |
| NATS adapters | Stable in-memory port contracts | EventStore and journal adapters, KV ViewStore, metadata/key contracts, feature-gated harness | NATS adapters pass reusable contracts without changing domain behavior |
| External Operations and Restate | ActionContext, OperationJournal, adapter contracts | typed provider boundary, idempotency, mocked provider, durable append recovery | External success followed by append failure retries without duplicate provider call or Event |
| Generated capabilities and bindings | Manifest/schema stability | capability JSON/Markdown, Rust binding stubs, manifest hash, generator version, drift checks | Human and machine artifacts derive from one manifest and remain synchronized |
| CLI and agent architecture tooling | Manifest, generated capabilities, stable checks | `check-architecture`, `explain-flow`, guided add commands, skill consistency checks | Agents can inspect and change capability without inferring architecture from source alone |

## Candidate Work By Capability

Candidates are planning aids. They become deliverable only when represented by an expanded, dependency-linked GitHub Issue.

### Delivery Rails

```text
Keep canonical skill contracts and concrete project-local skills synchronized.
Keep role authority explicit and mechanically checked; treat raw OpenCode permission patterns as defense in depth rather than a sandbox.
Keep publication evidence append-only and red/green commits distinct.
Retire stale queue/status/process instructions when ADRs change.
```

### Typed Runtime And Journals

```text
Harden multi-Event record/apply behavior and replay validation.
Keep typed Action failures and named runtime failures stable.
Resolve journal idempotency and partial-commit semantics explicitly.
Keep logical execution visibility independent of provider storage overlays.
```

### Manifest And Reference Flow

```text
Validate one Action target Resource and one Event owner Resource.
Validate declared External Operations and acyclic Reaction graphs.
Keep an Offer-to-Invoice fixture understandable and scenario-tested.
Add architecture reports only when they expose enforceable contracts.
```

### Reactions, Views, And Adapters

```text
Prove deterministic Reaction identity and retry behavior.
Prove View rebuildability and declared index queries.
Run shared adapter contracts against in-memory and NATS implementations.
Keep NATS and Restate feature-gated from default local development.
```

### External Operations, Generation, And Tooling

```text
Prove provider idempotency through OperationJournal-backed retry.
Generate synchronized capability Markdown, JSON, and binding stubs.
Add architecture drift checks before guided mutation commands.
Expose flow explanations that traverse Action, Event, Reaction, External Operation, View, and Query boundaries.
```

## Current And Open Work

- Issue #121 replaces queue instructions with this dependency-ordered roadmap, synchronizes agent skills, and automates the two-status lifecycle through the Publisher.
- Open capability work must be selected from unblocked GitHub Issues, not inferred from this document's order.
- Before opening dependent implementation work, inspect issue dependencies, accepted ADRs, current capability evidence, and unresolved debt/checkpoint decisions.
- Historical checkpoint artifacts remain evidence of what was known at their recorded time; they do not govern current issue ordering.

## Queue Record

For each active issue, the Orchestrator coordinates and passes forward:

```text
issue and dependency links
capability and milestone context
branch, base, and head provenance
role task/session IDs
accepted red evidence and immutable paths
green verification and exact changed paths
review findings and blocker state
Publisher evidence links and pull request URL
human review/merge outcome
```

The Orchestrator remains shell-free and delegates publication and both automatic issue-status changes to the Publisher. No human-applied routine label transition is part of delivery. The human performs final review and merge only, except when an explicit semantic conflict requires the documented Human Decision Loop before work can safely resume.
