#!/bin/sh
# tools/factory/land.sh NAME BRIEF.md "COMMIT SUBJECT": brings the accepted work of a session job on
# this repository into the core: merge, format, promotion by the installed binary, commit.
# A rejected promotion is told to the agent in its ticket with the end of the check log. The agent
# corrects its work in the next round and the landing is tried again, at most three times.
set -u
cd "$(dirname "$0")/../.." || exit 1
state="$HOME/.local/state/taik-factory"; name=$1; brief=$2; subject=$3
copy="$state/work/$name"; job="$state/jobs/job-$name"
attempt=1
while [ "$attempt" -le 3 ]; do
  # A rejected attempt leaves formatting changes behind. They are redone below.
  git reset -q; git checkout -q -- src tests
  git fetch -q "$copy" HEAD && git merge -q --no-edit FETCH_HEAD || { echo "MERGE FAILED"; git merge --abort 2>/dev/null; exit 1; }
  cargo fmt
  "$state/taik-seed" promote evidence/jobs/job-backend "$brief" tools/backend/gate.sh "$state/taik-seed" "opus-session-$name" >/dev/null 2>&1
  record=$(ls evidence/jobs/job-backend/round-[0-9][0-9].run | tail -1)
  outcome=$(grep -E '^(decision|checks)=' "$record" | cut -c1-240)
  echo "attempt $attempt: $(basename "$record")"; echo "$outcome"
  case "$outcome" in decision=accepted*) break;; esac
  [ "$attempt" -eq 3 ] && { echo "NOT PROMOTED after three attempts: left uncommitted"; exit 1; }
  log="${record%.run}.checks.log"
  finding=$(grep -E 'error(\[|:)|panicked at|^test .* FAILED|assertion|left:|right:|^  -->' "$log" 2>/dev/null | head -30 | cut -c1-300)
  rounds_before=$(ls "$job"/round-[0-9][0-9].run | wc -l | tr -d ' ')
  printf 'Die Übernahme deines angenommenen Stands in den Kern ist gescheitert. Dort laufen `cargo clippy --offline --locked --all-targets -- -D warnings` und die ganze Testsuite, auch die Tests, die das Gate deines Auftrags nicht ausführen kann. Befund: %s\n\nAus dem Protokoll:\n%s\n\nBitte behebe die Ursache. Wenn ein bestehender Test dem neuen Verhalten widerspricht, passe ihn an und nenne ihn in deiner Zusammenfassung. Ändere sonst nichts.' \
    "$(grep '^checks=' "$record" | cut -c8-300)" "$finding" | "$state/taik-seed" reply "$state" "job-$name" >/dev/null || exit 1
  i=0
  while [ $i -lt 240 ]; do
    now=$(ls "$job"/round-[0-9][0-9].run | wc -l | tr -d ' ')
    [ "$now" -gt "$rounds_before" ] && break
    msg=$(ls "$job"/message-*.md | tail -1); case "$msg" in *agent.md) echo "THE AGENT ASKS: $(tail -n +3 "$msg")"; exit 1;; esac
    sleep 3; i=$((i+1))
  done
  grep -q '^decision=accepted' "$(ls "$job"/round-[0-9][0-9].run | tail -1)" || { echo "THE NEXT ROUND WAS NOT ACCEPTED"; exit 1; }
  attempt=$((attempt + 1))
done
launchctl kickstart -k "gui/$(id -u)/local.taik.factory"
git add -A src tests README.md AGENTS.md tools evidence 2>/dev/null; git reset -q evidence/issues/.open.json 2>/dev/null
git commit -q -m "$subject

Work by an Opus session in job-$name, planned by the orchestrator. Promoted
in job-backend after $attempt attempt(s).

Co-Authored-By: Claude Fable 5.2 <noreply@anthropic.com>" && git push -q origin main; git log --oneline -1
