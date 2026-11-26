#!/bin/sh
# tools/factory/land.sh NAME BRIEF.md "COMMIT SUBJECT": merges the branch of an accepted session job, checks, promotes, commits.
set -u
cd "$(dirname "$0")/../.." || exit 1
state="$HOME/.local/state/taik-factory"; name=$1; brief=$2; subject=$3
copy="$state/work/$name"
# A rejected attempt leaves formatting changes behind. They are redone below.
git reset -q; git checkout -q -- src tests
git fetch -q "$copy" HEAD && git merge -q --no-edit FETCH_HEAD || { echo "MERGE FAILED"; git merge --abort 2>/dev/null; exit 1; }
cargo fmt
lint=$(cargo clippy --offline --locked --all-targets -- -D warnings 2>&1 | grep -E '^(error|warning)' -A6 | head -14)
[ -n "$lint" ] && { echo "CLIPPY:"; echo "$lint"; }
outcome=$("$state/taik-seed" promote evidence/jobs/job-backend "$brief" tools/backend/gate.sh "$state/taik-seed" "opus-session-$name" 2>&1 | grep -E '^(decision|checks)=' | cut -c1-240)
echo "$outcome"
launchctl kickstart -k "gui/$(id -u)/local.taik.factory"
git add -A src tests README.md AGENTS.md tools evidence 2>/dev/null; git reset -q evidence/issues/.open.json 2>/dev/null
case "$outcome" in decision=accepted*) git commit -q -m "$subject

Work by an Opus session in job-$name, planned by the orchestrator. Promoted
in job-backend.

Co-Authored-By: Claude Fable 5.2 <noreply@anthropic.com>" && git push -q origin main; git log --oneline -1;; *) echo "NOT PROMOTED: left uncommitted";; esac
