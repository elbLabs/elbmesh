---
name: ppp
description: Use local PPP OpenCode plugin tools for project context, task inputs, repo config, validation workflows, and dependency guidance.
---

# PPP

Use this skill when a user asks for PPP guidance, repo-specific task execution, task inputs, validation checks, dependency-sensitive changes, or review against PPP rules.

Repository: Elbmesh (elbmesh)

## Source Of Truth

The project-local `.ppp/` library and `.opencode/plugins/ppp-local.ts` are the source of truth for blocks, task bundles, validation guidance, dependency guidance, and context resolution.

Elbmesh keeps its repo mapping in `ppp.repo-config.json`. Use:

- `ppp_local_status` to verify local PPP library/schema status.
- `ppp_local_validate_library` to validate local PPP files.
- `ppp_local_get_repo_config_offline` to inspect the local repo config.
- `ppp_local_resolve_repo_context_offline` before substantial edits or review. Pass `paths` or `changedFiles`.
- `ppp_local_assemble_task_bundle` for task bundles and task prompts.

Do not call PPP MCP/API tools for this project. The local plugin is the only PPP component that reads `ppp.repo-config.json`, `.ppp/library`, and `.ppp/schemas`, and it resolves configured paths relative to the OpenCode project directory.
