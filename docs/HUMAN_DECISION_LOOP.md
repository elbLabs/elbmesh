# Human Decision Loop

The human should not be asked to review routine implementation details. Human input is most valuable when the team needs semantic judgment, prioritization, or an architecture trade-off.

## When To Ask The Human

Ask for human input at these gates:

```text
Phase start: confirm the next phase is still the right priority.
Task shaping: confirm business semantics and acceptance criteria for a new issue.
Domain boundary: decide Resource vs Component, Event naming, View scope, or Reaction graph shape.
External boundary: decide source of truth, freshness, External Operation semantics, or provider metadata.
Policy decision: decide whether a rule blocks, requires approval, or only warns.
Architecture trade-off: choose between minimal implementation and added abstraction.
Scope conflict: decide whether newly discovered work belongs in the current issue or a follow-up.
Review escalation: resolve disagreement between implementer and reviewer.
Demo checkpoint: confirm behavior is understandable and useful before advancing phases.
```

Do not ask the human about:

```text
routine formatting
obvious compiler errors
small internal naming choices that do not affect the model
mechanical test fixes
implementation details already decided by ADRs
```

## Decision Request Format

Every human decision request must be short, concrete, and option-based.

Use this format:

```markdown
# Decision Needed: <short title>

## Why You Are Being Asked

<One or two sentences explaining why this is a human/domain decision, not routine implementation.>

## Context

- Phase:
- Issue/PR:
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

If this is not important to you, the Orchestrator will choose Option A.

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
Avoid jargon unless it is already in the glossary.
State the default if the human does not care.
Capture the answer in the issue or ADR.
Create or update an ADR if the decision changes architecture.
```

## Examples

### Resource Boundary

```text
Decision: Is Order Confirmation a Resource or a Component of Sales Order?

Option A (Recommended): Resource
Meaning: It has its own lifecycle, events, external identity, and actions.
Consequence: Cross-document flow uses Reactions.

Option B: Component
Meaning: It only changes inside Sales Order.
Consequence: Simpler now, but harder if it later needs independent approval, download, sync, or audit.
```

### External Freshness

```text
Decision: Should Invoice status be observed_on_action or live_on_read?

Option A (Recommended for v1): observed_on_action
Meaning: Store what we observed during explicit Actions.
Consequence: Replay remains deterministic and no background sync is needed.

Option B: live_on_read
Meaning: Query LexOffice when humans/agents view the Invoice.
Consequence: More current read experience, but needs enrichment design and external read handling.
```

### Scope Control

```text
Decision: Should ActionJournal be added before Manifest validation?

Option A (Recommended): Finish ActionJournal first
Meaning: Build audit/recovery foundation before architecture checks.
Consequence: External Operations and Reactions have a place to record execution state later.

Option B: Start Manifest validation first
Meaning: Strengthen architecture checks before more runtime work.
Consequence: Slower path to external operation runtime.
```

## Human Response Handling

After the human answers, the Orchestrator must:

```text
Record the decision in the GitHub issue or PR.
Update issue labels from status:blocked/status:decision-needed to the next actionable status.
Create or update an ADR if the decision affects architecture.
Update task acceptance criteria if needed.
Tell the next assigned agent exactly what changed.
```
