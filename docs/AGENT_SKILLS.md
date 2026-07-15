# Agent Skills

Elbmesh skills are first-class repository contracts. Agents use explicit docs, expanded GitHub Issues, accepted tests, role boundaries, and architecture checks instead of inferring the system from source alone.

This file is the canonical skill catalog. Concrete project-local OpenCode skills live under `.opencode/skills/`; until generation/checking exists, catalog and concrete files change together.

## Skill Contract

Every concrete skill declares its purpose and trigger, inputs, permitted edit surface, required outputs, exact verification, and the architecture/process rules it preserves.

Required reading for all Elbmesh skills:

```text
docs/GOAL.md
docs/GLOSSARY.md
docs/DEVELOPMENT_WORKFLOW.md
docs/HUMAN_DECISION_LOOP.md
docs/DELIVERY_ROADMAP.md
docs/AGENT_SKILLS.md
docs/IMPLEMENTATION_PLAN.md
docs/adr/
```

When relevant and present, also read the expanded GitHub Issue, `docs/AGENT_DELIVERY_HARNESS.md`, `architecture.manifest.json`, `RESOURCE_CAPABILITIES.md`, and `resource-capabilities.json`.

OpenCode loads project agents, skills, and configuration at startup. After merged agent, skill, or other config-time changes, quit and restart OpenCode; a running pre-merge session continues to use its loaded definitions.

## Core Skill Set

### elbmesh-architecture-checker

Purpose: inspect an accepted change for Elbmesh architecture drift before completion.

Permitted edit surface: none while checking; report required fixes to the responsible role.

Required outputs: findings ordered by severity, pass/fail summary, missing automated checks, and docs/tests that must change.

Exact verification: `codehud . --diff origin/main`, the issue's focused test command, `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all` when the assigned role permits them.

Preserves: one-Resource Action ownership; one-Resource Event streams; deterministic Resource replay; declared External Operations; Reactions invoking Actions; rebuildable Views; separate journals; synchronized schemas/docs/skills. Semantic ambiguity follows `docs/HUMAN_DECISION_LOOP.md`.

### elbmesh-doc-maintainer

Purpose: keep ADRs, glossary, workflow, roadmap, indexes, plans, agent contracts, and generated-doc synchronization rules aligned.

Permitted edit surface: assigned Markdown/docs, issue/PR templates, project-local agent/skill/config-time files, and generated files only through their generator.

Required outputs: exact changed docs, ADR/index result, catalog/concrete synchronization note, generated-doc note, open decisions, and restart note for config-time changes.

Exact verification: issue-focused documentation tests followed by `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all` when required by the issue.

Preserves: Resource, Action, Event, Reaction, and View vocabulary and boundaries; historical ADR integrity; manifest/hash synchronization; explicit issue dependencies. Semantic ambiguity follows `docs/HUMAN_DECISION_LOOP.md`.

### elbmesh-driver

Purpose: shape the smallest coherent, dependency-linked implementation issue and test-first plan.

Permitted edit surface: assigned planning docs or issue task-card text only; no production code or accepted tests.

Required outputs: goal, dependency/capability context, acceptance criteria, tests to write first, non-goals, documentation impact, architecture rules, and exact gates.

Exact verification: no repository command applies to planning-only output; the task card must name the focused command plus `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all` for later roles.

Preserves: one-Resource Action and Event boundaries, deterministic replay, declared External Operations, Reaction-to-Action flow, rebuildable Views, tests before implementation, and one implementation direction. Semantic ambiguity follows `docs/HUMAN_DECISION_LOOP.md`.

### elbmesh-flow-explainer

Purpose: explain an Action or Event through Policies, Events, Reactions, downstream Actions, External Operations, Views, Queries, and journals.

Permitted edit surface: none unless the task separately assigns an explanatory document.

Required outputs: source evidence, direct Resource effects, emitted Events, Reaction/downstream Action paths, External Operations, updated Views/Queries, and uncertainties.

Exact verification: no repository command applies to explanation-only work; cite manifest/capability artifacts and executable tests used as evidence.

Preserves: Actions target Resources; Events are domain facts; Reactions invoke Actions rather than mutate Resources; External Operations contain external calls; Views remain derived and rebuildable.

### elbmesh-implementer

Purpose: make accepted focused failing tests pass with the smallest production/configuration/documentation change.

Permitted edit surface: production, configuration, agent, skill, and documentation paths required by the issue, excluding every accepted test and fixture path.

Required outputs: role task/session ID, provenance, exact non-test changed paths, focused/full command results, docs note, architecture impact, limitations, and blockers. Accepted tests and fixtures are immutable to Implementers, and Implementer outputs must exclude supporting test fixtures.

If an accepted test or fixture conflicts with the task card or architecture, the Implementer reports the conflict to the Orchestrator for human confirmation and stops. Only after human confirmation may a fresh Test Writer revise the accepted test or fixture; the Implementer must not revise it.

Exact verification: run the issue's focused command, then `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all`.

Preserves: explicit Resource/Action/Event behavior, deterministic replay, declared External Operations, Resource Event/journal separation, Reactions invoking Actions, rebuildable Views, immutable accepted tests, and no speculative abstraction.

### elbmesh-manifest-editor

Purpose: change declared Resources, Components, Actions, Events, Reactions, Views, Queries, Policies, External Operations, schemas, or manifest-derived surfaces.

Permitted edit surface: assigned manifest/schema sources and generated outputs through the documented generator; no unrelated runtime behavior.

Required outputs: manifest/schema paths, generated binding/capability paths or regeneration plan, validation result, architecture impact, and unresolved decisions.

Exact verification: issue-focused manifest tests, generation/drift command documented by the issue, `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all`.

Preserves: one target Resource per Action, one owner Resource per Event, declared Reaction and External Operation links, declared View/Query indexes, schema versions, and synchronized outputs. Semantic ambiguity follows `docs/HUMAN_DECISION_LOOP.md`.

### elbmesh-mr-reviewer

Purpose: provide optional compatibility/manual deep review when requested outside the canonical delivery sequence.

`elbmesh-mr-reviewer` is an optional compatibility/manual skill and not an additional required stage. It does not own or report merge readiness.

Permitted edit surface: none; this skill is read-only.

Required outputs: findings ordered by severity, gate observations, supplemental deep-review report, follow-up tasks, and residual risks.

Exact verification: `git status --short --branch`, `git diff --check origin/main...HEAD`, `codehud . --diff origin/main`, `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all` when authorized.

Preserves: Resource, Action, Event, Reaction, and View boundaries; External Operation and journal separation; tests-first provenance; read-only review. This compatibility/manual skill is optional and not an additional required stage. It does not own or report merge readiness; only `elbmesh-reviewer` reports final pull request merge readiness, and a human performs the merge.

### elbmesh-orchestrator

Purpose: coordinate dependency-ordered GitHub Issues through fresh role sessions and publication handoffs.

Permitted edit surface: none; the Orchestrator has Edit and Bash denied and delegates publication to the Publisher.

Required outputs: issue/dependency context, role assignment and task/session IDs, immutable handoff evidence, blocker state, pull request state, and next unblocked work.

Exact verification: no repository command applies to the shell-free Orchestrator; it requires exact Test Writer, Implementer, Publisher, Reviewer, and CI evidence from delegated roles.

Preserves: tests before implementation; immutable accepted tests; separate red test-only and green implementation/docs commits; fresh sessions; final `elbmesh-reviewer` report; append-only evidence; human-only merge. The Publisher owns automatic transitions between `status:implementation` and `status:review`. Semantic conflicts follow `docs/HUMAN_DECISION_LOOP.md`.

`elbmesh-reviewer` reports final pull request merge readiness or blockers. A human performs final review and merge; the Orchestrator does not merge.

### elbmesh-pr-publisher

Purpose: publish accepted role reports as an auditable draft-to-ready pull request and automate the two issue-status transitions without editing repository files.

Permitted edit surface: no repository file edits; only exact-path Git staging/commit/push and narrowly allowed issue/pull-request publication state.

Required outputs: branch/base/head provenance, separate red and green commits, linked pull request, append-only cumulative evidence links, issue-status result, ready state, URL, and residual risks.

Exact verification: `git status --short --branch`, exact-path `git diff`/cached-diff inspection, `gh issue view <issue>`, `gh pr view <pr>`, and `gh pr checks <pr>` within the agent allowlist.

Preserves: exact role path ownership; accepted red/green provenance; no file authorship; no broad staging or shell bypass; Resource/Action/Event/Reaction/View architecture evidence; append-only comments; no base push, merge, or auto-merge. After accepted red publication, set or keep `status:implementation`. Only after no-blocker Reviewer evidence and required CI pass, change to `status:review` while marking the pull request ready. Only a human may merge.

### elbmesh-reviewer

Purpose: perform the single active final pull request review and report merge readiness or blockers.

Permitted edit surface: none; review and GitHub state are read-only.

Required outputs: findings first, issue/branch/range, exact inspection/gate results, residual risks, blocker state, and final merge-readiness report.

Exact verification: `git status --short --branch`, `git log --oneline --decorate origin/main..HEAD`, `git diff --name-status origin/main...HEAD`, `git diff --check origin/main...HEAD`, `codehud . --diff origin/main`, `gh pr view --json number,title,body,state,isDraft,baseRefName,headRefName,url`, `gh pr checks`, `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all`.

Preserves: Resource/Action/Event boundaries, deterministic replay, declared External Operations, Event/journal separation, Reaction execution through Actions, rebuildable Views, immutable evidence, and read-only final review. `elbmesh-reviewer` reports merge readiness; a human performs the merge.

### elbmesh-test-writer

Purpose: write focused failing tests before production implementation.

Permitted edit surface: assigned `tests/**`, `fixtures/**`, `test-fixtures/**`, and nested equivalents only.

Required outputs: role task/session ID, provenance, exact test/fixture paths, focused command/output, intended failure reason, untestable criteria, and blockers.

Exact verification: run the issue's exact focused `cargo test ...` command and confirm failure is the intended missing behavior rather than compilation, infrastructure, or unrelated noise.

Preserves: Resource scenario behavior, Action typed errors, Event ownership, Reaction-to-Action flow, rebuildable View contracts, External Operation retry contracts, and tests before implementation; it does not implement production behavior.

## Packaging And Synchronization

Concrete project-local skill paths are:

```text
.opencode/skills/elbmesh-architecture-checker/SKILL.md
.opencode/skills/elbmesh-doc-maintainer/SKILL.md
.opencode/skills/elbmesh-driver/SKILL.md
.opencode/skills/elbmesh-flow-explainer/SKILL.md
.opencode/skills/elbmesh-implementer/SKILL.md
.opencode/skills/elbmesh-manifest-editor/SKILL.md
.opencode/skills/elbmesh-mr-reviewer/SKILL.md
.opencode/skills/elbmesh-orchestrator/SKILL.md
.opencode/skills/elbmesh-pr-publisher/SKILL.md
.opencode/skills/elbmesh-reviewer/SKILL.md
.opencode/skills/elbmesh-test-writer/SKILL.md
```

Do not hand-maintain generated skill files once generation exists. Until then, every catalog change updates all affected concrete skills and contract tests in the same issue.

## Definition Of Agentically Usable

The repository is agentically usable when an agent can find the goal and rules, select an unblocked issue, load the right skill, write tests before implementation, preserve accepted tests, make explicit changes, run exact checks, explain architecture impact, publish auditable evidence through the Publisher, and leave final review/merge to a human.
