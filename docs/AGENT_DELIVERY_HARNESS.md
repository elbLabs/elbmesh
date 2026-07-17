# Agent Delivery Harness

This document covers OpenCode-specific mechanics only. Delivery policy, statuses, evidence, gates, and merge authority are defined once in [DEVELOPMENT_WORKFLOW.md](DEVELOPMENT_WORKFLOW.md).

## Delegation

The primary `elbmesh-orchestrator` has Edit and Bash denied. Task starts with a broad deny, then allows only fresh `elbmesh-operations`, `elbmesh-test-writer`, `elbmesh-pr-publisher`, `elbmesh-implementer`, and `elbmesh-reviewer` sessions.

Operations is non-editing and cannot spawn nested tasks. Its Bash allowlist is limited to `gh issue create/view`, `git fetch`, and `git worktree list/add`; label-at-create, forced worktree reuse, and branch reset are denied.

The Publisher is also non-editing. It may publish exact role-reported paths and delivery state, but cannot force-push, target the base branch, change the PR base, merge, or enable auto-merge.

## Permission Limits

OpenCode permissions are defense in depth, not a sandbox. Instructions and provenance checks still matter, while branch protection, required CI, independent review, and human merge are the repository acceptance boundary.

Direct user `@`-invocation or `@`-mention of a subagent is an out-of-band human capability that Task permissions cannot prevent.

## Existing Exception

Issue #147 / draft PR #148 is a bootstrap exception: its initial pull request existed before the Publisher was introduced. All future runs follow the red -> Publisher -> green -> Publisher sequence.

## Reload Boundary

OpenCode reads project agent, skill, permission, and other config-time definitions at startup. After merged agent/skill/config-time changes, quit and restart OpenCode; the authoring session still uses its previously loaded configuration.
