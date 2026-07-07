import type { Plugin } from "@opencode-ai/plugin";
import { tool } from "@opencode-ai/plugin";
import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";
import { Liquid, TokenKind } from "liquidjs";
import { spawn } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs/promises";
import path from "node:path";

type ValidationMode = "none" | "record-only" | "enforce";
type ValidationPhase = "before-task" | "after-task";
type PhaseInput = ValidationPhase | "all";
type PppValidationIssue = { filePath: string; severity: "error" | "warning"; path: string; instancePath?: string; schemaPath?: string; message: string; referenced?: string };

const itemDirectories: Record<string, string> = {
  principle: "principles",
  architecture: "architectures",
  pattern: "patterns",
  stack: "stacks",
  recipe: "recipes",
  validation: "validations",
  antipattern: "antipatterns",
};

const DEFAULT_TIMEOUT_SECONDS = 120;
const MAX_CAPTURE_BYTES = 16 * 1024;
const PPP_LIBRARY_ROOT = "/home/tom/Projects/elbtech/ppp-library";
const allowedTemplateRoots = new Set(["task", "input", "overlay", "workflow", "job", "jobs", "intake", "paths", "inputPaths", "phase", "taskCard", "derived", "placeholders"]);
const templateIdentifier = /^[a-zA-Z_][a-zA-Z0-9_]*$/;
const liquid = new Liquid({
  strictVariables: true,
  strictFilters: true,
  ownPropertyOnly: true,
  keepOutputType: true,
  parseLimit: 64 * 1024,
  renderLimit: 1_000,
  memoryLimit: 16 * 1024 * 1024,
  cache: false,
});
liquid.registerFilter("json", (value: unknown) => JSON.stringify(toTemplateJsonValue(value)));
const liquidRenderOptions = { strictVariables: true, ownPropertyOnly: true, renderLimit: 1_000, memoryLimit: 16 * 1024 * 1024, templateLimit: 10_000 };

export default (async ({ directory }) => {
  return {
    tool: {
      ppp_status: tool({
        description: "Show PPP plugin configuration and resolved project library sources.",
        args: {},
        async execute() {
          const binding = await readBinding(directory);
          const library = await resolveLibrarySource(directory, binding);
          const validation = await validateLocalLibrary(directory);
          return JSON.stringify({
            ok: true,
            mode: "project",
            repoKey: binding.repoKey,
            repoConfigPath: binding.repoConfigPath || "ppp.repo-config.json",
            libraryRoot: library.root,
            librarySource: library.source,
            selfContained: library.selfContained,
            configuredLibraryPath: binding.libraryPath,
            envLibraryPath: process.env.PPP_LIBRARY_DIR,
            schemaRoot: validation.schemaRoot,
            schemaSource: validation.schemaSource,
            validation: summarizeValidation(validation.issues),
            note: "Pack generation is not implemented in this project plugin. Restart OpenCode after changing plugin files or .opencode/package.json.",
          }, null, 2);
        },
      }),
      ppp_validate_library: tool({
        description: "Validate PPP task, item, and repo-config JSON shapes with structured errors.",
        args: {},
        async execute() {
          const validation = await validateLocalLibrary(directory);
          return JSON.stringify({ ok: validation.issues.every((issue) => issue.severity !== "error"), ...summarizeValidation(validation.issues), issues: validation.issues }, null, 2);
        },
      }),
      ppp_validate_workflows: tool({
        description: "Validate root-level PPP workflow routing configuration.",
        args: {},
        async execute() {
          const validation = await validateWorkflows(directory);
          return JSON.stringify({ ok: validation.issues.every((issue) => issue.severity !== "error"), ...summarizeValidation(validation.issues), issues: validation.issues }, null, 2);
        },
      }),
      ppp_list_work_types: tool({
        description: "List PPP workflow work types and their ordered phases.",
        args: {},
        async execute() {
          const validation = await validateWorkflows(directory);
          const workTypes = listWorkflowWorkTypes(validation.workflows);
          return JSON.stringify({ ok: validation.issues.every((issue) => issue.severity !== "error"), workTypeCount: workTypes.length, workTypes, ...summarizeValidation(validation.issues), issues: validation.issues }, null, 2);
        },
      }),
      ppp_assemble_workflow: tool({
        description: "Assemble a PPP workflow task-card payload and task bundle call hints for a work type.",
        args: {
          workType: tool.schema.string().describe("PPP workflow work type id, such as feature."),
          intake: tool.schema.any().optional(),
          paths: tool.schema.any().optional(),
          phaseId: tool.schema.string().optional(),
          persistTaskCard: tool.schema.boolean().optional(),
        },
        async execute(args: any) {
          const validation = await validateWorkflows(directory);
          const validationSummary = summarizeValidation(validation.issues);
          if (validationSummary.errorCount > 0) return JSON.stringify({ ok: false, ...validationSummary, issues: validation.issues }, null, 2);
          const workTypeId = requiredString(args?.workType, "workType");
          const workTypes = listWorkflowWorkTypes(validation.workflows);
          const workType = workTypes.find((entry) => entry.id === workTypeId);
          if (!workType) return JSON.stringify({ ok: false, message: `Unknown work type: ${workTypeId}`, workType: workTypeId, availableWorkTypes: workTypes.map((entry) => entry.id).filter(Boolean), ...validationSummary, issues: validation.issues }, null, 2);
          const selectedPhase = typeof args?.phaseId === "string" && args.phaseId.trim() !== "" ? workType.phases.find((phase: any) => phase.id === args.phaseId) : undefined;
          if (args?.phaseId !== undefined && !selectedPhase) return JSON.stringify({ ok: false, message: `Unknown phaseId for work type ${workTypeId}: ${args.phaseId}`, workType, availablePhaseIds: workType.phases.map((phase: any) => phase.id).filter(Boolean), ...validationSummary, issues: validation.issues }, null, 2);
          const intake = normalizeWorkflowIntake(args?.intake);
          const paths = normalizeWorkflowPaths(args?.paths);
          const taskCard = renderWorkflowTaskCard(workType, intake, paths, args?.phaseId);
          let persistedTaskCard: any;
          if (args?.persistTaskCard === true) {
            try {
              persistedTaskCard = await persistWorkflowTaskCard(directory, workType, intake, taskCard);
            } catch (error) {
              const issue: PppValidationIssue = { filePath: ".ppp/task-cards", severity: "error", path: "/persistTaskCard", message: error instanceof Error ? error.message : String(error) };
              return JSON.stringify({ ok: false, message: "Failed to persist PPP workflow task card", persistedTaskCard: undefined, ...validationSummary, issues: [...validation.issues, issue] }, null, 2);
            }
          }
          const includedPhases = selectedPhase ? [selectedPhase] : workType.phases;
          const pathContext = paths ? summarizeWorkflowPathContext(await resolveRepoContext(directory, { paths })) : undefined;
          const workflowContext = { workType: workType.id, phaseId: selectedPhase?.id, phases: workType.phases.map((phase: any) => ({ id: phase.id, label: phase.label, task: phase.task, dependsOn: phase.dependsOn || [] })) };
          const phaseTaskHints = includedPhases.map((phase: any) => createWorkflowPhaseTaskHint(phase, { intake, paths, workflow: workflowContext, taskCard, persistedTaskCard }));
          const phaseInputIssues = await validateWorkflowPhaseHintInputs(directory, phaseTaskHints);
          const issues = [...validation.issues, ...phaseInputIssues];
          const summary = summarizeValidation(issues);
          return JSON.stringify(compact({
            ok: summary.errorCount === 0,
            workType: { id: workType.id, label: workType.label, description: workType.description, phaseCount: workType.phases.length, phaseIds: workType.phases.map((phase: any) => phase.id).filter(Boolean) },
            phases: includedPhases,
            selectedPhase,
            taskCard,
            persistedTaskCard,
            phaseTaskHints,
            pathContext,
            ...summary,
            issues,
          }), null, 2);
        },
      }),
      ppp_list_tasks: tool({
        description: "List reusable PPP tasks from project PPP library JSON files.",
        args: {
          slug: tool.schema.string().optional(),
          status: tool.schema.string().optional(),
          tagId: tool.schema.string().optional(),
          tagValue: tool.schema.string().optional(),
        },
        async execute(args: any) {
          const tasks = (await readTasks(directory)).filter((task) => taskMatches(task, args || {}));
          return markdown(["# PPP Tasks", "", ...tasks.map((task) => `- ${task.slug} (${task.id}): ${task.title} - ${task.summary || "No summary"}`)]);
        },
      }),
      ppp_get_task_bundle: tool({
        description: "Resolve a PPP task bundle by id or slug.",
        args: {
          taskId: tool.schema.string().describe("Task id or slug."),
          includePrompt: tool.schema.boolean().optional(),
          includeJson: tool.schema.boolean().optional(),
        },
        async execute(args: any) {
          const bundle = await createTaskBundle(directory, requiredString(args?.taskId, "taskId"));
          return formatTaskBundle(bundle, args?.includePrompt !== false, args?.includeJson === true);
        },
      }),
      ppp_assemble_task_bundle: tool({
        description: "Assemble a PPP task bundle with task inputs and overlays.",
        args: {
          taskId: tool.schema.string().describe("Task id or slug."),
          input: tool.schema.any().optional(),
          overlays: tool.schema.any().optional(),
          workflow: tool.schema.any().optional(),
          job: tool.schema.any().optional(),
          jobs: tool.schema.any().optional(),
          includePrompt: tool.schema.boolean().optional(),
          includeJson: tool.schema.boolean().optional(),
        },
        async execute(args: any) {
          const bundle = await createTaskBundle(directory, requiredString(args?.taskId, "taskId"), parseAssemblyInput(args));
          return formatTaskBundle(bundle, args?.includePrompt !== false, args?.includeJson === true);
        },
      }),
      ppp_list_repo_configs: tool({
        description: "List repo configs from project PPP sources.",
        args: {},
        async execute() {
          const configs = await readRepoConfigs(directory);
          return markdown(["# PPP Repo Configs", "", ...configs.map((config) => `- ${config.repoKey}: ${config.name} (${(config.mappings || []).length} mappings)`)]);
        },
      }),
      ppp_get_repo_config: tool({
        description: "Return a PPP repo config.",
        args: {
          repoKey: tool.schema.string().optional(),
          repoConfig: tool.schema.any().optional(),
          includeJson: tool.schema.boolean().optional(),
        },
        async execute(args: any) {
          const config = await resolveRepoConfig(directory, args || {});
          return formatRepoConfig(config, args?.includeJson === true);
        },
      }),
      ppp_resolve_repo_context: tool({
        description: "Resolve PPP repo context for paths, changed files, and dependencies.",
        args: {
          repoKey: tool.schema.string().optional(),
          repoConfig: tool.schema.any().optional(),
          paths: tool.schema.any().optional(),
          changedFiles: tool.schema.any().optional(),
          dependencies: tool.schema.any().optional(),
          includeJson: tool.schema.boolean().optional(),
        },
        async execute(args: any) {
          const resolved = await resolveRepoContext(directory, args || {});
          return formatRepoContext(resolved, args?.includeJson === true);
        },
      }),
      ppp_generate_opencode_pack: tool({
        description: "Generated OpenCode pack output is not implemented in this project plugin.",
        args: {
          repoKey: tool.schema.string().describe("Repo key to generate for."),
          target: tool.schema.string().describe("Target directory."),
          pathPrefix: tool.schema.string().optional(),
          force: tool.schema.boolean().optional(),
        },
        async execute(args: any) {
          return JSON.stringify({ ok: false, outOfScope: true, tool: "ppp_generate_opencode_pack", message: "Generated OpenCode pack output is not implemented in this project plugin." }, null, 2);
        },
      }),
      ppp_validate: tool({
        description: "Run PPP command validations from a task bundle, validations array, or bundle JSON path.",
        args: {
          bundle: tool.schema.any().optional(),
          bundlePath: tool.schema.string().optional(),
          validations: tool.schema.any().optional(),
          phase: tool.schema.string().optional(),
          mode: tool.schema.string().optional(),
          changedFiles: tool.schema.any().optional(),
          inputs: tool.schema.any().optional(),
        },
        async execute(args: any) {
          const binding = await readBinding(directory);
          const repoDir = resolveInside(directory, binding.root || ".", "PPP root escapes repository");
          const policy = binding.validationPolicy || {};
          const mode = normalizeMode(args?.mode, policy.defaultMode || "record-only");
          const phase = normalizePhase(args?.phase, policy.defaultPhase || "after-task");
          const validations = await resolveValidations(args || {}, repoDir);
          const changedFiles = normalizeStringArray(args?.changedFiles) || await gitChangedFiles(repoDir, false);
          const result = await runValidations({ validations, phase, mode, repoDir, changedFiles, inputs: args?.inputs, defaultTimeoutSeconds: policy.timeoutSeconds });
          if (result.enforcedFailureCount > 0) throw new Error(JSON.stringify(result, null, 2));
          return JSON.stringify(result, null, 2);
        },
      }),
      ppp_precommit_validate: tool({
        description: "Run PPP validations for staged files using validationPolicy.preCommitMode.",
        args: {
          bundle: tool.schema.any().optional(),
          bundlePath: tool.schema.string().optional(),
          validations: tool.schema.any().optional(),
          phase: tool.schema.string().optional(),
          inputs: tool.schema.any().optional(),
        },
        async execute(args: any) {
          const binding = await readBinding(directory);
          const repoDir = resolveInside(directory, binding.root || ".", "PPP root escapes repository");
          const policy = binding.validationPolicy || {};
          const mode = normalizeMode(policy.preCommitMode, "record-only");
          const phase = normalizePhase(args?.phase, "all");
          const validations = await resolveValidations(args || {}, repoDir);
          const changedFiles = await gitChangedFiles(repoDir, true);
          const result = await runValidations({ validations, phase, mode, repoDir, changedFiles, inputs: args?.inputs, defaultTimeoutSeconds: policy.timeoutSeconds });
          if (result.enforcedFailureCount > 0) throw new Error(JSON.stringify(result, null, 2));
          return JSON.stringify(result, null, 2);
        },
      }),
    },
  };
}) satisfies Plugin;

async function readBinding(directory: string) {
  const raw = await fs.readFile(path.join(directory, "ppp.config.json"), "utf8").catch(() => undefined);
  return raw ? JSON.parse(raw) : { repoKey: "unknown", root: ".", repoConfigPath: "ppp.repo-config.json" };
}

async function resolveLibraryRoot(directory: string, binding?: any) {
  return (await resolveLibrarySource(directory, binding)).root;
}

async function resolveLibrarySource(directory: string, binding?: any) {
  const candidates = [
    { source: "env:PPP_LIBRARY_DIR", value: process.env.PPP_LIBRARY_DIR },
    { source: "ppp.config.json:libraryPath", value: binding?.libraryPath },
    { source: "project:.ppp/library", value: ".ppp/library" },
    { source: "project:.ppp", value: ".ppp" },
    { source: "adjacent:../ppp-library/library", value: "../ppp-library/library" },
  ].filter((entry): entry is { source: string; value: string } => typeof entry.value === "string" && entry.value.trim() !== "");
  for (const candidate of candidates) {
    const resolved = path.isAbsolute(candidate.value) ? candidate.value : path.resolve(directory, candidate.value);
    if (await isLibraryRoot(resolved)) return { root: resolved, source: candidate.source, selfContained: isInsidePath(path.resolve(directory, ".ppp"), resolved) };
  }
  return { root: path.resolve(directory, ".ppp/library"), source: "missing:project:.ppp/library", selfContained: true };
}

async function readTasks(directory: string) {
  const libraryRoot = await resolveLibraryRoot(directory, await readBinding(directory));
  return readJsonDirectory(path.join(libraryRoot, "tasks"));
}

async function readItems(directory: string) {
  const libraryRoot = await resolveLibraryRoot(directory, await readBinding(directory));
  const items = [] as any[];
  for (const itemDir of Object.values(itemDirectories)) {
    items.push(...await readJsonDirectory(path.join(libraryRoot, itemDir)));
  }
  return items.sort((left, right) => String(left.name || left.slug).localeCompare(String(right.name || right.slug)));
}

async function readRepoConfigs(directory: string) {
  const binding = await readBinding(directory);
  const libraryRoot = await resolveLibraryRoot(directory, binding);
  const configs = new Map<string, any>();
  const localConfig = await readLocalRepoConfig(directory, binding).catch(() => undefined);
  if (localConfig?.repoKey) configs.set(localConfig.repoKey, localConfig);
  for (const config of await readJsonDirectory(path.join(directory, ".ppp", "repo-configs"))) if (config.repoKey) configs.set(config.repoKey, config);
  for (const config of await readJsonDirectory(path.join(libraryRoot, "repo-configs"))) if (config.repoKey) configs.set(config.repoKey, config);
  for (const extraDir of (process.env.PPP_REPO_CONFIG_DIRS || "").split(path.delimiter).filter(Boolean)) {
    for (const config of await readJsonDirectory(extraDir)) if (config.repoKey) configs.set(config.repoKey, config);
  }
  return Array.from(configs.values()).sort((left, right) => String(left.repoKey).localeCompare(String(right.repoKey)));
}

async function readJsonDirectory(directory: string) {
  return (await readJsonDirectoryWithFiles(directory)).map((entry) => entry.value);
}

async function readJsonDirectoryWithFiles(directory: string) {
  const files = await fs.readdir(directory).catch(() => [] as string[]);
  const values = [] as Array<{ filePath: string; value: any }>;
  for (const file of files.filter((entry) => entry.endsWith(".json")).sort()) {
    const filePath = path.join(directory, file);
    values.push({ filePath, value: JSON.parse(await fs.readFile(filePath, "utf8")) });
  }
  return values;
}

async function readLocalRepoConfig(directory: string, binding: any) {
  const repoConfigPath = typeof binding.repoConfigPath === "string" && binding.repoConfigPath.trim() !== "" ? binding.repoConfigPath : "ppp.repo-config.json";
  return JSON.parse(await fs.readFile(resolveInside(directory, repoConfigPath, "PPP repoConfigPath escapes repository"), "utf8"));
}

async function validateLocalLibrary(directory: string) {
  const binding = await readBinding(directory);
  const library = await resolveLibrarySource(directory, binding);
  const issues: PppValidationIssue[] = [];
  const schemas = await loadPppSchemas(directory, binding, issues);
  await validateNoLegacyPracticesDirectory(library.root, issues);
  await validateJsonFiles(path.join(library.root, "tasks"), "task", issues, schemas);
  await validateJsonFiles(path.join(library.root, "tags"), "tag", issues, schemas);
  for (const itemDir of Object.values(itemDirectories)) await validateJsonFiles(path.join(library.root, itemDir), "item", issues, schemas, itemDir);
  await validateJsonFiles(path.join(library.root, "repo-configs"), "repo-config", issues, schemas);
  const localRepoConfigPath = resolveInside(directory, typeof binding.repoConfigPath === "string" ? binding.repoConfigPath : "ppp.repo-config.json", "PPP repoConfigPath escapes repository");
  const localRepoConfig = await readJsonFileForValidation(localRepoConfigPath, issues);
  if (localRepoConfig !== undefined) validateJsonValue(localRepoConfig, localRepoConfigPath, "repo-config", issues, schemas);
  await validateCrossReferences(directory, library.root, localRepoConfigPath, localRepoConfig, issues);
  return { libraryRoot: library.root, librarySource: library.source, schemaRoot: schemas.root, schemaSource: schemas.source, selfContained: library.selfContained, issues };
}

async function validateWorkflows(directory: string) {
  const binding = await readBinding(directory);
  const repoDir = resolveInside(directory, binding.root || ".", "PPP root escapes repository");
  const filePath = path.join(repoDir, "ppp.workflows.json");
  const issues: PppValidationIssue[] = [];
  const workflows = await readJsonFileForValidation(filePath, issues);
  if (workflows === undefined) {
    if (!issues.some((issue) => issue.filePath === filePath)) issues.push({ filePath, severity: "error", path: "", instancePath: "", message: "missing root-level ppp.workflows.json" });
    return { filePath, workflows, issues };
  }
  const tasks = await readTasks(directory);
  const taskRefs = new Set(tasks.flatMap((task) => [task.id, task.slug].filter((value): value is string => typeof value === "string")));
  validateWorkflowConfig(workflows, filePath, taskRefs, issues);
  return { filePath, workflows, issues };
}

function listWorkflowWorkTypes(workflows: unknown) {
  if (!isRecord(workflows) || !Array.isArray(workflows.workTypes)) return [];
  return workflows.workTypes.filter(isRecord).map((workType) => ({
    id: typeof workType.id === "string" ? workType.id : undefined,
    label: typeof workType.label === "string" ? workType.label : undefined,
    description: typeof workType.description === "string" ? workType.description : undefined,
    phases: Array.isArray(workType.phases) ? workType.phases.filter(isRecord).map((phase) => ({
      id: typeof phase.id === "string" ? phase.id : undefined,
      label: typeof phase.label === "string" ? phase.label : undefined,
      task: typeof phase.task === "string" ? phase.task : undefined,
      dependsOn: Array.isArray(phase.dependsOn) ? phase.dependsOn.filter((entry): entry is string => typeof entry === "string") : undefined,
      inputMap: isRecord(phase.inputMap) ? phase.inputMap : undefined,
    })) : [],
  }));
}

function normalizeWorkflowIntake(value: unknown) {
  if (value === undefined) return {};
  if (!isRecord(value)) throwValidationError("Workflow intake must be an object", [{ path: "/intake", message: "must be an object" }]);
  return value;
}

function normalizeWorkflowPaths(value: unknown) {
  if (value === undefined) return undefined;
  if (!Array.isArray(value) || !value.every((entry) => typeof entry === "string" && isSafeRepoPath(entry))) throwValidationError("Workflow paths are invalid", [{ path: "/paths", message: "must be an array of safe relative paths" }]);
  return value;
}

function renderWorkflowTaskCard(workType: any, intake: Record<string, unknown>, paths: string[] | undefined, selectedPhaseId: unknown) {
  const phases = workType.phases || [];
  return markdown([
    `# PPP ${workType.label || workType.id} Task Card`,
    "",
    "## Goal",
    "",
    workflowMarkdownValue(intake.goal),
    "",
    "## Context",
    "",
    workflowMarkdownValue(intake.context),
    "",
    "## Inputs",
    "",
    workflowMarkdownValue(intake.inputs),
    "",
    "## Candidate Paths",
    "",
    ...workflowMarkdownList(paths || normalizeStringArray(intake.candidatePaths) || []),
    "",
    "## Acceptance Criteria",
    "",
    ...workflowMarkdownList(normalizeStringArray(intake.acceptanceCriteria) || []),
    "",
    "## Non-Goals",
    "",
    ...workflowMarkdownList(normalizeStringArray(intake.nonGoals) || []),
    "",
    "## Verification Commands",
    "",
    ...workflowMarkdownList(normalizeStringArray(intake.verificationCommands) || []),
    "",
    "## PPP Workflow / Phases",
    "",
    ...phases.map((phase: any) => `- ${phase.id === selectedPhaseId ? "**Selected:** " : ""}${phase.id}: ${phase.label || phase.id} -> ${phase.task}${phase.dependsOn?.length ? ` (depends on: ${phase.dependsOn.join(", ")})` : ""}`),
  ]);
}

async function persistWorkflowTaskCard(directory: string, workType: any, intake: Record<string, unknown>, taskCard: string) {
  const binding = await readBinding(directory);
  const repoDir = resolveInside(directory, binding.root || ".", "PPP root escapes repository");
  const relativeDir = ".ppp/task-cards";
  const targetDir = resolveInside(repoDir, relativeDir, "PPP task-card directory escapes repository");
  await fs.mkdir(targetDir, { recursive: true });
  const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
  const goalPart = typeof intake.goal === "string" ? sanitizeTaskCardFilenamePart(intake.goal) : undefined;
  const baseName = [sanitizeTaskCardFilenamePart(workType.id || "workflow"), timestamp, goalPart].filter(Boolean).join("-");
  for (let index = 0; index < 100; index += 1) {
    const suffix = index === 0 ? "" : `-${index + 1}`;
    const fileName = `${baseName}${suffix}.md`;
    const absolutePath = path.join(targetDir, fileName);
    try {
      await fs.writeFile(absolutePath, taskCard, { flag: "wx" });
      return { path: `${relativeDir}/${fileName}` };
    } catch (error: any) {
      if (error?.code === "EEXIST") continue;
      throw error;
    }
  }
  throw new Error("Could not allocate a unique PPP task-card filename");
}

function sanitizeTaskCardFilenamePart(value: unknown) {
  const sanitized = String(value).toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "").slice(0, 48);
  return sanitized || undefined;
}

function workflowMarkdownValue(value: unknown) {
  if (typeof value === "string" && value.trim() !== "") return value;
  if (value !== undefined) return ["```json", JSON.stringify(value, null, 2), "```"].join("\n");
  return "_Not provided._";
}

function workflowMarkdownList(values: string[]) {
  return values.length > 0 ? values.map((entry) => `- ${entry}`) : ["_Not provided._"];
}

function createWorkflowPhaseTaskHint(phase: any, context: { intake: Record<string, unknown>; paths?: string[]; workflow: any; taskCard: string; persistedTaskCard?: any }) {
  const input = isRecord(phase.inputMap) ? resolveWorkflowInputMap(phase.inputMap, phase, context) : legacyWorkflowPhaseInput(context.intake);
  return {
    phaseId: phase.id,
    label: phase.label,
    task: phase.task,
    suggestedCall: {
      tool: "ppp_assemble_task_bundle",
      args: {
        taskId: phase.task,
        input,
        workflow: context.workflow,
      },
    },
  };
}

function legacyWorkflowPhaseInput(intake: Record<string, unknown>) {
  return {
    goal: intake.goal ?? "<goal>",
    context: intake.context ?? "<context>",
    inputs: intake.inputs ?? {},
    candidatePaths: normalizeStringArray(intake.candidatePaths) || "<candidate paths>",
    acceptanceCriteria: normalizeStringArray(intake.acceptanceCriteria) || "<acceptance criteria>",
    nonGoals: normalizeStringArray(intake.nonGoals) || "<non-goals>",
    verificationCommands: normalizeStringArray(intake.verificationCommands) || "<verification commands>",
  };
}

function resolveWorkflowInputMap(inputMap: Record<string, unknown>, phase: any, context: { intake: Record<string, unknown>; paths?: string[]; workflow: any; taskCard: string; persistedTaskCard?: any }) {
  const renderContext = createWorkflowInputMapRenderContext(phase, context);
  return Object.fromEntries(Object.entries(inputMap).map(([key, value]) => [key, renderWorkflowInputMapValue(value, renderContext)]));
}

function createWorkflowInputMapRenderContext(phase: any, context: { intake: Record<string, unknown>; paths?: string[]; workflow: any; taskCard: string; persistedTaskCard?: any }) {
  const derivedPaths = context.paths || normalizeStringArray(context.intake.candidatePaths) || [];
  const intake = {
    goal: "<goal>",
    context: "<context>",
    inputs: {},
    candidatePaths: derivedPaths,
    acceptanceCriteria: ["<expected behavior from acceptance criteria>"],
    nonGoals: [],
    verificationCommands: [],
    ...context.intake,
  };
  return {
    intake,
    paths: context.paths || [],
    inputPaths: context.paths || [],
    workflow: context.workflow,
    phase,
    taskCard: {
      markdown: context.taskCard,
      path: typeof context.persistedTaskCard?.path === "string" ? context.persistedTaskCard.path : "",
    },
    derived: {
      sliceName: workflowSliceName(context.intake.goal),
      paths: derivedPaths,
      testCommand: normalizeStringArray(context.intake.verificationCommands)?.[0] || "<targeted test command>",
      changeSummary: workflowStringOrPlaceholder(context.intake.goal, "<change summary>"),
    },
    placeholders: {
      failingTestEvidence: "<failing test evidence from test phase>",
      verificationEvidence: "<verification evidence from completed phase>",
    },
  };
}

function renderWorkflowInputMapValue(value: unknown, renderContext: any): unknown {
  if (typeof value !== "string") return structuredClone(value);
  const rendered = renderTemplateString(value, renderContext);
  return parseRenderedWorkflowInputValue(rendered);
}

function parseRenderedWorkflowInputValue(value: string): unknown {
  try {
    return JSON.parse(value);
  } catch {
    return value;
  }
}

function workflowSliceName(goal: unknown) {
  const source = typeof goal === "string" && goal.trim() !== "" ? goal : "feature slice";
  return source.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "").slice(0, 64) || "feature-slice";
}

function workflowStringOrPlaceholder(value: unknown, placeholder: string) {
  return typeof value === "string" && value.trim() !== "" ? value : placeholder;
}

async function validateWorkflowPhaseHintInputs(directory: string, phaseTaskHints: any[]) {
  const tasks = await readTasks(directory);
  const taskByRef = new Map<string, any>();
  for (const task of tasks) for (const reference of [task.id, task.slug]) if (typeof reference === "string") taskByRef.set(reference, task);
  const issues: PppValidationIssue[] = [];
  phaseTaskHints.forEach((hint, index) => {
    const taskId = hint?.suggestedCall?.args?.taskId;
    const task = typeof taskId === "string" ? taskByRef.get(taskId) : undefined;
    if (!task) return;
    for (const issue of validateValueAgainstSchema(hint?.suggestedCall?.args?.input, task.inputSchema, `/phaseTaskHints/${index}/suggestedCall/args/input`)) {
      issues.push({ filePath: "ppp.workflows.json", severity: "error", path: issue.path, instancePath: issue.path, message: `phase task input for ${task.slug} ${issue.message}`, referenced: task.slug });
    }
  });
  return issues;
}

function summarizeWorkflowPathContext(context: any) {
  return {
    repoKey: context.repoKey,
    inputPaths: context.inputPaths,
    matchedMappings: context.matchedMappings.map((mapping: any) => ({ id: mapping.id, matchedPaths: mapping.matchedPaths || [] })),
    taskReferences: context.taskReferences,
    validationReferences: context.validationReferences,
    dependencies: context.dependencies,
  };
}

function validateWorkflowConfig(value: unknown, filePath: string, taskRefs: Set<string>, issues: PppValidationIssue[]) {
  if (!isRecord(value)) {
    issues.push({ filePath, severity: "error", path: "", instancePath: "", message: "workflow config must be an object" });
    return;
  }
  if (typeof value.schemaVersion !== "number" || !Number.isFinite(value.schemaVersion)) issues.push({ filePath, severity: "error", path: "/schemaVersion", instancePath: "/schemaVersion", message: "schemaVersion must be a number" });
  if (!Array.isArray(value.workTypes) || value.workTypes.length === 0) {
    issues.push({ filePath, severity: "error", path: "/workTypes", instancePath: "/workTypes", message: "workTypes must be a non-empty array" });
    return;
  }
  const workTypeIds = new Set<string>();
  value.workTypes.forEach((workType, workTypeIndex) => validateWorkType(workType, filePath, `/workTypes/${workTypeIndex}`, workTypeIds, taskRefs, issues));
}

function validateWorkType(value: unknown, filePath: string, basePath: string, workTypeIds: Set<string>, taskRefs: Set<string>, issues: PppValidationIssue[]) {
  if (!isRecord(value)) {
    issues.push({ filePath, severity: "error", path: basePath, instancePath: basePath, message: "work type must be an object" });
    return;
  }
  const id = validateNonEmptyString(value.id, filePath, `${basePath}/id`, "work type id", issues);
  validateNonEmptyString(value.label, filePath, `${basePath}/label`, "work type label", issues);
  validateNonEmptyString(value.description, filePath, `${basePath}/description`, "work type description", issues);
  if (id) {
    if (workTypeIds.has(id)) issues.push({ filePath, severity: "error", path: `${basePath}/id`, instancePath: `${basePath}/id`, message: "duplicate work type id", referenced: id });
    workTypeIds.add(id);
  }
  if (!Array.isArray(value.phases) || value.phases.length === 0) {
    issues.push({ filePath, severity: "error", path: `${basePath}/phases`, instancePath: `${basePath}/phases`, message: "phases must be a non-empty array" });
    return;
  }
  const phaseIds = new Set<string>();
  value.phases.forEach((phase) => {
    if (!isRecord(phase)) return;
    const phaseId = typeof phase.id === "string" && phase.id.trim() !== "" ? phase.id : undefined;
    if (phaseId) phaseIds.add(phaseId);
  });
  const seenPhaseIds = new Set<string>();
  value.phases.forEach((phase, phaseIndex) => validateWorkflowPhase(phase, filePath, `${basePath}/phases/${phaseIndex}`, seenPhaseIds, phaseIds, taskRefs, issues));
}

function validateWorkflowPhase(value: unknown, filePath: string, basePath: string, seenPhaseIds: Set<string>, phaseIds: Set<string>, taskRefs: Set<string>, issues: PppValidationIssue[]) {
  if (!isRecord(value)) {
    issues.push({ filePath, severity: "error", path: basePath, instancePath: basePath, message: "phase must be an object" });
    return;
  }
  const id = validateNonEmptyString(value.id, filePath, `${basePath}/id`, "phase id", issues);
  validateNonEmptyString(value.label, filePath, `${basePath}/label`, "phase label", issues);
  const task = validateNonEmptyString(value.task, filePath, `${basePath}/task`, "phase task", issues);
  if (id) {
    if (seenPhaseIds.has(id)) issues.push({ filePath, severity: "error", path: `${basePath}/id`, instancePath: `${basePath}/id`, message: "duplicate phase id", referenced: id });
    seenPhaseIds.add(id);
  }
  if (task && !taskRefs.has(task)) issues.push({ filePath, severity: "error", path: `${basePath}/task`, instancePath: `${basePath}/task`, message: "unknown PPP task slug/id reference", referenced: task });
  validateWorkflowInputMap(value.inputMap, filePath, `${basePath}/inputMap`, issues);
  if (value.dependsOn !== undefined && !Array.isArray(value.dependsOn)) {
    issues.push({ filePath, severity: "error", path: `${basePath}/dependsOn`, instancePath: `${basePath}/dependsOn`, message: "dependsOn must be an array of phase id strings" });
    return;
  }
  (value.dependsOn || []).forEach((reference: unknown, referenceIndex: number) => {
    const referencePath = `${basePath}/dependsOn/${referenceIndex}`;
    if (typeof reference !== "string") issues.push({ filePath, severity: "error", path: referencePath, instancePath: referencePath, message: "dependsOn entries must be strings" });
    else if (!phaseIds.has(reference)) issues.push({ filePath, severity: "error", path: referencePath, instancePath: referencePath, message: "unknown phase id dependency", referenced: reference });
  });
}

function validateWorkflowInputMap(value: unknown, filePath: string, basePath: string, issues: PppValidationIssue[]) {
  if (value === undefined) return;
  if (!isRecord(value)) {
    issues.push({ filePath, severity: "error", path: basePath, instancePath: basePath, message: "inputMap must be an object" });
    return;
  }
  for (const [key, entry] of Object.entries(value)) {
    const keyPath = `${basePath}/${escapeJsonPointer(key)}`;
    if (key.trim() === "") issues.push({ filePath, severity: "error", path: keyPath, instancePath: keyPath, message: "inputMap keys must be non-empty strings" });
    if (typeof entry !== "string" || entry.trim() === "") issues.push({ filePath, severity: "error", path: keyPath, instancePath: keyPath, message: "inputMap values must be non-empty Liquid template strings" });
  }
}

function validateNonEmptyString(value: unknown, filePath: string, pathName: string, label: string, issues: PppValidationIssue[]) {
  if (typeof value === "string" && value.trim() !== "") return value;
  issues.push({ filePath, severity: "error", path: pathName, instancePath: pathName, message: `${label} must be a non-empty string` });
  return undefined;
}

async function validateJsonFiles(directory: string, kind: "task" | "item" | "repo-config" | "tag", issues: PppValidationIssue[], schemas: PppSchemaValidators, folder?: string) {
  for (const entry of await readJsonDirectoryWithFiles(directory)) {
    validateJsonValue(entry.value, entry.filePath, kind, issues, schemas);
    if (kind === "item" && folder) validateItemFolderParity(entry.value, entry.filePath, folder, issues);
  }
}

async function readJsonFileForValidation(filePath: string, issues: PppValidationIssue[]) {
  try {
    return JSON.parse(await fs.readFile(filePath, "utf8"));
  } catch (error) {
    if (!isRecord(error) || error.code !== "ENOENT") issues.push({ filePath, severity: "error", path: "", instancePath: "", message: error instanceof Error ? error.message : String(error) });
    return undefined;
  }
}

type PppSchemaKind = "task" | "item" | "repo-config" | "tag";
type PppSchemaValidators = { root: string; source: string; validators: Partial<Record<PppSchemaKind, any>> };

async function loadPppSchemas(directory: string, binding: any, issues: PppValidationIssue[]): Promise<PppSchemaValidators> {
  const source = await resolveSchemaSource(directory, binding);
  const ajv = new Ajv2020({ allErrors: true, strict: false });
  addFormats(ajv);
  const validators: Partial<Record<PppSchemaKind, any>> = {};
  for (const kind of ["task", "item", "repo-config", "tag"] as const) {
    const schemaPath = path.join(source.root, `${kind}.schema.json`);
    const schema = await readJsonFileForValidation(schemaPath, issues);
    if (schema !== undefined) validators[kind] = ajv.compile(schema);
  }
  return { root: source.root, source: source.source, validators };
}

async function resolveSchemaSource(directory: string, binding: any) {
  const candidates = [
    { source: "env:PPP_SCHEMA_DIR", value: process.env.PPP_SCHEMA_DIR },
    { source: "ppp.config.json:schemaPath", value: binding?.schemaPath },
    { source: "project:.ppp/schemas", value: ".ppp/schemas" },
    { source: "reference:ppp-library/schemas", value: path.join(PPP_LIBRARY_ROOT, "schemas") },
  ].filter((entry): entry is { source: string; value: string } => typeof entry.value === "string" && entry.value.trim() !== "");
  for (const candidate of candidates) {
    const resolved = path.isAbsolute(candidate.value) ? candidate.value : path.resolve(directory, candidate.value);
    if (await fileExists(path.join(resolved, "task.schema.json"))) return { root: resolved, source: candidate.source };
  }
  return { root: path.join(PPP_LIBRARY_ROOT, "schemas"), source: "missing:reference:ppp-library/schemas" };
}

function validateJsonValue(value: unknown, filePath: string, kind: PppSchemaKind, issues: PppValidationIssue[], schemas: PppSchemaValidators) {
  validateLegacyPracticeReferences(value, filePath, issues);
  const validate = schemas.validators[kind];
  if (!validate) {
    issues.push({ filePath, severity: "error", path: "", instancePath: "", message: `missing ${kind} JSON Schema validator` });
    return;
  }
  if (validate(value)) {
    if (kind === "task" && isRecord(value) && typeof value.slug === "string" && !value.slug.startsWith("task.")) issues.push({ filePath, severity: "error", path: "/slug", instancePath: "/slug", message: "task slug prefix must be task.*" });
    if (kind === "repo-config") validateSafeRepoConfigPaths(value, filePath, issues);
    return;
  }
  for (const error of validate.errors || []) {
    const instancePath = error.instancePath || "";
    issues.push({ filePath, severity: "error", path: instancePath, instancePath, schemaPath: error.schemaPath, message: error.message || "validation failed" });
  }
}

function validateSafeRepoConfigPaths(value: unknown, filePath: string, issues: PppValidationIssue[]) {
  if (!isRecord(value) || !Array.isArray(value.mappings)) return;
  value.mappings.forEach((mapping: any, index: number) => {
    for (const pathValue of mapping.paths || []) if (typeof pathValue !== "string" || !isSafeRepoPath(pathValue.replace(/^!/, ""))) issues.push({ filePath, severity: "error", path: `/mappings/${index}/paths`, instancePath: `/mappings/${index}/paths`, message: "paths must be safe relative paths or glob patterns" });
  });
}

function validateLegacyPracticeReferences(value: unknown, filePath: string, issues: PppValidationIssue[]) {
  walkValue(value, (entry, instancePath, key) => {
    if (typeof entry === "string" && /\bpractice\.[a-z0-9][a-z0-9.-]*/.test(entry)) {
      const field = key && ["supports", "implements", "related", "slug", "requiredSlugs", "slugs"].includes(key) ? ` in ${key}` : "";
      issues.push({ filePath, severity: "error", path: instancePath, instancePath, message: `legacy practice.* reference${field} is not supported` });
    }
  });
}

function walkValue(value: unknown, visit: (value: unknown, instancePath: string, key?: string) => void, instancePath = "", key?: string) {
  visit(value, instancePath || "/", key);
  if (Array.isArray(value)) {
    value.forEach((entry, index) => walkValue(entry, visit, `${instancePath}/${index}`, undefined));
    return;
  }
  if (!value || typeof value !== "object") return;
  for (const [entryKey, entryValue] of Object.entries(value)) walkValue(entryValue, visit, `${instancePath}/${escapeJsonPointer(entryKey)}`, entryKey);
}

function escapeJsonPointer(value: string) {
  return value.replace(/~/g, "~0").replace(/\//g, "~1");
}

async function validateNoLegacyPracticesDirectory(root: string, issues: PppValidationIssue[]) {
  const practicesPath = path.join(root, "practices");
  const stat = await fs.stat(practicesPath).catch(() => undefined);
  if (stat?.isDirectory()) issues.push({ filePath: practicesPath, severity: "error", path: "", instancePath: "", message: "legacy practices directory is not supported; use plural PPP item folders" });
}

function validateItemFolderParity(item: unknown, filePath: string, folder: string, issues: PppValidationIssue[]) {
  if (!isRecord(item)) return;
  const expectedType = Object.entries(itemDirectories).find(([, itemFolder]) => itemFolder === folder)?.[0];
  if (expectedType && item.type !== expectedType) issues.push({ filePath, severity: "error", path: "/type", instancePath: "/type", message: `item type ${String(item.type)} must match folder ${folder}` });
  if (typeof item.type === "string" && typeof item.slug === "string" && !item.slug.startsWith(`${item.type}.`)) issues.push({ filePath, severity: "error", path: "/slug", instancePath: "/slug", message: "item slug prefix must match type" });
}

type JsonValidationEntry = { filePath: string; value: any; kind: "item" | "task" | "tag" | "repo-config" };
type ReferenceIndex = {
  itemsBySlug: Map<string, JsonValidationEntry>;
  tasksBySlug: Map<string, JsonValidationEntry>;
  tasksById: Map<string, JsonValidationEntry>;
  tagsById: Map<string, JsonValidationEntry>;
};

async function validateCrossReferences(directory: string, libraryRoot: string, localRepoConfigPath: string, localRepoConfig: unknown, issues: PppValidationIssue[]) {
  const entries = await readReferenceEntries(directory, libraryRoot, localRepoConfigPath, localRepoConfig);
  const index = buildReferenceIndex(entries, issues);
  for (const entry of entries) {
    if (entry.kind === "item") validateItemReferences(entry, index, issues);
    else if (entry.kind === "task") validateTaskReferences(entry, index, issues);
    else if (entry.kind === "repo-config") validateRepoConfigReferences(entry, index, issues);
    if (entry.kind !== "tag") validateTagAssignmentsInValue(entry.value, entry.filePath, index, issues);
  }
}

async function readReferenceEntries(directory: string, libraryRoot: string, localRepoConfigPath: string, localRepoConfig: unknown) {
  const entries: JsonValidationEntry[] = [];
  for (const itemDir of Object.values(itemDirectories)) for (const entry of await readJsonDirectoryWithFiles(path.join(libraryRoot, itemDir))) entries.push({ ...entry, kind: "item" });
  for (const entry of await readJsonDirectoryWithFiles(path.join(libraryRoot, "tasks"))) entries.push({ ...entry, kind: "task" });
  for (const entry of await readJsonDirectoryWithFiles(path.join(libraryRoot, "tags"))) entries.push({ ...entry, kind: "tag" });
  for (const entry of await readJsonDirectoryWithFiles(path.join(libraryRoot, "repo-configs"))) entries.push({ ...entry, kind: "repo-config" });
  for (const entry of await readJsonDirectoryWithFiles(path.join(directory, ".ppp", "repo-configs"))) entries.push({ ...entry, kind: "repo-config" });
  if (localRepoConfig !== undefined) entries.push({ filePath: localRepoConfigPath, value: localRepoConfig, kind: "repo-config" });
  return entries;
}

function buildReferenceIndex(entries: JsonValidationEntry[], issues: PppValidationIssue[]): ReferenceIndex {
  const index: ReferenceIndex = { itemsBySlug: new Map(), tasksBySlug: new Map(), tasksById: new Map(), tagsById: new Map() };
  for (const entry of entries) {
    const value = entry.value;
    if (!isRecord(value)) continue;
    if (entry.kind === "item" && typeof value.slug === "string") addUniqueReference(index.itemsBySlug, value.slug, entry, "/slug", "item slug", issues);
    else if (entry.kind === "task") {
      if (typeof value.slug === "string") addUniqueReference(index.tasksBySlug, value.slug, entry, "/slug", "task slug", issues);
      if (typeof value.id === "string") addUniqueReference(index.tasksById, value.id, entry, "/id", "task id", issues);
    } else if (entry.kind === "tag" && typeof value.id === "string") addUniqueReference(index.tagsById, value.id, entry, "/id", "tag id", issues);
  }
  return index;
}

function addUniqueReference(map: Map<string, JsonValidationEntry>, key: string, entry: JsonValidationEntry, pathName: string, label: string, issues: PppValidationIssue[]) {
  const existing = map.get(key);
  if (existing) {
    issues.push({ filePath: entry.filePath, severity: "error", path: pathName, instancePath: pathName, message: `duplicate ${label}; first defined in ${existing.filePath}`, referenced: key });
    return;
  }
  map.set(key, entry);
}

function validateItemReferences(entry: JsonValidationEntry, index: ReferenceIndex, issues: PppValidationIssue[]) {
  if (!isRecord(entry.value)) return;
  for (const field of ["supports", "implements", "related"] as const) validateItemSlugArray(entry.value[field], entry.filePath, `/${field}`, index, issues);
  validatePppRefBlocks(entry.value.body?.sections, entry.filePath, index, issues, "/body/sections");
}

function validateTaskReferences(entry: JsonValidationEntry, index: ReferenceIndex, issues: PppValidationIssue[]) {
  if (!isRecord(entry.value)) return;
  validatePppContext(entry.value.context?.ppp, entry.filePath, "/context/ppp", index, issues);
  validatePppRefBlocks(entry.value.sections, entry.filePath, index, issues, "/sections");
  const validationIds = new Set<string>();
  (entry.value.validations || []).forEach((validation: any, indexNumber: number) => {
    if (!isRecord(validation)) return;
    if (typeof validation.id === "string") {
      if (validationIds.has(validation.id)) issues.push({ filePath: entry.filePath, severity: "error", path: `/validations/${indexNumber}/id`, instancePath: `/validations/${indexNumber}/id`, message: "duplicate task validation id", referenced: validation.id });
      validationIds.add(validation.id);
    }
    validateTagAssignments(validation.tags, entry.filePath, `/validations/${indexNumber}/tags`, index, issues);
    validatePppContext(validation.context?.ppp, entry.filePath, `/validations/${indexNumber}/context/ppp`, index, issues);
    validatePppRefBlocks(validation.promptBlocks, entry.filePath, index, issues, `/validations/${indexNumber}/promptBlocks`);
    validatePppRefBlocks(validation.context?.additions, entry.filePath, index, issues, `/validations/${indexNumber}/context/additions`);
  });
  (entry.value.validations || []).forEach((validation: any, indexNumber: number) => {
    if (!isRecord(validation)) return;
    validateStringArrayReferences(validation.dependsOn, entry.filePath, `/validations/${indexNumber}/dependsOn`, validationIds, "task validation id", issues);
  });
}

function validateRepoConfigReferences(entry: JsonValidationEntry, index: ReferenceIndex, issues: PppValidationIssue[]) {
  if (!isRecord(entry.value)) return;
  validateTagAssignments(entry.value.tags, entry.filePath, "/tags", index, issues);
  (entry.value.mappings || []).forEach((mapping: any, mappingIndex: number) => {
    if (!isRecord(mapping)) return;
    const basePath = `/mappings/${mappingIndex}`;
    validateTagAssignments(mapping.tags, entry.filePath, `${basePath}/tags`, index, issues);
    validatePppContext(mapping.ppp, entry.filePath, `${basePath}/ppp`, index, issues);
    validateValidationItemSlugs(mapping.validations, entry.filePath, `${basePath}/validations`, index, issues);
    (mapping.tasks || []).forEach((taskReference: any, taskIndex: number) => validateTaskReference(taskReference, entry.filePath, `${basePath}/tasks/${taskIndex}`, index, issues));
  });
}

function validateTaskReference(reference: unknown, filePath: string, basePath: string, index: ReferenceIndex, issues: PppValidationIssue[]) {
  if (!isRecord(reference)) return;
  const taskEntry = typeof reference.slug === "string" ? index.tasksBySlug.get(reference.slug) : typeof reference.id === "string" ? index.tasksById.get(reference.id) : undefined;
  if (typeof reference.slug === "string" && !index.tasksBySlug.has(reference.slug)) issues.push({ filePath, severity: "error", path: `${basePath}/slug`, instancePath: `${basePath}/slug`, message: "unknown task slug reference", referenced: reference.slug });
  if (typeof reference.id === "string" && !index.tasksById.has(reference.id)) issues.push({ filePath, severity: "error", path: `${basePath}/id`, instancePath: `${basePath}/id`, message: "unknown task id reference", referenced: reference.id });
  if (taskEntry && Array.isArray(reference.validationIds)) {
    const validationIds = new Set((taskEntry.value.validations || []).map((validation: any) => validation?.id).filter((id: unknown): id is string => typeof id === "string"));
    validateStringArrayReferences(reference.validationIds, filePath, `${basePath}/validationIds`, validationIds, "task validation id", issues);
  }
}

function validatePppContext(context: unknown, filePath: string, basePath: string, index: ReferenceIndex, issues: PppValidationIssue[]) {
  if (!isRecord(context)) return;
  validateItemSlugArray(context.slugs, filePath, `${basePath}/slugs`, index, issues);
  validateItemSlugArray(context.requiredSlugs, filePath, `${basePath}/requiredSlugs`, index, issues);
  validateTagAssignments(context.tags, filePath, `${basePath}/tags`, index, issues);
  if (Array.isArray(context.sectionIds)) {
    const allSectionIds = new Set(Array.from(index.itemsBySlug.values()).flatMap((entry) => (entry.value.body?.sections || []).map((section: any) => section?.id).filter((id: unknown): id is string => typeof id === "string")));
    validateStringArrayReferences(context.sectionIds, filePath, `${basePath}/sectionIds`, allSectionIds, "PPP section id", issues);
  }
}

function validateItemSlugArray(value: unknown, filePath: string, basePath: string, index: ReferenceIndex, issues: PppValidationIssue[]) {
  if (!Array.isArray(value)) return;
  value.forEach((slug, indexNumber) => {
    if (typeof slug === "string" && !index.itemsBySlug.has(slug)) issues.push({ filePath, severity: "error", path: `${basePath}/${indexNumber}`, instancePath: `${basePath}/${indexNumber}`, message: "unknown PPP item slug reference", referenced: slug });
  });
}

function validateValidationItemSlugs(value: unknown, filePath: string, basePath: string, index: ReferenceIndex, issues: PppValidationIssue[]) {
  if (!Array.isArray(value)) return;
  value.forEach((slug, indexNumber) => {
    if (typeof slug !== "string") return;
    const item = index.itemsBySlug.get(slug)?.value;
    if (!item) issues.push({ filePath, severity: "error", path: `${basePath}/${indexNumber}`, instancePath: `${basePath}/${indexNumber}`, message: "unknown validation PPP item slug reference", referenced: slug });
    else if (item.type !== "validation") issues.push({ filePath, severity: "error", path: `${basePath}/${indexNumber}`, instancePath: `${basePath}/${indexNumber}`, message: "mapping validation reference must point to a validation item", referenced: slug });
  });
}

function validatePppRefBlocks(value: unknown, filePath: string, index: ReferenceIndex, issues: PppValidationIssue[], basePath: string) {
  if (!Array.isArray(value)) return;
  value.forEach((entry, entryIndex) => {
    const entryPath = `${basePath}/${entryIndex}`;
    if (!isRecord(entry)) return;
    if (Array.isArray(entry.blocks)) validatePppRefBlocks(entry.blocks, filePath, index, issues, `${entryPath}/blocks`);
    else if (entry.type === "ppp-ref" && typeof entry.slug === "string" && !index.itemsBySlug.has(entry.slug)) issues.push({ filePath, severity: "error", path: `${entryPath}/slug`, instancePath: `${entryPath}/slug`, message: "unknown PPP item slug reference", referenced: entry.slug });
  });
}

function validateTagAssignmentsInValue(value: unknown, filePath: string, index: ReferenceIndex, issues: PppValidationIssue[]) {
  if (!isRecord(value)) return;
  validateTagAssignments(value.tags, filePath, "/tags", index, issues);
}

function validateTagAssignments(tags: unknown, filePath: string, basePath: string, index: ReferenceIndex, issues: PppValidationIssue[]) {
  if (!Array.isArray(tags)) return;
  tags.forEach((tag, indexNumber) => {
    if (!isRecord(tag) || typeof tag.tagId !== "string") return;
    const pathName = `${basePath}/${indexNumber}/tagId`;
    const tagEntry = index.tagsById.get(tag.tagId);
    if (!tagEntry) {
      issues.push({ filePath, severity: "error", path: pathName, instancePath: pathName, message: "unknown tag id reference", referenced: tag.tagId });
      return;
    }
    if (tagEntry.value.valueType === "select" && Object.hasOwn(tag, "value") && !tagEntry.value.options?.includes(tag.value)) {
      issues.push({ filePath, severity: "error", path: `${basePath}/${indexNumber}/value`, instancePath: `${basePath}/${indexNumber}/value`, message: `tag value must be one of: ${tagEntry.value.options.join(", ")}`, referenced: tag.tagId });
    }
  });
}

function validateStringArrayReferences(value: unknown, filePath: string, basePath: string, allowed: Set<string>, label: string, issues: PppValidationIssue[]) {
  if (!Array.isArray(value)) return;
  value.forEach((reference, indexNumber) => {
    if (typeof reference === "string" && !allowed.has(reference)) issues.push({ filePath, severity: "error", path: `${basePath}/${indexNumber}`, instancePath: `${basePath}/${indexNumber}`, message: `unknown ${label} reference`, referenced: reference });
  });
}

function summarizeValidation(issues: PppValidationIssue[]) {
  return { issueCount: issues.length, errorCount: issues.filter((issue) => issue.severity === "error").length, warningCount: issues.filter((issue) => issue.severity === "warning").length };
}

async function resolveRepoConfig(directory: string, input: any) {
  if (isRecord(input.repoConfig)) return input.repoConfig;
  const repoKey = typeof input.repoKey === "string" ? input.repoKey : (await readBinding(directory)).repoKey;
  const config = (await readRepoConfigs(directory)).find((entry) => entry.repoKey === repoKey);
  if (!config) throw new Error(`Repo config not found for ${repoKey}`);
  return config;
}

async function resolveRepoContext(directory: string, input: any) {
  validateResolveRepoContextInput(input);
  const config = await resolveRepoConfig(directory, input);
  const inputPaths = unique([...(normalizeStringArray(input.paths) || []), ...(normalizeStringArray(input.changedFiles) || [])]);
  const matchedMappings = (config.mappings || []).flatMap((mapping: any) => {
    const matchedPaths = inputPaths.filter((inputPath) => matchesPathMapping(mapping.paths || [], inputPath, config.root));
    return matchedPaths.length > 0 ? [{ ...mapping, matchedPaths }] : [];
  });
  const dependencies = dedupeDependencies([...(config.dependencies || []), ...matchedMappings.flatMap((mapping: any) => mapping.dependencies || []), ...(Array.isArray(input.dependencies) ? input.dependencies : [])]);
  const pppBundle = await resolveAgentContext(directory, await buildAgentContextRequest(directory, matchedMappings, dependencies));
  return {
    repoKey: config.repoKey,
    inputPaths,
    matchedMappings,
    pppBundle,
    taskReferences: dedupeByJson(matchedMappings.flatMap((mapping: any) => mapping.tasks || [])),
    validationReferences: unique(matchedMappings.flatMap((mapping: any) => mapping.validations || [])),
    dependencies,
  };
}

function validateResolveRepoContextInput(input: any) {
  const errors: Array<{ path: string; message: string }> = [];
  if (!isRecord(input)) throwValidationError("Validation failed", [{ path: "", message: "body must be an object" }]);
  if (input.repoKey !== undefined && (typeof input.repoKey !== "string" || !/^[a-z0-9][a-z0-9._-]*$/.test(input.repoKey))) errors.push({ path: "/repoKey", message: "repoKey must be a safe repo key" });
  if (input.repoConfig !== undefined && !isRecord(input.repoConfig)) errors.push({ path: "/repoConfig", message: "repoConfig must be an object" });
  const paths = validatePathArrayInput(input, "paths", errors);
  const changedFiles = validatePathArrayInput(input, "changedFiles", errors);
  if ((paths?.length || 0) === 0 && (changedFiles?.length || 0) === 0) errors.push({ path: "/paths", message: "paths or changedFiles must include at least one path" });
  if (input.dependencies !== undefined && !Array.isArray(input.dependencies)) errors.push({ path: "/dependencies", message: "dependencies must be an array" });
  if (errors.length > 0) throwValidationError("Validation failed", errors);
}

function validatePathArrayInput(input: Record<string, unknown>, key: "paths" | "changedFiles", errors: Array<{ path: string; message: string }>) {
  if (!Object.hasOwn(input, key)) return undefined;
  const value = input[key];
  if (!Array.isArray(value) || !value.every((entry) => typeof entry === "string" && isSafeRepoPath(entry))) {
    errors.push({ path: `/${key}`, message: `${key} must be an array of safe relative paths` });
    return undefined;
  }
  return value;
}

async function buildAgentContextRequest(directory: string, mappings: any[], dependencies: any[] = []) {
  const dependencySlugs = await dependencyGuidanceSlugs(directory, dependencies);
  const request: any = {
    slugs: unique([...mappings.flatMap((mapping) => [...(mapping.ppp?.slugs || []), ...(mapping.ppp?.requiredSlugs || []), ...(mapping.validations || [])]), ...dependencySlugs]),
    tags: mappings.flatMap((mapping) => mapping.ppp?.tags || []),
  };
  const sectionTypes = unique(mappings.flatMap((mapping) => mapping.ppp?.sectionTypes || []));
  const sectionIds = unique(mappings.flatMap((mapping) => mapping.ppp?.sectionIds || []));
  const blockTypes = unique(mappings.flatMap((mapping) => mapping.ppp?.blockTypes || []));
  if (mappings.some((mapping) => mapping.ppp?.includeRelated === true)) request.includeRelated = true;
  if (mappings.some((mapping) => mapping.ppp?.includeBody === true)) request.includeBody = true;
  if (sectionTypes.length > 0) request.sectionTypes = sectionTypes;
  if (sectionIds.length > 0) request.sectionIds = sectionIds;
  if (blockTypes.length > 0) request.blockTypes = blockTypes;
  return request;
}

async function dependencyGuidanceSlugs(directory: string, dependencies: any[]) {
  if (dependencies.length === 0) return [];
  const dependencyKeys = new Set(dependencies.map(normalizeDependencyKey).filter((key): key is string => key !== undefined));
  if (dependencyKeys.size === 0) return [];
  const items = await readItems(directory);
  return items.filter((item) => (item.dependencies || []).some((dependency: any) => dependencyKeys.has(normalizeDependencyKey(dependency) || ""))).map((item) => item.slug);
}

function normalizeDependencyKey(dependency: any) {
  const ecosystem = dependency?.ecosystem || dependency?.type;
  const packageName = dependency?.packageName || dependency?.name;
  return typeof ecosystem === "string" && typeof packageName === "string" ? `${ecosystem}:${packageName}` : undefined;
}

async function resolveAgentContext(directory: string, request: any) {
  const items = await readItems(directory);
  const itemsBySlug = new Map(items.map((item) => [item.slug, item]));
  const explicitSlugs = unique(normalizeStringArray(request.slugs) || []);
  const missingSlugs = explicitSlugs.filter((slug) => !itemsBySlug.has(slug));
  if (missingSlugs.length > 0) throw new Error(`Unknown item slug(s): ${missingSlugs.join(", ")}`);
  const selectedSlugs = new Set(explicitSlugs);
  for (const item of items) if (matchesAnyTag(item, request.tags || [])) selectedSlugs.add(item.slug);
  expandParentContext(selectedSlugs, itemsBySlug);
  if (request.includeRelated) {
    for (const slug of Array.from(selectedSlugs)) for (const related of itemsBySlug.get(slug)?.related || []) if (itemsBySlug.has(related)) selectedSlugs.add(related);
  }
  const selectedItems = items.filter((item) => selectedSlugs.has(item.slug));
  return {
    task: request.task || {},
    items: selectedItems.map(summarizeItem),
    ...flattenAgentFields(selectedItems),
    pppContent: selectPppContent(selectedItems, request),
    dependencies: normalizeItemDependencies(selectedItems),
    relationships: normalizeRelationships(selectedItems, selectedSlugs),
  };
}

async function createTaskBundle(directory: string, taskId: string, assembly?: { inputs: Record<string, unknown>; overlays: Record<string, unknown>; workflow?: Record<string, unknown>; job?: Record<string, unknown>; jobs?: Record<string, unknown> }) {
  const task = (await readTasks(directory)).find((entry) => entry.id === taskId || entry.slug === taskId);
  if (!task) throw new Error(`Task not found locally: ${taskId}`);
  const assemblyState = assembly ? prepareTaskAssembly(task, assembly) : undefined;
  const taskForBundle = assemblyState?.task || task;
  const pppContext = task.context?.ppp || {};
  const ppp = await resolveAgentContext(directory, {
    slugs: unique([...(pppContext.slugs || []), ...(pppContext.requiredSlugs || [])]),
    tags: pppContext.tags,
    includeRelated: pppContext.includeRelated,
    includeBody: pppContext.includeBody,
    sectionTypes: pppContext.sectionTypes,
    sectionIds: pppContext.sectionIds,
    blockTypes: pppContext.blockTypes,
    task: { title: task.title, description: task.description, metadata: { taskId: task.id, taskSlug: task.slug, ...(task.metadata || {}) } },
  });
  const resolvedAt = new Date().toISOString();
  const bundle: any = {
    bundleVersion: 1,
    resolvedAt,
    task: compact({ id: task.id, slug: task.slug, title: taskForBundle.title, summary: taskForBundle.summary, description: taskForBundle.description, tags: taskForBundle.tags, metadata: taskForBundle.metadata, inputSchema: taskForBundle.inputSchema }),
    context: {
      rules: ppp.rules,
      constraints: ppp.constraints,
      checks: ppp.checks,
      promptFragments: ppp.promptFragments,
      applicability: ppp.applicability,
      antiPatterns: ppp.antiPatterns,
      pppContent: ppp.pppContent,
      sections: (taskForBundle.sections || []).map((section: any) => ({ id: section.id, type: section.type, title: section.title, blocks: section.blocks || [] })),
      examples: blocksForSectionTypes(taskForBundle.sections || [], ["examples"]),
      references: blocksForSectionTypes(taskForBundle.sections || [], ["references"]),
      acceptanceChecks: blocksForSectionTypes(taskForBundle.sections || [], ["acceptance", "acceptance-checks"]),
      commands: (taskForBundle.sections || []).flatMap((section: any) => (section.blocks || []).filter((block: any) => block.type === "command")),
      risks: blocksForSectionTypes(taskForBundle.sections || [], ["risks"]),
      openQuestions: blocksForSectionTypes(taskForBundle.sections || [], ["open-questions"]),
    },
    outputs: taskForBundle.outputs || [],
    outputSchema: createTaskOutputSchema(taskForBundle),
    validations: taskForBundle.validations || [],
    ppp,
    ...(assemblyState ? { overlays: assemblyState.overlays } : {}),
    sources: { taskId: task.id, taskSlug: task.slug, pppSlugs: ppp.items.map((item: any) => item.slug) },
  };
  if (assemblyState) bundle.assembly = { assembledAt: resolvedAt, inputs: assemblyState.inputs, overlays: assemblyState.overlays, renderedTemplatePaths: assemblyState.renderedTemplatePaths };
  bundle.prompt = { markdown: renderPipelineTaskPrompt(bundle) };
  bundle.provenance = {
    task: { id: task.id, slug: task.slug, version: task.version, status: task.status, sourceFile: localTaskSourceFile(task) },
    ppp: ppp.items.map((item: any) => ({ slug: item.slug, type: item.type, status: item.status, sourceFile: localItemSourceFile(item) })),
    pppContent: (ppp.pppContent || []).map((entry: any) => ({ sourceFile: entry.provenance.sourceFile, sourcePath: entry.provenance.sourcePath, contentHash: entry.provenance.contentHash, sourceSlug: entry.source.slug, sectionId: entry.section.id, blockType: entry.block.type })),
    ...(assemblyState ? { assembly: { assembledFromTaskId: task.id, assembledFromTaskSlug: task.slug, renderedTemplatePaths: assemblyState.renderedTemplatePaths, overlayKeys: overlayKeys(assemblyState.overlays) } } : {}),
    contentHash: "",
  };
  bundle.provenance.contentHash = hashBundleContent(bundle);
  return bundle;
}

function formatTaskBundle(bundle: any, includePrompt: boolean, includeJson: boolean) {
  const lines = [
    `# ${bundle.task.title}`,
    "",
    `Task: ${bundle.task.slug} (${bundle.task.id})`,
    `Summary: ${bundle.task.summary || "No summary"}`,
    `PPP sources: ${bundle.sources.pppSlugs.join(", ") || "none"}`,
    `Validations: ${bundle.validations.length}`,
    `Outputs: ${(bundle.outputs || []).map((output: any) => output.id).join(", ") || "none"}`,
  ];
  if (bundle.assembly) lines.push(`Rendered templates: ${bundle.assembly.renderedTemplatePaths.join(", ") || "simple inline templates only"}`);
  if (includePrompt) lines.push("", "## Prompt", "", trimPrompt(bundle.prompt.markdown));
  if (includeJson) lines.push("", "## JSON", "", "```json", JSON.stringify(bundle, null, 2), "```");
  return markdown(lines);
}

function formatRepoConfig(config: any, includeJson: boolean) {
  const lines = [
    `# ${config.name}`,
    "",
    `Repo key: ${config.repoKey}`,
    `Description: ${config.description || "No description"}`,
    "",
    "## Mappings",
    "",
    ...(config.mappings || []).map((mapping: any) => `- ${mapping.id}: ${(mapping.paths || []).join(", ")} -> ${(mapping.ppp?.requiredSlugs || []).join(", ") || "no required slugs"}`),
    "",
    "## Dependencies",
    "",
    ...dependenciesMarkdown(config.dependencies || []),
  ];
  if (includeJson) lines.push("", "## JSON", "", "```json", JSON.stringify(config, null, 2), "```");
  return markdown(lines);
}

function formatRepoContext(resolved: any, includeJson: boolean) {
  const lines = [
    `# PPP Repo Context: ${resolved.repoKey}`,
    "",
    `Input paths: ${resolved.inputPaths.join(", ") || "none"}`,
    "",
    "## Matched Mappings",
    "",
    ...resolved.matchedMappings.map((mapping: any) => `- ${mapping.id}: ${(mapping.matchedPaths || []).join(", ")} -> ${(mapping.ppp?.requiredSlugs || []).join(", ") || "no required slugs"}`),
    "",
    "## Task References",
    "",
    ...(resolved.taskReferences.length > 0 ? resolved.taskReferences.map((task: any) => `- ${task.slug || task.id}${task.validationIds?.length ? ` (${task.validationIds.join(", ")})` : ""}`) : ["- None"]),
    "",
    "## Validation References",
    "",
    ...(resolved.validationReferences.length > 0 ? resolved.validationReferences.map((slug: string) => `- ${slug}`) : ["- None"]),
    "",
    "## Dependencies",
    "",
    ...dependenciesMarkdown(resolved.dependencies),
    "",
    "## PPP Items",
    "",
    ...resolved.pppBundle.items.map((item: any) => `- ${item.slug} (${item.type}): ${item.summary || "No summary"}`),
  ];
  if (includeJson) lines.push("", "## JSON", "", "```json", JSON.stringify(resolved, null, 2), "```");
  return markdown(lines);
}

async function generateOpenCodePack(directory: string, input: any) {
  const repoKey = requiredString(input.repoKey, "repoKey");
  const target = path.resolve(directory, requiredString(input.target, "target"));
  const config = await resolveRepoConfig(directory, { repoKey });
  const files: Record<string, string> = {
    ".opencode/package.json": `${JSON.stringify({ type: "module" }, null, 2)}\n`,
    "ppp.config.json": `${JSON.stringify({ schemaVersion: 1, repoKey: config.repoKey, name: config.name, root: input.pathPrefix || config.root || ".", repoConfigPath: "ppp.repo-config.json", libraryPath: ".ppp/library", schemaPath: ".ppp/schemas", validationPolicy: { defaultMode: "record-only", preCommitMode: "record-only", defaultPhase: "after-task", timeoutSeconds: 120 } }, null, 2)}\n`,
    ".opencode/skills/ppp/SKILL.md": `---\nname: ppp\ndescription: Use PPP plugin tools for repo context, task bundles, and validation workflows.\n---\n\n# PPP\n\nUse the ppp_* tools. Restart OpenCode after changing plugin registration or plugin files.\n`,
  };
  const written: string[] = [];
  for (const [relativePath, content] of Object.entries(files)) {
    const filePath = path.join(target, relativePath);
    const exists = await fs.stat(filePath).then(() => true).catch(() => false);
    if (exists && input.force !== true) throw new Error(`Refusing to overwrite existing file without force: ${filePath}`);
    await fs.mkdir(path.dirname(filePath), { recursive: true });
    await fs.writeFile(filePath, content);
    written.push(relativePath);
  }
  return { repoKey, target, files: written.sort() };
}

async function resolveValidations(args: any, repoDir: string) {
  if (Array.isArray(args.validations)) return args.validations;
  if (Array.isArray(args.bundle?.validations)) return args.bundle.validations;
  if (typeof args.bundlePath === "string" && args.bundlePath.trim() !== "") {
    const bundle = JSON.parse(await fs.readFile(resolveInside(repoDir, args.bundlePath, "PPP bundlePath escapes repository"), "utf8"));
    if (Array.isArray(bundle.validations)) return bundle.validations;
  }
  return [];
}

async function runValidations({ validations, phase, mode, repoDir, changedFiles, inputs, defaultTimeoutSeconds }: { validations: any[]; phase: PhaseInput; mode: ValidationMode; repoDir: string; changedFiles: string[]; inputs: any; defaultTimeoutSeconds?: number }) {
  const phases: ValidationPhase[] = phase === "all" ? ["before-task", "after-task"] : [phase];
  const phaseResults = [];
  for (const when of phases) phaseResults.push(await runValidationPhase({ validations, when, mode, repoDir, changedFiles, inputs, defaultTimeoutSeconds }));
  const results = phaseResults.flatMap((entry) => entry.results);
  const failedCount = results.filter((entry) => entry.status === "failed").length;
  const skippedCount = results.filter((entry) => entry.status === "skipped").length;
  const enforcedFailureCount = results.filter((entry) => entry.enforced && entry.status === "failed").length;
  return { ok: enforcedFailureCount === 0, mode, phase, changedFiles, status: enforcedFailureCount > 0 || failedCount > 0 ? "failed" : skippedCount > 0 ? "skipped" : "passed", validationCount: phaseResults.reduce((sum, entry) => sum + entry.validationCount, 0), executedCount: results.filter((entry) => entry.status !== "skipped").length, passedCount: results.filter((entry) => entry.status === "passed").length, failedCount, skippedCount, enforcedFailureCount, phases: phaseResults, results };
}

async function runValidationPhase(context: { validations: any[]; when: ValidationPhase; mode: ValidationMode; repoDir: string; changedFiles: string[]; inputs: any; defaultTimeoutSeconds?: number }) {
  const candidates = context.validations.filter((validation) => (validation.when || "after-task") === context.when);
  const results = [] as any[];
  if (context.mode !== "none") {
    for (const validation of candidates) {
      if (validation?.type !== "command") results.push(skippedResult(validation, context.when, context.mode, "unsupported validation type"));
      else if (typeof validation.id !== "string" || typeof validation.command !== "string") results.push(skippedResult(validation, context.when, context.mode, "invalid command validation"));
      else results.push(await runCommandValidation(validation, context));
    }
  }
  const failedCount = results.filter((entry) => entry.status === "failed").length;
  const skippedCount = results.filter((entry) => entry.status === "skipped").length;
  const enforcedFailureCount = results.filter((entry) => entry.enforced && entry.status === "failed").length;
  return { mode: context.mode, when: context.when, status: enforcedFailureCount > 0 || failedCount > 0 ? "failed" : skippedCount > 0 ? "skipped" : "passed", validationCount: candidates.length, executedCount: results.filter((entry) => entry.status !== "skipped").length, passedCount: results.filter((entry) => entry.status === "passed").length, failedCount, skippedCount, enforcedFailureCount, results };
}

async function runCommandValidation(validation: any, context: { when: ValidationPhase; mode: ValidationMode; repoDir: string; changedFiles: string[]; inputs: any; defaultTimeoutSeconds?: number }) {
  const startedAt = Date.now();
  const shell = shellExecutable(validation.shell);
  if (!shell) return skippedResult(validation, context.when, context.mode, `unsupported command shell: ${validation.shell}`);
  const cwd = resolveInside(context.repoDir, renderTemplate(validation.cwd || ".", context.inputs), "PPP validation cwd escapes repository");
  const command = renderTemplate(validation.command, context.inputs);
  const env = Object.fromEntries(Object.entries(validation.env || {}).map(([key, value]) => [key, renderEnvValue(renderTemplate(String(value), context.inputs), context.changedFiles)]));
  const timeoutMs = Math.max(1, Math.floor((validation.timeoutSeconds || context.defaultTimeoutSeconds || DEFAULT_TIMEOUT_SECONDS) * 1000));
  const execution = await executeCommand(shell.command, shell.args(command), cwd, { ...env, PPP_CHANGED_FILES: context.changedFiles.join("\n") }, timeoutMs);
  const failureReasons = commandFailureReasons(validation, execution.exitCode, execution.stdout.text, execution.stderr.text);
  if (execution.timedOut) failureReasons.push(`command timed out after ${timeoutMs}ms`);
  if (execution.error) failureReasons.push(execution.error);
  return compact({ ...baseResult(validation, context.when, context.mode), status: failureReasons.length === 0 ? "passed" : "failed", durationMs: Date.now() - startedAt, exitCode: execution.exitCode, stdout: execution.stdout.text, stderr: execution.stderr.text, truncated: execution.stdout.truncated || execution.stderr.truncated, failureReason: failureReasons.length > 0 ? failureReasons.join("; ") : undefined });
}

function renderPipelineTaskPrompt(bundle: any) {
  const lines = [`# ${bundle.task.title}`, "", `Task id: ${bundle.sources.taskId}`, "", `Task slug: ${bundle.sources.taskSlug}`, "", "## Summary", "", bundle.task.summary || "", ""];
  if (bundle.task.description) lines.push("## Description", "", bundle.task.description, "");
  renderOverlays(lines, bundle.overlays);
  if (bundle.context.sections.length > 0) lines.push("## Task Sections", "");
  for (const section of bundle.context.sections) {
    lines.push(`### ${section.title} (${section.type})`, "");
    for (const block of section.blocks) renderBlock(lines, block);
  }
  if ((bundle.outputs || []).length > 0) renderExpectedOutputs(lines, bundle.outputs);
  lines.push("## PPP Library Context", "");
  if (bundle.sources.pppSlugs.length > 0) lines.push(`Sources: ${bundle.sources.pppSlugs.join(", ")}`, "");
  for (const [field, label] of [["rules", "Rules"], ["constraints", "Constraints"], ["checks", "Checks"], ["promptFragments", "Prompt Fragments"], ["applicability", "Applicability"], ["antiPatterns", "Anti-Patterns"]] as const) {
    if (bundle.context[field].length === 0) continue;
    lines.push(`### ${label}`, "");
    for (const entry of bundle.context[field]) lines.push(`- [${entry.source.slug}] ${entry.text}`);
    lines.push("");
  }
  renderPppContent(lines, bundle.context.pppContent || []);
  if ((bundle.validations || []).length > 0) renderValidations(lines, bundle.validations);
  return trimTrailingBlankLines(lines).join("\n") + "\n";
}

function renderBlock(lines: string[], block: any) {
  if (block.type === "markdown") lines.push(block.markdown || "", "");
  else if (block.type === "list") lines.push(...(block.items || []).map((item: string, index: number) => block.ordered ? `${index + 1}. ${item}` : `- ${item}`), "");
  else if (block.type === "checklist") lines.push(...(block.items || []).map((item: any) => `- [${item.checked ? "x" : " "}] ${item.text}${item.required ? " (required)" : ""}`), "");
  else if (block.type === "code") lines.push(...(block.filename ? [`File: ${block.filename}`, ""] : []), `\`\`\`${block.language || ""}`, block.code || "", "```", "");
  else if (block.type === "command") lines.push(...(block.description ? [block.description, ""] : []), ...(block.cwd ? [`Working directory: ${block.cwd}`, ""] : []), `\`\`\`${commandFenceLanguage(block.shell)}`, block.command || "", "```", "");
  else if (block.type === "json" || block.type === "schema") lines.push("```json", JSON.stringify(block.value || block.schema || {}, null, 2), "```", "");
  else if (block.type === "file-ref") lines.push(`- File reference: ${block.path}${detailsSuffix([block.purpose, block.description, block.glob ? "glob" : undefined])}`, "");
  else if (block.type === "ppp-ref") lines.push(`- PPP Library reference: ${block.slug}${detailsSuffix([block.required ? "required" : undefined, block.reason, block.note])}`, "");
  else if (block.type === "acceptance") lines.push(...(block.criteria || []).map((criterion: any) => `- ${criterion.id ? `${criterion.id}: ` : ""}${criterion.text}${detailsSuffix([criterion.kind, criterion.required ? "required" : undefined], "(", ")")}`), "");
}

function renderOverlays(lines: string[], overlays: any) {
  if (!isOverlay(overlays) || overlayKeys(overlays).length === 0) return;
  lines.push("## Overlay Context", "");
  if (overlays.constraints?.length) lines.push("### Overlay Constraints", "", ...overlays.constraints.map((entry: string) => `- ${entry}`), "");
  if (overlays.acceptanceCriteria?.length) lines.push("### Overlay Acceptance Criteria", "", ...overlays.acceptanceCriteria.map((entry: string) => `- ${entry}`), "");
  if (overlays.notes?.length) lines.push("### Overlay Notes", "", ...overlays.notes.map((entry: string) => `- ${entry}`), "");
  if (overlays.metadata && Object.keys(overlays.metadata).length > 0) lines.push("### Overlay Metadata", "", "```json", JSON.stringify(overlays.metadata, null, 2), "```", "");
}

function renderExpectedOutputs(lines: string[], outputs: any[]) {
  lines.push("## Expected Outputs", "");
  for (const output of outputs) lines.push(`- ${output.title} (${output.type},${output.required ? " required" : " optional"})${output.description ? `: ${output.description}` : ""}`);
  lines.push("", "Complete the pipeline by calling `complete_job` with an `output` object keyed exactly by PPP task output id. Each produced output object must include `summary`; include `filesChanged`, `commandsRun`, or `notes` when useful.", "", "```json", JSON.stringify(createCompleteJobExample(outputs), null, 2), "```", "");
}

function renderPppContent(lines: string[], content: any[]) {
  if (content.length === 0) return;
  lines.push("## PPP Content", "");
  let lastSource = "";
  let lastSection = "";
  for (const entry of content) {
    if (entry.source.slug !== lastSource) {
      lines.push(`### ${entry.source.name} (${entry.source.slug})`, "");
      lastSource = entry.source.slug;
      lastSection = "";
    }
    if (entry.section.id !== lastSection) {
      lines.push(`#### ${entry.section.title} (${entry.section.type})`, "");
      lastSection = entry.section.id;
    }
    lines.push(`Source: ${entry.provenance.sourceFile}${entry.provenance.sourcePath}`, "");
    renderBlock(lines, entry.block);
  }
}

function renderValidations(lines: string[], validations: any[]) {
  lines.push("## Validations", "", "These validations are task-level checks for an external pipeline. The pipeline may execute them separately; do not claim they passed unless you actually run the command/check or receive an explicit manual review result.", "");
  for (const validation of validations) {
    lines.push(`### ${validation.id}: ${validation.title}`, "", `- Type: ${validation.type}`, `- When: ${validation.when || "unspecified"}`, `- Required: ${validation.required === undefined ? "unspecified" : validation.required ? "yes" : "no"}`);
    if (validation.dependsOn?.length) lines.push(`- Depends on: ${validation.dependsOn.join(", ")}`);
    if (validation.description) lines.push(`- Description: ${validation.description}`);
    if (validation.type === "command") {
      if (validation.cwd) lines.push(`- CWD: ${validation.cwd}`);
      if (validation.timeoutSeconds !== undefined) lines.push(`- Timeout: ${validation.timeoutSeconds}s`);
      lines.push(`- Command: \`${validation.command}\``);
      if (validation.success) lines.push(`- Success: ${renderCommandSuccess(validation.success)}`);
    } else if (validation.type === "agent") {
      if (validation.prompt) lines.push(`- Agent prompt: ${validation.prompt}`);
      if (validation.promptBlocks?.length) for (const block of validation.promptBlocks) renderBlock(lines, block);
      if (validation.passCriteria?.length) lines.push(`- Pass criteria: ${validation.passCriteria.join("; ")}`);
    } else if (validation.type === "schema") {
      lines.push(`- Target: ${[validation.target, validation.targetId, validation.targetPath, validation.jsonPath].filter(Boolean).join(" ")}`, "- Schema: provided");
    } else if (validation.type === "manual") {
      if (validation.criteria?.length) lines.push(`- Criteria: ${validation.criteria.join("; ")}`);
      if (validation.checklist?.length) lines.push(`- Checklist: ${validation.checklist.map((item: any) => `${item.text}${item.required ? " (required)" : ""}`).join("; ")}`);
    }
    lines.push("");
  }
}

function parseAssemblyInput(args: any) {
  const inputs = args?.input ?? {};
  const overlays = args?.overlays ?? {};
  if (!isRecord(inputs)) throwValidationError("Assembly inputs must be an object", [{ path: "/inputs", message: "must be an object" }]);
  if (!isOverlay(overlays)) throwValidationError("Assembly overlays are invalid", [{ path: "/overlays", message: "must include only constraints, acceptanceCriteria, notes, and metadata" }]);
  return { inputs, overlays, workflow: isRecord(args?.workflow) ? args.workflow : {}, job: isRecord(args?.job) ? args.job : {}, jobs: isRecord(args?.jobs) ? args.jobs : {} };
}

function prepareTaskAssembly(task: any, assembly: { inputs: Record<string, unknown>; overlays: Record<string, unknown>; workflow?: Record<string, unknown>; job?: Record<string, unknown>; jobs?: Record<string, unknown> }) {
  const inputs = applyInputDefaults(task.inputSchema, assembly.inputs);
  const inputIssues = validateValueAgainstSchema(inputs, task.inputSchema, "/inputs");
  if (inputIssues.length > 0) throwValidationError("Task inputs failed inputSchema validation", inputIssues);
  const renderedTemplatePaths: string[] = [];
  const context = { task: { id: task.id, slug: task.slug, input: inputs }, input: inputs, overlay: assembly.overlays, workflow: assembly.workflow || {}, job: assembly.job || {}, jobs: assembly.jobs || {} };
  return { task: renderTaskTemplates(structuredClone(task), "", context, renderedTemplatePaths), inputs, overlays: assembly.overlays, renderedTemplatePaths };
}

function applyInputDefaults(schema: any, inputs: Record<string, unknown>) {
  const withDefaults = structuredClone(inputs);
  if (!isRecord(schema?.properties)) return withDefaults;
  for (const [key, propertySchema] of Object.entries(schema.properties)) if (!Object.hasOwn(withDefaults, key) && isRecord(propertySchema) && Object.hasOwn(propertySchema, "default")) withDefaults[key] = structuredClone(propertySchema.default);
  return withDefaults;
}

function validateValueAgainstSchema(value: unknown, schema: any, basePath: string): Array<{ path: string; message: string }> {
  if (!isRecord(schema)) return [];
  const ajv = new Ajv2020({ allErrors: true, strict: false });
  addFormats(ajv);
  const validate = ajv.compile(schema);
  if (validate(value)) return [];
  return (validate.errors || []).map((error) => ({ path: `${basePath}${error.instancePath || ""}`, message: error.message || "validation failed" }));
}

function renderTaskTemplates(value: any, pathName: string, context: any, renderedTemplatePaths: string[]): any {
  if (Array.isArray(value)) return value.map((entry, index) => renderTaskTemplates(entry, `${pathName}/${index}`, context, renderedTemplatePaths));
  if (!isRecord(value)) return value;
  if (value.template === true) return Object.fromEntries(Object.entries(value).map(([key, entry]) => key === "template" ? [key, entry] : [key, renderTemplateValue(entry, `${pathName}/${key}`, context, renderedTemplatePaths)]));
  return Object.fromEntries(Object.entries(value).map(([key, entry]) => [key, renderTaskTemplates(entry, `${pathName}/${key}`, context, renderedTemplatePaths)]));
}

function renderTemplateValue(value: unknown, pathName: string, context: any, renderedTemplatePaths: string[]): unknown {
  if (typeof value === "string") {
    renderedTemplatePaths.push(pathName);
    return renderTemplateString(value, context);
  }
  if (Array.isArray(value)) return value.map((entry, index) => renderTemplateValue(entry, `${pathName}/${index}`, context, renderedTemplatePaths));
  if (isRecord(value)) return Object.fromEntries(Object.entries(value).map(([key, entry]) => [key, renderTemplateValue(entry, `${pathName}/${key}`, context, renderedTemplatePaths)]));
  return value;
}

function selectPppContent(items: any[], request: any) {
  if (request.includeBody !== true) return [];
  const sectionTypes = request.sectionTypes ? new Set(request.sectionTypes) : undefined;
  const sectionIds = request.sectionIds ? new Set(request.sectionIds) : undefined;
  const blockTypes = request.blockTypes ? new Set(request.blockTypes) : undefined;
  return items.flatMap((item) => (item.body?.sections || []).flatMap((section: any, sectionIndex: number) => {
    if (sectionTypes && !sectionTypes.has(section.type)) return [];
    if (sectionIds && !sectionIds.has(section.id)) return [];
    return (section.blocks || []).flatMap((block: any, blockIndex: number) => blockTypes && !blockTypes.has(block.type) ? [] : [{ source: { slug: item.slug, name: item.name, type: item.type }, section: { id: section.id, type: section.type, title: section.title }, block, provenance: { sourceFile: localItemSourceFile(item), sourcePath: `/body/sections/${sectionIndex}/blocks/${blockIndex}`, contentHash: hashContent(block) } }]);
  }));
}

function flattenAgentFields(items: any[]) {
  const fields: Record<string, any[]> = { rules: [], constraints: [], checks: [], promptFragments: [], applicability: [], antiPatterns: [] };
  for (const item of items) for (const field of Object.keys(fields)) for (const text of item.agent?.[field] || []) fields[field].push({ text, source: { slug: item.slug, name: item.name, type: item.type } });
  return fields;
}

function summarizeItem(item: any) {
  return { slug: item.slug, type: item.type, name: item.name, summary: item.summary, status: item.status, tags: item.tags || [], dependencies: item.dependencies || [], relationships: { supports: item.supports || [], implements: item.implements || [], related: item.related || [] } };
}

function expandParentContext(selectedSlugs: Set<string>, itemsBySlug: Map<any, any>) {
  let changed = true;
  while (changed) {
    changed = false;
    for (const slug of Array.from(selectedSlugs)) for (const parent of [...(itemsBySlug.get(slug)?.implements || []), ...(itemsBySlug.get(slug)?.supports || [])]) if (itemsBySlug.has(parent) && !selectedSlugs.has(parent)) {
      selectedSlugs.add(parent);
      changed = true;
    }
  }
}

function normalizeItemDependencies(items: any[]) {
  const values = new Map<string, any>();
  for (const item of items) for (const dependency of item.dependencies || []) {
    const key = stableJsonStringify(dependency);
    const existing = values.get(key);
    if (existing) existing.sourceSlugs.push(item.slug);
    else values.set(key, { ...dependency, sourceSlugs: [item.slug] });
  }
  return Array.from(values.values());
}

function normalizeRelationships(items: any[], selectedSlugs: Set<string>) {
  return items.flatMap((item) => ["implements", "supports", "related"].flatMap((type) => (item[type] || []).filter((to: string) => selectedSlugs.has(to)).map((to: string) => ({ from: item.slug, type, to }))));
}

function matchesPathMapping(globs: string[], value: string, root: string | undefined) {
  const candidates = expandPathCandidates(value, root);
  const includeGlobs = globs.filter((glob) => !glob.startsWith("!"));
  const excludeGlobs = globs.filter((glob) => glob.startsWith("!")).map((glob) => glob.slice(1));
  return includeGlobs.some((glob) => candidates.some((candidate) => matchGlob(glob, candidate))) && !excludeGlobs.some((glob) => candidates.some((candidate) => matchGlob(glob, candidate)));
}

function expandPathCandidates(value: string, root: string | undefined) {
  const normalizedRoot = root?.trim().replace(/^\/+|\/+$/g, "");
  if (!normalizedRoot || normalizedRoot === ".") return [value];
  const stripped = value.startsWith(`${normalizedRoot}/`) ? value.slice(normalizedRoot.length + 1) : value;
  const prefixed = stripped === value ? `${normalizedRoot}/${value}` : value;
  return unique([value, stripped, prefixed]);
}

function matchGlob(glob: string, value: string) {
  const segments = glob.split("/");
  let pattern = "^";
  segments.forEach((segment, index) => {
    if (segment === "**") {
      if (index > 0) pattern += "\\/";
      pattern += "(?:[^/]+\\/)*";
      return;
    }
    if (index > 0 && segments[index - 1] !== "**") pattern += "\\/";
    pattern += segment.replace(/[.+^${}()|[\]\\]/g, "\\$&").replace(/\*/g, "[^/]*");
  });
  return new RegExp(`${pattern}$`).test(value);
}

function renderTemplateString(value: string, context: any) {
  const parsed = liquid.parse(value);
  for (const item of parsed) if (item.token.kind !== TokenKind.HTML && item.token.kind !== TokenKind.Output) throw new Error("unsupported template tag");
  assertAllowedTemplateRoots(parsed);
  return parsed.map((item) => {
    const rendered = liquid.renderSync([item], context, liquidRenderOptions);
    return item.token.kind === TokenKind.Output ? stringifyTemplateValue(rendered) : String(rendered);
  }).join("");
}

function assertAllowedTemplateRoots(parsed: any[]) {
  for (const rawSegments of liquid.globalVariableSegmentsSync(parsed, { partials: false })) {
    const segments = parseStaticTemplatePath(rawSegments as Array<string | number | Array<unknown>>);
    const [root] = segments;
    if (typeof root !== "string" || !allowedTemplateRoots.has(root)) throw new Error(`invalid template expression root: ${formatTemplateSegments(rawSegments as Array<unknown>)}`);
  }
}

function parseStaticTemplatePath(segments: Array<string | number | Array<unknown>>) {
  return segments.map((segment) => {
    if (Array.isArray(segment)) throw new Error(`unsupported dynamic template path: ${formatTemplateSegments(segments)}`);
    if (typeof segment === "string" && !templateIdentifier.test(segment)) throw new Error(`invalid template path: ${formatTemplateSegments(segments)}`);
    return segment;
  });
}

function formatTemplateSegments(segments: Array<unknown>): string {
  return segments.map((segment) => Array.isArray(segment) ? `[${formatTemplateSegments(segment)}]` : String(segment)).join(".");
}

function stringifyTemplateValue(value: unknown) {
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  return JSON.stringify(toTemplateJsonValue(value));
}

function toTemplateJsonValue(value: unknown): unknown {
  if (value === null || typeof value === "string" || typeof value === "boolean") return value;
  if (typeof value === "number") {
    if (!Number.isFinite(value)) throw new Error(`template value is not JSON: ${String(value)}`);
    return value;
  }
  if (Array.isArray(value)) return value.map((entry) => toTemplateJsonValue(entry));
  if (typeof value === "object") {
    const prototype = Object.getPrototypeOf(value);
    if (prototype !== Object.prototype && prototype !== null) throw new Error(`template value is not JSON: ${String(value)}`);
    return Object.fromEntries(Object.entries(value).map(([key, entry]) => [key, toTemplateJsonValue(entry)]));
  }
  throw new Error(`template value is not JSON: ${String(value)}`);
}

function commandFailureReasons(validation: any, exitCode: number | null, stdout: string, stderr: string) {
  const success = validation.success || {};
  const reasons: string[] = [];
  if (exitCode !== (success.exitCode ?? 0)) reasons.push(`expected exit code ${success.exitCode ?? 0}, got ${exitCode}`);
  for (const expected of success.stdoutIncludes || []) if (!stdout.includes(expected)) reasons.push(`stdout did not include ${JSON.stringify(expected)}`);
  for (const excluded of success.stdoutExcludes || []) if (stdout.includes(excluded)) reasons.push(`stdout included ${JSON.stringify(excluded)}`);
  for (const expected of success.stderrIncludes || []) if (!stderr.includes(expected)) reasons.push(`stderr did not include ${JSON.stringify(expected)}`);
  for (const excluded of success.stderrExcludes || []) if (stderr.includes(excluded)) reasons.push(`stderr included ${JSON.stringify(excluded)}`);
  return reasons;
}

function createTaskOutputSchema(task: any): Record<string, unknown> {
  const outputs = task.outputs || [];
  const required = outputs.filter((output: any) => output.required === true).map((output: any) => output.id);
  return {
    $schema: "http://json-schema.org/draft-07/schema#",
    type: "object",
    description: "complete_job output keyed by PPP task output id.",
    ...(required.length > 0 ? { required } : {}),
    additionalProperties: false,
    properties: Object.fromEntries(outputs.map((output: any) => [output.id, schemaForTaskOutput(output)])),
  };
}

function schemaForTaskOutput(output: any): Record<string, unknown> {
  return { description: output.description || output.title, title: output.title, type: "object", required: ["summary"], additionalProperties: false, properties: { summary: { type: "string" }, filesChanged: { type: "array", items: { type: "string" } }, commandsRun: { type: "array", items: { type: "string" } }, notes: { type: "array", items: { type: "string" } } } };
}

function createCompleteJobExample(outputs: any[]) {
  return { output: Object.fromEntries(outputs.map((output) => [output.id, { summary: `Describe the completed ${output.title}.`, filesChanged: ["path/to/changed-file"], commandsRun: ["command you actually ran"], notes: ["optional note"] }])), summary: "Short overall completion summary." };
}

function renderCommandSuccess(success: any) {
  const parts = [success.exitCode !== undefined ? `exit code ${success.exitCode}` : undefined, success.stdoutIncludes?.length ? `stdout includes ${success.stdoutIncludes.join(", ")}` : undefined, success.stdoutExcludes?.length ? `stdout excludes ${success.stdoutExcludes.join(", ")}` : undefined, success.stderrIncludes?.length ? `stderr includes ${success.stderrIncludes.join(", ")}` : undefined, success.stderrExcludes?.length ? `stderr excludes ${success.stderrExcludes.join(", ")}` : undefined].filter(Boolean);
  return parts.length > 0 ? parts.join("; ") : "command exits successfully";
}

function commandFenceLanguage(shell: string | undefined) {
  return shell === "powershell" ? "powershell" : shell === "sh" ? "sh" : "bash";
}

function detailsSuffix(values: Array<string | undefined | false>, open = " - ", close = "") {
  const details = values.filter(Boolean).join("; ");
  const prefix = open === "(" ? " (" : open;
  return details ? `${prefix}${details}${close}` : "";
}

function isOverlay(value: unknown) {
  if (!isRecord(value)) return false;
  const allowed = new Set(["constraints", "acceptanceCriteria", "notes", "metadata"]);
  if (Object.keys(value).some((key) => !allowed.has(key))) return false;
  return [value.constraints, value.acceptanceCriteria, value.notes].every((entry) => entry === undefined || isStringArray(entry)) && (value.metadata === undefined || isRecord(value.metadata));
}

function overlayKeys(overlays: any) {
  return (["constraints", "acceptanceCriteria", "notes", "metadata"] as const).filter((key) => {
    const value = overlays?.[key];
    return Array.isArray(value) ? value.length > 0 : value !== undefined && Object.keys(value).length > 0;
  });
}

function executeCommand(executable: string, args: string[], cwd: string, env: Record<string, string>, timeoutMs: number) {
  return new Promise<{ exitCode: number | null; stdout: { text: string; truncated: boolean }; stderr: { text: string; truncated: boolean }; timedOut: boolean; error?: string }>((resolve) => {
    const stdout = captureBuffer();
    const stderr = captureBuffer();
    let timedOut = false;
    let spawnError: string | undefined;
    const child = spawn(executable, args, { cwd, env: { ...process.env, ...env }, stdio: ["ignore", "pipe", "pipe"], detached: process.platform !== "win32" });
    const timer = setTimeout(() => {
      timedOut = true;
      if (child.pid && process.platform !== "win32") process.kill(-child.pid, "SIGTERM");
      else child.kill("SIGTERM");
    }, timeoutMs);
    child.stdout?.on("data", (chunk: Buffer) => stdout.append(chunk));
    child.stderr?.on("data", (chunk: Buffer) => stderr.append(chunk));
    child.on("error", (error) => { spawnError = error.message; });
    child.on("close", (exitCode) => {
      clearTimeout(timer);
      resolve({ exitCode, stdout: stdout.value(), stderr: stderr.value(), timedOut, error: spawnError });
    });
  });
}

function captureBuffer() {
  const chunks: Buffer[] = [];
  let size = 0;
  let truncated = false;
  return {
    append(chunk: Buffer) {
      const remaining = MAX_CAPTURE_BYTES - size;
      if (remaining <= 0) {
        truncated = true;
        return;
      }
      chunks.push(chunk.byteLength > remaining ? chunk.subarray(0, remaining) : chunk);
      size += Math.min(chunk.byteLength, remaining);
      if (chunk.byteLength > remaining) truncated = true;
    },
    value() {
      return { text: Buffer.concat(chunks).toString("utf8"), truncated };
    },
  };
}

async function gitChangedFiles(repoDir: string, staged: boolean) {
  const result = await executeCommand("git", staged ? ["diff", "--cached", "--name-only"] : ["diff", "HEAD", "--name-only"], repoDir, {}, 30_000);
  return result.exitCode === 0 ? result.stdout.text.split(/\r?\n/).map((entry) => entry.trim()).filter(Boolean) : [];
}

function shellExecutable(shell: string | undefined) {
  if (!shell || shell === "sh") return { command: "sh", args: (command: string) => ["-c", command] };
  if (shell === "bash") return { command: "bash", args: (command: string) => ["-lc", command] };
  if (shell === "powershell") return { command: "pwsh", args: (command: string) => ["-NoProfile", "-NonInteractive", "-Command", command] };
  return undefined;
}

function baseResult(validation: any, when: ValidationPhase, mode: ValidationMode) {
  return compact({ id: typeof validation?.id === "string" ? validation.id : "<unknown>", title: typeof validation?.title === "string" ? validation.title : undefined, type: typeof validation?.type === "string" ? validation.type : "<unknown>", when, required: validation?.required === true, enforced: mode === "enforce" && validation?.required === true && validation?.type === "command" });
}

function skippedResult(validation: any, when: ValidationPhase, mode: ValidationMode, skipReason: string) {
  return compact({ ...baseResult(validation, when, mode), status: "skipped", skipReason });
}

function resolveInside(base: string, relativePath: string, errorMessage: string) {
  if (path.isAbsolute(relativePath)) throw new Error(errorMessage);
  const resolved = path.resolve(base, relativePath);
  const relative = path.relative(base, resolved);
  if (relative.startsWith("..") || path.isAbsolute(relative)) throw new Error(errorMessage);
  return resolved;
}

async function fileExists(filePath: string) {
  return fs.stat(filePath).then(() => true).catch(() => false);
}

function renderTemplate(value: string, inputs: any) {
  return value.replace(/{{\s*(?:task\.input\.|input\.)([A-Za-z0-9_.-]+)\s*}}/g, (_match, key) => String(key).split(".").reduce((current: any, part: string) => current?.[part], inputs || {}) ?? "");
}

function renderEnvValue(value: string, changedFiles: string[]) {
  return value.replace(/\$\{([A-Za-z_][A-Za-z0-9_]*)\}/g, (_match, key) => key === "PPP_CHANGED_FILES" ? changedFiles.join("\n") : process.env[key] || "");
}

function taskMatches(task: any, filters: any) {
  if (typeof filters.slug === "string" && task.slug !== filters.slug) return false;
  if (typeof filters.status === "string" && task.status !== filters.status) return false;
  if (typeof filters.tagId === "string" && !(task.tags || []).some((tag: any) => tag.tagId === filters.tagId && (filters.tagValue === undefined || String(tag.value) === filters.tagValue))) return false;
  return true;
}

function matchesAnyTag(item: any, filters: any[]) {
  return filters.length > 0 && filters.some((filter) => (item.tags || []).some((tag: any) => tag.tagId === filter.tagId && (!Object.hasOwn(filter, "value") || stableJsonStringify(tag.value) === stableJsonStringify(filter.value))));
}

function dependenciesMarkdown(dependencies: any[]) {
  if (dependencies.length === 0) return ["- None"];
  return dependencies.map((dependency) => `- ${dependency.ecosystem || dependency.type}:${dependency.packageName || dependency.name} ${dependency.resolvedVersion || dependency.requirement || dependency.version || "unspecified"}${dependency.scope ? ` scope=${dependency.scope}` : ""}`);
}

function blocksForSectionTypes(sections: any[], types: string[]) {
  return sections.filter((section) => types.includes(section.type)).flatMap((section) => section.blocks || []);
}

function normalizeMode(value: unknown, fallback: ValidationMode): ValidationMode {
  return value === "none" || value === "record-only" || value === "enforce" ? value : fallback;
}

function normalizePhase(value: unknown, fallback: PhaseInput): PhaseInput {
  return value === "before-task" || value === "after-task" || value === "all" ? value : fallback;
}

function normalizeStringArray(value: unknown) {
  return Array.isArray(value) && value.every((entry) => typeof entry === "string") ? value : undefined;
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((entry) => typeof entry === "string");
}

function jsonTypeMatches(value: unknown, type: string) {
  if (type === "array") return Array.isArray(value);
  if (type === "object") return isRecord(value);
  if (type === "integer") return Number.isInteger(value);
  if (type === "number") return typeof value === "number" && Number.isFinite(value);
  if (type === "string") return typeof value === "string";
  if (type === "boolean") return typeof value === "boolean";
  if (type === "null") return value === null;
  return true;
}

function dedupeDependencies(dependencies: any[]) {
  const seen = new Set<string>();
  return dependencies.filter((dependency) => {
    const key = [dependency.ecosystem || dependency.type, dependency.packageName || dependency.name, dependency.scope || ""].join(":");
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

function dedupeByJson<T>(values: T[]) {
  const seen = new Set<string>();
  return values.filter((value) => {
    const key = stableJsonStringify(value);
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

function unique<T>(values: T[]) {
  return Array.from(new Set(values));
}

function compact<T extends Record<string, unknown>>(record: T) {
  return Object.fromEntries(Object.entries(record).filter((entry) => entry[1] !== undefined)) as T;
}

function requiredString(value: unknown, name: string) {
  if (typeof value !== "string" || value.trim() === "") throw new Error(`${name} must be a non-empty string`);
  return value;
}

function throwValidationError(message: string, details: Array<{ path: string; message: string }>): never {
  throw new Error(JSON.stringify({ code: "VALIDATION_FAILED", message, details }, null, 2));
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

async function isDirectory(value: string) {
  return fs.stat(value).then((stat) => stat.isDirectory()).catch(() => false);
}

async function isLibraryRoot(value: string) {
  if (!await isDirectory(value)) return false;
  if (await isDirectory(path.join(value, "tasks"))) return true;
  for (const itemDir of Object.values(itemDirectories)) if (await isDirectory(path.join(value, itemDir))) return true;
  return false;
}

function localItemSourceFile(item: any) {
  return `${itemDirectories[item.type] || item.type}/${String(item.slug || "unknown").replace(/^[^.]+\./, "")}.json`;
}

function localTaskSourceFile(task: any) {
  return `tasks/${String(task.slug || "unknown").replace(/^task\./, "")}.json`;
}

function isInsidePath(base: string, candidate: string) {
  const relative = path.relative(base, candidate);
  return relative === "" || (!relative.startsWith("..") && !path.isAbsolute(relative));
}

function isSafeRepoPath(value: string) {
  return value.length > 0 && !path.isAbsolute(value) && !value.split(/[\\/]+/).includes("..") && !value.includes("\0");
}

function hashContent(value: unknown) {
  return crypto.createHash("sha256").update(stableJsonStringify(value)).digest("hex");
}

function hashBundleContent(bundle: any) {
  return hashContent({ ...bundle, resolvedAt: undefined, provenance: undefined });
}

function stableJsonStringify(value: unknown): string {
  return JSON.stringify(canonicalize(value));
}

function canonicalize(value: unknown): unknown {
  if (Array.isArray(value)) return value.map((entry) => canonicalize(entry));
  if (value && typeof value === "object") return Object.fromEntries(Object.entries(value).filter(([, entry]) => entry !== undefined).sort(([left], [right]) => left.localeCompare(right)).map(([key, entry]) => [key, canonicalize(entry)]));
  return value;
}

function trimPrompt(prompt: string) {
  const maxLength = 20_000;
  return prompt.length > maxLength ? `${prompt.slice(0, maxLength)}\n\n[Prompt truncated at ${maxLength} characters. Use includeJson for full bundle data.]` : prompt;
}

function trimTrailingBlankLines(lines: string[]) {
  let end = lines.length;
  while (end > 0 && lines[end - 1] === "") end -= 1;
  return lines.slice(0, end);
}

function markdown(lines: string[]) {
  return lines.join("\n");
}
