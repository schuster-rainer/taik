//! The orchestrator decides how a request of the user is carried out: as one document or as an agent
//! session on a repository, which agent writes the gate and which one does the work. It assigns and
//! explains. It starts nothing: the job still waits for the approval of the user.
//!
//! The plan is a few `key=value` lines kept with the request and with the proposal.

use std::fs;
use std::path::{Path, PathBuf};

use crate::progress::Progress;
use crate::{Result, record, source};

pub const STANDARD: &str = "standard";
const MAX_PLAN_BYTES: usize = 2048;
const CONTRACT: &str = "You are the orchestrator of a software factory. A user asked for something. Decide how it is carried out. \
Reply with exactly these five lines and nothing else:\n\
Art: dokument | sitzung\n\
Repo: <name of a listed repository, or - >\n\
Gate: <name of a listed agent>\n\
Arbeit: <name of a listed agent>\n\
Grund: <one German sentence of at most 30 words: why this kind and these agents>\n\n\
'dokument' is right when the result is one new self-contained text document, such as a single HTML page or a small program in one bundle. \
'sitzung' is right when existing files of a listed repository must be read or changed; it needs a repository, and its work agent must be on the claude harness or be 'standard'. \
The gate is written before the work and judges it. Choose two different agents for Gate and Arbeit whenever two are listed, so that nobody judges their own work. \
Prefer the agent whose description fits the task; without a reason to prefer one, use 'standard' for the work.";

#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub session: bool,
    pub repo: Option<(String, PathBuf)>,
    pub gate: String,
    pub work: String,
    pub reason: String,
}

impl Plan {
    pub fn text(&self) -> String {
        format!(
            "kind={}\nrepo={}\nrepo_path={}\ngate={}\nwork={}\nreason={}\n",
            if self.session { "sitzung" } else { "dokument" },
            self.repo.as_ref().map_or("", |repo| repo.0.as_str()),
            self.repo
                .as_ref()
                .map_or_else(String::new, |repo| repo.1.display().to_string()),
            self.gate,
            self.work,
            source::single_line(&self.reason),
        )
    }

    pub fn read(path: &Path) -> Option<Self> {
        let text = record::read_regular(path, MAX_PLAN_BYTES).ok()?;
        let field = |key: &str| {
            text.lines()
                .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
                .unwrap_or("")
                .to_owned()
        };
        let repo = Some((field("repo"), PathBuf::from(field("repo_path"))))
            .filter(|repo| !repo.0.is_empty() && !repo.1.as_os_str().is_empty());
        Some(Self {
            session: field("kind") == "sitzung",
            repo,
            gate: Some(field("gate")).filter(|name| !name.is_empty())?,
            work: Some(field("work")).filter(|name| !name.is_empty())?,
            reason: field("reason"),
        })
    }
}

struct Member {
    name: String,
    claude: bool,
    about: String,
}

// The standard worker of the service and every agent of the crew.
fn crew(home: &Path) -> Vec<Member> {
    let mut members = vec![Member {
        name: STANDARD.to_owned(),
        claude: true,
        about: "the standard worker of the service".to_owned(),
    }];
    let mut names: Vec<String> = fs::read_dir(home.join("crew"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .strip_suffix(".agent")
                .map(str::to_owned)
        })
        .collect();
    names.sort_unstable();
    for name in names {
        if let Ok(agent) = crate::crew::load(home, &name) {
            members.push(Member {
                claude: agent.harness == crate::crew::Harness::Claude,
                about: format!(
                    "harness {:?}, model {}{}",
                    agent.harness,
                    if agent.model.is_empty() {
                        "default"
                    } else {
                        &agent.model
                    },
                    if agent.persona.is_empty() {
                        String::new()
                    } else {
                        format!(", {}", source::single_line(&agent.persona))
                    }
                ),
                name,
            });
        }
    }
    members
}

// Repositories the user made known: STATE/repos/NAME.repo holds one path.
fn repositories(home: &Path) -> Vec<(String, PathBuf)> {
    let mut found: Vec<(String, PathBuf)> = fs::read_dir(home.join("repos"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry
                .file_name()
                .to_string_lossy()
                .strip_suffix(".repo")?
                .to_owned();
            let path = record::read_regular(&entry.path(), 1024).ok()?;
            let path = PathBuf::from(path.trim()).canonicalize().ok()?;
            (crate::crew::valid_name(&name) && path.join(".git").exists()).then_some((name, path))
        })
        .collect();
    found.sort();
    found
}

fn parse(answer: &str, crew: &[Member], repositories: &[(String, PathBuf)]) -> Result<Plan> {
    let field = |label: &str| {
        answer
            .lines()
            .map(|line| {
                line.trim()
                    .trim_start_matches(['-', '*', ' '])
                    .replace("**", "")
            })
            .find_map(|line| line.strip_prefix(label).map(|rest| rest.trim().to_owned()))
            .unwrap_or_default()
    };
    let member = |name: String, role: &str| {
        let name = name.trim_start_matches('@').to_lowercase();
        crew.iter()
            .find(|member| member.name == name)
            .ok_or_else(|| format!("the orchestrator named no listed agent for {role}"))
    };
    let mut work = member(field("Arbeit:"), "the work")?;
    let mut gate = member(field("Gate:"), "the gate")?;
    let repo = repositories
        .iter()
        .find(|repo| repo.0 == field("Repo:"))
        .cloned();
    let session = field("Art:").to_lowercase().starts_with("sitzung");
    if session && repo.is_none() {
        return Err("the orchestrator chose a session without a listed repository".into());
    }
    let mut reason = field("Grund:");
    if reason.is_empty() {
        return Err("the orchestrator gave no reason".into());
    }
    // A session runs on the Claude harness. Another choice falls back to the standard worker.
    if session && !work.claude {
        work = &crew[0];
        reason.push_str(
            " Die Arbeit geht an den Standard, weil eine Sitzung den Claude-Harness braucht.",
        );
    }
    // Nobody judges their own work, as long as a second agent exists.
    if gate.name == work.name
        && let Some(other) = crew.iter().find(|member| member.name != work.name)
    {
        gate = other;
        reason.push_str(" Das Gate schreibt ein anderer Agent als die Arbeit.");
    }
    Ok(Plan {
        session,
        repo: repo.filter(|_| session),
        gate: gate.name.clone(),
        work: work.name.clone(),
        reason: reason.chars().take(600).collect(),
    })
}

/// The plans to try when the gate author of a plan fails: every other agent that is not the worker,
/// in the order of the crew. The reason of such a plan names who dropped out.
pub fn substitutes(home: &Path, plan: &Plan) -> Vec<Plan> {
    crew(home)
        .into_iter()
        .filter(|member| member.name != plan.gate && member.name != plan.work)
        .map(|member| Plan {
            gate: member.name.clone(),
            reason: format!(
                "{} @{} fiel als Autor des Gates aus, @{} sprang ein.",
                plan.reason, plan.gate, member.name
            ),
            ..plan.clone()
        })
        .collect()
}

/// Asks the standard worker for the plan of a request and keeps it beside the request.
pub fn plan(home: &Path, name: &str, request: &Path) -> Result<Plan> {
    let target = request.with_file_name(format!("{name}.plan"));
    if let Some(plan) = Plan::read(&target) {
        return Ok(plan);
    }
    let folder = request.parent().ok_or("request without a folder")?;
    let crew = crew(home);
    let repositories = repositories(home);
    let mut prompt = format!("{CONTRACT}\n\nAGENTS:\n");
    for member in &crew {
        prompt.push_str(&format!("- {}: {}\n", member.name, member.about));
    }
    prompt.push_str("\nREPOSITORIES:\n");
    if repositories.is_empty() {
        prompt.push_str("(none)\n");
    }
    for (name, path) in &repositories {
        let about = fs::read_to_string(path.join("README.md")).unwrap_or_default();
        prompt.push_str(&format!(
            "- {name}: {}\n",
            source::single_line(&about.chars().take(300).collect::<String>())
        ));
    }
    prompt.push_str(&format!(
        "\nREQUEST OF THE USER:\n{}",
        record::read_regular(request, record::MAX_RECORD_BYTES)?
    ));
    let scratch = tempfile::tempdir()?;
    let scratch = scratch.path().canonicalize()?;
    let workspace = scratch.join("worker");
    fs::create_dir(&workspace)?;
    let (answer, worker) = source::ask(&workspace, folder, &prompt, &mut Progress::silent());
    let (answer, usage) = answer?;
    // An answer that is no plan does not hold the request up: it runs the plain way, and says so.
    let plan = parse(&answer, &crew, &repositories).unwrap_or_else(|error| Plan {
        session: false,
        repo: None,
        gate: STANDARD.to_owned(),
        work: STANDARD.to_owned(),
        reason: format!("Der Orchestrator gab keinen gültigen Plan ({error}). Der Auftrag läuft als Dokument beim Standard."),
    });
    record::publish_text(&target, &plan.text(), MAX_PLAN_BYTES)?;
    record::publish_text(
        &folder.join(format!("{name}.plan.run")),
        &format!("decision=observed\nidentity={name}.plan\nworker={worker}\n{usage}end\n"),
        record::MAX_RECORD_BYTES,
    )?;
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(name: &str, claude: bool) -> Member {
        Member {
            name: name.into(),
            claude,
            about: String::new(),
        }
    }

    #[test]
    fn a_plan_is_read_checked_and_kept() {
        let crew = [member(STANDARD, true), member("savant", false)];
        let repos = [("taik".to_owned(), PathBuf::from("/tmp/taik"))];
        let plan = parse(
            "Art: sitzung\nRepo: taik\nGate: @Savant\n**Arbeit:** standard\nGrund: Bestehende Dateien ändern.",
            &crew,
            &repos,
        )
        .unwrap();
        assert!(plan.session && plan.repo == Some(repos[0].clone()));
        assert_eq!(
            (plan.gate.as_str(), plan.work.as_str()),
            ("savant", STANDARD)
        );

        // The same agent for both: the gate moves to another one.
        let same = parse(
            "Art: dokument\nRepo: -\nGate: standard\nArbeit: standard\nGrund: Einfach.",
            &crew,
            &repos,
        )
        .unwrap();
        assert_eq!(
            (same.gate.as_str(), same.work.as_str()),
            ("savant", STANDARD)
        );
        assert!(!same.session && same.repo.is_none());
        assert!(
            same.reason
                .ends_with("Das Gate schreibt ein anderer Agent als die Arbeit.")
        );

        // A session cannot run on another harness.
        let moved = parse(
            "Art: sitzung\nRepo: taik\nGate: standard\nArbeit: savant\nGrund: Test.",
            &crew,
            &repos,
        )
        .unwrap();
        assert_eq!(
            (moved.gate.as_str(), moved.work.as_str()),
            ("savant", STANDARD)
        );

        assert!(
            parse(
                "Art: sitzung\nRepo: fremd\nGate: savant\nArbeit: standard\nGrund: x",
                &crew,
                &repos
            )
            .is_err()
        );
        assert!(
            parse(
                "Art: dokument\nRepo: -\nGate: niemand\nArbeit: standard\nGrund: x",
                &crew,
                &repos
            )
            .is_err()
        );
        assert!(
            parse(
                "Art: dokument\nRepo: -\nGate: savant\nArbeit: standard",
                &crew,
                &repos
            )
            .is_err()
        );

        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join("a.plan");
        fs::write(&file, plan.text()).unwrap();
        assert_eq!(Plan::read(&file), Some(plan));
    }

    #[test]
    fn another_agent_steps_in_for_a_gate_author_that_fails() {
        let home = tempfile::tempdir().unwrap();
        fs::create_dir(home.path().join("crew")).unwrap();
        for name in ["anna", "bert"] {
            fs::write(
                home.path().join(format!("crew/{name}.agent")),
                "harness=claude\nmodel=m\neffort=low\npersona=\nend\n",
            )
            .unwrap();
        }
        let plan = Plan {
            session: false,
            repo: None,
            gate: "anna".into(),
            work: STANDARD.into(),
            reason: "Grund.".into(),
        };
        let others = substitutes(home.path(), &plan);
        // Neither the one that failed nor the one that does the work.
        assert_eq!(others.len(), 1);
        assert_eq!(
            (others[0].gate.as_str(), others[0].work.as_str()),
            ("bert", STANDARD)
        );
        assert_eq!(
            others[0].reason,
            "Grund. @anna fiel als Autor des Gates aus, @bert sprang ein."
        );
    }
}
