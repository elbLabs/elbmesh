use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const ACTIVE_PROCESS_DOCS: [&str; 6] = [
    "docs/README.md",
    "docs/DEVELOPMENT_WORKFLOW.md",
    "docs/HUMAN_DECISION_LOOP.md",
    "docs/AGENT_DELIVERY_HARNESS.md",
    "docs/AGENT_SKILLS.md",
    "docs/IMPLEMENTATION_PLAN.md",
];

const ACTIVE_AGENT_FILES: [&str; 5] = [
    ".opencode/agents/elbmesh-orchestrator.md",
    ".opencode/agents/elbmesh-test-writer.md",
    ".opencode/agents/elbmesh-pr-publisher.md",
    ".opencode/agents/elbmesh-implementer.md",
    ".opencode/agents/elbmesh-reviewer.md",
];

const ACTIVE_TEMPLATE_FILES: [&str; 4] = [
    ".github/ISSUE_TEMPLATE/decision-request.md",
    ".github/ISSUE_TEMPLATE/documentation-task.md",
    ".github/ISSUE_TEMPLATE/implementation-task.md",
    ".github/pull_request_template.md",
];

const CANONICAL_REQUIRED_READING: [&str; 8] = [
    "docs/GOAL.md",
    "docs/GLOSSARY.md",
    "docs/DEVELOPMENT_WORKFLOW.md",
    "docs/HUMAN_DECISION_LOOP.md",
    "docs/DELIVERY_ROADMAP.md",
    "docs/AGENT_SKILLS.md",
    "docs/IMPLEMENTATION_PLAN.md",
    "docs/adr/",
];

#[test]
fn delivery_roadmap_replaces_the_active_phased_plan() {
    let mut violations = Vec::new();
    let roadmap_path = project_path("docs/DELIVERY_ROADMAP.md");
    let phased_plan_path = project_path("docs/PHASED_DELIVERY_PLAN.md");

    if !roadmap_path.is_file() {
        violations.push("docs/DELIVERY_ROADMAP.md must exist".to_owned());
    } else {
        let roadmap = read_project_file("docs/DELIVERY_ROADMAP.md").to_ascii_lowercase();
        for term in ["dependency", "github issue", "source of truth"] {
            if !roadmap.contains(term) {
                violations.push(format!("docs/DELIVERY_ROADMAP.md must describe `{term}`"));
            }
        }
    }

    if phased_plan_path.exists() {
        violations.push(
            "docs/PHASED_DELIVERY_PLAN.md must be replaced, not retained as an active plan"
                .to_owned(),
        );
    }

    let mut roadmap_link_files = vec![
        "docs/README.md".to_owned(),
        "docs/DEVELOPMENT_WORKFLOW.md".to_owned(),
        "docs/AGENT_SKILLS.md".to_owned(),
        "docs/IMPLEMENTATION_PLAN.md".to_owned(),
        ".github/ISSUE_TEMPLATE/documentation-task.md".to_owned(),
        ".github/ISSUE_TEMPLATE/implementation-task.md".to_owned(),
    ];
    roadmap_link_files.extend(concrete_skill_paths());

    for path in roadmap_link_files {
        if !read_project_file(&path).contains("docs/DELIVERY_ROADMAP.md")
            && !read_project_file(&path).contains("DELIVERY_ROADMAP.md")
        {
            violations.push(format!("{path} must link or list docs/DELIVERY_ROADMAP.md"));
        }
    }

    for path in active_instruction_paths() {
        if read_project_file(&path).contains("PHASED_DELIVERY_PLAN.md") {
            violations.push(format!(
                "{path} is active guidance and must not reference PHASED_DELIVERY_PLAN.md"
            ));
        }
    }

    assert_contract(violations, "dependency roadmap contract violations");
}

#[test]
fn active_workflow_uses_dependency_order_and_milestone_checkpoints_not_phases() {
    let mut violations = Vec::new();
    let forbidden_contract_phrases = [
        "intentionally phased",
        "active phase",
        "planned phase",
        "phased delivery",
        "phased plan",
        "phase-scoped",
        "phase reference",
        "phase labels",
        "labels track phase",
        "owns phases",
        "phase priority",
        "phase start",
        "phase status",
        "phase sequencing",
        "after every two implementation phases",
        "next pair of phases",
        "every subsequent pair of phases",
        "two-phase review cadence",
        "red phase",
        "green phase",
        "review phase",
    ];

    for path in active_instruction_paths() {
        let document = read_project_file(&path);
        for (line_number, line) in document.lines().enumerate() {
            let normalized = line.trim().to_ascii_lowercase();
            for phrase in forbidden_contract_phrases {
                if normalized.contains(phrase) {
                    violations.push(format!(
                        "{path}:{} retains phase-governed delivery language `{}`",
                        line_number + 1,
                        line.trim()
                    ));
                }
            }

            if matches!(
                normalized.as_str(),
                "## phase"
                    | "## phase contract"
                    | "## phase checkpoint loop"
                    | "phase:"
                    | "- phase:"
            ) || normalized.starts_with("## phase and ")
                || normalized.starts_with("title: \"[phase")
                || (normalized.starts_with("labels:") && normalized.contains("phase:"))
            {
                violations.push(format!(
                    "{path}:{} retains an active Phase field, heading, title, or label `{}`",
                    line_number + 1,
                    line.trim()
                ));
            }
        }
    }

    let roadmap_path = project_path("docs/DELIVERY_ROADMAP.md");
    if !roadmap_path.is_file() {
        violations.push(
            "docs/DELIVERY_ROADMAP.md is required to verify dependency ordering and checkpoints"
                .to_owned(),
        );
    } else {
        let roadmap = read_project_file("docs/DELIVERY_ROADMAP.md").to_ascii_lowercase();
        for term in ["dependency", "capability", "milestone", "checkpoint"] {
            if !roadmap.contains(term) {
                violations.push(format!(
                    "docs/DELIVERY_ROADMAP.md must define {term}-driven delivery"
                ));
            }
        }
    }

    assert_contract(violations, "active phase-removal contract violations");
}

#[test]
fn active_status_contract_has_only_implementation_and_review() {
    let allowed_statuses = BTreeSet::from(["status:implementation", "status:review"]);
    let mut observed_statuses = BTreeSet::new();
    let mut violations = Vec::new();

    for path in active_instruction_paths() {
        let document = read_project_file(&path);
        for status in status_tokens(&document) {
            observed_statuses.insert(status.clone());
            if !allowed_statuses.contains(status.as_str()) {
                violations.push(format!(
                    "{path} uses retired active workflow status `{status}`"
                ));
            }
        }

        for paragraph in document.to_ascii_lowercase().split("\n\n") {
            if paragraph_requires_human_label_transition(paragraph) {
                violations.push(format!(
                    "{path} retains a human-applied routine label-transition gate: `{}`",
                    paragraph.lines().next().unwrap_or_default().trim()
                ));
            }
        }
    }

    for required in allowed_statuses {
        if !observed_statuses.contains(required) {
            violations.push(format!(
                "active instructions must define the workflow status `{required}`"
            ));
        }
    }

    assert_contract(violations, "active issue-status contract violations");
}

#[test]
fn publisher_automates_issue_status_transitions_without_merge_authority() {
    let publisher_agent = ".opencode/agents/elbmesh-pr-publisher.md";
    let publisher_skill = ".opencode/skills/elbmesh-pr-publisher/SKILL.md";
    let (frontmatter, agent_body) = frontmatter_and_body(publisher_agent);
    let bash_rules = yaml_nested_block(&frontmatter, "  bash:");
    let mut violations = Vec::new();

    for rule in [
        "\"*\": deny",
        "\"gh issue edit *\": allow",
        "\"git push origin main\": deny",
        "\"git merge\": deny",
        "\"git merge *\": deny",
        "\"gh pr merge\": deny",
        "\"gh pr merge *\": deny",
    ] {
        if !bash_rules.lines().any(|line| line.trim() == rule) {
            violations.push(format!(
                "{publisher_agent} Bash permissions must include `{rule}`"
            ));
        }
    }

    let broad_deny = bash_rules.find("\"*\": deny");
    let issue_edit = bash_rules.find("\"gh issue edit *\": allow");
    if !matches!((broad_deny, issue_edit), (Some(deny), Some(allow)) if deny < allow) {
        violations.push(
            "Publisher Bash permissions must place narrow gh issue edit allowance after broad deny"
                .to_owned(),
        );
    }

    let publisher_skill_body = read_project_file(publisher_skill);
    for (path, document) in [
        (publisher_agent, agent_body.as_str()),
        (publisher_skill, publisher_skill_body.as_str()),
    ] {
        let normalized = document.to_ascii_lowercase();
        if !has_paragraph_with_all(
            &normalized,
            &["red", "status:implementation"],
            &["set", "keep", "transition", "ensure"],
        ) {
            violations.push(format!(
                "{path} must automatically set or keep status:implementation after red publication"
            ));
        }
        if !has_paragraph_with_all(
            &normalized,
            &["reviewer", "ci", "ready", "status:review"],
            &["no blocker", "no-blocker", "no blocking"],
        ) {
            violations.push(format!(
                "{path} must move to status:review only after no-blocker Reviewer evidence and required CI while marking the PR ready"
            ));
        }
        if !normalized.contains("append-only") {
            violations.push(format!("{path} must preserve append-only evidence"));
        }
        if !normalized.contains("only a human") || !normalized.contains("merge") {
            violations.push(format!("{path} must preserve human-only merge authority"));
        }
    }

    assert_contract(
        violations,
        "Publisher status-automation contract violations",
    );
}

#[test]
fn orchestrator_delegates_status_and_preserves_delivery_safety() {
    let orchestrator_agent = ".opencode/agents/elbmesh-orchestrator.md";
    let orchestrator_skill = ".opencode/skills/elbmesh-orchestrator/SKILL.md";
    let (frontmatter, agent_body) = frontmatter_and_body(orchestrator_agent);
    let mut violations = Vec::new();

    if !frontmatter.lines().any(|line| line.trim() == "bash: deny") {
        violations.push(format!(
            "{orchestrator_agent} must keep Bash unconditionally denied"
        ));
    }

    let orchestrator_skill_body = read_project_file(orchestrator_skill);
    for (path, document) in [
        (orchestrator_agent, agent_body.as_str()),
        (orchestrator_skill, orchestrator_skill_body.as_str()),
    ] {
        let normalized = document.to_ascii_lowercase();

        if !normalized.contains("publisher")
            || !normalized.contains("status:implementation")
            || !normalized.contains("status:review")
            || !["delegate", "owns", "sets", "changes"]
                .iter()
                .any(|term| normalized.contains(term))
        {
            violations.push(format!(
                "{path} must delegate automatic issue-status changes to the Publisher"
            ));
        }

        if normalized
            .split("\n\n")
            .any(paragraph_requires_human_label_transition)
        {
            violations.push(format!(
                "{path} must not ask the human to perform routine label transitions"
            ));
        }

        for (contract, terms) in [
            (
                "tests before implementation",
                &["test writer", "before implementation"][..],
            ),
            (
                "immutable accepted tests",
                &["accepted tests", "immutable"][..],
            ),
            (
                "separate red and green commits",
                &["test-only commit", "implementation/docs commit", "separate"][..],
            ),
            ("final Reviewer", &["elbmesh-reviewer", "final"][..]),
            ("human-only merge", &["human", "merge", "only"][..]),
        ] {
            if !terms.iter().all(|term| normalized.contains(term)) {
                violations.push(format!("{path} must enforce {contract}"));
            }
        }
    }

    assert_contract(
        violations,
        "Orchestrator delivery-safety contract violations",
    );
}

#[test]
fn canonical_and_concrete_skill_sets_are_equal() {
    let catalog = read_project_file("docs/AGENT_SKILLS.md");
    let catalog_headings: Vec<_> = catalog
        .lines()
        .filter_map(|line| line.trim().strip_prefix("### "))
        .filter(|name| name.starts_with("elbmesh-"))
        .map(str::to_owned)
        .collect();
    let catalog_names: BTreeSet<_> = catalog_headings.iter().cloned().collect();
    let concrete_names: BTreeSet<_> = concrete_skill_names().into_iter().collect();
    let mut violations = Vec::new();

    if catalog_names.is_empty() {
        violations.push("docs/AGENT_SKILLS.md must catalog concrete Elbmesh skills".to_owned());
    }
    if catalog_names.len() != catalog_headings.len() {
        violations
            .push("docs/AGENT_SKILLS.md contains duplicate concrete skill headings".to_owned());
    }
    if catalog_names != concrete_names {
        let missing_concrete: Vec<_> = catalog_names.difference(&concrete_names).collect();
        let missing_catalog: Vec<_> = concrete_names.difference(&catalog_names).collect();
        violations.push(format!(
            "catalog/concrete skill mismatch; missing concrete: {missing_concrete:?}; missing catalog entries: {missing_catalog:?}"
        ));
    }

    assert_contract(violations, "canonical/concrete skill-set violations");
}

#[test]
fn concrete_skills_follow_the_canonical_skill_contract() {
    let catalog = read_project_file("docs/AGENT_SKILLS.md");
    let required_reading = fenced_block_after(&catalog, "Required reading for all Elbmesh skills:");
    let required_reading_set: BTreeSet<_> = required_reading.iter().map(String::as_str).collect();
    let mut violations = Vec::new();

    for required in CANONICAL_REQUIRED_READING {
        if !required_reading_set.contains(required) {
            violations.push(format!(
                "docs/AGENT_SKILLS.md canonical required reading must include `{required}`"
            ));
        }
    }
    if required_reading_set.contains("docs/PHASED_DELIVERY_PLAN.md") {
        violations.push(
            "docs/AGENT_SKILLS.md canonical required reading retains PHASED_DELIVERY_PLAN.md"
                .to_owned(),
        );
    }

    for path in concrete_skill_paths() {
        let skill = read_project_file(&path);
        let sections = level_two_sections(&skill);

        for required in &required_reading {
            if !skill.lines().any(|line| line.trim() == required) {
                violations.push(format!(
                    "{path} is stale against canonical required reading `{required}`"
                ));
            }
        }

        let edit_surface = find_section(&sections, |heading| {
            heading.contains("edit")
                && (heading.contains("surface")
                    || heading.contains("files")
                    || heading.contains("permission"))
        });
        if !section_has_content(edit_surface) {
            violations.push(format!(
                "{path} must declare an explicit permitted edit surface"
            ));
        }

        let outputs = find_section(&sections, |heading| heading.contains("output"));
        if !section_has_content(outputs) {
            violations.push(format!("{path} must declare required outputs"));
        }

        let verification = find_section(&sections, |heading| heading.contains("verification"));
        if !section_has_exact_verification(verification) {
            violations.push(format!(
                "{path} must declare exact verification commands (or explicitly state that no repository command applies)"
            ));
        }

        let architecture = find_section(&sections, |heading| {
            heading.contains("architecture rules") || heading.contains("preserve")
        });
        let architecture = architecture.unwrap_or_default().to_ascii_lowercase();
        for boundary in ["resource", "action", "event", "reaction", "view"] {
            if !architecture.contains(boundary) {
                violations.push(format!(
                    "{path} architecture-preservation section must preserve the {boundary} boundary"
                ));
            }
        }
    }

    assert_contract(violations, "concrete skill-contract violations");
}

#[test]
fn decision_capable_skills_reference_the_human_decision_loop() {
    let mut violations = Vec::new();

    for skill in [
        "elbmesh-driver",
        "elbmesh-orchestrator",
        "elbmesh-doc-maintainer",
        "elbmesh-manifest-editor",
        "elbmesh-architecture-checker",
    ] {
        let path = format!(".opencode/skills/{skill}/SKILL.md");
        if !read_project_file(&path).contains("docs/HUMAN_DECISION_LOOP.md") {
            violations.push(format!(
                "{path} can encounter semantic decisions and must reference docs/HUMAN_DECISION_LOOP.md"
            ));
        }
    }

    assert_contract(violations, "Human Decision Loop skill-contract violations");
}

#[test]
fn dependency_delivery_adr_supersedes_the_phased_delivery_decision() {
    let old_adr_path = "docs/adr/0014-phased-mr-based-multi-agent-delivery.md";
    let old_adr = read_project_file(old_adr_path);
    let mut violations = Vec::new();

    if !old_adr
        .lines()
        .any(|line| line.to_ascii_lowercase().starts_with("status: superseded"))
    {
        violations.push(format!(
            "{old_adr_path} must remain present and be marked Status: Superseded"
        ));
    }

    let mut superseding_adrs = Vec::new();
    for entry in fs::read_dir(project_path("docs/adr"))
        .unwrap_or_else(|error| panic!("docs/adr must be readable: {error}"))
    {
        let path = entry
            .unwrap_or_else(|error| panic!("ADR entry must be readable: {error}"))
            .path();
        if path.extension().and_then(|value| value.to_str()) != Some("md")
            || path.file_name().and_then(|value| value.to_str())
                == Some("0014-phased-mr-based-multi-agent-delivery.md")
        {
            continue;
        }

        let document = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{} must be readable: {error}", path.display()));
        let normalized = document.to_ascii_lowercase();
        if normalized.contains("dependency-ordered") && normalized.contains("github issue") {
            superseding_adrs.push((path, normalized));
        }
    }

    if superseding_adrs.is_empty() {
        violations.push(
            "a new dependency-ordered GitHub Issue delivery ADR must supersede ADR 0014".to_owned(),
        );
    }

    let docs_index = read_project_file("docs/README.md");
    for (path, adr) in superseding_adrs {
        for term in [
            "status: accepted",
            "source of truth",
            "status:implementation",
            "status:review",
            "publisher",
            "milestone",
            "checkpoint",
            "supersed",
            "0014",
        ] {
            if !adr.contains(term) {
                violations.push(format!(
                    "{} must record the superseding delivery decision term `{term}`",
                    path.display()
                ));
            }
        }

        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .expect("ADR filename must be UTF-8");
        if !docs_index.contains(&format!("adr/{file_name}")) {
            violations.push(format!(
                "docs/README.md ADR index must link adr/{file_name}"
            ));
        }
    }

    assert_contract(violations, "delivery ADR supersession contract violations");
}

#[test]
fn opencode_changes_require_a_post_merge_restart() {
    let relevant_docs = [
        "docs/DEVELOPMENT_WORKFLOW.md",
        "docs/AGENT_DELIVERY_HARNESS.md",
        "docs/AGENT_SKILLS.md",
    ];
    let mut documents_with_restart_rule = Vec::new();

    for path in relevant_docs {
        let document = read_project_file(path).to_ascii_lowercase();
        if document.split("\n\n").any(|paragraph| {
            paragraph.contains("opencode")
                && paragraph.contains("restart")
                && paragraph.contains("merge")
                && ["agent", "skill", "config"]
                    .iter()
                    .any(|term| paragraph.contains(term))
        }) {
            documents_with_restart_rule.push(path);
        }
    }

    let mut violations = Vec::new();
    if !documents_with_restart_rule.contains(&"docs/AGENT_DELIVERY_HARNESS.md") {
        violations.push(
            "docs/AGENT_DELIVERY_HARNESS.md must require an OpenCode restart after merged agent/skill/config-time changes"
                .to_owned(),
        );
    }
    if documents_with_restart_rule.len() < 2 {
        violations.push(
            "at least one canonical workflow/catalog doc must repeat the post-merge OpenCode restart rule"
                .to_owned(),
        );
    }

    assert_contract(violations, "OpenCode restart contract violations");
}

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn project_path(relative_path: &str) -> PathBuf {
    project_root().join(relative_path)
}

fn read_project_file(relative_path: &str) -> String {
    fs::read_to_string(project_path(relative_path)).unwrap_or_else(|error| {
        panic!("required repository contract file `{relative_path}` is unavailable: {error}")
    })
}

fn active_instruction_paths() -> Vec<String> {
    let mut paths: Vec<_> = ACTIVE_PROCESS_DOCS
        .into_iter()
        .chain(ACTIVE_AGENT_FILES)
        .chain(ACTIVE_TEMPLATE_FILES)
        .map(str::to_owned)
        .collect();
    paths.extend(concrete_skill_paths());
    if project_path("docs/DELIVERY_ROADMAP.md").is_file() {
        paths.push("docs/DELIVERY_ROADMAP.md".to_owned());
    }
    paths
}

fn concrete_skill_names() -> Vec<String> {
    let mut names = Vec::new();
    for entry in fs::read_dir(project_path(".opencode/skills"))
        .unwrap_or_else(|error| panic!(".opencode/skills must be readable: {error}"))
    {
        let entry = entry.unwrap_or_else(|error| panic!("skill entry must be readable: {error}"));
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if path.is_dir() && name.starts_with("elbmesh-") && path.join("SKILL.md").is_file() {
            names.push(name.to_owned());
        }
    }
    names.sort();
    names
}

fn concrete_skill_paths() -> Vec<String> {
    concrete_skill_names()
        .into_iter()
        .map(|name| format!(".opencode/skills/{name}/SKILL.md"))
        .collect()
}

fn status_tokens(document: &str) -> BTreeSet<String> {
    document
        .to_ascii_lowercase()
        .split(|character: char| {
            !(character.is_ascii_alphanumeric() || character == ':' || character == '-')
        })
        .filter(|token| token.starts_with("status:") && token.len() > "status:".len())
        .map(str::to_owned)
        .collect()
}

fn paragraph_requires_human_label_transition(paragraph: &str) -> bool {
    let describes_human_gate = paragraph.contains("human")
        && paragraph.contains("label")
        && (paragraph.contains("transition") || paragraph.contains("mutation"))
        && (paragraph.contains("request") || paragraph.contains("appl"));
    let rejects_human_gate = [
        "does not ask",
        "must not ask",
        "no human-applied",
        "without human-applied",
        "not require human",
        "publisher, not",
    ]
    .iter()
    .any(|term| paragraph.contains(term));

    describes_human_gate && !rejects_human_gate
}

fn frontmatter_and_body(path: &str) -> (String, String) {
    let document = read_project_file(path);
    let document = document
        .strip_prefix("---\n")
        .unwrap_or_else(|| panic!("{path} must begin with YAML frontmatter"));
    let (frontmatter, body) = document
        .split_once("\n---\n")
        .unwrap_or_else(|| panic!("{path} must close its YAML frontmatter"));
    (frontmatter.to_owned(), body.to_owned())
}

fn yaml_nested_block(document: &str, heading: &str) -> String {
    let heading_indent = indentation(heading);
    let mut in_block = false;
    let mut lines = Vec::new();

    for line in document.lines() {
        if !in_block {
            if line == heading {
                in_block = true;
            }
            continue;
        }
        if !line.trim().is_empty() && indentation(line) <= heading_indent {
            break;
        }
        lines.push(line);
    }

    lines.join("\n")
}

fn indentation(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn has_paragraph_with_all(document: &str, required_terms: &[&str], any_of_terms: &[&str]) -> bool {
    document.split("\n\n").any(|paragraph| {
        required_terms.iter().all(|term| paragraph.contains(term))
            && any_of_terms.iter().any(|term| paragraph.contains(term))
    })
}

fn fenced_block_after(document: &str, marker: &str) -> Vec<String> {
    let Some((_, after_marker)) = document.split_once(marker) else {
        return Vec::new();
    };
    let mut lines = after_marker.lines();
    for line in lines.by_ref() {
        if line.trim().starts_with("```") {
            break;
        }
    }

    lines
        .take_while(|line| !line.trim().starts_with("```"))
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

fn level_two_sections(document: &str) -> Vec<(String, String)> {
    let mut sections = Vec::new();
    let mut current: Option<(String, Vec<String>)> = None;

    for line in document.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            if let Some((heading, body)) = current.take() {
                sections.push((heading, body.join("\n")));
            }
            current = Some((heading.trim().to_ascii_lowercase(), Vec::new()));
        } else if let Some((_, body)) = current.as_mut() {
            body.push(line.to_owned());
        }
    }

    if let Some((heading, body)) = current {
        sections.push((heading, body.join("\n")));
    }

    sections
}

fn find_section(sections: &[(String, String)], predicate: impl Fn(&str) -> bool) -> Option<&str> {
    sections
        .iter()
        .find(|(heading, _)| predicate(heading))
        .map(|(_, body)| body.as_str())
}

fn section_has_content(section: Option<&str>) -> bool {
    section.is_some_and(|body| {
        body.lines()
            .map(str::trim)
            .any(|line| !line.is_empty() && !line.starts_with("```"))
    })
}

fn section_has_exact_verification(section: Option<&str>) -> bool {
    section.is_some_and(|body| {
        let normalized = body.to_ascii_lowercase();
        ["cargo ", "git ", "gh ", "codehud "]
            .iter()
            .any(|command| normalized.contains(command))
            || (normalized.contains("no repository") && normalized.contains("command"))
    })
}

fn assert_contract(violations: Vec<String>, contract: &str) {
    assert!(
        violations.is_empty(),
        "{contract}:\n- {}",
        violations.join("\n- ")
    );
}
