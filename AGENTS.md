# TAIK Factory

Read [TAIK-FACTORY-SEED-v0.4.md](TAIK-FACTORY-SEED-v0.4.md) in full before any work. It is the frozen specification and wins over this file. Do not edit it. Propose a change to it only with evidence from the running Factory.

The specification says what must hold. This file holds only what it cannot know: where things are in this checkout, how to run them, and what the user decided. It sets no rules of its own. If a line here reads like a rule, it is how the tools work today, and it may change through the Factory like anything else.

## Where things are

| What | Where |
|---|---|
| Core (Rust) | `src/`, tests in `tests/` |
| Viewer | `tools/factory-view.html`, shown by the Mac app `tools/factory-view.swift` |
| Records of all jobs | `evidence/jobs/`, indexed in `evidence/README.md` |
| State of the running service | `~/.local/state/taik-factory` (`jobs` links to `evidence/jobs`) |
| Service | launchd job `local.taik.factory`, working directory is this checkout |
| Commands and settings | `README.md` |

## How a change reaches the installed Factory

```sh
cargo test --offline --locked
cargo clippy --offline --locked --all-targets -- -D warnings

state="$HOME/.local/state/taik-factory"

# Core: the installed binary judges its successor and replaces itself.
"$state/taik-seed" promote evidence/jobs/job-backend BRIEF.md tools/backend/gate.sh "$state/taik-seed" WORKER
launchctl kickstart -k "gui/$(id -u)/local.taik.factory"

# Viewer: one round with the candidate on stdin, then the release of the accepted bytes.
"$state/taik-seed" source evidence/jobs/job-NAME BRIEF.md GATE.mjs --attached=WORKER < candidate.html
tools/release-view.sh evidence/jobs/job-NAME
```

Work the Factory does itself goes through its orchestrator:

```sh
tools/factory/wish.sh WISH.md      # send a request in plain words; prints the ticket and the orchestrator's plan
"$state/taik-seed" approve "$state" ticket-NNN
tools/factory/watch.sh ticket-NNN  # until the job is accepted, asks, or stops
printf 'answer' | "$state/taik-seed" reply "$state" job-ticket-NNN
tools/factory/land.sh ticket-NNN BRIEF.md "Commit subject"   # a session on this repository: merge, lint, promote, commit
```

A viewer job (product `ansicht`) is released by the service itself. A rejected promotion is told to the agent with `reply`; it corrects its work in the next round.

A brief names its intent in the first line and the observation that justifies it. `WORKER` names whoever supplied the candidate.

## Facts that are easy to get wrong

- The repository is public (`origin` is `taik` on GitHub). Nothing secret and nothing from outside this folder goes into a commit. The dialog of an agent session stays local (`*.dialog.jsonl` is ignored).
- A published record in `evidence/jobs/` cannot be rewritten; the core refuses it. A wrong record is answered by a new one.
- A gate is fixed for a job once its first round ran (`gate.sha256`). A different check needs a new job.
- The promotion of the core runs Clippy with warnings denied and the whole test suite. A gate for a session that changes `src/` should run `cargo clippy --offline --locked --all-targets -- -D warnings` too; one that did not let a change through that the promotion then refused (`job-ticket-013`, round 34 of `job-backend`).
- The gate of a session runs in a sandbox that cannot start another sandbox. It can test commands and its own test targets, not a round of the service.
- The Mac app is built and installed by `tools/open-factory-view.sh`. It installs only when the new host hands the viewer well-formed data (`tools/check-app.mjs`), the viewer passes its contract with it, and every message of the viewer does what it should against a throwaway state (`tools/check-app-handlers.sh`). Each attempt leaves a record in `evidence/app/`.
- Work done by an agent is attributed to that agent, not to the Factory (specification, section 7.2).

## Decisions of the user

- The Factory may propose jobs. A proposed job starts only after the user approved it.
- The standard worker of the service is Claude Code with Opus 5.5 at medium effort (2026-11-25).
- A ticket that waits never leaves the waiting group by age or by a limit. A job ends by a rejection on its merits or by the evolution.
- Limits in the code (sizes, durations, numbers of rounds) were set by agents. They are assumptions to justify or change, not rules of the user.
- Design: professional, slightly futuristic, minimal, neumorphic, no glow.
- Licence MIT.
