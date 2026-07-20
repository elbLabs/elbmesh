# Agent Skills

Elbmesh skills are first-class repository contracts. This catalog owns skill names and purposes; each concrete `.opencode/skills/<name>/SKILL.md` owns its detailed inputs, permissions, outputs, verification, and preserved architecture rules.

## Skill Contract

Every concrete skill states its trigger, required reading, permitted edit surface, required outputs, exact verification, and Resource/Action/Event/Reaction/View boundaries.

Required reading for all Elbmesh skills:

```text
docs/GOAL.md
docs/GLOSSARY.md
docs/DEVELOPMENT_WORKFLOW.md
docs/HUMAN_DECISION_LOOP.md
docs/DELIVERY_ROADMAP.md
docs/AGENT_SKILLS.md
docs/IMPLEMENTATION_PLAN.md
docs/adr/
```

Read the expanded issue, harness, manifest, capabilities, and generated artifacts when relevant.

## Catalog

### elbmesh-architecture-checker

Checks accepted changes for architecture drift and reports findings without editing.

### elbmesh-doc-maintainer

Keeps active docs, ADR indexes, templates, agent contracts, and generated-doc rules aligned.

### elbmesh-driver

Shapes the smallest dependency-linked task card with acceptance criteria, non-goals, first tests, and gates.

### elbmesh-flow-explainer

Explains an Action/Event path through Policies, Events, Reactions, External Operations, Views, Queries, and journals.

### elbmesh-implementer

Makes accepted failing tests pass with the smallest non-test change; accepted tests and fixtures remain immutable, including after a test-contract correction, and zero-path verification is reported without manufacturing a commit.

### elbmesh-manifest-editor

Changes manifest/schema sources and regenerates affected bindings and capability artifacts.

### elbmesh-mr-reviewer

Provides optional compatibility/manual deep review outside the canonical delivery sequence; it is not an additional required stage and does not report merge readiness.

### elbmesh-operations

Creates exact task-card issues and isolated worktrees through the narrow setup command allowlist.

### elbmesh-orchestrator

Coordinates dependency-ordered setup and fresh role handoffs while remaining shell-free and non-editing, including Reviewer blocker, human confirmation, and fresh Test Writer recovery decisions.

### elbmesh-pr-publisher

Publishes accepted role reports, commits, pull-request state, append-only issue audit deltas, the Reviewer-validated Human Review Briefing in a current concise pull-request body, and issue statuses without authoring files or merging. It may use only exact safe same-branch fast-forward recovery and may publish an authorized test-contract correction as its own test-only commit.

### elbmesh-reviewer

Performs the final read-only PR review, reports merge readiness or blockers (including accepted-test defects), and produces an evidence-backed Human Review Briefing; a human performs the merge.

### elbmesh-test-writer

Writes focused failing tests and fixtures before implementation, then returns exact red proof. After explicit human confirmation of a Reviewer blocker, a fresh Test Writer may instead report an authorized passing test-contract correction, never red proof.

## Synchronization

Concrete skills live under `.opencode/skills/`. The catalog headings and concrete skill directories must match. Until generation exists, change both in the same issue.

Project skills and agents are loaded at startup; see [AGENT_DELIVERY_HARNESS.md](AGENT_DELIVERY_HARNESS.md) for the OpenCode reload boundary.
