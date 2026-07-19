# Agent Delivery Harness

This document covers OpenCode-specific mechanics only. Delivery policy, statuses, evidence, gates, and merge authority are defined once in [DEVELOPMENT_WORKFLOW.md](DEVELOPMENT_WORKFLOW.md).

## Delegation

The primary `elbmesh-orchestrator` has Edit and Bash denied. Task starts with a broad deny, then allows only fresh `elbmesh-operations`, `elbmesh-test-writer`, `elbmesh-pr-publisher`, `elbmesh-implementer`, and `elbmesh-reviewer` sessions.

Operations is non-editing and cannot spawn nested tasks. Its Bash allowlist is limited to `gh issue create/view`, `git fetch`, and `git worktree list/add`; label-at-create, forced worktree reuse, and branch reset are denied.

The Reviewer remains read-only and returns findings, merge readiness, and an evidence-backed Human Review Briefing. The Publisher is also non-editing. It may publish exact role-reported paths and delivery state, append stage-specific audit comments to the issue, and refresh the current pull request body with the accepted Reviewer briefing. Routine evidence comments on the pull request are denied. It cannot force-push, target the base branch, change the PR base, merge, or enable auto-merge.

## Safe Published-Branch Recovery

The Publisher permission frontmatter allows exactly `git pull --ff-only` for same-branch recovery. Before using it, require the working tree and index to be clean; require the current branch to be the exact non-main issue branch and its configured upstream to be the exact same-named branch; verify exact issue provenance and that the pull request head matches that branch; and, using current fetch evidence, prove local HEAD is an ancestor of the fetched upstream. Stop before any Git or GitHub mutation if the worktree or index is dirty, the refs diverged, provenance mismatches, fetched ancestry is unverified or cannot be verified, or a fast-forward cannot be proved. After the pull, verify that the local, upstream, and pull request head commits are equal before any further mutation.

All broad `git pull` forms, pull arguments or refspecs, merge, reset, rebase, checkout, switch, force, base publication, pull-request base changes, auto-merge, and merge remain denied. Permission to run the exact fast-forward command does not authorize resolving divergence or selecting another branch.

## Accepted-Test Correction Recovery

When a Reviewer reports an accepted test defect as a blocker, the Orchestrator obtains explicit human confirmation before revision and starts a fresh Test Writer to check whether valid semantic red exists. Missing non-test behavior uses the canonical red/green flow. Only already-correct non-test behavior whose corrected tests pass immediately may produce an explicitly named test-contract correction with authorized paths, old/new hashes, passing proof, and why semantic red is impossible. Passing test-contract correction proof is never red proof.

The Publisher creates one separate test-only correction commit, appends one non-cumulative correction-stage issue delta, refreshes the current draft pull request body, and keeps `status:implementation` without claiming red, green, readiness, or merge authority. It then hands off to a fresh Implementer for immutable-path focused and full green verification and to a fresh Reviewer for final complete-range review.

If the fresh Implementer reports zero implementation paths, the Publisher creates no empty commit and retains the earlier separate green implementation/docs commit as provenance. Zero implementation paths still require a fresh final Reviewer, required CI, readiness publication, and human-only merge.

## Permission Limits

OpenCode permissions are defense in depth, not a sandbox. Instructions and provenance checks still matter, while branch protection, required CI, independent review, and human merge are the repository acceptance boundary.

Direct user `@`-invocation or `@`-mention of a subagent is an out-of-band human capability that Task permissions cannot prevent.

## Existing Exception

Issue #147 / draft PR #148 is a bootstrap exception: its initial pull request existed before the Publisher was introduced. All future runs follow the red -> Publisher -> green -> Publisher sequence.

## Reload Boundary

OpenCode reads project agent, skill, permission, and other config-time definitions at startup. After merged agent/skill/config-time changes, quit and restart OpenCode; the authoring session still uses its previously loaded configuration.
