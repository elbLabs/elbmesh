# Agent Delivery Harness

The project-local OpenCode harness coordinates one GitHub issue on one branch through separate test, implementation, and review roles. The primary `elbmesh-orchestrator` coordinates evidence and status only; it does not implement or review.

## Phase Contract

Each phase is immutable after the Orchestrator accepts its evidence. The red phase adds only focused tests or fixtures and ends with an intended failing test. The green phase preserves those accepted tests, adds the smallest implementation, and ends with focused and full quality gates passing. The review phase does not change the branch and records findings against the accepted branch range. Rework starts a fresh implementation session and produces new evidence rather than rewriting an earlier phase report.

All roles use the same issue branch sequentially. The Orchestrator waits for one role to finish before spawning the next and always creates a fresh `elbmesh-test-writer`, `elbmesh-implementer`, or `elbmesh-reviewer` session. Sessions are never reused across roles or rework cycles.

## Evidence

Every role report records its OpenCode task/session ID, role, issue, branch, base and head revisions, changed paths, exact commands and results, and blockers. This provenance is passed unchanged to later roles.

Red evidence includes the focused command, output, intended failure reason, and changed test or fixture paths. Green evidence includes the focused pass, `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all`, implementation paths, documentation or no-docs note, architecture impact, and limitations. Review evidence includes the reviewed revision range, findings ordered by severity, independently checked commands, residual risks, and blocker status.

## Issue Labels

The normal issue-label transition is `status:tests-needed` -> `status:tests-ready` -> `status:implementation` -> `status:review` -> `status:merged`. The shell-free Orchestrator, for which Bash is denied, reports readiness only after accepting the preceding evidence and requests each label transition; the human applies each transition. The Orchestrator requests `status:merged` only after observing the human merge. A blocker uses `status:blocked` or `status:decision-needed`; after resolution, the issue returns to the appropriate actionable status with a fresh role session.

## Review And Merge

The Reviewer is read-only: its OpenCode agent denies edits and permits shell access only for named inspection and quality commands. It reports findings first and cannot modify files, update GitHub state, or merge. Findings return to a fresh Implementer session.

Human merge authority is absolute. The Orchestrator may report merge readiness only after review has no blocking findings and required CI passes, but only the human may merge. The Orchestrator records `status:merged` after observing the human merge.

## Permission Boundary

OpenCode permissions constrain tool calls, not the operating-system account or repository hosting controls. Path-based edit rules are ordered broad-first and narrow-last because the last matching OpenCode rule wins. They are defense in depth, not a sandbox: branch protection, CI, human review, and role instructions remain necessary, and shell access must never be used to bypass an edit denial.

Direct user `@`-invocation or `@`-mention of a subagent is an out-of-band human capability that Task permissions cannot prevent. Task permissions govern agent tool calls, not the user's direct selection or mention of a subagent.
