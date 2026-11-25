# TAIK Factory

Read [TAIK-FACTORY-SEED-v0.4.md](TAIK-FACTORY-SEED-v0.4.md) in full before any work. It is the frozen core specification and takes precedence over this file. Do not edit it. When you have evidence that a change of the core is necessary for self-improvement, propose it. 

This file holds what the work needs and the core specification leaves open.

## Standing mandate

- The Factory improves itself. Research here means analysing this system and applying the same process to it: its backend, its viewer, its gates and its own procedures.
- When the Factory stops because an observation is missing, obtain that observation. Send agents if needed. They work through the same pattern of brief, pinned gate, round and record.
- Each completed round should leave the next one cheaper or better informed. Say so only where a measurement shows it.
- The backend is Rust. Speed claims need a measurement against the installed binary.
- The viewer is part of the loop. A design change starts from a capture of the current state, goes through a source job and is installed from the accepted bytes. The right-hand panes show the work while it runs.
- Keep the job history current and open in the viewer.
- Ask the user a question when the answer would change the direction. Each answer should remove a future question.

## How work is done here

Changes go through the Factory, including changes to the Factory itself.

1. State the intent in a brief. Its first line names the round.
2. Write the independent gate before the candidate exists. The gate is pinned for the life of the job. Never change a gate or a test to make a candidate pass (S-09).
3. Submit the candidate as a round. The Factory runs the gate in isolation and writes the decision to a `.run` record.
4. Install only the bytes the gate accepted. Keep the previous version in the job folder as the rollback.
5. Derive the next job from an observation or a measurement. If there is none, stop (S-08, section 8).

A rejected candidate and a negative result are evidence. Record them.

## Commands

```sh
cargo test --offline --locked
cargo clippy --offline --locked --all-targets -- -D warnings

# One attached source round. WORKER names whoever supplies the source on stdin.
taik-seed source evidence/jobs/job-NAME BRIEF.md GATE.mjs --attached=WORKER < candidate.html

# Queue a job for the running service.
state="$HOME/.local/state/taik-factory"
"$state/taik-seed" queue-source "$state" NAME BRIEF.md GATE.mjs
```

`README.md` describes the remaining commands. `evidence/README.md` indexes the jobs.

## Rules that are easy to break

- Attribute work to its real author. A record names the worker that supplied the source. Do not describe an agent's change as learning by the seed (section 7.2).
- A check that did not run is reported as not run (S-04, T-06).
- The service runs under launchd as `local.taik.factory`. Its binary changes only through `taik-seed promote` in `evidence/jobs/job-backend`, run by the installed binary with `tools/backend/gate.sh`. Never copy a binary into place by hand.
- The viewer is `tools/factory-view.html`. It changes only through a source job and is installed with `tools/open-factory-view.sh`. Run `swift tools/check-factory-design.swift --require-open` on a snapshot before installing.
- Git records the checkout, `origin` is the public repository `taik` on GitHub. Commit after each accepted round. Never commit secrets, tokens or anything from outside this folder. Rollbacks of installed artifacts still live in the job folders.
- Records in `evidence/jobs/` are create-only. Do not rewrite a published `.run`.
