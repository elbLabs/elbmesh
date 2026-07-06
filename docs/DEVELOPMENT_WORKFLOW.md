# Development Workflow

This document defines how Elbmesh should be built. The workflow is intentionally phased, MR-based, test-first, documentation-backed, and agent-friendly.

The concrete agent skill catalog is maintained in [Agent Skills](AGENT_SKILLS.md). The roles in this workflow map directly to those skills.

The implementation phases are maintained in [Phased Delivery Plan](PHASED_DELIVERY_PLAN.md). Work should not start unless it belongs to an active phase and GitHub Issue task card.

Human decision gates are defined in [Human Decision Loop](HUMAN_DECISION_LOOP.md). The Orchestrator should ask the human only for domain, priority, scope, and architecture decisions.

## Principles

```text
Architecture decisions are explicit.
Every implementation slice starts from tests.
Behavior remains explicit in Rust.
Docs and tests are part of the definition of done.
Agents should not infer architecture from source files alone.
No implementation work happens outside a planned phase, GitHub Issue, and PR/MR.
```

The framework implementation should follow its own modelling rules:

```text
Resource = event-sourced aggregate root.
Action = command/capability.
Event = stored domain fact.
Reaction = Event -> Action edge.
View = materialized read model.
Query = declared read capability.
```

## Agent Roles

### Orchestrator Agent

Skill: `elbmesh-orchestrator`

The Orchestrator owns phases, GitHub Issue task cards, PR/MR queue, and sequencing.

Responsibilities:

```text
Read the phased delivery plan before assigning work.
Select the active phase and next smallest Issue.
Create GitHub Issues with acceptance criteria and quality gates.
Spawn Test Writer and Implementation Agents only for planned work.
Keep parallel work independent.
Track dependencies, issue status, PR/MR status, verification, review, and merge state.
Reject unplanned implementation or refactor work.
Create Human Decision Requests when a domain or architecture decision blocks progress.
```

The Orchestrator is not the same as the Implementation Agent. It coordinates the team and keeps the roadmap coherent.

### Driver Agent

Skill: `elbmesh-driver`

The Driver owns the slice plan.

Responsibilities:

```text
Read current ADRs, goal, glossary, implementation plan, and workflow.
Define the smallest useful implementation slice.
Write a task card with acceptance criteria.
Identify architecture rules the slice must preserve.
Assign or request test-writing work before implementation.
Resolve conflicts between tests, docs, and ADRs.
Keep exactly one implementation direction active.
```

The Driver should not let implementation start until the expected behavior is captured in tests or an explicit test plan.

### Test Agent

Skill: `elbmesh-test-writer`

The Test Agent writes failing tests first.

Responsibilities:

```text
Translate the task card into executable tests.
Prefer given/when/then event-sourcing scenarios.
Assert emitted Events, typed errors, metadata, versions, and journals where relevant.
Add regression tests for architecture rules.
Avoid implementing production behavior beyond minimal test scaffolding.
Report what is still untestable and why.
```

Example test shape:

```text
Given historical Resource Events
When an Action is executed
Then these Resource Events are appended
And this Receipt is returned
And these journal records exist
```

### Implementation Agent

Skill: `elbmesh-implementer`

The Implementation Agent writes the smallest production code that satisfies the tests.

Responsibilities:

```text
Preserve the documented vocabulary and boundaries.
Implement behavior through explicit traits.
Do not hide domain behavior behind macros.
Do not change tests unless the Driver confirms the test is wrong.
Keep implementation minimal and slice-focused.
Run the required verification commands.
```

### Review Agent

Skill: `elbmesh-reviewer`

The Review Agent checks correctness, architecture fit, and documentation drift.

Responsibilities:

```text
Review code against ADRs and architecture rules.
Check that tests prove the intended behavior.
Check that docs were updated if architecture changed.
Look for hidden external calls, replay impurity, cross-Resource mutation, and journal/event mixing.
Confirm generated or derived docs remain in sync when generation exists.
```

### MR Reviewer/Merger Agent

Skill: `elbmesh-mr-reviewer`

The MR Reviewer/Merger reviews complete MRs and merges only after all gates pass.

Responsibilities:

```text
Review the MR against its task card and phase.
Verify tests were written for the changed behavior.
Verify named errors and Rust quality rules.
Run or inspect required verification commands.
Request changes for unplanned work, missing tests, or architecture drift.
Merge only when the MR satisfies quality gates.
Record residual risks and follow-up tasks.
```

## MR Loop

Every implementation slice should become one GitHub Issue and one PR/MR unless the Orchestrator explicitly splits it.

Follow this loop:

1. Orchestrator selects the active phase and creates a GitHub Issue task card.
2. Test Writer writes failing tests or a precise test plan.
3. Orchestrator confirms tests match the architecture intent.
4. Implementation Agent makes tests pass with minimal production code.
5. Implementation Agent opens a PR/MR with verification results and links the issue.
6. Review Agent or MR Reviewer/Merger reviews behavior, architecture rules, Rust quality, and docs.
7. Implementation Agent addresses requested changes.
8. MR Reviewer/Merger merges only when all gates pass.
9. Orchestrator updates phase status, open questions, and next task dependencies.

## MR Requirements

Every MR must include:

```text
phase reference
GitHub Issue reference
tests added or changed
implementation summary
verification commands and results
documentation update or explicit no-docs-needed note
architecture-rule impact note
known limitations or follow-up tasks
```

An MR must not include unplanned refactors or unrelated cleanup. If cleanup is needed, the Orchestrator creates a separate GitHub Issue.

## GitHub Issue Rules

GitHub Issues are the operational queue.

Rules:

```text
No issue, no implementation.
No failing tests or explicit test plan, no implementation.
One implementation issue maps to one PR/MR unless the Orchestrator explicitly splits it.
Every PR/MR closes or links its issue.
Labels track phase, status, agent role, and quality needs.
Decision blockers use status:blocked and needs:human-decision.
```

## Human Decision Gates

Ask the human when work needs semantic or strategic judgment:

```text
phase priority
Resource vs Component boundary
Action/Event naming
source of truth and freshness
External Operation semantics
Policy outcome semantics
scope conflict
architecture trade-off
review escalation
demo checkpoint
```

Do not ask the human for routine implementation issues already decided by ADRs.

Human questions must use the decision request format from `docs/HUMAN_DECISION_LOOP.md`.

## Task Card Template

```markdown
# Task: <short name>

## Goal

<What capability or framework behavior should exist?>

## Architecture Context

- Relevant ADRs:
- Relevant glossary terms:
- Affected crates/modules:

## Acceptance Criteria

- Given ... When ... Then ...
- Given ... When ... Then ...

## Tests To Write First

- Unit/scenario tests:
- Integration tests:
- Architecture-rule tests:

## Non-Goals

- <What must not be solved in this slice?>

## Quality Gates

- cargo fmt --check
- cargo clippy --all-targets --all-features -- -D warnings
- cargo test --all
- named errors for public/runtime failure paths
- docs updated or no-docs-needed explained

## Documentation Updates

- ADR needed: yes/no
- Glossary update needed: yes/no
- Implementation plan update needed: yes/no
- Capability docs update needed: yes/no
```

## Test Strategy

Use layered tests.

### Scenario Tests

Use for Resource behavior.

```text
Given Events
When Action
Then Events or typed error
```

These should be the default tests for `Handle<Action>` and `Apply<Event>` behavior.

### Contract Tests

Use for framework ports and adapters.

Examples:

```text
EventStore append/stream contract
expected version conflict contract
ActionJournal idempotency contract
ExternalOperation retry contract
ViewStore get/list-by-index contract
```

Each adapter should pass the same contract tests where possible.

### Integration Tests

Use for NATS, Restate, and mocked external APIs.

The key external-operation test must prove:

```text
External API succeeds.
Resource Event append fails once.
Restate retries the append.
External API is not called twice.
Resource Event is recorded exactly once.
```

### Architecture Tests

Use for rules that agents might violate.

Examples:

```text
Action targets exactly one Resource.
Event belongs to exactly one Resource.
Replay/apply code does not call external systems.
External HTTP calls happen only through declared External Operations.
Reactions call Actions rather than mutating Resources directly.
```

Some architecture tests can be static checks later. Until the CLI exists, they should be documented review checks or Rust tests where possible.

## Rust Quality Rules

Rust code should remain explicit, named, and stable.

Rules:

```text
Use named error enums or typed error traits for framework boundaries.
Avoid raw String errors at public/runtime boundaries.
Avoid anyhow in core framework public boundaries.
Domain Action errors implement ActionFailure and expose stable error codes.
Use thiserror or equivalent for named errors.
Keep handlers explicit and route execution through ActionExecutor/ActionContext.
Add abstractions only where they protect a boundary, support an adapter, or remove real duplication.
Do not add speculative abstraction.
Do not do unplanned refactors inside feature MRs.
```

## Documentation Rules

Docs must stay close to the code.

Rules:

```text
New or changed architecture decision -> add or update an ADR.
Changed vocabulary -> update GLOSSARY.md.
Changed build order or slice scope -> update IMPLEMENTATION_PLAN.md.
Changed agent/developer process -> update DEVELOPMENT_WORKFLOW.md.
Generated docs must not be edited manually once generation exists.
Generated Markdown and JSON must include manifest hash and generator version.
```

Documentation drift is a defect.

Definition of done includes:

```text
Tests pass.
Formatting and lint gates pass or current limitation is documented.
Docs are updated or explicitly not needed.
ADR index is updated if an ADR was added.
Open questions are updated if a decision remains unresolved.
MR was reviewed and merged by a non-implementing agent.
```

## First Slice Recommendation

Start with the typed core before NATS and Restate:

```text
Resource trait
Action trait or marker
Event trait or marker
Apply<Event>
Handle<Action>
ActionContext with record_applied
In-memory EventStore
ActionScenario given/when/then tests
typed Action errors
```

This gives the multi-agent loop a stable base before infrastructure complexity enters.
