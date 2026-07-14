# Agent Skills

Elbmesh should be agentically usable. Agents should not infer the architecture from source files alone. They should use explicit skills, docs, task cards, tests, and architecture checks.

This file is the canonical skill catalog. Concrete project-local opencode skills live under `.opencode/skills/` and must stay aligned with this catalog until generation/checking exists.

## Skill Contract

Every skill should include:

```text
Purpose
When to use it
Inputs required
Files to read first
Files it may edit
Required outputs
Required verification
Architecture rules it must preserve
```

Required reading for all Elbmesh skills:

```text
docs/GOAL.md
docs/GLOSSARY.md
docs/DEVELOPMENT_WORKFLOW.md
docs/HUMAN_DECISION_LOOP.md
docs/PHASED_DELIVERY_PLAN.md
docs/IMPLEMENTATION_PLAN.md
docs/adr/
```

When generated capability docs exist, agents must also read:

```text
RESOURCE_CAPABILITIES.md
resource-capabilities.json
architecture.manifest.json
```

## Core Skill Set

### elbmesh-orchestrator

Purpose: own phases, GitHub Issue task cards, automatic PR/MR publication handoffs, dependencies, and multi-agent sequencing.

Use when:

```text
Starting or continuing implementation work.
Selecting the next phase-scoped MR.
Creating GitHub Issues from the phased plan.
Spawning implementation agents.
Coordinating parallel work.
Resolving scope, dependency, or quality-gate conflicts.
Creating Human Decision Requests when progress needs human judgment.
```

Outputs:

```text
Active phase
Task card
GitHub Issue
PR/MR queue entry
Agent assignment
Dependency notes
Quality gates
Merge readiness state
Draft and ready pull request state
Human decision requests
```

Must preserve:

```text
No implementation outside a planned phase.
No implementation without a GitHub Issue.
No PR/MR without tests or a test plan.
No parallel work on conflicting modules or traits.
No unplanned refactors.
No silent architecture decisions when human input is required.
No agent merge; merge authority remains human-only.
```

### elbmesh-driver

Purpose: define the next implementation slice and coordinate the test-first loop.

Use when:

```text
Starting a new feature or architecture slice.
Breaking a large goal into tasks.
Resolving conflict between tests, docs, and implementation.
```

Outputs:

```text
Task card
Acceptance criteria
Architecture context
Test plan
Documentation update plan
```

Must preserve:

```text
Smallest useful slice.
Tests before implementation.
No hidden architecture changes.
One active implementation direction.
```

### elbmesh-test-writer

Purpose: write failing tests before implementation.

Use when:

```text
A task card has acceptance criteria.
New Resource/Action/Event behavior is planned.
New adapter behavior is planned.
An architecture rule needs enforcement.
```

Outputs:

```text
Scenario tests
Contract tests
Integration tests where appropriate
Architecture-rule tests where possible
```

Preferred test forms:

```text
Given Events -> When Action -> Then Events
Given Events -> When Action -> Then typed error
Given operation journal state -> When retry -> Then no duplicate external call
Given Event -> When Reaction runs -> Then Action is invoked once
```

Must not:

```text
Implement production behavior just to make tests pass.
Assert only string errors when typed errors are available.
Hide missing architecture decisions inside test fixtures.
```

### elbmesh-pr-publisher

Purpose: publish accepted role handoffs as an auditable draft-to-ready pull request without modifying files or merging.

Use when:

```text
Accepted red proof needs a branch, test-only commit, push, and linked draft pull request.
Accepted green proof needs a separate implementation/docs commit and push.
A no-blocker review and passing CI allow the pull request to become ready.
Role evidence must be appended to the issue and pull request.
```

Outputs:

```text
Issue branch and pushed revisions
Separate red test and green implementation/docs commits
Linked draft pull request
Append-only role evidence in the pull request and issue
Ready pull request after no-blocker review and required CI
Pull request URL and residual risks
```

Must preserve:

```text
No repository file modifications.
Only exact role-reported paths are staged after status/diff verification.
Red and green provenance remains distinct and immutable.
No shell separators, redirection, broad staging, or unreported paths.
OpenCode permissions are defense in depth, not a sandbox.
No merge operation or base-branch push; only a human may review and merge.
```

### elbmesh-implementer

Purpose: implement the smallest production change that satisfies failing tests.

Use when:

```text
Tests and fixtures exist and are accepted by the Orchestrator.
The target slice is clear.
The work belongs to the active phase and task card.
```

Outputs:

```text
Production code
Updated docs when behavior or architecture changed
Verification results
```

Accepted tests and fixtures are immutable to Implementers. Implementer outputs must exclude supporting test fixtures.

If an accepted test or fixture conflicts with the task card or architecture, the Implementer reports the conflict to the Orchestrator for human confirmation. Only after human confirmation may a fresh Test Writer revise accepted tests or fixtures; the Implementer must not revise them.

Must preserve:

```text
Explicit trait impls for behavior.
No domain behavior hidden behind macros.
No external calls outside declared External Operations.
Replay/apply stays deterministic.
Resource event streams contain only Resource Events.
Accepted tests and fixtures stay immutable to Implementers.
No unplanned refactors or speculative abstractions.
```

### elbmesh-reviewer

Purpose: review changes for correctness, architecture drift, and missing tests.

Use when:

```text
Implementation claims a task is complete.
An ADR or architecture rule may be affected.
Infrastructure behavior changes.
```

Outputs:

```text
Findings ordered by severity
Missing tests
Architecture-rule violations
Documentation drift
Residual risks
```

Must check:

```text
Resource/Action/Event boundaries.
Typed errors and receipts.
Expected version handling.
Journal/Event separation.
External operation idempotency.
View rebuildability.
Docs/index updates.
```

### elbmesh-mr-reviewer

Purpose: review phase-scoped MRs and report merge readiness after quality gates pass. A human performs the merge and retains all merge authority.

Use when:

```text
An implementation agent marks an MR ready.
An MR needs final architecture and Rust quality review.
An MR needs a merge-readiness or change-request recommendation.
```

Outputs:

```text
Findings ordered by severity
Gate pass/fail status
Merge-readiness report
Required follow-up tasks
Residual risks
```

Must check:

```text
MR matches task card and phase.
MR links or closes its GitHub Issue.
Tests exist for changed behavior.
All verification commands passed or limitation is documented.
Errors are named and typed where required.
No unplanned behavior or refactor is included.
Docs and skills are updated when needed.
```

### elbmesh-doc-maintainer

Purpose: keep docs, ADRs, plans, and generated docs aligned.

Use when:

```text
A decision is made.
Vocabulary changes.
Build order changes.
Agent workflow changes.
Generated docs are introduced or updated.
```

Outputs:

```text
New or updated ADR
Updated glossary
Updated implementation plan
Updated README/index
Updated open questions
```

Rules:

```text
Docs are source for architecture decisions.
Generated docs are not edited manually once generation exists.
Markdown and machine-readable docs must share manifest hash and generator version.
```

### elbmesh-architecture-checker

Purpose: verify implementation against the architecture rules.

Use when:

```text
Before marking a task complete.
Before generated capability docs are trusted.
Before an agent claims an architectural change is safe.
```

Checks:

```text
Every Action targets exactly one Resource.
Every Event belongs to exactly one Resource.
Every Action version has exactly one registered handler.
Every Event version has explicit Apply logic.
External HTTP calls appear only through declared External Operations.
Replay/apply code does not call external systems.
Reactions invoke Actions rather than mutating Resource state directly.
Views only derive from Events.
Schemas and generated docs are in sync with the manifest.
```

This skill should eventually become the `elbmesh check-architecture` CLI command.

### elbmesh-flow-explainer

Purpose: explain how an Action or Event flows through the system.

Use when:

```text
A human or agent needs to understand consequences.
A review needs to inspect downstream behavior.
A new Action/Event/Reaction is added.
```

Outputs should answer:

```text
Which Resource does the Action target?
Which Policies apply?
Which Events may be recorded?
Which Reactions subscribe?
Which downstream Actions may run?
Which External Operations are used?
Which Views are updated?
Which Queries expose the result?
```

This skill should eventually become the `elbmesh explain-flow` CLI command.

### elbmesh-manifest-editor

Purpose: safely update the architecture manifest and generated contract surfaces.

Use when:

```text
Adding or changing Resources, Components, Actions, Events, Reactions, Views, Queries, Policies, or External Operations.
```

Outputs:

```text
Manifest update
Schema update
Generated binding update or stub plan
Capability docs update
Architecture check results
```

Must preserve:

```text
Schema IDs and versions.
One Action target Resource.
One Event owner Resource.
Declared External Operations.
Generated docs stay in sync.
```

## Skill Packaging

Project-local opencode skill files:

```text
.opencode/skills/elbmesh-driver/SKILL.md
.opencode/skills/elbmesh-orchestrator/SKILL.md
.opencode/skills/elbmesh-test-writer/SKILL.md
.opencode/skills/elbmesh-pr-publisher/SKILL.md
.opencode/skills/elbmesh-implementer/SKILL.md
.opencode/skills/elbmesh-reviewer/SKILL.md
.opencode/skills/elbmesh-mr-reviewer/SKILL.md
.opencode/skills/elbmesh-doc-maintainer/SKILL.md
.opencode/skills/elbmesh-architecture-checker/SKILL.md
.opencode/skills/elbmesh-flow-explainer/SKILL.md
.opencode/skills/elbmesh-manifest-editor/SKILL.md
```

Do not hand-maintain generated skill files once generation exists.

Until generation exists, update this file and the matching `.opencode/skills/*/SKILL.md` file together.

## Definition Of Agentically Usable

The repo is agentically usable when an agent can:

```text
Find the goal and architecture rules quickly.
Pick the right skill for a task.
Locate the active phase and MR scope.
Read a task card and write failing tests first.
Implement explicit behavior without violating boundaries.
Run tests and architecture checks.
Update docs or explain why no docs changed.
Explain the resulting Action/Event/Reaction/View flow.
```
