# ADR 0016: Use Human Decision Requests for Domain and Architecture Gates

Status: Accepted

Date: 2026-07-04

## Context

Elbmesh should be built by agents, but the human remains essential for domain semantics, product priorities, and architecture trade-offs. Without a clear human-in-the-loop mechanism, agents either ask too many low-value questions or make high-impact decisions silently.

## Decision

Use structured Human Decision Requests for decisions that need human judgment.

Human input is required for:

```text
phase priority changes
domain boundary decisions
business naming decisions
External System/source-of-truth decisions
Policy semantics
architecture trade-offs
scope conflicts
review escalations
demo checkpoints
```

Human input is not required for routine compiler errors, formatting, mechanical test fixes, or implementation details already decided by ADRs.

Every request must include:

```text
why the human is being asked
context
two or three concrete options
one recommended option
consequences
default if the human does not care
exact response requested
```

## Consequences

The human sees decisions in product/domain language instead of implementation noise.

The Orchestrator can pause work without losing context.

Decisions that affect architecture are captured in issues and ADRs.

Agents stay autonomous for routine engineering while preserving human control over meaningful direction.

## Rejected Approach

Do not ask the human open-ended questions like:

```text
What should we do now?
How should this be implemented?
Is this okay?
```

Ask option-based questions with a recommendation and consequences.
