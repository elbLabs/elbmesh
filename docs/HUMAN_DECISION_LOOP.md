# Human Decision Loop

Routine agent delivery is autonomous. A human is not asked to apply issue labels, confirm red/green handoffs, or approve intermediate publication; the planned human interaction in the pull request flow is final review and merge.

This loop is an exception for semantic conflicts that agents cannot safely resolve from the expanded GitHub Issue, accepted tests, ADRs, glossary, and architecture rules. When invoked, delivery stops until the decision is recorded. It does not become a routine gate.

## When A Decision Is Required

Use a Human Decision Request only for:

```text
Business or domain semantics not fixed by the issue.
Resource versus Component ownership.
Action/Event naming with different domain meaning.
Source of truth, freshness, Policy, or External Operation semantics.
An architecture trade-off with materially different consequences.
A scope conflict that changes acceptance criteria or issue dependencies.
A conflict between accepted tests/fixtures and the task card or architecture.
A Reviewer escalation that cannot be resolved from existing contracts.
A capability/milestone checkpoint that exposes a product decision.
```

Do not ask the human about formatting, compiler errors, mechanical fixes, normal test/implementation/publication handoffs, routine issue-label mutation, or implementation details already decided by ADRs.

## Decision Request Format

Every request is short, concrete, option-based, and linked to the relevant GitHub Issue.

```markdown
# Decision Needed: <short title>

## Why You Are Being Asked

<Why this is semantic judgment rather than routine implementation.>

## Dependency And Capability Context

- Issue/PR:
- Depends on / blocks:
- Capability or milestone:
- Relevant docs/ADRs:
- Current blocker:

## Options

### Option A: <name> (Recommended)

What it means:

Consequences:

### Option B: <name>

What it means:

Consequences:

### Option C: <name>

What it means:

Consequences:

## Recommendation

Choose Option A because ...

## Default If You Do Not Care

If this is not important, the Orchestrator recommends Option A; work remains stopped until the decision is recorded.

## Decision Needed

Please answer with `A`, `B`, `C`, or a short custom answer.
```

## Decision Quality Rules

The Orchestrator must:

```text
Present at most three options by default.
Recommend one option.
Explain consequences in domain/product language first.
Mention implementation consequences second.
Use glossary vocabulary.
State what issue dependency or acceptance criterion changes.
Capture the answer in the issue and an ADR when architecture changes.
```

The Orchestrator remains shell-free. It coordinates the decision record but does not mutate GitHub state. A decision does not require a workflow status change; delivery stays in the implementation status while stopped and resumes with fresh role sessions when the recorded contract is complete.

## Accepted-Test Conflict Handling

Accepted tests and fixtures are immutable to Implementers. If one conflicts with the task card or architecture, the Implementer reports the conflict to the Orchestrator for human confirmation and stops. Only after human confirmation may a fresh Test Writer revise the accepted test or fixture; the Implementer never revises it.

## Response Handling

After the human answers the exceptional request, the Orchestrator:

```text
Confirms the decision is recorded in the GitHub Issue or pull request.
Delegates ADR creation or supersession when architecture changes.
Delegates acceptance-criteria and explicit-dependency updates when needed.
Passes the exact changed contract to fresh role sessions.
Delegates any later routine issue-status publication to the Publisher.
```

The normal final step remains human review and merge. Only a human performs a merge.
