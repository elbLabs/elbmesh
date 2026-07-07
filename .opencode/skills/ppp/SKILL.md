---
name: ppp
description: Use PPP OpenCode plugin tools for project context, task inputs, repo config, validation workflows, and dependency guidance.
---

# PPP

Use this skill when a user asks for PPP guidance, repo-specific task execution, Elbmesh workflow routing, task inputs, validation checks, dependency-sensitive changes, or review against PPP rules.

Repository: Elbmesh (elbmesh)

## Source Of Truth

The `.ppp/` library, `ppp.repo-config.json`, `ppp.config.json`, and `.opencode/plugins/ppp.ts` are the source of truth for blocks, task bundles, validation guidance, dependency guidance, and context resolution. The former `elbmesh-*` OpenCode workflow skills were migrated to PPP tasks; do not recreate wrapper skills for those routes.

## Tools

- `ppp_status` to verify PPP library/schema status.
- `ppp_validate_library` to validate PPP files.
- `ppp_list_tasks` to discover reusable tasks in the project library.
- `ppp_get_task_bundle` to resolve a task bundle by id or slug without custom inputs.
- `ppp_assemble_task_bundle` to resolve a task bundle with inputs, overlays, workflow/job data, rendered prompts, and optional JSON.
- `ppp_list_repo_configs` to list project repo configs.
- `ppp_get_repo_config` to inspect the repo config.
- `ppp_resolve_repo_context` before substantial edits or review. Pass `paths` or `changedFiles`.
- `ppp_validate` to run command validations from a bundle, validation array, or bundle JSON path.
- `ppp_precommit_validate` to run staged-file validations using `validationPolicy.preCommitMode`.
- `ppp_generate_opencode_pack` is not implemented in this project plugin and should not be used for generation.

Use `includePrompt: true` when the agent needs rendered task instructions. Use `includeJson: true` when structured bundle or repo-context data is needed for validation, automation, or precise follow-up.

## Default Workflow

1. Run or trust a fresh `ppp_status` and `ppp_validate_library` before substantial PPP-guided work.
2. Use `ppp_resolve_repo_context` with relevant `paths` or `changedFiles` to collect repo-specific PPP context.
3. Select the closest task with `ppp_list_tasks` or the route map below.
4. Use `ppp_assemble_task_bundle` with task inputs and follow the rendered prompt, expected output, PPP content, and validation guidance.
5. Run task-relevant project checks plus `ppp_validate` or `ppp_precommit_validate` when validations are present.

## Dependency Context

`ppp.repo-config.json` defines Cargo dependencies at repo and mapping scope. `ppp_resolve_repo_context` returns the matching dependency inventory and also pulls in PPP items whose item-level dependencies match those Cargo packages, so dependency-sensitive Rust paths receive both package context and relevant architecture/validation guidance.

## Elbmesh Task Routes

- Architecture checks: `ppp_assemble_task_bundle` with `task.elbmesh-check-architecture-boundaries`
- Documentation maintenance: `ppp_assemble_task_bundle` with `task.elbmesh-maintain-docs`
- Slice planning: `ppp_assemble_task_bundle` with `task.elbmesh-plan-implementation-slice`
- Phase coordination: `ppp_assemble_task_bundle` with `task.elbmesh-coordinate-phase-work`
- MR readiness review: `ppp_assemble_task_bundle` with `task.elbmesh-review-mr-readiness`
- Failing tests: `ppp_assemble_task_bundle` with `task.elbmesh-write-failing-tests`
- Runtime implementation: `ppp_assemble_task_bundle` with `task.elbmesh-implement-runtime-slice`
- Change review: `ppp_assemble_task_bundle` with `task.elbmesh-review-change`
- Flow explanation: `ppp_assemble_task_bundle` with `task.elbmesh-explain-action-event-flow`
- Manifest updates: `ppp_assemble_task_bundle` with `task.elbmesh-update-architecture-manifest`

The project library is repo-local and intentionally limited to Elbmesh tasks, Rust core guidance, documentation guidance, OpenCode PPP integration guidance, and directly supporting PPP items.
