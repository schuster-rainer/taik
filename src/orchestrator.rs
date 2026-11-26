//! The orchestrator decides how a request of the user is carried out: as one document or as an agent
//! session on a repository, which agent writes the gate and which one does the work. It assigns and
//! explains. It starts nothing: the job still waits for the approval of the user.
//!
//! The plan is a few `key=value` lines kept with the request and with the proposal.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::progress::Progress;
use crate::{Result, record, source, talk};

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
The gate is written before the work and judges it. For a 'sitzung' prefer a Gate agent on the claude harness other than the work agent: only such an agent can read the repository before it writes the gate. Choose two different agents for Gate and Arbeit whenever two are listed, so that nobody judges their own work. \
Prefer the agent whose description fits the task; without a reason to prefer one, use 'standard' for the work.\n\n\
Instead of the five lines you may reply with exactly one line 'Frage: <one question in German>' and nothing else. \
Ask only when a wrong assumption would make the work order worthless; otherwise decide. The user answers, and then you plan again. \
You may ask at most three questions about one request.";
const RULES: &str = "A change to a listed product is of the kind 'dokument': the job starts from the product's accepted source and is judged by its own pinned gate. \
A session may not change the protected paths listed under its repository; a commit that touches one is rejected. \
An agent marked '(failed recently)' could not be reached a short while ago; do not choose it for the gate while another agent can write it.";
/// How many questions the orchestrator may ask about one request.
pub const MAX_QUESTIONS: usize = 3;
// How long a failed call of a gate author keeps it out of the gate.
const FAILED_FOR: Duration = Duration::from_secs(6 * 60 * 60);

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
    failed: bool,
}

fn failed_marker(home: &Path, name: &str) -> PathBuf {
    home.join("crew").join(format!("{name}.failed"))
}

// STATE/crew/NAME.failed younger than six hours: the last call of this gate author failed.
fn failed_recently(home: &Path, name: &str) -> bool {
    fs::symlink_metadata(failed_marker(home, name))
        .and_then(|data| data.modified())
        .ok()
        .and_then(|changed| changed.elapsed().ok())
        .is_some_and(|age| age < FAILED_FOR)
}

/// Keeps the outcome of a call of a named gate author: a failure leaves STATE/crew/NAME.failed with
/// the error, a delivered gate removes it. The standard worker gets no such file.
pub fn mark_gate_author(home: &Path, name: &str, failure: Option<&str>) {
    if name == STANDARD || !crate::crew::valid_name(name) {
        return;
    }
    let marker = failed_marker(home, name);
    match failure {
        Some(error) => {
            if fs::create_dir_all(home.join("crew")).is_ok() {
                let _ = fs::write(&marker, format!("{}\n", source::single_line(error)));
            }
        }
        None => {
            let _ = fs::remove_file(&marker);
        }
    }
}

// The standard worker of the service and every agent of the crew.
fn crew(home: &Path) -> Vec<Member> {
    let mut members = vec![Member {
        name: STANDARD.to_owned(),
        claude: true,
        about: "the standard worker of the service".to_owned(),
        failed: failed_recently(home, STANDARD),
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
                failed: failed_recently(home, &name),
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
    // A gate author that failed recently gives way to one that did not, as long as one exists.
    if gate.failed
        && let Some(other) = crew
            .iter()
            .find(|member| !member.failed && member.name != work.name)
    {
        reason.push_str(&format!(
            " @{} fiel kürzlich aus, deshalb schreibt @{} das Gate.",
            gate.name, other.name
        ));
        gate = other;
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

// Paths a session may not change in a repository: the lines of its .taik/protected without comments.
fn protected(repository: &Path) -> Vec<String> {
    fs::read_to_string(repository.join(".taik/protected"))
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

// The text the orchestrator sends to its worker for a request.
fn compose(
    home: &Path,
    crew: &[Member],
    repositories: &[(String, PathBuf)],
    request: &Path,
) -> Result<String> {
    let mut prompt = format!("{CONTRACT}\n\n{RULES}\n\nAGENTS:\n");
    for member in crew {
        prompt.push_str(&format!(
            "- {}: {}{}\n",
            member.name,
            member.about,
            if member.failed {
                " (failed recently)"
            } else {
                ""
            }
        ));
    }
    prompt.push_str("\nREPOSITORIES:\n");
    if repositories.is_empty() {
        prompt.push_str("(none)\n");
    }
    for (name, path) in repositories {
        let about = fs::read_to_string(path.join("README.md")).unwrap_or_default();
        prompt.push_str(&format!(
            "- {name}: {}\n",
            source::single_line(&about.chars().take(300).collect::<String>())
        ));
        let protected = protected(path);
        prompt.push_str(&format!(
            "  protected: {}\n",
            if protected.is_empty() {
                "(none)".to_owned()
            } else {
                protected.join(", ")
            }
        ));
    }
    prompt.push_str("\nPRODUCTS:\n");
    let products = crate::proposal::products(home);
    if products.is_empty() {
        prompt.push_str("(none)\n");
    }
    for product in &products {
        prompt.push_str(&format!("- {}: {}\n", product.name, product.about));
    }
    prompt.push_str(&format!(
        "\nREQUEST OF THE USER:\n{}",
        record::read_regular(request, record::MAX_RECORD_BYTES)?
    ));
    let said = talk::read(&conversation(request));
    prompt.push_str(&talk::prompt(&said));
    if questions(&said) >= MAX_QUESTIONS {
        prompt.push_str(
            "\n\nYou asked three questions about this request already. Do not ask again: reply with the five lines of the plan.",
        );
    }
    Ok(prompt)
}

/// The conversation about a request STATE/requests/NAME.run: the folder STATE/requests/NAME.talk.
pub fn conversation(request: &Path) -> PathBuf {
    request.with_extension("talk")
}

// The questions the orchestrator asked so far.
fn questions(said: &[talk::Message]) -> usize {
    said.iter()
        .filter(|message| message.author == talk::Author::Agent)
        .count()
}

// An answer of exactly one line 'Frage: ...' is a question to the user.
fn question(answer: &str) -> Option<String> {
    let mut lines = answer.lines().map(str::trim).filter(|line| !line.is_empty());
    let line = lines.next()?;
    if lines.next().is_some() {
        return None;
    }
    let line = line.trim_start_matches(['-', '*', ' ']).replace("**", "");
    let text = line.strip_prefix("Frage:")?.trim();
    (!text.is_empty() && text.len() <= talk::MAX_MESSAGE_BYTES).then(|| text.to_owned())
}

#[derive(Debug, PartialEq)]
enum Answer {
    Plan(Plan),
    Question(String),
}

// What the worker of the orchestrator answered: a plan, or a question while questions are left.
fn decide(
    answer: &str,
    asked: usize,
    crew: &[Member],
    repositories: &[(String, PathBuf)],
) -> Result<Answer> {
    if let Some(text) = question(answer) {
        if asked >= MAX_QUESTIONS {
            return Err("the orchestrator asked more than three questions".into());
        }
        return Ok(Answer::Question(text));
    }
    parse(answer, crew, repositories).map(Answer::Plan)
}

/// The text the orchestrator would send to its worker for a request. Nothing is called or written.
pub fn prompt(home: &Path, request: &Path) -> Result<String> {
    compose(home, &crew(home), &repositories(home), request)
}

/// Asks the standard worker for the plan of a request and keeps it beside the request. `None`: the
/// orchestrator asked the user a question, or one waits for an answer, and there is no plan yet.
pub fn plan(home: &Path, name: &str, request: &Path) -> Result<Option<Plan>> {
    let target = request.with_file_name(format!("{name}.plan"));
    if let Some(plan) = Plan::read(&target) {
        return Ok(Some(plan));
    }
    let chat = conversation(request);
    let said = talk::read(&chat);
    if talk::waits_for_user(&said) {
        return Ok(None);
    }
    let folder = request.parent().ok_or("request without a folder")?;
    let crew = crew(home);
    let repositories = repositories(home);
    let prompt = compose(home, &crew, &repositories, request)?;
    let scratch = tempfile::tempdir()?;
    let scratch = scratch.path().canonicalize()?;
    let workspace = scratch.join("worker");
    fs::create_dir(&workspace)?;
    let (answer, worker) = source::ask(&workspace, folder, &prompt, &mut Progress::silent());
    let (answer, usage) = answer?;
    let asked = questions(&said);
    // An answer that is no plan does not hold the request up: it runs the plain way, and says so.
    let plan = match decide(&answer, asked, &crew, &repositories) {
        Ok(Answer::Question(text)) => {
            fs::create_dir_all(&chat)?;
            talk::write(&chat, talk::Author::Agent, 0, &text)?;
            let _ = record::publish_text(
                &folder.join(format!("{name}.question-{}.run", asked + 1)),
                &format!(
                    "decision=observed\nidentity={name}.question-{}\nworker={worker}\n{usage}end\n",
                    asked + 1
                ),
                record::MAX_RECORD_BYTES,
            );
            return Ok(None);
        }
        Ok(Answer::Plan(plan)) => plan,
        Err(error) => Plan {
            session: false,
            repo: None,
            gate: STANDARD.to_owned(),
            work: STANDARD.to_owned(),
            reason: format!("Der Orchestrator gab keinen gültigen Plan ({error}). Der Auftrag läuft als Dokument beim Standard."),
        },
    };
    record::publish_text(&target, &plan.text(), MAX_PLAN_BYTES)?;
    record::publish_text(
        &folder.join(format!("{name}.plan.run")),
        &format!("decision=observed\nidentity={name}.plan\nworker={worker}\n{usage}end\n"),
        record::MAX_RECORD_BYTES,
    )?;
    Ok(Some(plan))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(name: &str, claude: bool) -> Member {
        Member {
            name: name.into(),
            claude,
            about: String::new(),
            failed: false,
        }
    }

    #[test]
    fn a_failed_gate_agent_is_replaced() {
        let failed = |name: &str| Member {
            failed: true,
            ..member(name, true)
        };
        let crew = [
            member(STANDARD, true),
            failed("anna"),
            member("bert", true),
            member("carl", true),
        ];
        let plan = parse(
            "Art: dokument\nRepo: -\nGate: anna\nArbeit: standard\nGrund: Ein Dokument.",
            &crew,
            &[],
        )
        .unwrap();
        // The first agent in the order of the crew that did not fail and does not do the work.
        assert_eq!((plan.gate.as_str(), plan.work.as_str()), ("bert", STANDARD));
        assert_eq!(
            plan.reason,
            "Ein Dokument. @anna fiel kürzlich aus, deshalb schreibt @bert das Gate."
        );

        // An agent that did not fail keeps the gate.
        let kept = parse(
            "Art: dokument\nRepo: -\nGate: carl\nArbeit: standard\nGrund: Ein Dokument.",
            &crew,
            &[],
        )
        .unwrap();
        assert_eq!(
            (kept.gate.as_str(), kept.reason.as_str()),
            ("carl", "Ein Dokument.")
        );

        // Without another agent the choice of the worker stays.
        let alone = [member(STANDARD, true), failed("anna")];
        let stays = parse(
            "Art: dokument\nRepo: -\nGate: anna\nArbeit: standard\nGrund: Ein Dokument.",
            &alone,
            &[],
        )
        .unwrap();
        assert_eq!(
            (stays.gate.as_str(), stays.reason.as_str()),
            ("anna", "Ein Dokument.")
        );
    }

    #[test]
    fn the_failed_marker_is_written_and_removed() {
        let home = tempfile::tempdir().unwrap();
        let marker = home.path().join("crew/anna.failed");
        mark_gate_author(home.path(), "anna", Some("the call\nfailed"));
        assert_eq!(fs::read_to_string(&marker).unwrap(), "the call failed\n");
        assert!(failed_recently(home.path(), "anna"));

        // The standard worker gets no marker.
        mark_gate_author(home.path(), STANDARD, Some("down"));
        assert!(!home.path().join("crew/standard.failed").exists());
        assert!(!failed_recently(home.path(), STANDARD));

        // An old marker does nothing.
        let old = std::time::SystemTime::now() - Duration::from_secs(7 * 60 * 60);
        fs::File::options()
            .write(true)
            .open(&marker)
            .unwrap()
            .set_modified(old)
            .unwrap();
        assert!(!failed_recently(home.path(), "anna"));

        // A delivered gate removes the marker.
        mark_gate_author(home.path(), "anna", None);
        assert!(!marker.exists());
        mark_gate_author(home.path(), "anna", None);
        assert!(!marker.exists());
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
    fn a_question_is_recognised() {
        assert_eq!(
            question("Frage: Welche Farbe soll der Titel haben?").as_deref(),
            Some("Welche Farbe soll der Titel haben?")
        );
        assert_eq!(
            question("\n  - **Frage:** Für welches Repo?  \n\n").as_deref(),
            Some("Für welches Repo?")
        );
        // An empty question, a second line or another label is no question.
        assert_eq!(question("Frage:   "), None);
        assert_eq!(question("Frage: Welche Farbe?\nArt: dokument"), None);
        assert_eq!(question("Art: dokument"), None);
        assert_eq!(question("Ich frage: Welche Farbe?"), None);
        assert_eq!(question(""), None);
        assert_eq!(
            question(&format!("Frage: {}", "x".repeat(talk::MAX_MESSAGE_BYTES + 1))),
            None
        );

        let crew = [member(STANDARD, true), member("anna", true)];
        assert_eq!(
            decide("Frage: Welche Farbe?", 0, &crew, &[]).unwrap(),
            Answer::Question("Welche Farbe?".into())
        );
        let planned = decide(
            "Art: dokument\nRepo: -\nGate: anna\nArbeit: standard\nGrund: Ein Dokument.",
            0,
            &crew,
            &[],
        )
        .unwrap();
        assert!(matches!(planned, Answer::Plan(plan) if plan.gate == "anna"));
    }

    #[test]
    fn after_three_questions_the_orchestrator_must_plan() {
        let crew = [member(STANDARD, true), member("anna", true)];
        for asked in 0..MAX_QUESTIONS {
            assert!(matches!(
                decide("Frage: Noch etwas?", asked, &crew, &[]),
                Ok(Answer::Question(_))
            ));
        }
        // A fourth question is an answer that is no plan.
        assert!(decide("Frage: Noch etwas?", MAX_QUESTIONS, &crew, &[]).is_err());
        // A plan is still taken.
        assert!(matches!(
            decide(
                "Art: dokument\nRepo: -\nGate: anna\nArbeit: standard\nGrund: Klar.",
                MAX_QUESTIONS,
                &crew,
                &[]
            ),
            Ok(Answer::Plan(_))
        ));

        // The prompt carries the conversation and, after three questions, says that none is left.
        let home = tempfile::tempdir().unwrap();
        let requests = home.path().join("requests");
        fs::create_dir_all(&requests).unwrap();
        let request = requests.join("ticket-001.run");
        fs::write(&request, "decision=observed\nidentity=ticket-001\nend\n").unwrap();
        let plain = prompt(home.path(), &request).unwrap();
        assert!(!plain.contains("CONVERSATION ABOUT THIS WORK ORDER"));
        assert!(plain.contains("Frage: <one question in German>"));
        let chat = conversation(&request);
        assert_eq!(chat, requests.join("ticket-001.talk"));
        fs::create_dir(&chat).unwrap();
        for number in 1..=MAX_QUESTIONS {
            talk::write(&chat, talk::Author::Agent, 0, &format!("Frage {number}?")).unwrap();
            let asked = prompt(home.path(), &request).unwrap();
            assert!(asked.contains(&format!("[you] Frage {number}?\n")), "{asked}");
            assert_eq!(
                asked.contains("Do not ask again"),
                number == MAX_QUESTIONS,
                "{asked}"
            );
            talk::write(&chat, talk::Author::User, 0, &format!("Antwort {number}.")).unwrap();
        }
        let said = talk::read(&chat);
        assert_eq!(questions(&said), MAX_QUESTIONS);
        assert!(!talk::waits_for_user(&said));
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
