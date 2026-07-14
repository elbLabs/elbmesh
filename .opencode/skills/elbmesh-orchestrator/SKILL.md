---
name: elbmesh-orchestrator
description: Use when coordinating Elbmesh phases, creating GitHub Issues, assigning agents, managing PR/MR queues, dependencies, and merge readiness.
---

# Elbmesh Orchestrator

Use this skill to coordinate phased, GitHub Issue and PR/MR based multi-agent delivery.

## Read First

```text
docs/GOAL.md
docs/GLOSSARY.md
docs/DEVELOPMENT_WORKFLOW.md
docs/HUMAN_DECISION_LOOP.md
docs/PHASED_DELIVERY_PLAN.md
docs/AGENT_SKILLS.md
docs/IMPLEMENTATION_PLAN.md
docs/adr/
```

## Responsibilities

```text
Select the active phase.
Create the next smallest GitHub Issue task card.
Define acceptance criteria and quality gates.
Spawn fresh Test Writer, PR Publisher, Implementer, and Reviewer sessions only for planned work.
Keep parallel tasks independent.
Maintain Issue and PR/MR queue state.
Reject unplanned implementation and refactors.
Create Human Decision Requests for domain, priority, scope, and architecture blockers.
Update phase status after observing the human merge.
```

## Delivery Sequence

```text
Test Writer produces accepted red proof.
PR Publisher creates the branch, red test-only commit, push, and linked draft PR.
Implementer preserves accepted tests and produces green proof.
PR Publisher creates and pushes the separate implementation/docs commit.
Reviewer reviews the pull request and reports blockers.
PR Publisher appends no-blocker evidence, marks the pull request ready, and reports its URL.
Human reviews and merges.
```

Use a fresh role session at every handoff and rework step. The Orchestrator remains shell-free, requests human-applied issue-label transitions, and never publishes or merges directly.

## MR Queue Entry

Track:

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
follow-up tasks
human decision requests
```

## Quality Gates

Every MR must include:

```text
tests for changed behavior
named errors for public/runtime failures
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
docs updated or no-docs-needed explained
architecture impact note
```

## Preserve

```text
No implementation outside the active phase.
No implementation without a GitHub Issue.
No PR/MR without tests or a test plan.
No parallel work on conflicting traits/modules.
No speculative abstraction.
No unplanned refactor inside feature MRs.
No silent human-level architecture decisions.
No agent performs a merge; merge authority remains human-only.
```

## Human Decision Requests

Ask the human only for decisions listed in `docs/HUMAN_DECISION_LOOP.md`.

Use option-based requests with:

```text
why the human is being asked
context
two or three options
one recommendation
consequences
default if the human does not care
```
