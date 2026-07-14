use std::{fs, path::Path};

use serde_json::Value;

const ORCHESTRATOR_AGENT: &str = ".opencode/agents/elbmesh-orchestrator.md";
const TEST_WRITER_AGENT: &str = ".opencode/agents/elbmesh-test-writer.md";
const IMPLEMENTER_AGENT: &str = ".opencode/agents/elbmesh-implementer.md";
const REVIEWER_AGENT: &str = ".opencode/agents/elbmesh-reviewer.md";

#[test]
fn project_config_selects_the_primary_elbmesh_orchestrator() {
    let config = project_file("opencode.json");
    let config: Value = serde_json::from_str(&config)
        .unwrap_or_else(|error| panic!("opencode.json must be valid JSON: {error}"));

    assert_eq!(
        config.get("default_agent").and_then(Value::as_str),
        Some("elbmesh-orchestrator"),
        "opencode.json must select elbmesh-orchestrator as default_agent"
    );

    let (frontmatter, body) = agent_file(ORCHESTRATOR_AGENT);
    assert_agent_mode(ORCHESTRATOR_AGENT, &frontmatter, "primary");
    assert_skill_reference(ORCHESTRATOR_AGENT, &body, "elbmesh-orchestrator");
}

#[test]
fn orchestrator_sequences_fresh_role_sessions_after_red_and_green_gates() {
    let (_, body) = agent_file(ORCHESTRATOR_AGENT);
    let body = body.to_ascii_lowercase();

    let has_gated_sequence = word_positions(&body, "red").into_iter().any(|red| {
        word_positions(&body[red + "red".len()..], "green")
            .into_iter()
            .any(|relative_green| {
                let green = red + "red".len() + relative_green;

                describes_fresh_spawn(&body[..red], "elbmesh-test-writer")
                    && describes_fresh_spawn(&body[red + "red".len()..green], "elbmesh-implementer")
                    && describes_fresh_spawn(&body[green + "green".len()..], "elbmesh-reviewer")
            })
    });

    assert!(
        has_gated_sequence,
        "{ORCHESTRATOR_AGENT} must spawn fresh test-writer, implementer, and reviewer sessions in that order, with red before implementation and green before review"
    );
}

#[test]
fn role_subagents_enforce_test_implementation_and_review_boundaries() {
    let (test_frontmatter, test_body) = agent_file(TEST_WRITER_AGENT);
    let (implementer_frontmatter, implementer_body) = agent_file(IMPLEMENTER_AGENT);
    let (reviewer_frontmatter, reviewer_body) = agent_file(REVIEWER_AGENT);

    for (path, frontmatter, body, skill) in [
        (
            TEST_WRITER_AGENT,
            &test_frontmatter,
            &test_body,
            "elbmesh-test-writer",
        ),
        (
            IMPLEMENTER_AGENT,
            &implementer_frontmatter,
            &implementer_body,
            "elbmesh-implementer",
        ),
        (
            REVIEWER_AGENT,
            &reviewer_frontmatter,
            &reviewer_body,
            "elbmesh-reviewer",
        ),
    ] {
        assert_agent_mode(path, frontmatter, "subagent");
        assert_skill_reference(path, body, skill);
    }

    let test_rules = edit_permission_rules(&test_frontmatter);
    assert!(
        test_rules
            .iter()
            .any(|(pattern, action)| pattern == "*" && action == "deny"),
        "{TEST_WRITER_AGENT} must deny edits by default"
    );
    let allowed_test_paths: Vec<_> = test_rules
        .iter()
        .filter(|(_, action)| action == "allow")
        .map(|(pattern, _)| pattern.as_str())
        .collect();
    assert!(
        !allowed_test_paths.is_empty() && allowed_test_paths.iter().all(|path| is_test_path(path)),
        "{TEST_WRITER_AGENT} may allow edits only under tests or test-fixture paths, found {allowed_test_paths:?}"
    );
    assert_contains_all(
        TEST_WRITER_AGENT,
        &test_body,
        &["focused", "red", "proof", "reason", "fixture"],
    );

    let implementer_body_lowercase = implementer_body.to_ascii_lowercase();
    assert!(
        implementer_body_lowercase.split("\n\n").any(|paragraph| {
            paragraph.contains("accepted test")
                && ["change", "modify", "edit"]
                    .iter()
                    .any(|action| paragraph.contains(action))
                && ["do not", "must not", "never", "without"]
                    .iter()
                    .any(|prohibition| paragraph.contains(prohibition))
        }),
        "{IMPLEMENTER_AGENT} must prohibit changing accepted tests without escalation"
    );
    assert_contains_all(
        IMPLEMENTER_AGENT,
        &implementer_body,
        &["blocker", "escalat"],
    );

    assert!(
        edit_is_denied(&reviewer_frontmatter),
        "{REVIEWER_AGENT} must deny edit permission"
    );
    assert_prohibits_action(REVIEWER_AGENT, &reviewer_body, &["modify", "edit"]);
    assert_prohibits_action(REVIEWER_AGENT, &reviewer_body, &["merge"]);
}

#[test]
fn harness_agent_skills_include_canonical_required_reading() {
    let required_reading = [
        "docs/GOAL.md",
        "docs/GLOSSARY.md",
        "docs/DEVELOPMENT_WORKFLOW.md",
        "docs/HUMAN_DECISION_LOOP.md",
        "docs/PHASED_DELIVERY_PLAN.md",
        "docs/IMPLEMENTATION_PLAN.md",
        "docs/adr/",
    ];
    let skill_paths = [
        ".opencode/skills/elbmesh-orchestrator/SKILL.md",
        ".opencode/skills/elbmesh-test-writer/SKILL.md",
        ".opencode/skills/elbmesh-implementer/SKILL.md",
        ".opencode/skills/elbmesh-reviewer/SKILL.md",
    ];
    let mut violations = Vec::new();

    for path in skill_paths {
        let skill = project_file(path);
        for required_path in required_reading {
            if !skill.lines().any(|line| line.trim() == required_path) {
                violations.push(format!(
                    "{path} must include canonical required reading `{required_path}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "harness agent skill required-reading violations:\n- {}",
        violations.join("\n- ")
    );
}

#[test]
fn every_concrete_elbmesh_skill_includes_canonical_required_reading() {
    let required_reading = [
        "docs/GOAL.md",
        "docs/GLOSSARY.md",
        "docs/DEVELOPMENT_WORKFLOW.md",
        "docs/HUMAN_DECISION_LOOP.md",
        "docs/PHASED_DELIVERY_PLAN.md",
        "docs/IMPLEMENTATION_PLAN.md",
        "docs/adr/",
    ];
    let skill_paths = [
        ".opencode/skills/elbmesh-driver/SKILL.md",
        ".opencode/skills/elbmesh-orchestrator/SKILL.md",
        ".opencode/skills/elbmesh-test-writer/SKILL.md",
        ".opencode/skills/elbmesh-implementer/SKILL.md",
        ".opencode/skills/elbmesh-reviewer/SKILL.md",
        ".opencode/skills/elbmesh-mr-reviewer/SKILL.md",
        ".opencode/skills/elbmesh-doc-maintainer/SKILL.md",
        ".opencode/skills/elbmesh-architecture-checker/SKILL.md",
        ".opencode/skills/elbmesh-flow-explainer/SKILL.md",
        ".opencode/skills/elbmesh-manifest-editor/SKILL.md",
    ];
    let mut violations = Vec::new();

    for path in skill_paths {
        let skill = project_file(path);
        for required_path in required_reading {
            if !skill.lines().any(|line| line.trim() == required_path) {
                violations.push(format!(
                    "{path} must include canonical required reading `{required_path}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "concrete Elbmesh skill required-reading violations:\n- {}",
        violations.join("\n- ")
    );
}

#[test]
fn canonical_queue_docs_keep_label_mutations_human_applied() {
    let paths = [
        "docs/HUMAN_DECISION_LOOP.md",
        "docs/adr/0015-use-github-issues-as-operational-queue.md",
    ];
    let mut violations = Vec::new();

    for path in paths {
        let document = project_file(path).to_ascii_lowercase();
        let has_shell_free_handoff = document.split("\n\n").any(|paragraph| {
            paragraph.contains("orchestrator")
                && (paragraph.contains("issue-label transition")
                    || paragraph.contains("label transition"))
                && paragraph.contains("report")
                && paragraph.contains("request")
                && paragraph.contains("human")
                && paragraph.contains("appl")
                && paragraph.contains("label mutation")
                && [
                    "bash is denied",
                    "bash denied",
                    "no shell",
                    "without shell",
                    "shell-free",
                ]
                .iter()
                .any(|term| paragraph.contains(term))
        });
        if !has_shell_free_handoff {
            violations.push(format!(
                "{path} must state that the shell-free Orchestrator reports and requests issue-label transitions while a human applies label mutations"
            ));
        }

        let manages_desired_queue_state = document.split("\n\n").any(|paragraph| {
            paragraph.contains("orchestrator")
                && paragraph.contains("manag")
                && paragraph.contains("desired queue state")
        });
        if !manages_desired_queue_state {
            violations.push(format!(
                "{path} must preserve the Orchestrator's responsibility to manage desired queue state"
            ));
        }

        for line in document.lines() {
            if grants_orchestrator_direct_label_mutation(line) {
                violations.push(format!(
                    "{path} grants direct label mutation authority to the Orchestrator: `{}`",
                    line.trim()
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "canonical issue-label authority violations:\n- {}",
        violations.join("\n- ")
    );
}

#[test]
fn harness_documentation_keeps_merge_authority_human_and_tracks_issue_transitions() {
    let path = "docs/AGENT_DELIVERY_HARNESS.md";
    let documentation = project_file(path).to_ascii_lowercase();

    assert!(
        documentation.split("\n\n").any(|paragraph| {
            ["human", "merge", "authorit"]
                .iter()
                .all(|term| paragraph.contains(term))
        }),
        "{path} must state that merge authority remains human"
    );

    let labels = [
        "status:tests-needed",
        "status:tests-ready",
        "status:implementation",
        "status:review",
        "status:merged",
    ];
    assert!(
        documentation.split("\n\n").any(|paragraph| {
            (paragraph.contains("transition") || paragraph.contains("->"))
                && contains_in_order(paragraph, &labels)
        }),
        "{path} must document the normal issue-label transition from tests-needed through human-approved merge"
    );
}

#[test]
fn harness_documentation_makes_label_transitions_human_applied() {
    let path = "docs/AGENT_DELIVERY_HARNESS.md";
    let documentation = project_file(path).to_ascii_lowercase();
    let labels = [
        "status:tests-needed",
        "status:tests-ready",
        "status:implementation",
        "status:review",
        "status:merged",
    ];
    let transition_paragraph = documentation
        .split("\n\n")
        .find(|paragraph| contains_in_order(paragraph, &labels))
        .unwrap_or_else(|| panic!("{path} must preserve the normal issue-label transition"));
    let shell_free_handoff = transition_paragraph.contains("orchestrator")
        && transition_paragraph.contains("report")
        && transition_paragraph.contains("request")
        && transition_paragraph.contains("human")
        && ["human applies", "human performs", "human makes"]
            .iter()
            .any(|term| transition_paragraph.contains(term))
        && [
            "bash is denied",
            "bash denied",
            "no shell",
            "without shell",
            "shell-free",
        ]
        .iter()
        .any(|term| transition_paragraph.contains(term));

    assert!(
        shell_free_handoff,
        "{path} must state that the shell-free Orchestrator reports readiness and requests each label transition while the human applies it"
    );
}

#[test]
fn only_the_orchestrator_can_spawn_delivery_role_agents() {
    let config = project_file("opencode.json");
    let config: Value = serde_json::from_str(&config)
        .unwrap_or_else(|error| panic!("opencode.json must be valid JSON: {error}"));
    let mut violations = Vec::new();

    if project_permission_default(&config, "task").as_deref() != Some("deny") {
        violations.push("opencode.json must deny Task by default".to_owned());
    }

    for path in [TEST_WRITER_AGENT, IMPLEMENTER_AGENT, REVIEWER_AGENT] {
        let (frontmatter, _) = agent_file(path);
        if permission_default_action(&frontmatter, "task").as_deref() != Some("deny") {
            violations.push(format!("{path} must explicitly deny Task"));
        }
    }

    let (orchestrator_frontmatter, _) = agent_file(ORCHESTRATOR_AGENT);
    let task_rules = permission_rules(&orchestrator_frontmatter, "task");
    if task_rules
        .first()
        .map(|(pattern, action)| (pattern.as_str(), action.as_str()))
        != Some(("*", "deny"))
    {
        violations.push(format!(
            "{ORCHESTRATOR_AGENT} Task rules must begin with a broad deny"
        ));
    }
    if task_rules.len() != 4 {
        violations.push(format!(
            "{ORCHESTRATOR_AGENT} must have only one broad Task deny followed by the three role allows, found {task_rules:?}"
        ));
    }

    let mut allowed_agents: Vec<_> = task_rules
        .iter()
        .filter(|(_, action)| action == "allow")
        .map(|(pattern, _)| pattern.as_str())
        .collect();
    allowed_agents.sort_unstable();
    let mut expected_agents = [
        "elbmesh-test-writer",
        "elbmesh-implementer",
        "elbmesh-reviewer",
    ];
    expected_agents.sort_unstable();
    if allowed_agents != expected_agents {
        violations.push(format!(
            "{ORCHESTRATOR_AGENT} may allow Task only for the three delivery roles, found {allowed_agents:?}"
        ));
    }

    for agent in expected_agents {
        if permission_decision(&orchestrator_frontmatter, "task", agent).as_deref() != Some("allow")
        {
            violations.push(format!(
                "{ORCHESTRATOR_AGENT} must effectively allow Task for {agent} after its broad deny"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Task coordination contract violations:\n- {}",
        violations.join("\n- ")
    );
}

#[test]
fn reviewer_bash_allows_only_exact_quality_commands() {
    let (frontmatter, _) = agent_file(REVIEWER_AGENT);
    let required_quality_commands = [
        "cargo fmt --check",
        "cargo clippy --all-targets --all-features -- -D warnings",
        "cargo test --all",
    ];
    let mut violations = Vec::new();

    if !edit_is_denied(&frontmatter) {
        violations.push("edit permission is not denied".to_owned());
    }
    if permission_default_action(&frontmatter, "task").as_deref() != Some("deny") {
        violations.push("Task is not explicitly denied".to_owned());
    }

    let bash_rules = permission_rules(&frontmatter, "bash");
    let default_rule = bash_rules
        .iter()
        .enumerate()
        .find(|(_, (pattern, _))| pattern == "*");
    if !matches!(default_rule, Some((0, (_, action))) if action == "ask" || action == "deny") {
        violations.push("Bash rules must begin with a broad ask or deny".to_owned());
    }

    let mut allowed_commands: Vec<_> = bash_rules
        .iter()
        .filter(|(_, action)| action == "allow")
        .map(|(pattern, _)| pattern.as_str())
        .collect();
    allowed_commands.sort_unstable();
    let mut expected_commands = required_quality_commands;
    expected_commands.sort_unstable();
    if allowed_commands != expected_commands {
        violations.push(format!(
            "Bash allow rules must be the exact required quality commands, found {allowed_commands:?}"
        ));
    }

    for command in required_quality_commands {
        if permission_decision(&frontmatter, "bash", command).as_deref() != Some("allow") {
            violations.push(format!(
                "required quality command is not allowed exactly: {command}"
            ));
        }
    }

    for command in [
        "codehud edit crates/elbmesh-core/src/lib.rs Resource",
        "git diff --output=review.patch",
        "git show --output=review.txt HEAD",
        "cargo test --all > review.txt",
    ] {
        if effective_agent_permission(&frontmatter, "bash", command) == "allow" {
            violations.push(format!("Bash bypass remains allowed: {command}"));
        }
    }

    assert!(
        violations.is_empty(),
        "{REVIEWER_AGENT} read-only permission violations:\n- {}",
        violations.join("\n- ")
    );
}

#[test]
fn reviewer_and_orchestrator_permissions_are_effectively_read_only() {
    let (reviewer_frontmatter, _) = agent_file(REVIEWER_AGENT);
    let (orchestrator_frontmatter, _) = agent_file(ORCHESTRATOR_AGENT);
    let expected_reviewer_bash_rules = [
        ("*", "deny"),
        ("cargo fmt --check", "allow"),
        (
            "cargo clippy --all-targets --all-features -- -D warnings",
            "allow",
        ),
        ("cargo test --all", "allow"),
    ];
    let reviewer_bash_rules = permission_rules(&reviewer_frontmatter, "bash");
    let reviewer_bash_rules: Vec<_> = reviewer_bash_rules
        .iter()
        .map(|(pattern, action)| (pattern.as_str(), action.as_str()))
        .collect();
    let mut violations = Vec::new();

    if reviewer_bash_rules.as_slice() != expected_reviewer_bash_rules {
        violations.push(format!(
            "{REVIEWER_AGENT} Bash rules must be broad deny followed only by the exact three quality-gate allows, found {reviewer_bash_rules:?}"
        ));
    }

    for (agent, frontmatter) in [
        (REVIEWER_AGENT, &reviewer_frontmatter),
        (ORCHESTRATOR_AGENT, &orchestrator_frontmatter),
    ] {
        for path in [
            "crates/elbmesh-core/src/lib.rs",
            "docs/AGENT_DELIVERY_HARNESS.md",
        ] {
            let decision = effective_agent_permission(frontmatter, "edit", path);
            if decision != "deny" {
                violations.push(format!(
                    "{agent} effective Edit permission for `{path}` is {decision} instead of deny"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "effective read-only permission violations:\n- {}",
        violations.join("\n- ")
    );
}

#[test]
fn test_writer_cannot_bypass_path_limited_edits() {
    let (frontmatter, _) = agent_file(TEST_WRITER_AGENT);
    let mut violations = Vec::new();

    if permission_default_action(&frontmatter, "task").as_deref() != Some("deny") {
        violations.push("Task is not explicitly denied".to_owned());
    }
    for command in [
        "printf bypass > crates/elbmesh-core/src/lib.rs",
        "python -c 'open(\"crates/elbmesh-core/src/lib.rs\", \"w\").write(\"bypass\")'",
        "git apply implementation.patch",
        "cargo test -p elbmesh-core > crates/elbmesh-core/src/lib.rs",
    ] {
        if effective_agent_permission(&frontmatter, "bash", command) == "allow" {
            violations.push(format!(
                "unrestricted Bash permits an edit bypass: {command}"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "{TEST_WRITER_AGENT} path-boundary violations:\n- {}",
        violations.join("\n- ")
    );
}

#[test]
fn implementer_cannot_edit_accepted_tests_or_spawn_tasks() {
    let (frontmatter, _) = agent_file(IMPLEMENTER_AGENT);
    let mut violations = Vec::new();

    if permission_default_action(&frontmatter, "task").as_deref() != Some("deny") {
        violations.push("Task is not explicitly denied".to_owned());
    }
    for path in [
        "tests/accepted.rs",
        "fixtures/accepted.json",
        "test-fixtures/accepted.json",
        "crates/elbmesh-core/tests/accepted.rs",
        "crates/elbmesh-core/fixtures/accepted.json",
        "crates/elbmesh-core/test-fixtures/accepted.json",
    ] {
        if effective_agent_permission(&frontmatter, "edit", path) != "deny" {
            violations.push(format!("accepted test or fixture remains editable: {path}"));
        }
    }

    assert!(
        violations.is_empty(),
        "{IMPLEMENTER_AGENT} accepted-test boundary violations:\n- {}",
        violations.join("\n- ")
    );
}

#[test]
fn implementer_cannot_bypass_accepted_test_and_fixture_edits_through_bash() {
    let (frontmatter, _) = agent_file(IMPLEMENTER_AGENT);
    let mut violations = Vec::new();

    if !matches!(
        permission_default_action(&frontmatter, "bash").as_deref(),
        Some("ask" | "deny")
    ) {
        violations.push("Bash must have an explicit restrictive ask or deny default".to_owned());
    }
    if permission_default_action(&frontmatter, "task").as_deref() != Some("deny") {
        violations.push("Task is not explicitly denied".to_owned());
    }

    for command in [
        "printf bypass > crates/elbmesh-core/tests/accepted.rs",
        "python -c 'open(\"crates/elbmesh-core/fixtures/accepted.json\", \"w\").write(\"bypass\")'",
        "git apply --include=crates/elbmesh-core/tests/accepted.rs accepted-tests.patch",
        "cargo test -p elbmesh-core > crates/elbmesh-core/test-fixtures/accepted.log",
    ] {
        if effective_agent_permission(&frontmatter, "bash", command) == "allow" {
            violations.push(format!(
                "Bash resolves to allow for an accepted test or fixture write: {command}"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "{IMPLEMENTER_AGENT} Bash boundary violations:\n- {}",
        violations.join("\n- ")
    );
}

#[test]
fn orchestrator_cannot_implement_or_merge() {
    let (frontmatter, _) = agent_file(ORCHESTRATOR_AGENT);
    let mut violations = Vec::new();

    for path in [
        "crates/elbmesh-core/src/lib.rs",
        "crates/elbmesh-core/src/runtime.rs",
    ] {
        if effective_agent_permission(&frontmatter, "edit", path) != "deny" {
            violations.push(format!("implementation edit remains permitted: {path}"));
        }
    }
    for command in [
        "git merge implementation",
        "git merge --continue",
        "gh pr merge 147",
    ] {
        if effective_agent_permission(&frontmatter, "bash", command) != "deny" {
            violations.push(format!("merge command is not explicitly denied: {command}"));
        }
    }

    assert!(
        violations.is_empty(),
        "{ORCHESTRATOR_AGENT} coordination-only boundary violations:\n- {}",
        violations.join("\n- ")
    );
}

#[test]
fn orchestrator_denies_all_bash_and_reports_transitions_to_the_human() {
    let (frontmatter, body) = agent_file(ORCHESTRATOR_AGENT);
    let mut violations = Vec::new();

    if permission_default_action(&frontmatter, "bash").as_deref() != Some("deny") {
        violations.push("Bash default is not deny".to_owned());
    }
    match permission_setting(&frontmatter, "bash") {
        Some(PermissionSetting::Scalar(action)) if action == "deny" => {}
        Some(PermissionSetting::Rules(rules))
            if !rules.is_empty() && rules.iter().all(|(_, action)| action == "deny") => {}
        setting => violations.push(format!(
            "Bash must be unconditionally denied without ask or allow exceptions, found {setting:?}"
        )),
    }

    for command in [
        "git status",
        "gh issue edit 147 --remove-label status:tests-needed --add-label status:tests-ready",
        "git\tmerge implementation",
        "git -c merge.ff=only merge implementation",
        "gh\tpr merge 147 --auto",
        "gh pr merge 147 --auto",
        "cargo test --all",
    ] {
        let decision = effective_agent_permission(&frontmatter, "bash", command);
        if decision != "deny" {
            violations.push(format!(
                "Bash command resolves to {decision} instead of deny: {command}"
            ));
        }
    }

    let body = body.to_ascii_lowercase();
    let reports_transitions = body.split("\n\n").any(|paragraph| {
        paragraph.contains("label")
            && paragraph.contains("transition")
            && paragraph.contains("report")
            && paragraph.contains("human")
            && ["bash", "shell", "command"]
                .iter()
                .any(|term| paragraph.contains(term))
            && ["do not", "must not", "cannot", "never", "rather than"]
                .iter()
                .any(|term| paragraph.contains(term))
    });
    if !reports_transitions {
        violations.push(
            "guidance must make coordination and label transitions reports for the human rather than shell mutations"
                .to_owned(),
        );
    }

    assert!(
        violations.is_empty(),
        "{ORCHESTRATOR_AGENT} Bash coordination violations:\n- {}",
        violations.join("\n- ")
    );
}

#[test]
fn canonical_implementer_guidance_protects_accepted_tests_and_fixtures() {
    let paths = [
        IMPLEMENTER_AGENT,
        ".opencode/skills/elbmesh-implementer/SKILL.md",
        "docs/DEVELOPMENT_WORKFLOW.md",
        "docs/AGENT_SKILLS.md",
    ];
    let mut violations = Vec::new();

    for path in paths {
        let document = project_file(path)
            .to_ascii_lowercase()
            .replace("test-writer", "test writer");
        let protects_accepted_tests_and_fixtures = document.split("\n\n").any(|paragraph| {
            paragraph.contains("accepted test")
                && paragraph.contains("fixture")
                && (paragraph.contains("immutable")
                    || (["do not", "must not", "cannot", "never"]
                        .iter()
                        .any(|term| paragraph.contains(term))
                        && ["change", "modify", "edit", "write"]
                            .iter()
                            .any(|term| paragraph.contains(term))))
        });
        if !protects_accepted_tests_and_fixtures {
            violations.push(format!(
                "{path} must make accepted tests and fixtures immutable to Implementers"
            ));
        }

        let defines_conflict_handoff = document.split("\n\n").any(|paragraph| {
            paragraph.contains("conflict")
                && paragraph.contains("orchestrator")
                && paragraph.contains("human")
                && ["confirm", "approval"]
                    .iter()
                    .any(|term| paragraph.contains(term))
                && paragraph.contains("fresh")
                && paragraph.contains("test writer")
                && ["revise", "change", "edit", "update"]
                    .iter()
                    .any(|term| paragraph.contains(term))
                && ["then", "after", "following"]
                    .iter()
                    .any(|term| paragraph.contains(term))
        });
        if !defines_conflict_handoff {
            violations.push(format!(
                "{path} must route conflicts to the Orchestrator for human confirmation, then to a fresh Test Writer for revision"
            ));
        }

        let prohibits_supporting_fixture_output = document.split("\n\n").any(|paragraph| {
            paragraph.contains("output")
                && paragraph.contains("support")
                && paragraph.contains("test fixture")
                && ["do not", "must not", "cannot", "never", "exclude"]
                    .iter()
                    .any(|term| paragraph.contains(term))
        });
        if !prohibits_supporting_fixture_output {
            violations.push(format!(
                "{path} must prohibit supporting test fixtures in Implementer output"
            ));
        }
        if document.contains("minimal supporting test fixture") {
            violations.push(format!(
                "{path} still lists minimal supporting test fixtures as Implementer output"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "canonical Implementer guidance violations:\n- {}",
        violations.join("\n- ")
    );
}

#[test]
fn canonical_workflow_sources_reserve_merge_authority_for_humans() {
    let paths = [
        "docs/DEVELOPMENT_WORKFLOW.md",
        "docs/AGENT_SKILLS.md",
        "docs/adr/0014-phased-mr-based-multi-agent-delivery.md",
        "docs/adr/0015-use-github-issues-as-operational-queue.md",
        ".opencode/skills/elbmesh-mr-reviewer/SKILL.md",
    ];
    let mut violations = Vec::new();

    for path in paths {
        let document = project_file(path).to_ascii_lowercase();
        if !paragraph_has_reviewer_merge_readiness(&document) {
            violations.push(format!(
                "{path} must say that the reviewer reports merge readiness"
            ));
        }
        if !paragraph_reserves_merge_for_human(&document) {
            violations.push(format!("{path} must say that a human performs the merge"));
        }
        for statement in agent_merge_authority_statements(&document) {
            violations.push(format!(
                "{path} grants merge authority to an agent: `{statement}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "canonical merge-authority contract violations:\n- {}",
        violations.join("\n- ")
    );
}

#[test]
fn harness_discloses_direct_user_subagent_invocation_boundary() {
    let path = "docs/AGENT_DELIVERY_HARNESS.md";
    let documentation = project_file(path).to_ascii_lowercase();
    let discloses_boundary = documentation.split("\n\n").any(|paragraph| {
        paragraph.contains("out-of-band")
            && paragraph.contains("user")
            && paragraph.contains("subagent")
            && paragraph.contains("task")
            && paragraph.contains("permission")
            && paragraph.contains('@')
            && (paragraph.contains("invok") || paragraph.contains("mention"))
            && (paragraph.contains("cannot prevent")
                || paragraph.contains("cannot block")
                || paragraph.contains("not preventable"))
    });

    assert!(
        discloses_boundary,
        "{path} must disclose that direct user @-invocation is an out-of-band human capability that Task permissions cannot prevent"
    );
}

fn project_file(relative_path: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative_path);

    fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("required OpenCode delivery harness file `{relative_path}` is unavailable: {error}")
    })
}

fn agent_file(path: &str) -> (String, String) {
    let contents = project_file(path);
    let contents = contents
        .strip_prefix("---\n")
        .unwrap_or_else(|| panic!("{path} must begin with YAML frontmatter"));
    let (frontmatter, body) = contents
        .split_once("\n---\n")
        .unwrap_or_else(|| panic!("{path} must close its YAML frontmatter"));

    (frontmatter.to_owned(), body.to_owned())
}

fn assert_agent_mode(path: &str, frontmatter: &str, mode: &str) {
    let expected = format!("mode: {mode}");
    assert!(
        frontmatter.lines().any(|line| line.trim() == expected),
        "{path} must declare `{expected}` in its frontmatter"
    );
}

fn assert_skill_reference(path: &str, body: &str, skill: &str) {
    let body = body.to_ascii_lowercase();
    assert!(
        body.contains("skill") && body.contains(skill),
        "{path} must instruct the agent to use the `{skill}` project skill"
    );
}

fn word_positions(text: &str, word: &str) -> Vec<usize> {
    text.match_indices(word)
        .filter_map(|(position, _)| {
            let before = text[..position].chars().next_back();
            let after = text[position + word.len()..].chars().next();
            let is_boundary = |character: Option<char>| {
                character.is_none_or(|character| !character.is_ascii_alphanumeric())
            };

            (is_boundary(before) && is_boundary(after)).then_some(position)
        })
        .collect()
}

fn describes_fresh_spawn(stage: &str, agent: &str) -> bool {
    ["spawn", "fresh", "session", agent]
        .iter()
        .all(|term| stage.contains(term))
}

fn edit_permission_rules(frontmatter: &str) -> Vec<(String, String)> {
    let lines: Vec<_> = frontmatter.lines().collect();
    let Some((edit_index, edit_line)) = lines
        .iter()
        .enumerate()
        .find(|(_, line)| line.trim() == "edit:")
    else {
        return Vec::new();
    };
    let edit_indent = indentation(edit_line);

    lines[edit_index + 1..]
        .iter()
        .take_while(|line| line.trim().is_empty() || indentation(line) > edit_indent)
        .filter_map(|line| {
            let (pattern, action) = line.trim().rsplit_once(':')?;
            Some((
                pattern
                    .trim()
                    .trim_matches(|character| character == '\'' || character == '"')
                    .to_owned(),
                action.trim().to_owned(),
            ))
        })
        .collect()
}

fn indentation(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn is_test_path(pattern: &str) -> bool {
    let pattern = pattern.trim_start_matches("./");
    pattern.starts_with("tests/")
        || pattern.starts_with("fixtures/")
        || pattern.contains("/tests/")
        || pattern.contains("/fixtures/")
        || pattern.contains("test-fixture")
}

fn edit_is_denied(frontmatter: &str) -> bool {
    frontmatter.lines().any(|line| line.trim() == "edit: deny")
        || edit_permission_rules(frontmatter)
            .iter()
            .any(|(pattern, action)| pattern == "*" && action == "deny")
}

fn assert_contains_all(path: &str, body: &str, terms: &[&str]) {
    let body = body.to_ascii_lowercase();
    let missing: Vec<_> = terms
        .iter()
        .copied()
        .filter(|term| !body.contains(term))
        .collect();

    assert!(
        missing.is_empty(),
        "{path} is missing required behavioral markers: {missing:?}"
    );
}

fn assert_prohibits_action(path: &str, body: &str, actions: &[&str]) {
    let body = body.to_ascii_lowercase();
    let prohibitions = ["do not", "must not", "cannot", "never"];
    assert!(
        body.split("\n\n").any(|paragraph| {
            prohibitions
                .iter()
                .any(|prohibition| paragraph.contains(prohibition))
                && actions.iter().any(|action| paragraph.contains(action))
        }),
        "{path} must explicitly prohibit this action: {actions:?}"
    );
}

fn contains_in_order(text: &str, markers: &[&str]) -> bool {
    let mut remaining = text;
    for marker in markers {
        let Some(position) = remaining.find(marker) else {
            return false;
        };
        remaining = &remaining[position + marker.len()..];
    }
    true
}

fn grants_orchestrator_direct_label_mutation(line: &str) -> bool {
    let words: Vec<_> = line
        .split(|character: char| !character.is_ascii_alphabetic())
        .filter(|word| !word.is_empty())
        .collect();
    let mutation_verbs = [
        "add", "adds", "adding", "apply", "applies", "applying", "change", "changes", "changing",
        "edit", "edits", "editing", "manage", "manages", "managing", "mutate", "mutates",
        "mutating", "remove", "removes", "removing", "set", "sets", "setting", "update", "updates",
        "updating",
    ];
    let negations = ["cannot", "never", "no", "not", "rather", "without"];

    if words
        .first()
        .is_some_and(|word| mutation_verbs.contains(word))
        && words
            .iter()
            .take(4)
            .any(|word| ["label", "labels"].contains(word))
    {
        return true;
    }

    let Some(orchestrator) = words.iter().position(|word| *word == "orchestrator") else {
        return false;
    };

    words
        .iter()
        .enumerate()
        .skip(orchestrator + 1)
        .filter(|(_, word)| ["label", "labels"].contains(word))
        .any(|(label, _)| {
            let authority = &words[orchestrator + 1..label];
            authority.iter().any(|word| mutation_verbs.contains(word))
                && !authority.contains(&"human")
                && !authority.iter().any(|word| negations.contains(word))
        })
}

#[derive(Debug)]
enum PermissionSetting {
    Scalar(String),
    Rules(Vec<(String, String)>),
}

fn project_permission_default(config: &Value, tool: &str) -> Option<String> {
    let permission = config.get("permission")?;
    if let Some(action) = permission.as_str() {
        return Some(action.to_owned());
    }

    let permission = permission.as_object()?;
    if let Some(action) = permission.get(tool) {
        return action
            .as_str()
            .map(str::to_owned)
            .or_else(|| action.get("*").and_then(Value::as_str).map(str::to_owned));
    }

    permission
        .get("*")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn project_permission_decision(config: &Value, tool: &str, input: &str) -> Option<String> {
    let permission = config.get("permission")?;
    if let Some(action) = permission.as_str() {
        return Some(action.to_owned());
    }

    let permission = permission.as_object()?;
    let tool_decision = permission.get(tool).and_then(|setting| {
        setting.as_str().map(str::to_owned).or_else(|| {
            setting.as_object().and_then(|rules| {
                rules
                    .iter()
                    .filter(|(pattern, _)| wildcard_matches(pattern, input))
                    .filter_map(|(_, action)| action.as_str())
                    .map(str::to_owned)
                    .next_back()
            })
        })
    });

    tool_decision.or_else(|| {
        permission
            .get("*")
            .and_then(Value::as_str)
            .map(str::to_owned)
    })
}

fn permission_setting(frontmatter: &str, tool: &str) -> Option<PermissionSetting> {
    let lines: Vec<_> = frontmatter.lines().collect();
    let (permission_index, permission_line) = lines
        .iter()
        .enumerate()
        .find(|(_, line)| line.trim() == "permission:")?;
    let permission_indent = indentation(permission_line);
    let permission_block: Vec<_> = lines[permission_index + 1..]
        .iter()
        .copied()
        .take_while(|line| line.trim().is_empty() || indentation(line) > permission_indent)
        .collect();
    let child_indent = permission_block
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| indentation(line))
        .min()?;
    let (tool_index, tool_line) = permission_block.iter().enumerate().find(|(_, line)| {
        if line.trim().is_empty() || indentation(line) != child_indent {
            return false;
        }
        line.trim()
            .split_once(':')
            .is_some_and(|(key, _)| unquote(key.trim()) == tool)
    })?;
    let (_, scalar) = tool_line.trim().split_once(':')?;
    if !scalar.trim().is_empty() {
        return Some(PermissionSetting::Scalar(unquote(scalar.trim()).to_owned()));
    }

    let tool_indent = indentation(tool_line);
    let rules = permission_block[tool_index + 1..]
        .iter()
        .take_while(|line| line.trim().is_empty() || indentation(line) > tool_indent)
        .filter_map(|line| {
            let (pattern, action) = line.trim().rsplit_once(':')?;
            Some((
                unquote(pattern.trim()).to_owned(),
                unquote(action.trim()).to_owned(),
            ))
        })
        .collect();
    Some(PermissionSetting::Rules(rules))
}

fn permission_rules(frontmatter: &str, tool: &str) -> Vec<(String, String)> {
    match permission_setting(frontmatter, tool) {
        Some(PermissionSetting::Rules(rules)) => rules,
        _ => Vec::new(),
    }
}

fn permission_default_action(frontmatter: &str, tool: &str) -> Option<String> {
    match permission_setting(frontmatter, tool)? {
        PermissionSetting::Scalar(action) => Some(action),
        PermissionSetting::Rules(rules) => rules
            .into_iter()
            .filter(|(pattern, _)| pattern == "*")
            .map(|(_, action)| action)
            .next_back(),
    }
}

fn permission_decision(frontmatter: &str, tool: &str, input: &str) -> Option<String> {
    match permission_setting(frontmatter, tool)? {
        PermissionSetting::Scalar(action) => Some(action),
        PermissionSetting::Rules(rules) => rules
            .into_iter()
            .filter(|(pattern, _)| wildcard_matches(pattern, input))
            .map(|(_, action)| action)
            .next_back(),
    }
}

fn effective_agent_permission(frontmatter: &str, tool: &str, input: &str) -> String {
    if let Some(action) = permission_decision(frontmatter, tool, input).or_else(|| {
        permission_setting(frontmatter, "*").and_then(|setting| match setting {
            PermissionSetting::Scalar(action) => Some(action),
            PermissionSetting::Rules(_) => None,
        })
    }) {
        return action;
    }

    let config = project_file("opencode.json");
    let config: Value = serde_json::from_str(&config)
        .unwrap_or_else(|error| panic!("opencode.json must be valid JSON: {error}"));
    project_permission_decision(&config, tool, input).unwrap_or_else(|| "allow".to_owned())
}

fn wildcard_matches(pattern: &str, input: &str) -> bool {
    let pattern: Vec<_> = pattern.chars().collect();
    let input: Vec<_> = input.chars().collect();
    let mut matches = vec![vec![false; input.len() + 1]; pattern.len() + 1];
    matches[0][0] = true;

    for pattern_index in 1..=pattern.len() {
        if pattern[pattern_index - 1] == '*' {
            matches[pattern_index][0] = matches[pattern_index - 1][0];
        }
        for input_index in 1..=input.len() {
            matches[pattern_index][input_index] = match pattern[pattern_index - 1] {
                '*' => {
                    matches[pattern_index - 1][input_index]
                        || matches[pattern_index][input_index - 1]
                }
                '?' => matches[pattern_index - 1][input_index - 1],
                character => {
                    character == input[input_index - 1]
                        && matches[pattern_index - 1][input_index - 1]
                }
            };
        }
    }

    matches[pattern.len()][input.len()]
}

fn unquote(value: &str) -> &str {
    value.trim_matches(|character| character == '\'' || character == '"')
}

fn paragraph_has_reviewer_merge_readiness(document: &str) -> bool {
    document.split("\n\n").any(|paragraph| {
        paragraph.contains("review")
            && paragraph.contains("report")
            && (paragraph.contains("merge readiness")
                || (paragraph.contains("readiness") && paragraph.contains("merge")))
    })
}

fn paragraph_reserves_merge_for_human(document: &str) -> bool {
    document
        .lines()
        .flat_map(|line| line.split('.'))
        .any(|statement| {
            statement.contains("human")
                && statement.contains("merge")
                && (statement.contains("merge authorit")
                    || ["only", "must", "may", "performs", "merges"]
                        .iter()
                        .any(|marker| !word_positions(statement, marker).is_empty()))
        })
}

fn agent_merge_authority_statements(document: &str) -> Vec<String> {
    let mut reviewer_context = false;
    let mut violations = Vec::new();
    let authority_markers = [
        "reviewer/merger",
        "reviewer-merger",
        "reviewer and merger",
        "review and merge",
        "reviewing and merging",
        "merges only",
        "merge only",
        "merge decision",
        "may merge",
        "must merge",
        "performs the merge",
        "merged by",
    ];
    let prohibition_markers = [
        "do not merge",
        "does not merge",
        "must not merge",
        "cannot merge",
        "never merge",
        "no merge authority",
        "not merged by",
        "never merged by",
        "must not be merged by",
        "cannot be merged by",
    ];

    for raw_line in document.lines() {
        let line = raw_line.trim();
        if line.starts_with('#') {
            reviewer_context = line.contains("reviewer") || line.contains("merger");
        }
        let mentions_reviewer = reviewer_context
            || line.contains("reviewer")
            || line.contains("review agent")
            || line.contains("merger agent")
            || line.contains("agent");
        let grants_authority = authority_markers.iter().any(|marker| line.contains(marker));
        let prohibits_authority = prohibition_markers
            .iter()
            .any(|marker| line.contains(marker));

        if mentions_reviewer && grants_authority && !prohibits_authority && !line.contains("human")
        {
            violations.push(line.to_owned());
        }
    }

    violations
}
