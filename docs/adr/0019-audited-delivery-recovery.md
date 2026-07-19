# ADR 0019: Use Audited Recovery for Published Branches and Accepted-Test Defects

Status: Accepted

Date: 2026-07-19

Extends: ADR 0017 delivery sequencing, provenance, status, and merge-authority decisions and ADR 0018 evidence-placement decisions. It does not supersede their accepted decisions.

## Context

Published issue worktrees can be safely behind their own pull-request head, but the Publisher previously had no permitted way to fast-forward a clean local branch. Separately, a Reviewer can prove that an accepted test is defective after red/green publication. When non-test behavior is already correct, manufacturing a failing test would create false red evidence, while silently changing an accepted test would break immutability and auditability.

Both cases need narrow recovery without granting general Git reconciliation, weakening tests-first delivery, or moving merge authority away from humans.

## Decision

### Same-Branch Fast-Forward

The Publisher may run exactly `git pull --ff-only` only in an existing clean worktree on the exact non-main issue branch whose configured upstream is the exact same-named branch. Exact issue provenance and pull-request head branch must match. Current fetch evidence must prove local HEAD is an ancestor of the fetched upstream. Dirty working tree or index state, divergence, unverified ancestry, a non-fast-forward result, or any provenance mismatch stops the flow before Git or GitHub mutation.

After the pull, local HEAD, configured upstream, and pull-request head must be equal before publication continues. Broad pull, pull arguments/refspecs, merge, reset, rebase, checkout, switch, force, base publication, pull-request base edits, auto-merge, and merge remain denied. The Publisher does not resolve divergence.

### Reviewer-Driven Test-Contract Correction

An accepted test or fixture may be reconsidered only when a Reviewer reports a path-specific blocker and a human explicitly confirms the defect and authorized paths. The Orchestrator then starts a fresh Test Writer to determine whether valid semantic red exists.

If missing non-test behavior can produce semantic red, the canonical separate red/green flow remains mandatory. If non-test behavior is already correct and corrected tests pass immediately, the Test Writer may change only authorized test/fixture paths and must report an explicitly named **test-contract correction** with old/new hashes, exact focused passing proof, and why semantic red is impossible. Passing correction proof is never red proof.

The Publisher publishes one separate test-only correction commit, appends one non-cumulative correction-stage issue delta, refreshes the current draft pull-request body, and keeps `status:implementation`. Correction publication claims no red, green, readiness, or merge authority.

A fresh Implementer treats corrected paths as immutable and runs focused and full green verification. If no non-test change is needed, the report states zero implementation paths and no empty commit is created; the earlier separate green implementation/docs commit remains implementation provenance. A fresh final Reviewer, required CI, readiness publication, and human review and merge remain mandatory.

## Evidence And Authority

Issue evidence remains append-only. The pull-request body remains the one current review summary. Exactly one of `status:implementation` and `status:review` remains active. No agent merges or enables auto-merge; only a human reviews and merges.

The correction path does not authorize missing behavior to bypass failing tests. Any missing non-test behavior, scope conflict, or architecture ambiguity returns to the Human Decision Loop and canonical tests-first flow.

## Consequences

- A clean behind-only issue worktree can catch up to its already-published same branch without general reconciliation authority.
- Accepted-test correction is exceptional, explicit, and distinguishable from semantic red and green implementation evidence.
- Zero-path verification does not manufacture repository history.
- Existing branch protection, required CI, independent review, append-only issue evidence, and human-only merge remain the repository acceptance boundary.
- Project-local agent, skill, and permission changes are loaded only at OpenCode startup. After this ADR and its contracts merge, quit and restart OpenCode before relying on the new permission or recovery behavior.

## Rejected Approaches

Do not grant broad pull, merge, reset, rebase, checkout, force, base mutation, PR-base edit, auto-merge, or merge permissions. Do not call immediately passing correction proof red. Do not let an Implementer revise accepted paths. Do not create an empty implementation commit. Do not skip fresh final review or required CI.
