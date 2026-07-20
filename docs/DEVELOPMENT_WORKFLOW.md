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
| Test Writer | Focused failing tests/red proof; human-authorized test-contract correction | Implement production behavior |
| Publisher | Branch reconciliation/publication, commits, push, PR/evidence/status state | Author files, change PR base, force-push, or merge |
| Implementer | Smallest code/config/docs change or zero-path green verification | Change accepted tests or fixtures |
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

Accepted tests and fixtures are immutable to Implementers. Implementer output must exclude supporting test fixtures.

An Implementer-discovered accepted-test or fixture conflict with the task card or architecture must stop with the Implementer reporting it to the Orchestrator. Only after explicit human confirmation may a fresh Test Writer revise an authorized path to produce canonical semantic red followed by green; this route must not use immediately passing test-contract correction.

## Reviewer-Driven Test-Contract Correction

Immediately passing test-contract correction has one sole entry: a final Reviewer's path-specific accepted test blocker, followed by explicit human confirmation and a fresh Test Writer proving that non-test behavior is already correct and legitimate semantic red is impossible, so the corrected test would pass immediately. This final-Reviewer requirement does not make the Reviewer the sole entry to every accepted-test revision; Implementer-conflict revisions use the canonical semantic-red/green route above. Passing test-contract correction proof is never red proof.

The Publisher publishes that correction as one separate test-only commit containing only the authorized reported paths, appends one non-cumulative correction-stage issue delta, refreshes the current draft pull request body, and keeps `status:implementation`; it claims no red, green, readiness, or merge authority. A fresh Implementer then preserves the corrected accepted paths as immutable and runs focused and full green verification, followed by a fresh Reviewer of the final complete range.

When the fresh Implementer reports zero implementation paths because no non-test change is needed, no empty commit is created. The earlier separate green implementation/docs commit remains the implementation provenance. Zero implementation paths still require a fresh Reviewer for the final no-blocker report, required CI, readiness publication, and human review and merge.

If correction exposes missing non-test behavior, scope conflict, or architecture ambiguity, stop and return to the Human Decision Loop and canonical tests-first sequence rather than weakening the test.

## Safe Published-Branch Fast-Forward

The Publisher may reconcile an existing published branch only with exactly `git pull --ff-only`. Before using it, require the working tree and index to be clean; require the current branch to be the exact non-main issue branch and its configured upstream to be the exact same-named branch; verify exact issue provenance and that the pull request head matches that branch; and, using current fetch evidence, prove local HEAD is an ancestor of the fetched upstream. Stop before any Git or GitHub mutation if the worktree or index is dirty, the refs diverged, provenance mismatches, fetched ancestry is unverified or cannot be verified, or a fast-forward cannot be proved. After the pull, verify that the local, upstream, and pull request head commits are equal before any further mutation.

Broad `git pull`, pull arguments or refspecs, merge, reset, rebase, checkout, switch, force, base publication, pull-request base changes, auto-merge, and merge remain prohibited. The recovery path never resolves divergence or selects another branch.

## Status And Evidence

Only two active issue statuses exist:

```text
status:implementation
status:review
```

The Publisher keeps implementation status from red publication through rework. It moves the issue to review only when the Reviewer reports no blockers and required CI passes. GitHub merged/closed state replaces a completion label.

The issue is the immutable audit trail. Red, green, correction, rework, and readiness evidence is append-only there as one stage-specific delta per publication. Each delta records only that stage's role task/session IDs, exact changed paths, commit SHA, exact commands and concise results, blocker status, and PR URL. A correction delta also records the human authorization, old/new hashes, passing proof, and why semantic red was impossible. Readiness also records the review task, reviewed range, findings, CI state, and residual risks. A later delta links to earlier evidence instead of repeating it.

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

`cargo test --all` is the infrastructure-independent local gate. Pull requests also run dedicated `Live NATS` and `Live Restate` jobs. Each live job provisions its named service, requires the corresponding URL, rejects an empty live-test selection, and runs the complete live contract binary without a test-name filter. A required `Rust CI` aggregate fails unless Rust Quality and both live jobs succeed, so a live adapter failure blocks publication readiness and merge.

Feature-gated live tests remain optional for local development. When their URL is absent, run them with `-- --nocapture` to expose the explicit `skipping ... integration test` message; that result means the live contract was not executed and is not live proof. Required CI mode never treats that optional path as proof: a missing URL or zero selected live tests fails before the unfiltered contract binary runs.

Tests precede implementation; accepted tests remain immutable; red and green commits stay separate; public/runtime errors are named; unrelated refactors and speculative abstractions stay out.

Architecture changes must preserve the rules in [GOAL.md](GOAL.md) and vocabulary in [GLOSSARY.md](GLOSSARY.md). Changed decisions add or supersede an ADR; changed vocabulary updates the glossary; changed capability dependencies update the roadmap and issue links; generated artifacts change only through their generator.

Use [HUMAN_DECISION_LOOP.md](HUMAN_DECISION_LOOP.md) only for genuine semantic conflicts, not routine handoffs or labels.

OpenCode loads agent, skill, and config-time files at startup. After merged changes to those files, quit and restart OpenCode before relying on the new contract.
