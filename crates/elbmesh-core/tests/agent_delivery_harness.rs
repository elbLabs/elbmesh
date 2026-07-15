use std::{collections::BTreeSet, fs, path::Path, process::Command};

use serde_json::Value;

const ORCHESTRATOR_AGENT: &str = ".opencode/agents/elbmesh-orchestrator.md";
const TEST_WRITER_AGENT: &str = ".opencode/agents/elbmesh-test-writer.md";
const IMPLEMENTER_AGENT: &str = ".opencode/agents/elbmesh-implementer.md";
const REVIEWER_AGENT: &str = ".opencode/agents/elbmesh-reviewer.md";
const PR_PUBLISHER_AGENT: &str = ".opencode/agents/elbmesh-pr-publisher.md";
const RUST_CI_WORKFLOW: &str = ".github/workflows/rust-ci.yml";
const REVIEWER_BASH_ALLOWLIST: [&str; 10] = [
    "cargo fmt --check",
    "cargo clippy --all-targets --all-features -- -D warnings",
    "cargo test --all",
    "git status --short --branch",
    "git log --oneline --decorate origin/main..HEAD",
    "git diff --name-status origin/main...HEAD",
    "git diff --check origin/main...HEAD",
    "codehud . --diff origin/main",
    "gh pr view --json number,title,body,state,isDraft,baseRefName,headRefName,url",
    "gh pr checks",
];

#[test]
fn github_pull_request_enforcement_has_rust_ci_workflow() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(RUST_CI_WORKFLOW);
    let workflow = fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "pull request Rust CI workflow `{RUST_CI_WORKFLOW}` must exist and be readable: {error}"
        )
    });
    let lines: Vec<_> = workflow.lines().map(str::trim).collect();

    assert!(
        lines.iter().any(|line| {
            *line == "pull_request:"
                || *line == "on: pull_request"
                || (line.starts_with("on: [") && line.contains("pull_request"))
        }),
        "{RUST_CI_WORKFLOW} must trigger for pull_request events"
    );

    for command in [
        "cargo fmt --check",
        "cargo clippy --all-targets --all-features -- -D warnings",
        "cargo test --all",
    ] {
        assert!(
            workflow.contains(command),
            "{RUST_CI_WORKFLOW} must run required Rust gate `{command}`"
        );
    }
}

#[test]
#[ignore = "live GitHub ruleset contract; run explicitly with authenticated gh"]
fn github_pull_request_enforcement_has_main_branch_rules() {
    let output = Command::new("gh")
        .args(["api", "repos/elbLabs/elbmesh/rules/branches/main"])
        .output()
        .expect("run `gh api repos/elbLabs/elbmesh/rules/branches/main`");

    assert!(
        output.status.success(),
        "`gh api repos/elbLabs/elbmesh/rules/branches/main` must succeed before rules can be verified: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let response: Value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("GitHub branch rules response must be valid JSON: {error}"));
    let rules = response
        .as_array()
        .expect("GitHub branch rules response must be a JSON array");
    let approval_count = rules
        .iter()
        .filter(|rule| rule.get("type").and_then(Value::as_str) == Some("pull_request"))
        .filter_map(|rule| rule.pointer("/parameters/required_approving_review_count"))
        .filter_map(Value::as_u64)
        .max()
        .unwrap_or_default();
    let required_check_contexts: Vec<_> = rules
        .iter()
        .filter(|rule| rule.get("type").and_then(Value::as_str) == Some("required_status_checks"))
        .filter_map(|rule| rule.pointer("/parameters/required_status_checks"))
        .filter_map(Value::as_array)
        .flatten()
        .filter_map(|check| check.get("context").and_then(Value::as_str))
        .collect();
    let has_required_rust_ci = required_check_contexts.iter().any(|context| {
        let context = context.to_ascii_lowercase();
        context.contains("rust") && (context.contains("ci") || context.contains("quality"))
    }) || ["fmt", "clippy", "test"].iter().all(|marker| {
        required_check_contexts
            .iter()
            .any(|context| context.to_ascii_lowercase().contains(marker))
    });
    let mut violations = Vec::new();

    if approval_count < 1 {
        violations.push("no pull_request rule requires at least one approving review".to_owned());
    }
    if !has_required_rust_ci {
        violations.push(format!(
            "no required_status_checks rule requires the Rust CI checks; contexts were {required_check_contexts:?}"
        ));
    }

    assert!(
        violations.is_empty(),
        "GitHub API returned successfully, but `main` does not enforce CI and independent review:\n- {}\nresponse: {}",
        violations.join("\n- "),
        String::from_utf8_lossy(&output.stdout)
    );
}

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
fn orchestrator_automates_pull_request_delivery_with_fresh_role_sessions() {
    let (_, body) = agent_file(ORCHESTRATOR_AGENT);
    let body = body.to_ascii_lowercase();
    let paragraphs: Vec<_> = body.split("\n\n").collect();
    let stages: [(&str, &[&str]); 7] = [
        (
            "red proof",
            &["fresh", "spawn", "elbmesh-test-writer", "red", "proof"],
        ),
        (
            "draft PR publication",
            &[
                "fresh",
                "spawn",
                "elbmesh-pr-publisher",
                "branch",
                "only",
                "accepted",
                "test",
                "fixture",
                "commit",
                "push",
                "draft",
                "pull request",
            ],
        ),
        (
            "green proof",
            &[
                "fresh",
                "spawn",
                "elbmesh-implementer",
                "accepted test",
                "immutable",
                "green",
                "proof",
            ],
        ),
        (
            "green publication",
            &[
                "fresh",
                "spawn",
                "elbmesh-pr-publisher",
                "green",
                "only",
                "reviewed",
                "implementation",
                "documentation",
                "path",
                "commit",
                "push",
            ],
        ),
        (
            "PR review",
            &[
                "fresh",
                "spawn",
                "elbmesh-reviewer",
                "review",
                "pull request",
            ],
        ),
        (
            "ready publication",
            &[
                "no block",
                "fresh",
                "spawn",
                "elbmesh-pr-publisher",
                "ready",
                "url",
            ],
        ),
        ("human review and merge", &["human", "review", "merge"]),
    ];
    let mut next_paragraph = 0;

    for (stage, required_terms) in stages {
        let Some(relative_position) = paragraphs[next_paragraph..]
            .iter()
            .position(|paragraph| required_terms.iter().all(|term| paragraph.contains(term)))
        else {
            panic!(
                "{ORCHESTRATOR_AGENT} must document the `{stage}` stage after the preceding delivery stage with markers {required_terms:?}"
            );
        };
        next_paragraph += relative_position + 1;
    }
}

#[test]
fn canonical_active_flow_assigns_merge_readiness_only_to_reviewer() {
    let paths = [
        REVIEWER_AGENT,
        ".opencode/skills/elbmesh-reviewer/SKILL.md",
        ".opencode/agents/elbmesh-orchestrator.md",
        ".opencode/skills/elbmesh-orchestrator/SKILL.md",
        "docs/AGENT_DELIVERY_HARNESS.md",
        "docs/DEVELOPMENT_WORKFLOW.md",
        "docs/AGENT_SKILLS.md",
    ];
    let mr_reviewer_names = ["elbmesh-mr-reviewer", "mr reviewer"];
    let mut violations = Vec::new();

    for path in paths {
        let document = project_file(path).to_ascii_lowercase();
        let paragraphs: Vec<_> = document.split("\n\n").collect();
        let canonical_reviewer_reports_readiness = paragraphs.iter().any(|paragraph| {
            paragraph.contains("elbmesh-reviewer")
                && paragraph.contains("report")
                && paragraph.contains("merge readiness")
        });
        if !canonical_reviewer_reports_readiness {
            violations.push(format!(
                "{path} must assign the final PR merge-readiness report to `elbmesh-reviewer`"
            ));
        }

        let mentions_mr_reviewer = paragraphs.iter().any(|paragraph| {
            mr_reviewer_names
                .iter()
                .any(|name| paragraph.contains(name))
        });
        if !mentions_mr_reviewer {
            continue;
        }

        let marks_mr_reviewer_as_noncanonical = paragraphs.iter().any(|paragraph| {
            mr_reviewer_names
                .iter()
                .any(|name| paragraph.contains(name))
                && ["compatibility", "manual"]
                    .iter()
                    .any(|marker| paragraph.contains(marker))
                && [
                    "not required",
                    "not a required",
                    "optional",
                    "not an additional",
                ]
                .iter()
                .any(|marker| paragraph.contains(marker))
        });
        if !marks_mr_reviewer_as_noncanonical {
            violations.push(format!(
                "{path} mentions `elbmesh-mr-reviewer` without marking it as a compatibility/manual skill that is not an additional required stage"
            ));
        }

        let assigns_readiness = |text: &str| {
            text.contains("merge readiness")
                || (text.contains("readiness") && text.contains("merge"))
        };
        let denies_readiness_ownership = |text: &str| {
            [
                "does not own",
                "does not report",
                "must not own",
                "must not report",
                "no readiness ownership",
                "only `elbmesh-reviewer`",
                "only elbmesh-reviewer",
            ]
            .iter()
            .any(|marker| text.contains(marker))
        };
        let mut mr_reviewer_owns_readiness = paragraphs.iter().any(|paragraph| {
            mr_reviewer_names
                .iter()
                .any(|name| paragraph.contains(name))
                && assigns_readiness(paragraph)
                && !denies_readiness_ownership(paragraph)
        });
        for heading in ["### elbmesh-mr-reviewer", "### mr reviewer"] {
            let Some(section_start) = document.find(heading) else {
                continue;
            };
            let section = &document[section_start + heading.len()..];
            let section = &section[..section.find("\n### ").unwrap_or(section.len())];
            if assigns_readiness(section) && !denies_readiness_ownership(section) {
                mr_reviewer_owns_readiness = true;
            }
        }
        if mr_reviewer_owns_readiness {
            violations.push(format!(
                "{path} gives `elbmesh-mr-reviewer` merge-readiness ownership"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "canonical Reviewer role violations:\n- {}",
        violations.join("\n- ")
    );
}

#[test]
fn pr_publisher_is_a_non_editing_delivery_subagent_with_auditable_handoffs() {
    let (frontmatter, body) = agent_file(PR_PUBLISHER_AGENT);
    assert_agent_mode(PR_PUBLISHER_AGENT, &frontmatter, "subagent");
    assert_skill_reference(PR_PUBLISHER_AGENT, &body, "elbmesh-pr-publisher");
    assert!(
        edit_is_denied(&frontmatter),
        "{PR_PUBLISHER_AGENT} must deny Edit"
    );
    assert_eq!(
        permission_default_action(&frontmatter, "task").as_deref(),
        Some("deny"),
        "{PR_PUBLISHER_AGENT} must explicitly deny Task"
    );
    assert_prohibits_action(PR_PUBLISHER_AGENT, &body, &["merge"]);

    let body = body.to_ascii_lowercase();
    assert!(
        body.split("\n\n").any(|paragraph| {
            ["stage", "only", "role", "report", "path"]
                .iter()
                .all(|term| paragraph.contains(term))
        }),
        "{PR_PUBLISHER_AGENT} must stage only paths reported by the preceding role"
    );
    assert_contains_all(PR_PUBLISHER_AGENT, &body, &["git status", "git diff"]);
    assert!(
        body.split("\n\n").any(|paragraph| {
            ["red", "green", "commit"]
                .iter()
                .all(|term| paragraph.contains(term))
                && ["separate", "distinct"]
                    .iter()
                    .any(|term| paragraph.contains(term))
        }),
        "{PR_PUBLISHER_AGENT} must preserve separate red and green commits"
    );
    assert!(
        body.split("\n\n").any(|paragraph| {
            paragraph.contains("issue")
                && ["link", "close"]
                    .iter()
                    .any(|term| paragraph.contains(term))
        }),
        "{PR_PUBLISHER_AGENT} must link the pull request to its issue"
    );
    assert!(
        body.contains("evidence")
            && (body.contains("pr body") || body.contains("pull request body"))
            && body.contains("comment"),
        "{PR_PUBLISHER_AGENT} must carry role evidence into the pull request body and comments"
    );
    assert!(
        body.split("\n\n").any(|paragraph| {
            paragraph.contains("return")
                && paragraph.contains("url")
                && (paragraph.contains("pr") || paragraph.contains("pull request"))
        }),
        "{PR_PUBLISHER_AGENT} must return the pull request URL"
    );
}

#[test]
fn publisher_green_and_readiness_evidence_is_append_only_on_issue_and_pull_request() {
    let paths = [
        PR_PUBLISHER_AGENT,
        ".opencode/skills/elbmesh-pr-publisher/SKILL.md",
        "docs/AGENT_DELIVERY_HARNESS.md",
    ];
    let required_evidence_fields: [(&str, &[&str]); 10] = [
        (
            "role task IDs",
            &["role task id", "role task/session id", "task/session id"],
        ),
        ("role session IDs", &["role session id", "task/session id"]),
        ("exact changed paths", &["exact changed path"]),
        ("red commit SHA", &["red commit sha"]),
        ("green commit SHA", &["green commit sha"]),
        ("exact commands", &["exact command"]),
        (
            "command results",
            &[
                "command result",
                "commands/results",
                "command/results",
                "commands and results",
            ],
        ),
        ("review task ID", &["review task id", "reviewer task id"]),
        ("blocker status", &["blocker status"]),
        ("PR URL", &["pr url", "pull request url"]),
    ];
    let mut violations = Vec::new();

    for path in paths {
        let document = project_file(path).to_ascii_lowercase();
        let has_append_only_green_and_readiness_evidence =
            document.split("\n\n").any(|paragraph| {
                paragraph.contains("green")
                    && paragraph.contains("readiness")
                    && (paragraph.contains("append-only")
                        || (paragraph.contains("append") && paragraph.contains("without rewrit")))
                    && paragraph.contains("issue")
                    && (paragraph.contains("pull request") || paragraph.contains("pr"))
                    && paragraph.contains("comment")
            });
        if !has_append_only_green_and_readiness_evidence {
            violations.push(format!(
                "{path} must append green and readiness evidence as new comments on both the GitHub issue and pull request without rewriting prior evidence"
            ));
        }

        for (field, alternatives) in required_evidence_fields {
            if !alternatives
                .iter()
                .any(|alternative| document.contains(alternative))
            {
                violations.push(format!(
                    "{path} append-only publication evidence is missing {field}"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "PR Publisher evidence contract violations:\n- {}",
        violations.join("\n- ")
    );
}

#[test]
fn pr_publisher_permissions_allow_publication_but_deny_direct_merge() {
    let (frontmatter, _) = agent_file(PR_PUBLISHER_AGENT);
    let mut violations = Vec::new();
    let bash_rules = permission_rules(&frontmatter, "bash");

    if bash_rules
        .first()
        .map(|(pattern, action)| (pattern.as_str(), action.as_str()))
        != Some(("*", "deny"))
    {
        violations.push("Bash rules must begin with a broad deny".to_owned());
    }

    if !bash_rules
        .iter()
        .any(|(pattern, action)| pattern == "gh issue edit *" && action == "allow")
    {
        violations.push(
            "Bash rules must narrowly allow `gh issue edit *` for automatic status transitions"
                .to_owned(),
        );
    }

    for (operation, command) in [
        ("create the issue branch", "git switch -c issue-147"),
        ("inspect status", "git status --short"),
        ("inspect the staged diff", "git diff --cached"),
        (
            "stage a role-reported path",
            "git add -- crates/elbmesh-core/tests/agent_delivery_harness.rs",
        ),
        (
            "create the red commit",
            "git commit -m \"test: add red proof\"",
        ),
        (
            "push the issue branch",
            "git push --set-upstream origin HEAD",
        ),
        (
            "open the draft pull request",
            "gh pr create --draft --title \"Issue 147\" --body \"Closes #147\"",
        ),
        ("push the green commit", "git push origin HEAD"),
        (
            "publish evidence",
            "gh pr comment 148 --body \"Green proof\"",
        ),
        (
            "set implementation status after red publication",
            "gh issue edit 121 --remove-label status:review --add-label status:implementation",
        ),
        (
            "set review status after review readiness gates",
            "gh issue edit 121 --remove-label status:implementation --add-label status:review",
        ),
        ("mark the pull request ready", "gh pr ready 148"),
    ] {
        let decision = effective_agent_permission(&frontmatter, "bash", command);
        if decision != "allow" {
            violations.push(format!(
                "permission for `{operation}` resolves to {decision} instead of allow: {command}"
            ));
        }
    }

    for command in [
        "git merge main",
        "git merge --continue",
        "gh pr merge 148",
        "gh pr merge 148 --auto",
        "git push origin main",
    ] {
        let decision = effective_agent_permission(&frontmatter, "bash", command);
        if decision != "deny" {
            violations.push(format!(
                "direct merge or base-branch publication resolves to {decision} instead of deny: {command}"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "{PR_PUBLISHER_AGENT} publication permissions are defense in depth and must expose only the required non-merging delivery operations:\n- {}",
        violations.join("\n- ")
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
        "docs/DELIVERY_ROADMAP.md",
        "docs/AGENT_SKILLS.md",
        "docs/IMPLEMENTATION_PLAN.md",
        "docs/adr/",
    ];
    let skill_paths = [
        ".opencode/skills/elbmesh-orchestrator/SKILL.md",
        ".opencode/skills/elbmesh-test-writer/SKILL.md",
        ".opencode/skills/elbmesh-pr-publisher/SKILL.md",
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
        "docs/DELIVERY_ROADMAP.md",
        "docs/AGENT_SKILLS.md",
        "docs/IMPLEMENTATION_PLAN.md",
        "docs/adr/",
    ];
    let skill_paths = [
        ".opencode/skills/elbmesh-driver/SKILL.md",
        ".opencode/skills/elbmesh-orchestrator/SKILL.md",
        ".opencode/skills/elbmesh-test-writer/SKILL.md",
        ".opencode/skills/elbmesh-pr-publisher/SKILL.md",
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
fn publisher_automates_issue_status_transitions_with_delivery_prerequisites() {
    let paths = [
        PR_PUBLISHER_AGENT,
        ".opencode/skills/elbmesh-pr-publisher/SKILL.md",
    ];
    let mut violations = Vec::new();

    for path in paths {
        let document = project_file(path).to_ascii_lowercase();
        let sets_implementation_after_red = document.split("\n\n").any(|paragraph| {
            paragraph.contains("red")
                && paragraph.contains("status:implementation")
                && ["set", "keep", "transition", "move"]
                    .iter()
                    .any(|term| paragraph.contains(term))
        });
        if !sets_implementation_after_red {
            violations.push(format!(
                "{path} must automatically set or keep `status:implementation` after accepted red publication"
            ));
        }

        let sets_review_after_readiness_gates = document.split("\n\n").any(|paragraph| {
            paragraph.contains("reviewer")
                && paragraph.contains("ci")
                && paragraph.contains("ready")
                && paragraph.contains("status:review")
                && ["no blocker", "no-blocker", "no blocking"]
                    .iter()
                    .any(|term| paragraph.contains(term))
        });
        if !sets_review_after_readiness_gates {
            violations.push(format!(
                "{path} must set `status:review` only with no-blocker Reviewer evidence and required CI while marking the pull request ready"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Publisher issue-status automation violations:\n- {}",
        violations.join("\n- ")
    );
}

#[test]
fn harness_documentation_keeps_human_merge_and_tracks_automated_issue_statuses() {
    let path = "docs/AGENT_DELIVERY_HARNESS.md";
    let documentation = project_file(path).to_ascii_lowercase();
    let paragraphs: Vec<_> = documentation.split("\n\n").collect();

    assert!(
        paragraphs.iter().any(|paragraph| {
            ["human", "merge", "authorit"]
                .iter()
                .all(|term| paragraph.contains(term))
        }),
        "{path} must state that merge authority remains human"
    );

    let observed_statuses: BTreeSet<_> = documentation
        .split(|character: char| {
            !(character.is_ascii_alphanumeric() || character == ':' || character == '-')
        })
        .filter(|token| token.starts_with("status:") && token.len() > "status:".len())
        .collect();
    assert_eq!(
        observed_statuses,
        BTreeSet::from(["status:implementation", "status:review"]),
        "{path} must use only the two active issue statuses"
    );

    assert!(
        paragraphs.iter().any(|paragraph| {
            paragraph.contains("publisher")
                && paragraph.contains("red")
                && paragraph.contains("status:implementation")
                && ["set", "keep", "transition", "move"]
                    .iter()
                    .any(|term| paragraph.contains(term))
        }),
        "{path} must assign the post-red `status:implementation` transition to the Publisher"
    );
    assert!(
        paragraphs.iter().any(|paragraph| {
            paragraph.contains("publisher")
                && paragraph.contains("reviewer")
                && paragraph.contains("ci")
                && paragraph.contains("ready")
                && paragraph.contains("status:review")
                && ["no blocker", "no-blocker", "no blocking"]
                    .iter()
                    .any(|term| paragraph.contains(term))
        }),
        "{path} must assign `status:review` to the Publisher only after no-blocker Reviewer evidence and required CI"
    );
    assert!(
        paragraphs.iter().any(|paragraph| {
            paragraph.contains("github")
                && paragraph.contains("merged")
                && paragraph.contains("closed")
                && paragraph.contains("state")
        }),
        "{path} must use merged/closed GitHub state instead of a merged status label"
    );
}

#[test]
fn harness_documents_automatic_pull_request_creation_and_human_only_merge() {
    let path = "docs/AGENT_DELIVERY_HARNESS.md";
    let documentation = project_file(path).to_ascii_lowercase();

    assert!(
        documentation.split("\n\n").any(|paragraph| {
            paragraph.contains("automatic")
                && (paragraph.contains("pr creation")
                    || paragraph.contains("pull request creation")
                    || ((paragraph.contains("create") || paragraph.contains("open"))
                        && paragraph.contains("pull request")))
        }),
        "{path} must explicitly state that pull request creation is automatic"
    );
    assert!(
        documentation.split("\n\n").any(|paragraph| {
            paragraph.contains("only")
                && paragraph.contains("merge")
                && paragraph.contains("human")
                && (paragraph.contains("human action")
                    || paragraph.contains("human intervention")
                    || paragraph.contains("requires the human"))
        }),
        "{path} must explicitly state that only merge requires human action in the pull request delivery flow"
    );
    assert!(
        documentation.split("\n\n").any(|paragraph| {
            paragraph.contains("permission")
                && paragraph.contains("defense in depth")
                && paragraph.contains("not a sandbox")
        }),
        "{path} must document OpenCode permissions as defense in depth rather than a sandbox"
    );
}

#[test]
fn harness_documentation_uses_dependency_order_without_an_active_phase_gate() {
    let path = "docs/AGENT_DELIVERY_HARNESS.md";
    let documentation = project_file(path).to_ascii_lowercase();
    let forbidden_phase_contracts = [
        "## phase contract",
        "active phase",
        "planned phase",
        "phase-scoped",
        "red phase",
        "green phase",
        "review phase",
    ];
    let retained: Vec<_> = forbidden_phase_contracts
        .iter()
        .copied()
        .filter(|term| documentation.contains(term))
        .collect();

    assert!(
        retained.is_empty(),
        "{path} must not retain active phase gates: {retained:?}"
    );
    assert!(
        documentation.split("\n\n").any(|paragraph| {
            paragraph.contains("dependency")
                && paragraph.contains("github issue")
                && ["order", "sequence"]
                    .iter()
                    .any(|term| paragraph.contains(term))
        }),
        "{path} must describe dependency-ordered GitHub Issue delivery"
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
    if task_rules.len() != 5 {
        violations.push(format!(
            "{ORCHESTRATOR_AGENT} must have only one broad Task deny followed by the four role allows, found {task_rules:?}"
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
        "elbmesh-pr-publisher",
    ];
    expected_agents.sort_unstable();
    if allowed_agents != expected_agents {
        violations.push(format!(
            "{ORCHESTRATOR_AGENT} may allow Task only for the four delivery roles, found {allowed_agents:?}"
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
fn reviewer_bash_allows_only_exact_review_and_quality_commands() {
    let (frontmatter, _) = agent_file(REVIEWER_AGENT);
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
    let mut expected_commands = REVIEWER_BASH_ALLOWLIST;
    expected_commands.sort_unstable();
    if allowed_commands != expected_commands {
        violations.push(format!(
            "Bash allow rules must be only the exact current-branch review and quality commands, found {allowed_commands:?}"
        ));
    }

    for command in REVIEWER_BASH_ALLOWLIST {
        if permission_decision(&frontmatter, "bash", command).as_deref() != Some("allow") {
            violations.push(format!(
                "required review or quality command is not allowed exactly: {command}"
            ));
        }
    }

    for command in [
        "codehud edit crates/elbmesh-core/src/lib.rs Resource",
        "git diff --output=review.patch",
        "git show --output=review.txt HEAD",
        "cargo test --all > review.txt",
        "git add -- crates/elbmesh-core/src/lib.rs",
        "gh pr comment 148 --body \"ready\"",
        "git status --short --branch --porcelain=v2",
        "git log --oneline --decorate origin/main..HEAD --format=full",
        "git diff --name-status origin/main...HEAD --output=review.patch",
        "codehud . --diff origin/main --json",
        "gh pr view --json number,title,body,state,isDraft,baseRefName,headRefName,url --jq .url",
        "gh pr checks --watch",
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
    let reviewer_bash_rules = permission_rules(&reviewer_frontmatter, "bash");
    let reviewer_bash_rules: Vec<_> = reviewer_bash_rules
        .iter()
        .map(|(pattern, action)| (pattern.as_str(), action.as_str()))
        .collect();
    let mut violations = Vec::new();

    if reviewer_bash_rules.first().copied() != Some(("*", "deny")) {
        violations.push(format!(
            "{REVIEWER_AGENT} Bash rules must begin with broad deny, found {reviewer_bash_rules:?}"
        ));
    }
    let mut reviewer_allow_rules: Vec<_> = reviewer_bash_rules
        .iter()
        .filter(|(_, action)| *action == "allow")
        .map(|(pattern, _)| *pattern)
        .collect();
    reviewer_allow_rules.sort_unstable();
    let mut expected_reviewer_allow_rules = REVIEWER_BASH_ALLOWLIST;
    expected_reviewer_allow_rules.sort_unstable();
    if reviewer_allow_rules != expected_reviewer_allow_rules {
        violations.push(format!(
            "{REVIEWER_AGENT} Bash rules must contain only the exact review and quality-gate allows, found {reviewer_allow_rules:?}"
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
fn orchestrator_denies_all_bash_and_delegates_status_transitions_to_the_publisher() {
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
        "gh issue edit 147 --remove-label status:implementation --add-label status:review",
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
    let delegates_transitions = body.contains("publisher")
        && body.contains("status:implementation")
        && body.contains("status:review")
        && ["delegate", "owns", "sets", "changes"]
            .iter()
            .any(|term| body.contains(term));
    if !delegates_transitions {
        violations.push(
            "guidance must delegate automatic `status:implementation` and `status:review` transitions to the Publisher"
                .to_owned(),
        );
    }

    if body
        .split("\n\n")
        .any(paragraph_requires_human_label_transition)
    {
        violations.push(
            "guidance must not request routine human-applied issue-label transitions".to_owned(),
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

#[test]
fn harness_documents_147_148_as_a_publisher_bootstrap_exception() {
    let path = "docs/AGENT_DELIVERY_HARNESS.md";
    let documentation = project_file(path).to_ascii_lowercase();
    let documents_current_exception = documentation.split("\n\n").any(|paragraph| {
        paragraph.contains("#147")
            && paragraph.contains("#148")
            && paragraph.contains("bootstrap exception")
            && paragraph.contains("initial")
            && (paragraph.contains("pull request") || paragraph.contains("pr"))
            && paragraph.contains("exist")
            && paragraph.contains("before")
            && paragraph.contains("publisher")
            && paragraph.contains("introduc")
    });
    let requires_future_publisher_sequence = documentation.split("\n\n").any(|paragraph| {
        paragraph.contains("future")
            && (paragraph.contains("all") || paragraph.contains("every"))
            && paragraph.contains("sequence")
            && contains_in_order(paragraph, &["red", "publisher", "green", "publisher"])
    });

    assert!(
        documents_current_exception,
        "{path} must identify issue #147 / draft PR #148 as a bootstrap exception whose initial PR existed before the Publisher role was introduced"
    );
    assert!(
        requires_future_publisher_sequence,
        "{path} must require all future runs to follow the red-Publisher then green-Publisher sequence"
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
