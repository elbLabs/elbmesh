# ADR 0017: Use Dependency-Ordered Issue Delivery and Publisher-Managed Statuses

Status: Accepted

Date: 2026-07-15

Supersedes: ADR 0014 and ADR 0015 delivery ordering, queue-status, and label-transition decisions. Their original decision text remains as historical context.

Evidence publication on both issues and pull requests is superseded by ADR 0018. The dependency ordering, status ownership, and merge-authority decisions remain accepted.

## Context

The earlier delivery contract organized work by numbered roadmap groups, used several transient queue labels, and required a shell-free Orchestrator to ask a human for routine label changes. In practice, expanded GitHub Issues already carry the actionable acceptance criteria and dependency information. The Publisher is the role that verifies commits, pull request state, evidence, review, and CI, so it is also the narrowest role able to publish status changes safely.

Elbmesh must retain its tests-first and architecture safeguards while removing ordering and status ceremony that does not improve correctness.

## Decision

Use dependency-ordered GitHub Issue delivery.

```text
GitHub Issue and explicit dependencies = delivery source of truth
docs/DELIVERY_ROADMAP.md = capability and milestone context
Pull request = publication, review, and human merge artifact
```

There is no roadmap-group gate, label, task-card field, or pull-request reference requirement. An issue may proceed when its explicit dependencies are resolved and its acceptance criteria, tests, non-goals, quality gates, and architecture context are complete.

Use **stage** for red, green, and review delivery steps. Keep the safeguards introduced by ADR 0014:

```text
Tests are written before implementation.
Accepted tests and fixtures are immutable to Implementers.
Red tests and green implementation/docs use separate commits.
Each role and rework handoff uses a fresh session.
Evidence is append-only on the issue and pull request.
elbmesh-reviewer performs the final agent review and reports merge readiness or blockers.
A human performs final review and merge; no agent merges or enables auto-merge.
```

Use only these active issue status labels:

```text
status:implementation
status:review
```

The implementation status covers test authoring, accepted red publication, implementation, accepted green publication, agent review, and rework. The Publisher sets or keeps it after accepted red publication.

The Publisher changes the issue to the review status only while marking the pull request ready, and only after no-blocker Reviewer evidence and required CI pass. Test Writer, Implementer, Reviewer, and Orchestrator remain non-publishing. The Publisher has an explicitly broad `gh issue edit *` allowance for autonomy, while its required operational behavior still uses only complete paired transitions that remove the opposite status before adding the target status. Merge commands, auto-merge, pull request base edits, direct literal base-branch pushes, force pushes, and base refspec pushes remain denied.

Before any push or GitHub mutation, the Publisher verifies that the current non-`main` branch equals the branch in task-card provenance, the pull request head matches that branch, and the target issue matches issue task-card provenance. It stops on any mismatch. After successful preflight, the generic `git push origin HEAD` and `git push --set-upstream origin HEAD` forms provide the autonomous fast path without a typed helper or hardcoded branch.

GitHub merged/closed state records completion. No completion status label is used.

## Capability And Milestone Checkpoints

Trigger a checkpoint when a coherent capability becomes demonstrable, a dependency boundary is about to change, or accumulated debt may invalidate dependent work. A fixed delivery-count cadence is not used.

Checkpoint evidence may include a flow diagram, failure-mode matrix, coverage matrix, debt register, demonstration run, and open decision list. Historical checkpoint and ADR records retain their original wording as records of the decisions and evidence at that time.

## Human And Automation Boundary

Routine issue-status publication is automatic and owned by the Publisher. The Orchestrator remains `bash: deny` and delegates both status transitions. A human is not asked to apply routine labels or confirm test handoffs.

The normal human interaction is final review and merge. A genuine semantic conflict may stop work and invoke the Human Decision Loop; it does not authorize an agent to rewrite accepted tests or make an architecture decision silently.

The human explicitly accepts the residual risk of wrong issue mutation from broad `gh issue edit *` autonomy. Mandatory provenance preflight reduces but does not eliminate that risk. OpenCode permission patterns are defense in depth, not a sandbox; GitHub branch protection, required CI, and independent review are the hard boundary for repository acceptance.

## Consequences

- GitHub Issues and explicit dependencies determine delivery order without a duplicated queue.
- Capability/milestone checkpoints happen when evidence is useful rather than on a fixed cadence.
- The two labels express whether agents are still delivering or the pull request is ready for human review.
- The role that verifies publication evidence performs the corresponding status mutation.
- Generic current-branch publication avoids per-issue hardcoding while mandatory provenance checks stop mismatched publication.
- Broad issue-edit autonomy carries the explicitly accepted residual wrong-issue mutation risk.
- Shell-free coordination, immutable tests, separate red/green commits, independent review, CI, append-only evidence, and human-only merge remain intact.
- Project-local OpenCode agent/skill/config-time changes take effect only after the merged changes are loaded by quitting and restarting OpenCode.

## Rejected Approaches

Do not maintain a second active queue in Markdown. Do not use roadmap grouping as an authorization gate. Do not ask humans for routine label transitions. Do not grant issue mutation to the Orchestrator or to non-publishing roles. Do not infer completion from a label when GitHub already records merged/closed state.
