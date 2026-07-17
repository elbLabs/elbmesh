# ADR 0014: Use Phased MR-Based Multi-Agent Delivery

Status: Superseded by ADR 0017

Date: 2026-07-04

## Context

Elbmesh is intended to be built by a small team of humans and coding agents. The work must remain planned, test-first, reviewable, and high-quality as the framework grows from typed core to NATS, Restate, external operations, reactions, views, generated docs, and agent tooling.

Uncoordinated agents can easily create drift:

```text
tests added after implementation
architecture changes without ADRs
stringly typed errors
hidden external calls
over-abstracted code
unplanned refactors
docs that do not match code
```

## Decision

Build Elbmesh through phased, MR-based, multi-agent delivery.

Use these roles:

```text
Orchestrator: owns phases, task cards, MR queue, and sequencing.
Test Writer: writes failing tests first for a task card.
PR Publisher: creates the issue branch, publishes separate red and green commits, opens the linked draft PR, appends evidence, and marks it ready without editing files or merging.
Implementation Agent: implements one planned MR at a time.
Reviewer (`elbmesh-reviewer`): performs the single active final PR review, requests changes, and reports merge readiness after gates pass; a human performs the merge and retains all merge authority.
Doc Maintainer: keeps ADRs, glossary, plans, and skills aligned.
Architecture Checker: verifies architecture rules before completion.
```

`elbmesh-mr-reviewer` remains an optional compatibility/manual deep-review skill. It is not an additional required stage and does not own or report merge readiness; only `elbmesh-reviewer` owns the canonical final PR merge-readiness report.

All work must belong to an explicit phase and task card before implementation starts.

Every MR must include:

```text
task card reference
phase reference
tests
implementation
verification results
documentation update or explicit no-docs-needed note
architecture-rule impact note
```

## Quality Gates

An MR cannot be merged unless:

```text
tests exist for the changed behavior
all tests pass
Rust formatting passes
Clippy passes with warnings denied once configured
public/runtime errors are named error types
domain errors implement ActionFailure where relevant
no unplanned behavior or refactor is included
docs are updated when architecture or workflow changes
Resource/Action/Event/Reaction/View boundaries are preserved
```

## Rust Quality Rules

Use named errors by default.

Rules:

```text
Core public APIs expose named error enums or typed error traits.
Avoid returning raw String errors from framework APIs.
Avoid anyhow in core framework boundaries.
Use thiserror or equivalent for named error types.
Domain Action errors implement ActionFailure and expose stable error codes.
Keep abstractions where they protect boundaries or enable adapters.
Do not add abstraction only because future flexibility is imaginable.
Handlers stay explicit and use ActionExecutor/ActionContext instead of custom execution plumbing.
```

## Consequences

The development process becomes part of the architecture.

Agents can work in parallel only when the Orchestrator has created independent task cards and MR scopes.

The canonical Reviewer readiness report and the human merge are separate from implementation.

Pull request publication is automatic after accepted role handoffs, while every merge remains a human action.

Unplanned work is rejected even if technically correct.

The project can scale through phases without losing code quality or architectural coherence.

## Rejected Approach

Do not let implementation agents pick arbitrary next work from the full roadmap.

Do not merge changes that pass tests but violate the phase plan, architecture rules, error-quality rules, or documentation requirements.
