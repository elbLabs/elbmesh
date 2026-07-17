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
4. A fresh Publisher publishes a test-only red commit and linked draft pull request.
5. A fresh Implementer produces focused and full green proof without changing accepted tests.
6. A fresh Publisher publishes a separate implementation/docs commit and appends green evidence.
7. A fresh Reviewer reports findings and merge readiness against the complete pull request.
8. Blocking findings repeat Implementer, Publisher, and Reviewer with fresh sessions.
9. After a no-blocker review and required CI, a fresh Publisher appends readiness evidence, marks the PR ready, and moves the issue to review.
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

Red, green, and readiness evidence is append-only on both the issue and pull request. It carries role task/session IDs, exact changed paths, red and green commit SHAs, exact commands/results, review task ID, blocker status, and PR URL. Later evidence adds fields without rewriting earlier comments.

## Required Gates

Every implementation issue runs its focused test plus:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

`cargo test --all` is the infrastructure-independent local gate. Pull requests also run dedicated `Live NATS` and `Live Restate` jobs. Each live job provisions its named service, requires the corresponding URL, rejects an empty live-test selection, and runs the complete live contract binary without a test-name filter. A required `Rust CI` aggregate fails unless Rust Quality and both live jobs succeed, so a live adapter failure blocks publication readiness and merge.

Feature-gated live tests remain optional for local development. When their URL is absent, run them with `-- --nocapture` to expose the explicit `skipping ... integration test` message; that result means the live contract was not executed and is not live proof. Required CI mode never treats that optional path as proof: a missing URL or zero selected live tests fails before the unfiltered contract binary runs.

Tests precede implementation; accepted tests remain immutable; red and green commits stay separate; public/runtime errors are named; unrelated refactors and speculative abstractions stay out.

Architecture changes must preserve the rules in [GOAL.md](GOAL.md) and vocabulary in [GLOSSARY.md](GLOSSARY.md). Changed decisions add or supersede an ADR; changed vocabulary updates the glossary; changed capability dependencies update the roadmap and issue links; generated artifacts change only through their generator.

Use [HUMAN_DECISION_LOOP.md](HUMAN_DECISION_LOOP.md) only for genuine semantic conflicts, not routine handoffs or labels.

OpenCode loads agent, skill, and config-time files at startup. After merged changes to those files, quit and restart OpenCode before relying on the new contract.
