use std::{collections::BTreeSet, fs, path::Path, process::Command};

use serde_json::Value;

const ORCHESTRATOR_AGENT: &str = ".opencode/agents/elbmesh-orchestrator.md";
const OPERATIONS_AGENT: &str = ".opencode/agents/elbmesh-operations.md";
const TEST_WRITER_AGENT: &str = ".opencode/agents/elbmesh-test-writer.md";
const IMPLEMENTER_AGENT: &str = ".opencode/agents/elbmesh-implementer.md";
const REVIEWER_AGENT: &str = ".opencode/agents/elbmesh-reviewer.md";
const PR_PUBLISHER_AGENT: &str = ".opencode/agents/elbmesh-pr-publisher.md";
const OPERATIONS_SKILL: &str = ".opencode/skills/elbmesh-operations/SKILL.md";
const RUST_CI_WORKFLOW: &str = ".github/workflows/rust-ci.yml";
const OPERATIONS_BASH_ALLOWLIST: [&str; 7] = [
    "gh issue create *",
    "gh issue view *",
    "git fetch",
    "git fetch origin",
    "git worktree list",
    "git worktree list *",
    "git worktree add *",
];
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
fn github_pull_request_enforcement_has_required_live_adapter_jobs() {
    let workflow = project_file(RUST_CI_WORKFLOW);

    let nats_job = workflow_job_containing(&workflow, "--features nats-tests")
        .expect("Rust CI must define a dedicated job that runs live NATS tests");
    assert!(
        nats_job.contains("docker compose up -d nats") || nats_job.contains("image: nats:"),
        "the live NATS job must provision NATS"
    );
    assert!(
        nats_job.contains("ELBMESH_NATS_URL"),
        "the live NATS job must require ELBMESH_NATS_URL instead of allowing a skip"
    );
    assert!(
        workflow_job_runs_exact_command(
            &nats_job,
            "cargo test -p elbmesh-core --features nats-tests --test event_store_contract",
        ),
        "the live NATS job must run the complete event_store_contract test binary without a filter that can match zero tests"
    );
    assert!(
        !nats_job.contains("continue-on-error: true"),
        "the live NATS job must block on failures"
    );

    let restate_job = workflow_job_containing(&workflow, "--features restate-tests")
        .expect("Rust CI must define a dedicated job that runs live Restate tests");
    assert!(
        restate_job.contains("docker compose up -d restate")
            || restate_job.contains("image: docker.io/restatedev/restate:")
            || restate_job.contains("image: restatedev/restate:"),
        "the live Restate job must provision Restate"
    );
    assert!(
        restate_job.contains("ELBMESH_RESTATE_URL"),
        "the live Restate job must require ELBMESH_RESTATE_URL instead of allowing a skip"
    );
    assert!(
        workflow_job_runs_exact_command(
            &restate_job,
            "cargo test -p elbmesh-core --features restate-tests --test operation_journal",
        ),
        "the live Restate job must run the complete operation_journal test binary without a filter that can match zero tests"
    );
    assert!(
        !restate_job.contains("continue-on-error: true"),
        "the live Restate job must block on failures"
    );
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
fn operations_subagent_has_only_issue_and_worktree_setup_permissions() {
    let (frontmatter, body) = agent_file(OPERATIONS_AGENT);
    assert_agent_mode(OPERATIONS_AGENT, &frontmatter, "subagent");
    assert_skill_reference(OPERATIONS_AGENT, &body, "elbmesh-operations");

    let mut violations = Vec::new();
    if !edit_is_denied(&frontmatter) {
        violations.push("Edit is not denied".to_owned());
    }
    if permission_default_action(&frontmatter, "task").as_deref() != Some("deny") {
        violations.push("Task is not explicitly denied".to_owned());
    }

    let bash_rules = permission_rules(&frontmatter, "bash");
    if bash_rules
        .first()
        .map(|(pattern, action)| (pattern.as_str(), action.as_str()))
        != Some(("*", "deny"))
    {
        violations.push(format!(
            "Bash rules must begin with a broad deny, found {bash_rules:?}"
        ));
    }
    let mut allowed_commands: Vec<_> = bash_rules
        .iter()
        .filter(|(_, action)| action == "allow")
        .map(|(pattern, _)| pattern.as_str())
        .collect();
    allowed_commands.sort_unstable();
    let mut expected_commands = OPERATIONS_BASH_ALLOWLIST;
    expected_commands.sort_unstable();
    if allowed_commands != expected_commands {
        violations.push(format!(
            "Bash allows must contain only issue create/view, fetch, and worktree list/add patterns, found {allowed_commands:?}"
        ));
    }

    for command in [
        "gh issue create --title 'Task' --body 'Complete task card'",
        "gh issue view 123",
        "git fetch",
        "git fetch origin",
        "git worktree list",
        "git worktree list --porcelain",
        "git worktree add -b issue-123 ../elbmesh-issue-123 origin/main",
    ] {
        if permission_decision(&frontmatter, "bash", command).as_deref() != Some("allow") {
            violations.push(format!("required setup command is not allowed: {command}"));
        }
    }

    for command in [
        "gh issue create --title 'Task' --body 'Body' --label status:implementation",
        "gh issue create -l bug --title 'Task' --body 'Body'",
        "gh issue edit 123 --add-label status:review",
        "gh issue close 123",
        "gh pr create --draft --title 'Task' --body 'Body'",
        "gh pr comment 148 --body 'ready'",
        "gh pr merge 148",
        "git status --short --branch",
        "git commit -m 'bootstrap'",
        "git push origin HEAD",
        "git merge implementation",
        "git branch -D issue-123",
        "git worktree remove ../elbmesh-issue-123",
        "git worktree add --force ../elbmesh-issue-123 issue-123",
        "git worktree add -B issue-123 ../elbmesh-issue-123 origin/main",
        "printf bypass > docs/AGENT_SKILLS.md",
    ] {
        if effective_agent_permission(&frontmatter, "bash", command) != "deny" {
            violations.push(format!("prohibited command is not denied: {command}"));
        }
    }

    for path in [
        "crates/elbmesh-core/src/lib.rs",
        "docs/AGENT_SKILLS.md",
        ".opencode/agents/elbmesh-orchestrator.md",
    ] {
        if effective_agent_permission(&frontmatter, "edit", path) != "deny" {
            violations.push(format!("repository path remains editable: {path}"));
        }
    }

    assert_contains_all(
        OPERATIONS_AGENT,
        &body,
        &[
            "task card",
            "issue",
            "worktree",
            "one permitted command at a time",
            "no file modifications",
            "nested task",
        ],
    );
    assert!(
        violations.is_empty(),
        "{OPERATIONS_AGENT} narrow operations permission violations:\n- {}",
        violations.join("\n- ")
    );
}

#[test]
fn orchestrator_delegates_issue_and_worktree_setup_to_operations() {
    let (_, body) = agent_file(ORCHESTRATOR_AGENT);
    let body = body.to_ascii_lowercase();

    assert!(
        body.split("\n\n").any(|paragraph| {
            paragraph.contains("fresh")
                && paragraph.contains("spawn")
                && paragraph.contains("elbmesh-operations")
                && paragraph.contains("issue")
                && paragraph.contains("worktree")
        }),
        "{ORCHESTRATOR_AGENT} must delegate issue creation and worktree setup to a fresh elbmesh-operations session"
    );
}

#[test]
fn operations_contract_is_synchronized_across_agent_skill_and_active_docs() {
    for path in [
        OPERATIONS_AGENT,
        OPERATIONS_SKILL,
        "docs/AGENT_SKILLS.md",
        "docs/DEVELOPMENT_WORKFLOW.md",
        "docs/AGENT_DELIVERY_HARNESS.md",
    ] {
        let document = project_file(path);
        assert_contains_all(path, &document, &["operations", "issue", "worktree"]);
    }

    let catalog = project_file("docs/AGENT_SKILLS.md");
    assert_contains_all(
        "docs/AGENT_SKILLS.md",
        &catalog,
        &["### elbmesh-operations"],
    );
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
        "docs/DEVELOPMENT_WORKFLOW.md",
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
        body.contains("issue") && body.contains("evidence") && body.contains("comment"),
        "{PR_PUBLISHER_AGENT} must append immutable role evidence to the issue"
    );
    assert!(
        (body.contains("pr body") || body.contains("pull request body"))
            && body.contains("current")
            && ["update", "refresh", "replace"]
                .iter()
                .any(|term| body.contains(term)),
        "{PR_PUBLISHER_AGENT} must keep the pull request body updated as the current review summary"
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
fn publisher_evidence_is_issue_only_stage_deltas_and_pr_body_stays_current() {
    let paths = [
        PR_PUBLISHER_AGENT,
        ".opencode/skills/elbmesh-pr-publisher/SKILL.md",
    ];
    let required_evidence_fields: [(&str, &[&str]); 8] = [
        (
            "role task IDs",
            &["role task id", "role task/session id", "task/session id"],
        ),
        ("role session IDs", &["role session id", "task/session id"]),
        ("exact changed paths", &["exact changed path"]),
        ("stage commit SHA", &["stage commit sha", "commit sha"]),
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
        ("blocker status", &["blocker status"]),
        ("PR URL", &["pr url", "pull request url"]),
    ];
    let mut violations = Vec::new();

    for path in paths {
        let document = project_file(path).to_ascii_lowercase();
        let has_issue_only_stage_delta = document.split("\n\n").any(|paragraph| {
            paragraph.contains("append-only")
                && paragraph.contains("issue")
                && paragraph.contains("comment")
                && (paragraph.contains("stage delta") || paragraph.contains("stage-specific"))
                && ["not cumulative", "never cumulative", "do not repeat"]
                    .iter()
                    .any(|term| paragraph.contains(term))
        });
        if !has_issue_only_stage_delta {
            violations.push(format!(
                "{path} must append non-cumulative, stage-specific evidence only to the GitHub issue"
            ));
        }

        let has_current_pr_body = document.split("\n\n").any(|paragraph| {
            (paragraph.contains("pull request body") || paragraph.contains("pr body"))
                && paragraph.contains("current")
                && paragraph.contains("concise")
                && ["update", "refresh", "replace"]
                    .iter()
                    .any(|term| paragraph.contains(term))
        });
        if !has_current_pr_body {
            violations.push(format!(
                "{path} must update one concise pull request body as the current review summary"
            ));
        }

        if !document.contains("do not post routine evidence comments on the pull request") {
            violations.push(format!(
                "{path} must prohibit routine evidence comments on the pull request"
            ));
        }

        for (field, alternatives) in required_evidence_fields {
            if !alternatives
                .iter()
                .any(|alternative| document.contains(alternative))
            {
                violations.push(format!("{path} issue audit evidence is missing {field}"));
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
fn pull_request_template_is_a_concise_current_review_summary() {
    let path = ".github/pull_request_template.md";
    let template = project_file(path).to_ascii_lowercase();

    for section in [
        "## human review briefing",
        "### 60-second summary",
        "### flow",
        "### review guide",
        "### risks and approval",
        "## current state",
        "## scope",
        "## verification",
        "## architecture and docs",
        "## commits",
        "## audit trail",
    ] {
        assert!(
            template.contains(section),
            "{path} must include the current-review section `{section}`"
        );
    }

    for field in [
        "stage:",
        "ci:",
        "blockers:",
        "approve when:",
        "open questions:",
        "issue:",
    ] {
        assert!(
            template.contains(field),
            "{path} must expose the current-review field `{field}`"
        );
    }
}

#[test]
fn reviewer_hands_a_human_review_briefing_to_the_publisher() {
    let reviewer_paths = [REVIEWER_AGENT, ".opencode/skills/elbmesh-reviewer/SKILL.md"];
    let required_briefing_terms = [
        "human review briefing",
        "60-second summary",
        "change map",
        "mermaid",
        "architecture impact",
        "risk map",
        "suggested review order",
        "proof",
        "approval criteria",
        "open questions",
        "non-goals",
        "residual risks",
        "700 words",
        "evidence-backed",
    ];

    for path in reviewer_paths {
        let document = project_file(path).to_ascii_lowercase();
        for term in required_briefing_terms {
            assert!(
                document.contains(term),
                "{path} Human Review Briefing contract is missing `{term}`"
            );
        }
    }

    for path in [
        PR_PUBLISHER_AGENT,
        ".opencode/skills/elbmesh-pr-publisher/SKILL.md",
    ] {
        let document = project_file(path).to_ascii_lowercase();
        assert!(
            document.split("\n\n").any(|paragraph| {
                paragraph.contains("reviewer")
                    && paragraph.contains("human review briefing")
                    && paragraph.contains("verbatim")
                    && paragraph.contains("top")
                    && (paragraph.contains("pull request body")
                        || paragraph.contains("pr body"))
            }),
            "{path} must publish the Reviewer-validated briefing verbatim at the top of the pull request body"
        );
    }

    for path in [
        ".opencode/agents/elbmesh-orchestrator.md",
        ".opencode/skills/elbmesh-orchestrator/SKILL.md",
    ] {
        let document = project_file(path).to_ascii_lowercase();
        assert!(
            document.split("\n\n").any(|paragraph| {
                paragraph.contains("reviewer")
                    && paragraph.contains("human review briefing")
                    && paragraph.contains("publisher")
                    && (paragraph.contains("pull request body")
                        || paragraph.contains("pr body"))
            }),
            "{path} must hand the Reviewer briefing to the Publisher for the current pull request body"
        );
    }
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
            "open the draft pull request",
            "gh pr create --draft --title \"Issue 147\" --body \"Closes #147\"",
        ),
        (
            "publish issue evidence",
            "gh issue comment 147 --body \"Green proof\"",
        ),
        (
            "refresh the pull request body",
            "gh pr edit 148 --body \"Current review summary\"",
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
        "git push origin main",
        "git push --set-upstream origin main",
        "git push origin HEAD:main",
        "git push origin HEAD:refs/heads/main",
        "git push --force origin HEAD",
        "git push --force-with-lease origin HEAD",
        "git merge main",
        "git merge --continue",
        "gh pr edit 148 --base main",
        "gh pr comment 148 --body \"Green proof\"",
        "gh pr merge 148",
        "gh pr merge 148 --auto",
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
fn publisher_pr_edit_permissions_deny_every_base_change_form() {
    let (frontmatter, _) = agent_file(PR_PUBLISHER_AGENT);
    let mut violations = Vec::new();

    for command in [
        "gh pr edit 152 --title \"Update delivery evidence\"",
        "gh pr edit --body \"Current cumulative evidence\"",
        "gh pr edit https://github.com/elbLabs/elbmesh/pull/152 --add-label reviewable",
    ] {
        let decision = effective_agent_permission(&frontmatter, "bash", command);
        if decision != "allow" {
            violations.push(format!(
                "non-base pull-request edit resolves to {decision} instead of allow: {command}"
            ));
        }
    }

    for (form, command) in [
        (
            "long separated without selector",
            "gh pr edit --base release/2026-q3",
        ),
        (
            "long separated after numeric selector",
            "gh pr edit 152 --base integration",
        ),
        (
            "long separated before numeric selector",
            "gh pr edit --base next 152",
        ),
        (
            "long separated after URL selector",
            "gh pr edit https://github.com/elbLabs/elbmesh/pull/152 --base stable",
        ),
        (
            "long separated before URL selector",
            "gh pr edit --base release/candidate https://github.com/elbLabs/elbmesh/pull/152",
        ),
        (
            "long separated among extra options",
            "gh pr edit 152 --title \"Retarget\" --base staging --add-label urgent",
        ),
        (
            "long assignment without selector",
            "gh pr edit --base=release/2026-q4",
        ),
        (
            "long assignment after numeric selector",
            "gh pr edit 152 --base=integration-next",
        ),
        (
            "long assignment before numeric selector",
            "gh pr edit --base=next-stable 152",
        ),
        (
            "long assignment after URL selector",
            "gh pr edit https://github.com/elbLabs/elbmesh/pull/152 --base=stable/2026",
        ),
        (
            "long assignment before URL selector",
            "gh pr edit --base=release/candidate https://github.com/elbLabs/elbmesh/pull/152",
        ),
        (
            "long assignment among extra options",
            "gh pr edit --add-label urgent --base=staging 152 --remove-label backlog",
        ),
        (
            "short separated without selector",
            "gh pr edit -B release/2026-q3",
        ),
        (
            "short separated after numeric selector",
            "gh pr edit 152 -B integration",
        ),
        (
            "short separated before numeric selector",
            "gh pr edit -B next 152",
        ),
        (
            "short separated after URL selector",
            "gh pr edit https://github.com/elbLabs/elbmesh/pull/152 -B stable",
        ),
        (
            "short separated before URL selector",
            "gh pr edit -B release/candidate https://github.com/elbLabs/elbmesh/pull/152",
        ),
        (
            "short separated among extra options",
            "gh pr edit 152 --title \"Retarget\" -B staging --add-label urgent",
        ),
        (
            "short assignment without selector",
            "gh pr edit -B=release/2026-q4",
        ),
        (
            "short assignment after numeric selector",
            "gh pr edit 152 -B=integration-next",
        ),
        (
            "short assignment before numeric selector",
            "gh pr edit -B=next-stable 152",
        ),
        (
            "short assignment after URL selector",
            "gh pr edit https://github.com/elbLabs/elbmesh/pull/152 -B=stable/2026",
        ),
        (
            "short assignment before URL selector",
            "gh pr edit -B=release/candidate https://github.com/elbLabs/elbmesh/pull/152",
        ),
        (
            "short assignment among extra options",
            "gh pr edit --add-label urgent -B=staging 152 --remove-label backlog",
        ),
        (
            "short attached without selector",
            "gh pr edit -Brelease/2026-q3",
        ),
        (
            "short attached after numeric selector",
            "gh pr edit 152 -Bintegration",
        ),
        (
            "short attached before numeric selector",
            "gh pr edit -Bnext 152",
        ),
        (
            "short attached after URL selector",
            "gh pr edit https://github.com/elbLabs/elbmesh/pull/152 -Bstable",
        ),
        (
            "short attached before URL selector",
            "gh pr edit -Brelease/candidate https://github.com/elbLabs/elbmesh/pull/152",
        ),
        (
            "short attached among extra options",
            "gh pr edit 152 --title \"Retarget\" -Bstaging --add-label urgent",
        ),
    ] {
        let decision = effective_agent_permission(&frontmatter, "bash", command);
        if decision != "deny" {
            violations.push(format!(
                "{form} base change resolves to {decision} instead of deny: {command}"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "{PR_PUBLISHER_AGENT} must deny every valid pull-request base-change form while retaining required non-base edits under effective last-match permissions:\n- {}",
        violations.join("\n- ")
    );
}

#[test]
fn publisher_push_permissions_allow_preflighted_head_but_deny_base_and_force_pushes() {
    let (frontmatter, _) = agent_file(PR_PUBLISHER_AGENT);
    let mut violations = Vec::new();

    for command in [
        "git push origin HEAD",
        "git push --set-upstream origin HEAD",
    ] {
        let decision = effective_agent_permission(&frontmatter, "bash", command);
        if decision != "allow" {
            violations.push(format!(
                "generic current-branch publication resolves to {decision} instead of allow: {command}"
            ));
        }
    }

    for command in [
        "git push origin main",
        "git push origin refs/heads/main",
        "git push --set-upstream origin main",
        "git push --set-upstream origin refs/heads/main",
        "git push -u origin main",
        "git push --force origin HEAD",
        "git push --force-with-lease origin HEAD",
        "git push origin HEAD --force",
        "git push origin +HEAD",
        "git push origin HEAD:main",
        "git push origin HEAD:refs/heads/main",
    ] {
        let decision = effective_agent_permission(&frontmatter, "bash", command);
        if decision != "deny" {
            violations.push(format!(
                "base-branch, force, or refspec push resolves to {decision} instead of deny: {command}"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "{PR_PUBLISHER_AGENT} must allow generic HEAD publication after provenance preflight while denying direct base, force, and base-refspec pushes:\n- {}",
        violations.join("\n- ")
    );
}

#[test]
fn publisher_issue_edit_permissions_allow_broad_autonomous_mutation() {
    let (frontmatter, _) = agent_file(PR_PUBLISHER_AGENT);
    let bash_rules = permission_rules(&frontmatter, "bash");
    let mut violations = Vec::new();

    if !bash_rules
        .iter()
        .any(|(pattern, action)| pattern == "gh issue edit *" && action == "allow")
    {
        violations
            .push("Bash permissions must include broad `gh issue edit *` autonomy".to_owned());
    }

    for command in [
        "gh issue edit 121 --remove-label status:review --add-label status:implementation",
        "gh issue edit 121 --remove-label status:implementation --add-label status:review",
        "gh issue edit 987 --remove-label status:review --add-label status:implementation",
        "gh issue edit 987 --remove-label status:implementation --add-label status:review",
        "gh issue edit 121 --title \"Autonomous correction\"",
    ] {
        let decision = effective_agent_permission(&frontmatter, "bash", command);
        if decision != "allow" {
            violations.push(format!(
                "autonomous issue mutation resolves to {decision} instead of allow: {command}"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "{PR_PUBLISHER_AGENT} issue-edit permissions must allow the accepted broad autonomous fast path:\n- {}",
        violations.join("\n- ")
    );
}

#[test]
fn publisher_fast_path_requires_branch_pr_and_issue_provenance_preflight() {
    let paths = [
        PR_PUBLISHER_AGENT,
        ".opencode/skills/elbmesh-pr-publisher/SKILL.md",
    ];
    let mut violations = Vec::new();

    for path in paths {
        let document = project_file(path).to_ascii_lowercase();
        for command in [
            "git push origin head",
            "git push --set-upstream origin head",
        ] {
            if !document.contains(command) {
                violations.push(format!(
                    "{path} must document the generic preflighted push form `{command}`"
                ));
            }
        }
        if document.split("\n\n").any(|paragraph| {
            paragraph.contains("push") && paragraph.contains("head") && paragraph.contains("never")
        }) {
            violations.push(format!(
                "{path} must not prohibit the accepted preflighted HEAD push forms"
            ));
        }

        let has_provenance_preflight = document.split("\n\n").any(|paragraph| {
            (paragraph.contains("preflight") || paragraph.contains("before"))
                && ["branch", "issue", "provenance"]
                    .iter()
                    .all(|term| paragraph.contains(term))
                && (paragraph.contains("pull request") || paragraph.contains(" pr "))
                && ["verify", "match"]
                    .iter()
                    .any(|term| paragraph.contains(term))
        });
        if !has_provenance_preflight {
            violations.push(format!(
                "{path} must require branch, pull-request, and issue provenance verification before publication mutation"
            ));
        }

        if !document
            .split("\n\n")
            .any(|paragraph| paragraph.contains("stop") && paragraph.contains("mismatch"))
        {
            violations.push(format!(
                "{path} must require the Publisher to stop on any provenance mismatch"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Publisher fast-path provenance violations:\n- {}",
        violations.join("\n- ")
    );
}

#[test]
fn publisher_fast_path_documents_defense_in_depth_and_accepted_wrong_issue_risk() {
    let paths = [
        PR_PUBLISHER_AGENT,
        ".opencode/skills/elbmesh-pr-publisher/SKILL.md",
    ];
    let mut violations = Vec::new();

    for path in paths {
        let document = project_file(path).to_ascii_lowercase();
        if !document.contains("gh issue edit *") || !document.contains("broad") {
            violations.push(format!(
                "{path} must explicitly document the accepted broad `gh issue edit *` permission"
            ));
        }
        if !document.contains("defense in depth") || !document.contains("not a sandbox") {
            violations.push(format!(
                "{path} must describe OpenCode permissions as defense in depth, not a sandbox"
            ));
        }

        let has_hard_boundary = document.split("\n\n").any(|paragraph| {
            paragraph.contains("branch protection")
                && paragraph.contains("ci")
                && (paragraph.contains("independent review")
                    || paragraph.contains("independent reviewer"))
                && paragraph.contains("hard bound")
        });
        if !has_hard_boundary {
            violations.push(format!(
                "{path} must identify GitHub branch protection, CI, and independent review as the hard boundary"
            ));
        }

        let accepts_wrong_issue_risk = document.split("\n\n").any(|paragraph| {
            paragraph.contains("wrong issue")
                && paragraph.contains("residual risk")
                && paragraph.contains("accept")
        });
        if !accepts_wrong_issue_risk {
            violations.push(format!(
                "{path} must explicitly document acceptance of the residual wrong-issue mutation risk"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Publisher fast-path boundary and residual-risk violations:\n- {}",
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
        OPERATIONS_SKILL,
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
        OPERATIONS_SKILL,
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
        for command in [
            "gh issue edit <issue> --remove-label status:review --add-label status:implementation",
            "gh issue edit <issue> --remove-label status:implementation --add-label status:review",
        ] {
            if !document.contains(command) {
                violations.push(format!(
                    "{path} must state the exact paired status transition `{command}`"
                ));
            }
        }

        if !document.split("\n\n").any(|paragraph| {
            paragraph.contains("exactly one")
                && paragraph.contains("status:implementation")
                && paragraph.contains("status:review")
        }) {
            violations.push(format!(
                "{path} must state that exactly one of status:implementation and status:review is active"
            ));
        }

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

        if !document.contains("only a human") || !document.contains("merge") {
            violations.push(format!(
                "{path} must retain human-only merge authority while documenting status transitions"
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
fn canonical_workflow_keeps_human_merge_and_tracks_automated_issue_statuses() {
    let path = "docs/DEVELOPMENT_WORKFLOW.md";
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

    assert_contains_all(
        path,
        &documentation,
        &[
            "publisher",
            "red publication",
            "status:implementation",
            "status:review",
            "reviewer reports no blockers",
            "required ci passes",
        ],
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
fn workflow_documents_automatic_pull_request_creation_and_human_only_merge() {
    let path = "docs/DEVELOPMENT_WORKFLOW.md";
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
    let harness = project_file("docs/AGENT_DELIVERY_HARNESS.md").to_ascii_lowercase();
    assert!(
        harness.split("\n\n").any(|paragraph| {
            paragraph.contains("permission")
                && paragraph.contains("defense in depth")
                && paragraph.contains("not a sandbox")
        }),
        "docs/AGENT_DELIVERY_HARNESS.md must document OpenCode permissions as defense in depth rather than a sandbox"
    );
}

#[test]
fn roadmap_uses_dependency_order_without_an_active_phase_gate() {
    let path = "docs/DELIVERY_ROADMAP.md";
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
fn only_the_orchestrator_can_spawn_delivery_and_operations_role_agents() {
    let config = project_file("opencode.json");
    let config: Value = serde_json::from_str(&config)
        .unwrap_or_else(|error| panic!("opencode.json must be valid JSON: {error}"));
    let mut violations = Vec::new();

    if project_permission_default(&config, "task").as_deref() != Some("deny") {
        violations.push("opencode.json must deny Task by default".to_owned());
    }

    for path in [
        OPERATIONS_AGENT,
        TEST_WRITER_AGENT,
        PR_PUBLISHER_AGENT,
        IMPLEMENTER_AGENT,
        REVIEWER_AGENT,
    ] {
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
    if task_rules.len() != 6 {
        violations.push(format!(
            "{ORCHESTRATOR_AGENT} must have only one broad Task deny followed by the five role allows, found {task_rules:?}"
        ));
    }

    let mut allowed_agents: Vec<_> = task_rules
        .iter()
        .filter(|(_, action)| action == "allow")
        .map(|(pattern, _)| pattern.as_str())
        .collect();
    allowed_agents.sort_unstable();
    let mut expected_agents = [
        "elbmesh-operations",
        "elbmesh-test-writer",
        "elbmesh-implementer",
        "elbmesh-reviewer",
        "elbmesh-pr-publisher",
    ];
    expected_agents.sort_unstable();
    if allowed_agents != expected_agents {
        violations.push(format!(
            "{ORCHESTRATOR_AGENT} may allow Task only for operations and the four delivery roles, found {allowed_agents:?}"
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

fn workflow_job_containing(workflow: &str, marker: &str) -> Option<String> {
    let jobs = workflow.split_once("\njobs:\n")?.1;
    let mut current = String::new();
    let mut blocks = Vec::new();

    for line in jobs.lines() {
        let starts_job =
            line.starts_with("  ") && !line.starts_with("    ") && line.trim_end().ends_with(':');

        if starts_job && !current.is_empty() {
            blocks.push(std::mem::take(&mut current));
        }

        if starts_job || !current.is_empty() {
            current.push_str(line);
            current.push('\n');
        }
    }

    if !current.is_empty() {
        blocks.push(current);
    }

    blocks.into_iter().find(|block| block.contains(marker))
}

fn workflow_job_runs_exact_command(job: &str, command: &str) -> bool {
    job.lines()
        .map(str::trim)
        .any(|line| line == command || line.strip_prefix("run: ").is_some_and(|run| run == command))
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
