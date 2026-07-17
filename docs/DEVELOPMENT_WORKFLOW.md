# Development Workflow

This is the canonical delivery policy. [DELIVERY_ROADMAP.md](DELIVERY_ROADMAP.md) provides capability context; GitHub Issues and their explicit dependencies determine work order. OpenCode mechanics live in [AGENT_DELIVERY_HARNESS.md](AGENT_DELIVERY_HARNESS.md).

## Source Of Truth

An expanded GitHub Issue is the task card. It must contain dependencies, capability context, acceptance criteria, tests to write first, non-goals, quality gates, documentation impact, and architecture constraints. Use the repository's implementation or documentation issue template rather than copying a template here.

Independent issues may run concurrently only when dependencies are resolved and edit surfaces do not conflict. One implementation issue normally maps to one branch, worktree, and pull request.

## Roles

| Role | Owns | Must not do |
| --- | --- | --- |
| Orchestrator | Dependency selection, handoffs, provenance, blockers | Edit files, use Bash, publish, review, or merge |
| Operations | Create an exact task-card issue; list/fetch/add an isolated worktree | Edit files, mutate existing issues/PRs, label, commit, push, merge, delete branches, remove worktrees, or spawn tasks |
| Test Writer | Focused failing tests and red proof | Implement production behavior |
| Publisher | Branch publication, commits, push, PR/evidence/status state | Author files, change PR base, force-push, or merge |
| Implementer | Smallest code/config/docs change that satisfies accepted tests | Change accepted tests or fixtures |
| Reviewer | Read-only final PR findings and merge-readiness report | Fix files, publish state, or merge |

`elbmesh-reviewer` reports final pull request merge readiness or blockers. Merge authority remains human: a human performs final review and merge, and no agent merges or enables auto-merge.

## Delivery Sequence

1. The Orchestrator selects an unblocked issue or receives a complete task card.
2. Operations creates/verifies the issue and isolated worktree when either is missing.
3. A fresh Test Writer produces focused red proof.
4. A fresh Publisher publishes a test-only red commit and linked draft pull request, appends a red-stage audit delta to the issue, and creates the current pull request summary.
5. A fresh Implementer produces focused and full green proof without changing accepted tests.
6. A fresh Publisher publishes a separate implementation/docs commit, appends a green-stage audit delta to the issue, and refreshes the pull request summary.
7. A fresh Reviewer reports findings and merge readiness against the complete pull request, then produces an evidence-backed Human Review Briefing.
8. Blocking findings repeat Implementer, Publisher, and Reviewer with fresh sessions; each publication appends only its new issue-stage delta and refreshes the pull request summary.
9. After a no-blocker review and required CI, a fresh Publisher appends a readiness-stage audit delta to the issue, places the Reviewer briefing at the top of the pull request summary, marks the PR ready, and moves the issue to review.
10. A human reviews and merges. GitHub merged/closed state records completion.

Pull request creation and routine issue/worktree/status setup are automatic. The only planned human action in routine delivery is final review and merge.

## Immutable Tests

Accepted tests and fixtures are immutable to Implementers. If they conflict with the task card or architecture, the Implementer stops and reports the conflict to the Orchestrator for human confirmation. Only after confirmation may a fresh Test Writer revise them. Implementer output must exclude supporting test fixtures.

## Status And Evidence

Only two active issue statuses exist:

```text
status:implementation
status:review
```

The Publisher keeps implementation status from red publication through rework. It moves the issue to review only when the Reviewer reports no blockers and required CI passes. GitHub merged/closed state replaces a completion label.

The issue is the immutable audit trail. Red, green, rework, and readiness evidence is append-only there as one stage-specific delta per publication. Each delta records only that stage's role task/session IDs, exact changed paths, commit SHA, exact commands and concise results, blocker status, and PR URL. Readiness also records the review task, reviewed range, findings, CI state, and residual risks. A later delta links to earlier evidence instead of repeating it.

The pull request is the current human review surface. Its body is concise and updated in place at every publication stage with the current state, scope, changed paths, commits, verification summary, blockers, residual risks, and links to the issue audit trail. Routine delivery evidence comments are prohibited on the pull request; pull request comments remain available for human review discussion and actionable findings.

## Human Review Briefing

The final Reviewer produces a Human Review Briefing of no more than 700 words after its findings. It contains a 60-second summary, change map, one evidence-backed Mermaid graph, architecture impact, risk map, suggested review order with file or symbol references, proof from focused tests and quality gates, approval criteria, open questions, non-goals, and residual risks. Runtime changes show the affected Action/Event path; manifest changes show ownership or dependency edges; delivery and documentation changes use a process or decision graph. Every graph edge and technical claim must be supported by the diff, manifest/capability documents, tests, or accepted role evidence.

At readiness, the Publisher places the accepted Reviewer briefing verbatim at the top of the current pull request body and fills the remaining state, commit, verification, and audit-link sections from verified publication evidence. A later accepted rework briefing replaces the earlier body briefing. The briefing is never published as a routine pull request comment.

## Required Gates

Every implementation issue runs its focused test plus:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

Tests precede implementation; accepted tests remain immutable; red and green commits stay separate; public/runtime errors are named; unrelated refactors and speculative abstractions stay out.

Architecture changes must preserve the rules in [GOAL.md](GOAL.md) and vocabulary in [GLOSSARY.md](GLOSSARY.md). Changed decisions add or supersede an ADR; changed vocabulary updates the glossary; changed capability dependencies update the roadmap and issue links; generated artifacts change only through their generator.

Use [HUMAN_DECISION_LOOP.md](HUMAN_DECISION_LOOP.md) only for genuine semantic conflicts, not routine handoffs or labels.

OpenCode loads agent, skill, and config-time files at startup. After merged changes to those files, quit and restart OpenCode before relying on the new contract.
