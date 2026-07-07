# Local PPP Prototype

This directory contains project-local PPP content. The local OpenCode plugin is implemented in `.opencode/plugins/ppp-local.ts` and exposes only `ppp_local_*` tools. This project no longer configures PPP MCP/API access.

## Library Source Order

The plugin resolves the local library in this order:

1. `PPP_LOCAL_LIBRARY_DIR`
2. `ppp.config.json` `localLibraryPath`
3. `ppp.config.json` `pppLocalLibraryPath`
4. `.ppp/library`
5. `.ppp`
6. `../ppp-library/library` as a developer fallback only

`ppp_local_status` reports the selected source, resolved root, whether the project is self-contained, and a local validation summary. This repository now carries copied PPP `library/` and `schemas/` content under `.ppp/`, so normal local status should report `librarySource: project:.ppp/library`, `schemaSource: project:.ppp/schemas`, and `selfContained: true`.

## Implemented Tools

Local-only tools can list and assemble tasks, validate local PPP files, resolve repo configs and repo context, and run command validations. `ppp_local_generate_opencode_pack` is intentionally out of scope for this runtime parity phase and returns a clear out-of-scope result instead of generating files.

## OpenCode Plugin Dependencies

Official OpenCode plugin docs say project plugins are auto-loaded from `.opencode/plugins/` and local plugin dependencies belong in `.opencode/package.json`; OpenCode runs `bun install` for that config directory at startup. This prototype follows that pattern and declares:

1. `@opencode-ai/plugin` for OpenCode tool/plugin types and helpers.
2. `ajv` and `ajv-formats` for JSON Schema 2020-12 validation.
3. `liquidjs` for strict task template rendering.

After dependency or plugin changes, run `npm install` or let OpenCode install dependencies at restart, then quit and restart OpenCode because plugin files are loaded at startup.

Notable local runtime behavior:

1. `ppp_local_validate_library` validates tasks, items, tags, and repo configs with AJV against PPP JSON Schemas loaded from `.ppp/schemas`, `PPP_LOCAL_SCHEMA_DIR`, configured schema paths, or the reference `/home/tom/Projects/elbtech/ppp-library/schemas` fallback.
2. Validation issues now include `filePath`, `severity`, `path`, `instancePath`, `schemaPath`, `message`, and `referenced` where available, while preserving local semantic checks for item folder/type parity, slug prefixes, safe repo paths, legacy `practice.*` references, and legacy `practices/` directories.
3. Cross-reference validation builds a local index of item slugs, task slugs, task ids, and tag ids. It checks item `supports`, `implements`, `related`, `ppp-ref` blocks, PPP context `slugs` and `requiredSlugs`, tag assignments and select tag values, task validation `dependsOn`, repo-config `mapping.validations`, repo-config task references, and repo-config task `validationIds`.
4. `ppp_local_assemble_task_bundle` validates task inputs with AJV against the task `inputSchema`, applies property defaults, validates overlay shape, tracks rendered template paths, and renders task templates with LiquidJS using the reference strict options and `task`, `overlay`, `workflow`, `job`, and `jobs` roots.
5. Liquid rendering rejects unsupported tags, dynamic paths, invalid roots, unknown variables, unknown filters, and non-JSON output values to match the API implementation closely.
6. Task bundles still include `outputSchema`, top-level `overlays`, expected-output prompt instructions, PPP content rendering, validation prompt sections, source-file provenance, PPP content hashes, assembly provenance, and bundle content hashes.
7. `ppp_local_resolve_repo_context_offline` performs safer path/dependency input checks and preserves API-like mapping, task reference, validation reference, dependency, related item, and PPP content behavior.

## Test Commands

Run these after editing the plugin:

```bash
node --experimental-strip-types --check ".opencode/plugins/ppp-local.ts"
```

Install or refresh plugin dependencies from the config directory:

```bash
npm install --prefix .opencode
```

Smoke test direct import from the repo root:

```bash
node --experimental-strip-types --input-type=module -e 'const plugin=(await import("./.opencode/plugins/ppp-local.ts")).default; const instance=await plugin({directory:process.cwd()}); console.log(await instance.tool.ppp_local_status.execute({}));'
```

Smoke test AJV validation and Liquid rendering:

```bash
node --experimental-strip-types --input-type=module -e 'const plugin=(await import("./.opencode/plugins/ppp-local.ts")).default; const instance=await plugin({directory:process.cwd()}); console.log(await instance.tool.ppp_local_validate_library.execute({}));'
node --experimental-strip-types --input-type=module -e 'const plugin=(await import("./.opencode/plugins/ppp-local.ts")).default; const instance=await plugin({directory:process.cwd()}); console.log(await instance.tool.ppp_local_assemble_task_bundle.execute({taskId:"task.elbmesh-explain-action-event-flow",input:{flowSubject:"PlaceOrder",subjectKind:"action",paths:["crates/elbmesh-core/src/lib.rs"],depth:"summary",includeCurrentVsTarget:true},includePrompt:true,includeJson:false}));'
node --experimental-strip-types --input-type=module -e 'const plugin=(await import("./.opencode/plugins/ppp-local.ts")).default; const instance=await plugin({directory:process.cwd()}); console.log(await instance.tool.ppp_local_resolve_repo_context_offline.execute({paths:["crates/elbmesh-core/src/reaction.rs"],includeJson:false}));'
```

To prove cross-reference validation catches missing references, create or copy a temporary library outside the repo, add a semantic bad reference such as an item `related` slug that does not exist, then run validation with an environment override:

```bash
PPP_LOCAL_LIBRARY_DIR="/tmp/opencode/ppp-invalid-library" node --experimental-strip-types --input-type=module -e 'const plugin=(await import("./.opencode/plugins/ppp-local.ts")).default; const instance=await plugin({directory:process.cwd()}); console.log(await instance.tool.ppp_local_validate_library.execute({}));'
```

Then restart OpenCode and call `ppp_local_status`, `ppp_local_validate_library`, and a context tool such as `ppp_local_resolve_repo_context_offline` with a path from `ppp.repo-config.json`.

## Remaining Gaps

The local plugin still keeps reference fallbacks for developer convenience, but this repository is self-contained when `.ppp/library` and `.ppp/schemas` are present. It does not implement generated OpenCode pack output in this phase. A recommended next phase is validating content drift against the upstream PPP library and adding focused tests around the plugin tool contract.
