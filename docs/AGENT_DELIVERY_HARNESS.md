# Agent Delivery Harness

The project-local OpenCode harness coordinates one GitHub issue on one branch through separate test, publication, implementation, and review roles. The primary `elbmesh-orchestrator` coordinates evidence and status only; it does not implement, publish, or review.

Pull request creation is automatic: after accepting red proof, the Orchestrator delegates branch creation, the red commit, push, and draft pull request creation to a fresh PR Publisher. Within the pull request delivery flow, only merge requires the human as a human action; issue-label transitions are separate queue controls and remain human-applied.

## Phase Contract

Each phase is immutable after the Orchestrator accepts its evidence. The red phase adds only focused tests or fixtures and ends with an intended failing test. The first Publisher creates the issue branch and a test-only red commit whose authoring provenance is the Test Writer report, pushes it, and opens a linked draft pull request. The green phase preserves accepted tests, adds the smallest implementation, and ends with focused and full quality gates passing. A fresh Publisher creates a separate green commit containing only role-reported implementation and documentation paths; its authoring provenance is the Implementer report. The review phase does not change files and records findings against the pull request range. Rework uses fresh sessions and new evidence rather than rewriting accepted reports or commits.

All roles use the same issue branch sequentially. The Orchestrator waits for one role to finish before spawning the next and always creates a fresh `elbmesh-test-writer`, `elbmesh-pr-publisher`, `elbmesh-implementer`, or `elbmesh-reviewer` session. Sessions are never reused across roles or rework cycles. The normal sequence is Test Writer red proof -> Publisher red commit and draft PR -> Implementer green proof -> Publisher green commit -> Reviewer PR review -> Publisher ready transition and URL -> human review and merge.

The pull request stays draft during implementation and review. After the Reviewer reports no blocking findings and required CI passes, a fresh Publisher appends review evidence without replacing earlier evidence, marks the pull request ready, and returns its URL. Ready means reviewable, not merged.

## Evidence

Every role report records its OpenCode task/session ID, role, issue, branch, base and head revisions, changed paths, exact commands and results, and blockers. This provenance is passed unchanged to later roles and copied into the pull request body or append-only comments without rewriting accepted evidence.

Red evidence includes the focused command, output, intended failure reason, changed test or fixture paths, and the red commit revision. Green evidence includes the focused pass, `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all`, implementation paths, documentation or no-docs note, architecture impact, limitations, and the distinct green commit revision. Review evidence includes the reviewed pull request range, findings ordered by severity, independently checked commands, residual risks, and blocker status.

## Issue Labels

The normal issue-label transition is `status:tests-needed` -> `status:tests-ready` -> `status:implementation` -> `status:review` -> `status:merged`. The shell-free Orchestrator, for which Bash is denied, reports readiness only after accepting the preceding evidence and requests each label transition; the human applies each transition. The Orchestrator requests `status:merged` only after observing the human merge. A blocker uses `status:blocked` or `status:decision-needed`; after resolution, the issue returns to the appropriate actionable status with a fresh role session.

## Review And Merge

The Reviewer is read-only: its OpenCode agent denies edits and permits shell access only for named inspection and quality commands. It reports findings first and cannot modify files, update GitHub state, or merge. Findings return to a fresh Implementer session and pass through another green publication and review cycle.

The Publisher is non-editing: it verifies status and diffs, stages only exact role-reported paths, commits and pushes those paths, creates and updates the pull request, appends issue/PR evidence, and marks it ready. It cannot merge or push the base branch.

Human merge authority is absolute. The Orchestrator and Publisher may report merge readiness only after review has no blocking findings and required CI passes, but only the human may review and merge. The Orchestrator requests `status:merged` after observing the human merge, and the human applies it.

## Permission Boundary

OpenCode permission rules constrain tool calls, not the operating-system account or repository hosting controls. Path-based edit and Bash rules are ordered broad-first and narrow-last because the last matching OpenCode rule wins. Permissions are defense in depth, not a sandbox: branch protection, CI, human review, exact-path status/diff checks, and role instructions remain necessary, and shell access must never bypass an edit denial. Publisher prompts additionally prohibit separators, redirection, broad staging, and every unreported path.

Direct user `@`-invocation or `@`-mention of a subagent is an out-of-band human capability that Task permissions cannot prevent. Task permissions govern agent tool calls, not the user's direct selection or mention of a subagent.
