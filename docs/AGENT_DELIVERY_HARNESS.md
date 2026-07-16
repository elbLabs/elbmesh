# Agent Delivery Harness

The project-local OpenCode harness coordinates one dependency-linked GitHub Issue on one branch through separate test, publication, implementation, and review roles. GitHub Issues and their explicit dependencies determine delivery order; `docs/DELIVERY_ROADMAP.md` supplies capability and milestone context. The primary `elbmesh-orchestrator` coordinates evidence only and does not implement, publish, review, or merge.

Pull request creation is automatic: after accepted red proof, the Orchestrator delegates branch creation, red commit, push, and draft pull request creation to a fresh Publisher. Within routine pull request delivery, only final review and merge require human action. No human-applied routine issue-label transition is required.

## Delivery Stage Contract

Accepted evidence is immutable. The red stage adds only focused tests or fixtures and proves the intended behavior is missing. The first Publisher creates the issue branch when needed, commits only accepted test/fixture paths in a test-only commit, pushes, opens a linked draft pull request, appends red evidence, and sets or keeps `status:implementation`. The green stage preserves accepted tests and fixtures, adds the smallest coherent implementation/docs change, and proves focused and full gates pass. A fresh Publisher creates a separate green commit containing only Implementer-reported paths. The final review stage is read-only and records findings against the complete pull request range. Rework uses fresh sessions and new append-only evidence.

All roles use the same issue branch sequentially. The Orchestrator waits for each report and always creates a fresh `elbmesh-test-writer`, `elbmesh-pr-publisher`, `elbmesh-implementer`, or `elbmesh-reviewer` session; sessions are not reused across roles or rework.

The required ordered handoffs are:

```text
Test Writer red proof
-> Publisher red test-only commit and linked draft pull request
-> Implementer green proof with accepted tests immutable
-> Publisher separate green implementation/docs commit
-> Reviewer pull request review and merge-readiness report
-> Publisher ready/status publication and URL
-> human final review and merge
```

The pull request stays draft during implementation and agent review. After `elbmesh-reviewer` reports merge readiness with no blocking findings and required CI passes, a fresh Publisher appends readiness evidence, marks the pull request ready, changes the issue to `status:review`, and returns the URL. Ready means reviewable, never merged.

## Evidence

Every role report records its role task/session ID (both role task ID and role session ID when exposed), role, issue, dependency links, branch, base/head revisions, exact changed paths, exact commands, command results, and blocker status. Later roles receive that provenance unchanged.

Red evidence includes the focused command/output, intended failure reason, exact test/fixture paths, red commit SHA, and PR URL. Green evidence includes focused pass, `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all`, exact implementation/docs paths, docs note, architecture impact, limitations, red commit SHA, green commit SHA, and PR URL. Readiness evidence includes review task ID, Reviewer session ID, reviewed range, findings, independently checked commands/results, CI state, residual risks, blocker status, both commit SHAs, and pull request URL.

Green and readiness evidence is append-only: the Publisher appends each as a new comment on both the GitHub issue and pull request without rewriting prior comments. Every cumulative comment includes all available role task IDs, role session IDs, exact changed paths, red commit SHA, green commit SHA, exact commands, command results, review task ID, blocker status, and PR URL; fields not yet available are marked pending.

Issue #147 / draft PR #148 is a bootstrap exception: its initial pull request existed before the Publisher role was introduced, so comments reconstructed publication evidence. All future runs must follow the full sequence red -> Publisher -> green -> Publisher -> Reviewer -> Publisher readiness.

## Issue Status Automation

The only active issue statuses are:

```text
status:implementation
status:review
```

The Publisher sets or keeps `status:implementation` after accepted red publication; it remains throughout tests, implementation, green publication, agent review, and rework. Only when no-blocker Reviewer evidence and required CI pass does the Publisher change the issue to `status:review` while marking the pull request ready. Test Writer, Implementer, Reviewer, and Orchestrator do not publish status changes.

GitHub merged/closed state records completion instead of a merged status label. The shell-free Orchestrator delegates status publication; no human-applied label mutation is part of routine delivery.

## Review And Merge

The Reviewer is read-only and may run only exact current-branch/PR inspection and quality commands. `elbmesh-reviewer` is the single active final pull request role that reports merge readiness or blockers. Findings return to fresh Implementer, Publisher, and Reviewer sessions.

The Publisher is non-editing. Before any push or GitHub mutation, it verifies that the current non-`main` branch matches reported task-card provenance, the pull request head matches that branch, and the target issue matches issue task-card provenance; it stops on any mismatch. It then verifies status/diffs, stages only exact role-reported paths, commits and pushes the verified current branch through generic `HEAD`, creates/updates the pull request, appends evidence, applies the two authorized issue-status changes, and marks a qualified pull request ready. It cannot merge, enable auto-merge, edit the pull request base, or push the base branch.

Human merge authority is absolute. The only human action in routine pull request delivery is final review and merge; no agent has merge authority. Readiness evidence and ready state are invitations for human review, not approval to merge.

## Permission Boundary

OpenCode permission rules constrain tool calls, not the operating-system account or hosting controls. Path-based Edit and Bash rules are broad-first and narrow-last because the last matching OpenCode rule wins. The Publisher permission set intentionally allows `git push origin HEAD`, `git push --set-upstream origin HEAD`, and broad `gh issue edit *` after the broad deny, then places direct-base, force, base-refspec, pull-request-base-edit, and merge denials later so they win.

OpenCode permissions are defense in depth, not a sandbox. GitHub branch protection, required CI, and independent review are the hard boundary for repository acceptance; exact-path status/diff checks and role instructions remain necessary. The human explicitly accepts the residual risk of wrong issue mutation created by broad issue-edit autonomy. Mandatory issue provenance preflight reduces but cannot eliminate that risk. Shell access never bypasses an Edit denial.

Direct user `@`-invocation or `@`-mention of a subagent is an out-of-band human capability that Task permissions cannot prevent. Task permissions govern agent tool calls, not the user's direct subagent selection.

## OpenCode Reload Boundary

OpenCode loads project agent, skill, permission, and other config-time definitions at startup. After merged agent/skill/config-time changes, quit and restart OpenCode before using the new harness. The session that authored or reviewed those changes keeps its pre-merge definitions and is not proof that a fresh process loaded them.
