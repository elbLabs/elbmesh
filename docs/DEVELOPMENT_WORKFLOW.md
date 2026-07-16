# Development Workflow

Elbmesh delivery is dependency-ordered, GitHub-Issue-based, test-first, documentation-backed, and agent-friendly. GitHub Issues and their explicit dependencies are the delivery source of truth; [Delivery Roadmap](DELIVERY_ROADMAP.md) supplies capability and milestone context without acting as a second queue.

The concrete agent skill catalog is maintained in [Agent Skills](AGENT_SKILLS.md). Human semantic decisions and exceptional conflict handling use the [Human Decision Loop](HUMAN_DECISION_LOOP.md).

## Principles

```text
Architecture decisions are explicit.
Every implementation slice starts from tests.
Behavior remains explicit in Rust.
Docs and tests are part of the definition of done.
Agents should not infer architecture from source files alone.
No issue and no accepted red proof means no implementation.
Issue dependencies, not roadmap grouping, determine delivery order.
```

The framework implementation follows its own modelling rules:

```text
Resource = event-sourced aggregate root.
Action = command/capability targeting one Resource.
Event = stored domain fact in one Resource stream.
Reaction = Event -> Action edge, never direct Resource mutation.
View = rebuildable materialized read model.
Query = declared read capability against a View.
External calls = declared External Operations only.
```

## Delivery Source Of Truth

The expanded GitHub Issue is the task card. It records the capability context, explicit `Depends on` and `Blocks` links, acceptance criteria, tests to write first, non-goals, quality gates, documentation impact, and architecture rules. Work may run concurrently only when issues are unblocked and their edit surfaces do not conflict.

One implementation issue maps to one pull request unless the Orchestrator records an explicit split. Every pull request links or closes its issue. A roadmap candidate does not authorize implementation until an issue carries the complete task contract.

## Agent Roles

### Orchestrator Agent

Skill: `elbmesh-orchestrator`

The Orchestrator coordinates issue dependencies, role handoffs, evidence, and queue visibility. It remains shell-free and does not implement, publish, review, mutate GitHub state, or merge.

Responsibilities:

```text
Select the next unblocked GitHub Issue by explicit dependencies.
Confirm acceptance criteria, non-goals, capability context, and quality gates.
Spawn fresh Test Writer, PR Publisher, Implementer, and Reviewer sessions in order.
Keep parallel work independent.
Pass immutable role reports and provenance forward without rewriting them.
Reject unplanned implementation and refactors.
Use the Human Decision Loop only for genuine semantic conflicts.
Delegate both automatic issue-status changes to the Publisher.
```

### Driver Agent

Skill: `elbmesh-driver`

The Driver shapes the smallest coherent issue slice. It identifies dependencies, architecture rules, acceptance criteria, first tests, non-goals, and documentation impact. It does not authorize implementation without accepted red proof.

### Test Writer Agent

Skill: `elbmesh-test-writer`

The Test Writer translates the expanded issue into focused failing tests before implementation. It prefers given/when/then Resource scenarios, typed errors, reusable adapter contracts, and architecture-rule tests. It changes only tests and test fixtures, does not implement production behavior, and returns exact red proof.

Example:

```text
Given historical Resource Events
When an Action is executed
Then these Resource Events are appended
And this Receipt is returned
And these separate journal records exist
```

### PR Publisher Agent

Skill: `elbmesh-pr-publisher`

The Publisher owns branch, commit, push, pull request, evidence-comment, ready-state, and issue-status publication. It never authors repository files and never merges.

Responsibilities:

```text
Inspect status and diffs before every publication action.
Stage only exact paths from the preceding role report.
Publish a test-only red commit and linked draft pull request.
Set or keep status:implementation after accepted red publication.
Publish a separate green implementation/docs commit.
Append cumulative evidence to the issue and pull request without rewriting prior evidence.
After no-blocker Reviewer evidence and required CI pass, mark the pull request ready and change the issue to status:review.
Return the pull request URL and residual risks.
Never merge, enable auto-merge, or push the base branch; only a human may merge.
```

Before any push or GitHub mutation, the Publisher verifies that the current non-`main` branch matches reported task-card provenance, the pull request head matches that branch, and the target issue matches issue task-card provenance; it stops on any mismatch. The verified branch is published through generic `git push origin HEAD` or `git push --set-upstream origin HEAD`, never through a hardcoded issue branch or typed helper.

The Publisher's OpenCode Bash permissions permit that generic fast path and broad `gh issue edit *` autonomy. They are defense in depth, not a sandbox. Instructions retain the exact paired status operations and prohibit shell separators, redirection, broad staging, scripts, unreported paths, direct literal base-branch pushes, force pushes, base refspec pushes, pull request base edits, and every merge mechanism.

The human explicitly accepts the residual risk of wrong issue mutation from broad issue-edit autonomy. Mandatory provenance preflight reduces but cannot eliminate that risk. GitHub branch protection, required CI, and independent review are the hard boundary for repository acceptance.

### Implementer Agent

Skill: `elbmesh-implementer`

The Implementer writes the smallest production/configuration/documentation change that satisfies the accepted failing tests. Accepted tests and fixtures are immutable to Implementers: they must not change, modify, edit, or write them. Implementer outputs must exclude supporting test fixtures.

If an accepted test or fixture conflicts with the task card or architecture, the Implementer stops and reports the conflict to the Orchestrator for human confirmation. Only after that confirmation may a fresh Test Writer revise the accepted test or fixture; the Implementer must not revise it.

Responsibilities:

```text
Preserve documented vocabulary and boundaries.
Implement behavior through explicit traits.
Keep replay/apply deterministic and free of external calls.
Keep Resource Events separate from execution journals.
Avoid unrelated refactors and speculative abstractions.
Run the focused test and every required quality gate.
Return exact changed paths, command results, docs impact, architecture impact, limitations, and blockers.
```

### Reviewer Agent

Skill: `elbmesh-reviewer`

`elbmesh-reviewer` performs the single active final pull request review and reports merge readiness or blockers after checking correctness, architecture, tests, documentation, evidence, and required gates. It remains read-only. A human performs the merge and retains all merge authority.

### Compatibility MR Reviewer Skill

`elbmesh-mr-reviewer` is an optional compatibility/manual deep-review skill and is not an additional required stage. It does not own or report merge readiness; only `elbmesh-reviewer` reports final pull request merge readiness in the canonical delivery flow. A human performs every merge.

## Issue Delivery Loop

Use **stage** for the red, green, and review steps below.

1. The Orchestrator selects an unblocked expanded GitHub Issue and records dependency/capability context.
2. A fresh Test Writer writes focused failing tests and returns exact red proof.
3. The Orchestrator accepts red proof only when it fails for the intended missing behavior.
4. A fresh Publisher creates the issue branch when needed, publishes only accepted tests/fixtures in a test-only commit, pushes, opens a linked draft pull request, appends red evidence, and sets or keeps the implementation status.
5. A fresh Implementer preserves accepted tests and fixtures and returns focused and full green proof for the smallest coherent change.
6. A fresh Publisher publishes only reported implementation/docs paths in a separate commit and appends cumulative green evidence.
7. A fresh `elbmesh-reviewer` reviews the complete pull request and reports final merge readiness or blockers without changing files.
8. Blocking findings return to fresh Implementer, Publisher, and Reviewer sessions with new append-only evidence.
9. Only after no-blocker Reviewer evidence and required CI pass, a fresh Publisher appends readiness evidence, marks the pull request ready, changes the issue to the review status, and returns the URL.
10. A human performs final review and merge. No agent merges or enables auto-merge.
11. GitHub merged/closed state records completion; no human-applied completion label transition exists.

Tests remain before implementation, accepted tests stay immutable, red and green changes remain in separate commits, every role and rework handoff uses a fresh session, and evidence remains append-only.

## Issue Status Contract

The only active status labels are:

```text
status:implementation
status:review
```

The implementation status remains throughout test authoring, red publication, implementation, green publication, agent review, and rework. The review status means the pull request is ready for final human review. The Publisher, not a human, applies these routine issue-label transitions because it verifies the publication evidence. Test Writer, Implementer, Reviewer, and shell-free Orchestrator remain non-publishing.

## Capability And Milestone Checkpoints

Schedule a higher-level review checkpoint when a coherent capability becomes demonstrable, before a dependency boundary changes, or when accumulated debt could invalidate dependent work. There is no fixed cadence based on a count of roadmap groups.

A checkpoint should answer:

```text
Can a human understand the runtime or architecture flow?
Can behavior be demonstrated without reading source code?
Do tests cover success, rejection, failure, and recovery?
What debt or ambiguity affects the next dependent capability?
Do infrastructure and tooling observations match the logical model?
```

Checkpoint artifacts may include a flow diagram, failure-mode matrix, test-coverage matrix, technical-debt register, demonstration run, and decision list. Existing named checkpoint records remain historical evidence and are not current delivery gates.

## Pull Request Requirements

Every pull request includes:

```text
GitHub Issue and dependency/capability context
tests added or changed
separate red test and green implementation/docs commit provenance
implementation summary
exact verification commands and results
documentation update or explicit no-docs-needed note
architecture-rule impact note
known limitations and follow-up issues
```

No pull request includes unrelated cleanup. If cleanup is needed, create a dependency-linked issue.

### Enforced Pull Request Gates

Pull requests targeting `main` run required Rust CI:

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

GitHub blocks the normal merge path until required CI passes and an independent approval exists. Repository enforcement does not replace local Implementer verification, independent Reviewer findings, Publisher readiness checks, or human-only merge authority.

## Human Decision Handling

Routine agent delivery does not ask a human to apply labels, confirm test handoffs, or approve intermediate publication. The planned human interaction in the pull request flow is final review and merge.

If a genuine domain, architecture, scope, or accepted-test conflict prevents safe execution, stop the delivery run and use `docs/HUMAN_DECISION_LOOP.md`. Record the answer in the issue and ADR when applicable, revise acceptance criteria, and resume with fresh role sessions. This exception does not grant an agent authority to change accepted tests, silently choose semantics, or merge.

## Task Card Template

```markdown
# Task: <short name>

## Goal

<What capability or framework behavior should exist?>

## Dependency And Capability Context

- Depends on:
- Blocks:
- Capability/milestone:
- Relevant ADRs and glossary terms:
- Affected crates/modules/docs:

## Acceptance Criteria

- Given ... When ... Then ...

## Tests To Write First

- Focused unit/scenario/contract/integration test:
- Architecture-rule test:

## Non-Goals

- <What must not be solved in this issue?>

## Quality Gates

- cargo fmt --check
- cargo clippy --all-targets --all-features -- -D warnings
- cargo test --all
- named errors for public/runtime failures
- docs updated or no-docs-needed explained

## Documentation Updates

- ADR/glossary/workflow/roadmap/capability docs impact:
```

## Test Strategy

Use scenario tests for Resource behavior, reusable contract tests for ports and adapters, integration tests for NATS/Restate/providers, and architecture tests for boundaries agents might violate.

The key External Operation recovery proof remains:

```text
External API succeeds.
Resource Event append fails once.
Execution retries the append.
External API is not called twice.
Resource Event is recorded exactly once.
```

## Rust And Architecture Quality Rules

```text
Use named error enums or typed error traits at framework boundaries.
Avoid raw String errors and anyhow in core public/runtime boundaries.
Domain Action errors implement ActionFailure with stable codes.
Keep handlers explicit and route execution through ActionExecutor/ActionContext.
An Action targets and appends Events to exactly one Resource stream.
Replay/apply uses Resource Events only and never calls external systems.
External calls use declared External Operations.
Reactions invoke Actions rather than mutating Resources directly.
Views derive from Events and remain rebuildable.
Add abstraction only for an existing boundary, adapter, or real duplication.
```

## Documentation And Configuration Rules

```text
Changed architecture decision -> add or supersede an ADR and update the index.
Changed vocabulary -> update GLOSSARY.md.
Changed capability ordering -> update DELIVERY_ROADMAP.md and issue dependencies.
Changed agent/developer process -> update workflow, harness, catalog, agents, and concrete skills together.
Generated docs are not manually edited once generation exists.
Generated Markdown and JSON share manifest hash and generator version.
```

OpenCode loads project agents, skills, and configuration at startup. After merged agent, skill, or other config-time changes, quit and restart OpenCode before relying on the new contract; the running pre-merge session does not hot-reload it.
