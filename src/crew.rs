//! Named agents and the harnesses behind them.
//!
//! An agent is a small file in STATE/crew: a harness, a model, an effort and a persona. A job may name
//! the agent that does its next round in a file `agent` in its folder. The service reads both before
//! every round, so a change takes effect without a restart. A job without that file is worked by the
//! worker the environment configures, as before.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::{Result, record};

const MAX_AGENT_BYTES: usize = 2048;
const MAX_PERSONA_BYTES: usize = 600;
const PROBE_TIMEOUT: Duration = Duration::from_secs(12);
pub const EFFORTS: &[&str] = &["off", "minimal", "low", "medium", "high", "xhigh", "max"];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Harness {
    Claude,
    Codex,
    Savant,
    Grok,
    Pi,
}

impl Harness {
    pub const ALL: [Self; 5] = [
        Self::Claude,
        Self::Codex,
        Self::Savant,
        Self::Grok,
        Self::Pi,
    ];

    fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|harness| harness.name() == name)
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Savant => "savant",
            Self::Grok => "grok",
            Self::Pi => "pi",
        }
    }

    /// The worker name a round record carries.
    pub fn label(self) -> &'static str {
        match self {
            Self::Claude => "claude-cli",
            Self::Codex => "codex-subscription",
            Self::Savant => "savant",
            Self::Grok => "grok-cli",
            Self::Pi => "pi-cli",
        }
    }
}

impl fmt::Display for Harness {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str(self.name())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Agent {
    pub name: String,
    pub harness: Harness,
    pub model: String,
    pub effort: String,
    pub persona: String,
}

impl Agent {
    /// The persona comes before the Factory's own contract sentence and never replaces it.
    pub fn prompt(&self, contract: &str) -> String {
        if self.persona.is_empty() {
            contract.to_owned()
        } else {
            format!("{}\n\n{contract}", self.persona)
        }
    }

    fn parse(name: &str, text: &str) -> Result<Self> {
        if !text.ends_with("end\n") {
            return Err("incomplete agent definition".into());
        }
        let field = |key: &str| {
            text.lines()
                .filter_map(|line| line.split_once('='))
                .find_map(|(name, value)| (name == key).then_some(value.trim()))
                .unwrap_or("")
        };
        let harness = Harness::parse(field("harness")).ok_or("unknown harness")?;
        let model = field("model");
        let plain = |value: &str| {
            value.len() <= 80
                && value.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'/' | b'-')
                })
        };
        if !plain(model) {
            return Err("invalid model name".into());
        }
        let effort = field("effort");
        if !effort.is_empty() && !EFFORTS.contains(&effort) {
            return Err("unknown effort".into());
        }
        let persona = field("persona");
        if persona.len() > MAX_PERSONA_BYTES {
            return Err("the persona exceeds its size limit".into());
        }
        Ok(Self {
            name: name.to_owned(),
            harness,
            model: model.to_owned(),
            effort: effort.to_owned(),
            persona: persona.to_owned(),
        })
    }
}

pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 32
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn directory(state: &Path) -> PathBuf {
    state.join("crew")
}

pub fn load(state: &Path, name: &str) -> Result<Agent> {
    if !valid_name(name) {
        return Err("invalid agent name".into());
    }
    let text = record::read_regular(
        &directory(state).join(format!("{name}.agent")),
        MAX_AGENT_BYTES,
    )
    .map_err(|_| format!("agent {name} is not defined"))?;
    Agent::parse(name, &text)
}

/// The agent a job names for its next round. A job without a choice gives None. A choice that cannot
/// be honoured is an error: the round must not fall back to another account without the user knowing.
pub fn for_job(state: &Path, job: &Path) -> Result<Option<Agent>> {
    let choice = job.join("agent");
    if !choice.try_exists()? {
        return Ok(None);
    }
    let name = record::read_regular(&choice, 64)?;
    load(state, name.trim()).map(Some)
}

/// Finds a harness program without relying on the PATH of the service alone.
pub fn program(name: &str) -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut places: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect())
        .unwrap_or_default();
    if let Some(home) = home {
        places.push(home.join(".local/bin"));
        places.push(home.join(".grok/bin"));
    }
    places.push(PathBuf::from("/opt/homebrew/bin"));
    places.push(PathBuf::from("/usr/local/bin"));
    places
        .into_iter()
        .map(|place| place.join(name))
        .find(|candidate| candidate.is_file())
}

fn probe(program: &Path, arguments: &[&str]) -> Option<String> {
    let mut child = Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + PROBE_TIMEOUT;
    loop {
        match child.try_wait().ok()? {
            Some(_) => break,
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    }
    // Some programs report their state on stderr.
    let output = child.wait_with_output().ok()?;
    Some(format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

/// What the viewer shows as the overview: which harnesses exist here, whether they are signed in,
/// what they offer, and which agents are defined. Only the sign-in state is read, never a credential.
pub fn overview(state: &Path) -> Result<serde_json::Value> {
    use serde_json::json;
    let mut harnesses = Vec::new();
    for harness in Harness::ALL {
        let found = match harness {
            Harness::Savant => Some(PathBuf::from("/usr/bin/curl")),
            other => program(other.name()),
        };
        let mut entry = json!({
            "name": harness.name(),
            "worker": harness.label(),
            "kind": if harness == Harness::Savant { "api" } else { "cli" },
            "billing": match harness {
                Harness::Savant => "api",
                Harness::Pi => "je Anbieter",
                _ => "subscription",
            },
            "program": found.as_ref().map(|path| path.display().to_string()),
            "login": "unbekannt",
            "models": [],
            "efforts": [],
        });
        if let Some(path) = found.as_deref() {
            match harness {
                Harness::Claude => {
                    let status = probe(path, &["auth", "status"])
                        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
                    if let Some(status) = status {
                        entry["login"] = json!(if status["loggedIn"] == true {
                            "angemeldet"
                        } else {
                            "nicht angemeldet"
                        });
                        entry["method"] = status["authMethod"].clone();
                        entry["expires"] = status["loginExpiresAt"].clone();
                    }
                    entry["models"] =
                        json!(["claude-opus-5-5", "opus", "sonnet", "haiku", "fable"]);
                    entry["efforts"] = json!(["low", "medium", "high", "xhigh", "max"]);
                }
                Harness::Codex => {
                    if let Some(text) = probe(path, &["login", "status"]) {
                        entry["login"] = json!(if text.contains("Logged in") {
                            "angemeldet"
                        } else {
                            "nicht angemeldet"
                        });
                        entry["method"] = json!(if text.contains("ChatGPT") {
                            "ChatGPT"
                        } else {
                            ""
                        });
                    }
                }
                Harness::Grok => {
                    if let Some(text) = probe(path, &["models"]) {
                        entry["login"] = json!(if text.contains("logged in") {
                            "angemeldet"
                        } else {
                            "nicht angemeldet"
                        });
                        let models: Vec<&str> = text
                            .lines()
                            .filter_map(|line| {
                                let line = line.trim();
                                line.strip_prefix("* ").or_else(|| line.strip_prefix("- "))
                            })
                            .filter_map(|line| line.split_whitespace().next())
                            .collect();
                        entry["models"] = json!(models);
                    }
                    entry["efforts"] = json!(["low", "medium", "high", "xhigh"]);
                }
                Harness::Pi => {
                    let ready: Vec<&str> = ["anthropic", "openai", "openai-codex", "xai", "google"]
                        .into_iter()
                        .filter(|provider| {
                            probe(path, &["auth", "check", "--provider", provider])
                                .is_some_and(|text| text.lines().any(|line| line.trim() == "ready"))
                        })
                        .collect();
                    entry["login"] = json!(if ready.is_empty() {
                        "nicht angemeldet"
                    } else {
                        "angemeldet"
                    });
                    entry["method"] = json!(ready.join(", "));
                    if let Some(text) = probe(path, &["--list-models"]) {
                        let models: Vec<String> = text
                            .lines()
                            .filter_map(|line| {
                                let mut columns = line.split_whitespace();
                                let provider = columns.next()?;
                                let model = columns.next()?;
                                ready
                                    .contains(&provider)
                                    .then(|| format!("{provider}/{model}"))
                            })
                            .take(200)
                            .collect();
                        entry["models"] = json!(models);
                    }
                    entry["efforts"] = json!(EFFORTS);
                }
                Harness::Savant => {
                    let key = std::env::var("SUBQ_API_KEY").is_ok_and(|key| !key.trim().is_empty())
                        || std::env::var("TAIK_SUBQ_KEYCHAIN")
                            .ok()
                            .is_some_and(|entry| {
                                entry.split_once('/').is_some_and(|(service, account)| {
                                    Command::new("/usr/bin/security")
                                        .args([
                                            "find-generic-password",
                                            "-s",
                                            service,
                                            "-a",
                                            account,
                                        ])
                                        .stdin(Stdio::null())
                                        .stdout(Stdio::null())
                                        .stderr(Stdio::null())
                                        .status()
                                        .is_ok_and(|status| status.success())
                                })
                            });
                    entry["login"] = json!(if key {
                        "Schlüssel vorhanden"
                    } else {
                        "kein Schlüssel"
                    });
                    entry["method"] = json!("SubQ API");
                    entry["models"] = json!([crate::source::SAVANT_MODEL]);
                    entry["efforts"] = json!(["low", "high", "max"]);
                }
            }
        } else {
            entry["login"] = json!("nicht installiert");
        }
        harnesses.push(entry);
    }
    // Installed but not wired into the core. Shown so that the gap is visible.
    for other in ["gemini", "opencode"] {
        if let Some(path) = program(other) {
            harnesses.push(json!({
                "name": other, "kind": "cli", "billing": "subscription", "worker": null,
                "program": path.display().to_string(), "login": "nicht eingebunden", "models": [], "efforts": [],
            }));
        }
    }
    let mut agents = Vec::new();
    if let Ok(entries) = fs::read_dir(directory(state)) {
        let mut names: Vec<String> = entries
            .flatten()
            .filter_map(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .and_then(|name| name.strip_suffix(".agent"))
                    .map(str::to_owned)
            })
            .collect();
        names.sort();
        for name in names {
            match load(state, &name) {
                Ok(agent) => agents.push(json!({
                    "name": agent.name, "harness": agent.harness.name(), "model": agent.model,
                    "effort": agent.effort, "persona": agent.persona,
                })),
                Err(error) => agents.push(json!({ "name": name, "error": error.to_string() })),
            }
        }
    }
    let checked = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    let (prices, prices_from) = prices();
    Ok(
        json!({ "checked_at": checked, "harnesses": harnesses, "agents": agents,
        "prices": prices, "prices_from": prices_from }),
    )
}

/// List prices in USD per million tokens, keyed by "provider/model". They come from the model
/// catalog that Pi keeps on this machine, plus the documented prices of Savant.
pub fn prices() -> (serde_json::Map<String, serde_json::Value>, String) {
    use serde_json::json;
    let mut map = serde_json::Map::new();
    map.insert(
        format!("subq/{}", crate::source::SAVANT_MODEL),
        json!({ "input": 0.10, "output": 0.50, "cache_read": 0.10 }),
    );
    let mut source = String::from("SubQ documentation of 2026-11-25");
    let catalog = std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join(".pi/agent/models-store.json"));
    let read = catalog.as_ref().and_then(|path| {
        let size = path.metadata().ok()?.len();
        (size <= 4 * 1024 * 1024).then(|| fs::read_to_string(path).ok())?
    });
    if let Some(store) = read.and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
    {
        for provider in ["anthropic", "openai", "xai"] {
            let models = store[provider]["models"]
                .as_array()
                .or_else(|| store[provider].as_array());
            for model in models.into_iter().flatten() {
                let (Some(id), Some(input), Some(output)) = (
                    model["id"].as_str(),
                    model["cost"]["input"].as_f64(),
                    model["cost"]["output"].as_f64(),
                ) else {
                    continue;
                };
                map.insert(
                    format!("{provider}/{id}"),
                    json!({ "input": input, "output": output,
                        "cache_read": model["cost"]["cacheRead"].as_f64().unwrap_or(input) }),
                );
            }
        }
        let changed = catalog
            .and_then(|path| path.metadata().ok()?.modified().ok())
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |elapsed| elapsed.as_secs());
        source = format!("Pi model catalog, changed at {changed}; {source}");
    }
    (map, source)
}

/// The amount for a call from its tokens, when the model has a list price. The model may be given
/// with or without its provider.
pub fn cost(model: &str, input: u64, cached: u64, output: u64) -> Option<f64> {
    if model.is_empty() {
        return None;
    }
    let (map, _) = prices();
    let price = map.get(model).or_else(|| {
        map.iter()
            .find(|(key, _)| key.split_once('/').is_some_and(|(_, id)| id == model))
            .map(|(_, price)| price)
    })?;
    let fresh = input.saturating_sub(cached) as f64;
    Some(
        (fresh * price["input"].as_f64()?
            + cached as f64 * price["cache_read"].as_f64()?
            + output as f64 * price["output"].as_f64()?)
            / 1e6,
    )
}

/// Writes the overview where the viewer reads it and returns its text.
pub fn publish_overview(state: &Path) -> Result<String> {
    let text = serde_json::to_string_pretty(&overview(state)?)?;
    let folder = directory(state);
    fs::create_dir_all(&folder)?;
    let next = folder.join(".overview.next");
    fs::write(&next, &text)?;
    fs::rename(next, folder.join("overview.json"))?;
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_agent_definition_is_read_strictly() {
        let agent = Agent::parse(
            "reviewer",
            "harness=pi\nmodel=anthropic/claude-opus-5-5\neffort=high\npersona=Du prüfst streng.\nend\n",
        )
        .unwrap();
        assert_eq!(agent.harness, Harness::Pi);
        assert_eq!(agent.model, "anthropic/claude-opus-5-5");
        assert_eq!(agent.effort, "high");
        assert_eq!(agent.prompt("CONTRACT"), "Du prüfst streng.\n\nCONTRACT");
        let plain = Agent::parse("a", "harness=savant\nend\n").unwrap();
        assert_eq!((plain.model.as_str(), plain.effort.as_str()), ("", ""));
        assert_eq!(plain.prompt("CONTRACT"), "CONTRACT");
        for broken in [
            "harness=pi\n",
            "harness=gemini\nend\n",
            "harness=pi\nmodel=a b\nend\n",
            "harness=pi\nmodel=x;rm\nend\n",
            "harness=pi\neffort=extreme\nend\n",
            "model=x\nend\n",
        ] {
            assert!(Agent::parse("a", broken).is_err(), "{broken}");
        }
        assert!(
            valid_name("code-reviewer-2")
                && !valid_name("Bad")
                && !valid_name("../x")
                && !valid_name("")
        );
    }
}
