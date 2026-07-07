# Project PPP

This directory contains the project PPP library and schemas. The OpenCode plugin is implemented in `.opencode/plugins/ppp.ts` and exposes canonical `ppp_*` tools.

## Project Files

1. `.ppp/library` contains repo-local Elbmesh tasks, principles, architectures, patterns, validations, antipatterns, and tags.
2. `.ppp/schemas` contains the JSON Schemas used to validate PPP library content.
3. `ppp.config.json` binds this repository to its PPP library and schema paths.
4. `ppp.repo-config.json` maps repository paths to relevant PPP guidance, tasks, validations, and dependencies.

## Implemented Tools

The project plugin can list and assemble tasks, validate PPP files, resolve repo configs and repo context, and run command validations. `ppp_generate_opencode_pack` is not implemented in this project plugin and returns a clear out-of-scope result instead of generating files.

## OpenCode Skill Migration

Elbmesh-specific OpenCode workflow skills under `.opencode/skills/elbmesh-*/SKILL.md` were removed after migration. Detailed workflow rules, checks, output shapes, and validation guidance are canonical in `.ppp/library/tasks/*.json` and related PPP items. Use `.opencode/skills/ppp/SKILL.md` for the task route map, and assemble routes with `ppp_assemble_task_bundle`.

## OpenCode Plugin Dependencies

OpenCode project plugins are loaded from `.opencode/plugins/`, and plugin dependencies are declared in `.opencode/package.json`. This project declares:

1. `@opencode-ai/plugin` for OpenCode tool/plugin types and helpers.
2. `ajv` and `ajv-formats` for JSON Schema 2020-12 validation.
3. `liquidjs` for strict task template rendering.

After dependency or plugin changes, run `npm install --prefix .opencode` if needed, then quit and restart OpenCode because plugin files are loaded at startup.

Notable behavior:

1. `ppp_validate_library` validates tasks, items, tags, and repo configs with AJV against PPP JSON Schemas loaded from `.ppp/schemas` or configured schema paths.
2. Validation issues include `filePath`, `severity`, `path`, `instancePath`, `schemaPath`, `message`, and `referenced` where available, while preserving semantic checks for item folder/type parity, slug prefixes, safe repo paths, legacy `practice.*` references, and legacy `practices/` directories.
3. Cross-reference validation builds an index of item slugs, task slugs, task ids, and tag ids. It checks item `supports`, `implements`, `related`, `ppp-ref` blocks, PPP context `slugs` and `requiredSlugs`, tag assignments and select tag values, task validation `dependsOn`, repo-config `mapping.validations`, repo-config task references, and repo-config task `validationIds`.
4. `ppp_assemble_task_bundle` validates task inputs with AJV against the task `inputSchema`, applies property defaults, validates overlay shape, tracks rendered template paths, and renders task templates with LiquidJS using strict options and `task`, `overlay`, `workflow`, `job`, and `jobs` roots.
5. Liquid rendering rejects unsupported tags, dynamic paths, invalid roots, unknown variables, unknown filters, and non-JSON output values.
6. Task bundles include `outputSchema`, top-level `overlays`, expected-output prompt instructions, PPP content rendering, validation prompt sections, source-file provenance, PPP content hashes, assembly provenance, and bundle content hashes.
7. `ppp_resolve_repo_context` performs safe path/dependency input checks and preserves mapping, task reference, validation reference, dependency, related item, and PPP content behavior.

## Test Commands

Run these after editing the plugin:

```bash
node --experimental-strip-types --check ".opencode/plugins/ppp.ts"
```

Install or refresh plugin dependencies from the config directory:

```bash
npm install --prefix .opencode
```

Smoke test direct import from the repo root:

```bash
node --experimental-strip-types --input-type=module -e 'const plugin=(await import("./.opencode/plugins/ppp.ts")).default; const instance=await plugin({directory:process.cwd()}); console.log(await instance.tool.ppp_status.execute({}));'
```

Smoke test AJV validation and Liquid rendering:

```bash
node --experimental-strip-types --input-type=module -e 'const plugin=(await import("./.opencode/plugins/ppp.ts")).default; const instance=await plugin({directory:process.cwd()}); console.log(await instance.tool.ppp_validate_library.execute({}));'
node --experimental-strip-types --input-type=module -e 'const plugin=(await import("./.opencode/plugins/ppp.ts")).default; const instance=await plugin({directory:process.cwd()}); console.log(await instance.tool.ppp_assemble_task_bundle.execute({taskId:"task.elbmesh-explain-action-event-flow",input:{flowSubject:"PlaceOrder",subjectKind:"action",paths:["crates/elbmesh-core/src/lib.rs"],depth:"summary",includeCurrentVsTarget:true},includePrompt:true,includeJson:false}));'
node --experimental-strip-types --input-type=module -e 'const plugin=(await import("./.opencode/plugins/ppp.ts")).default; const instance=await plugin({directory:process.cwd()}); console.log(await instance.tool.ppp_resolve_repo_context.execute({paths:["crates/elbmesh-core/src/reaction.rs"],includeJson:false}));'
```

Then restart OpenCode and call `ppp_status`, `ppp_validate_library`, and a context tool such as `ppp_resolve_repo_context` with a path from `ppp.repo-config.json`.
