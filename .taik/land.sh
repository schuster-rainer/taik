#!/bin/sh
# .taik/land.sh NAME [BRIEF.md] [COMMIT SUBJECT]
# How this repository takes the accepted work of a session job into itself: merge the job's commit,
# format, let the installed binary judge and install the result, commit and push.
# The Factory app runs it when the user presses "Übernehmen" on the ticket; tools/factory/land.sh runs
# it from a terminal. It writes landed.txt or land-failed.txt into the job folder.
# A rejected promotion is told to the agent in its ticket; it corrects its work in the next round and
# the landing is tried again, at most three times. Sessions cannot change this file (.taik/protected).
set -u
cd "$(dirname "$0")/.." || exit 1
state="$HOME/.local/state/taik-factory"; name=${1:?usage: .taik/land.sh NAME [BRIEF.md] [SUBJECT]}
copy="$state/work/$name"; job="$state/jobs/job-$name"
fail() { printf '%s\n' "$*" > "$job/land-failed.txt"; echo "LAND FAILED: $*"; rmdir "$lock" 2>/dev/null; exit 1; }
lock="$state/landing.lock"
mkdir "$lock" 2>/dev/null || { echo "another landing is running"; exit 1; }
[ -d "$job" ] || { rmdir "$lock"; echo "no such job: $job"; exit 1; }
[ -e "$job/landed.txt" ] && { rmdir "$lock"; echo "already landed: $(cat "$job/landed.txt")"; exit 0; }
git diff --quiet HEAD -- src tests tools Cargo.toml Cargo.lock 2>/dev/null || fail "Der Checkout hat ungesicherte Änderungen an src, tests oder tools."
last=$(ls "$job"/round-[0-9][0-9].run 2>/dev/null | tail -1)
[ -n "$last" ] && grep -q '^decision=accepted' "$last" || fail "Die letzte Runde des Auftrags ist nicht angenommen."
intent=$(head -1 "$job/session.brief.md" 2>/dev/null)
[ -n "$intent" ] || fail "Der Auftrag ist kein Sitzungsauftrag."
brief=${2:-}
if [ -z "$brief" ]; then
  brief="tools/backend/brief-$name.md"
  plan=$(grep -E '^(gate|work)=' "$job/plan.txt" 2>/dev/null | tr '\n' ' ')
  printf '%s\n\nVon der Factory hervorgebracht. Auftrag `job-%s`, angenommen in %s. Plan des Orchestrators: %s\n\nDer Kandidat dieser Runde ist der Merge des angenommenen Commits aus der Kopie des Auftrags. Von der Übernahme stammt nur `cargo fmt`. Ausgelöst durch `.taik/land.sh`.\n\nPrüfung: `tools/backend/gate.sh`.\n' \
    "$intent" "$name" "$(basename "$last" .run)" "${plan:-ohne Plan}" > "$brief"
fi
subject=${3:-$(printf '%s' "$intent" | cut -c1-72)}
attempt=1
while [ "$attempt" -le 3 ]; do
  # A rejected attempt leaves formatting changes behind. They are redone below.
  git reset -q; git checkout -q -- src tests
  git fetch -q "$copy" HEAD && git merge -q --no-edit FETCH_HEAD || { git merge --abort 2>/dev/null; fail "Der Commit des Auftrags lässt sich nicht mit dem Checkout zusammenführen."; }
  cargo fmt
  "$state/taik-seed" promote evidence/jobs/job-backend "$brief" tools/backend/gate.sh "$state/taik-seed" "opus-session-$name" >/dev/null 2>&1
  record=$(ls evidence/jobs/job-backend/round-[0-9][0-9].run | tail -1)
  outcome=$(grep -E '^(decision|checks)=' "$record" | cut -c1-240)
  echo "attempt $attempt: $(basename "$record")"; echo "$outcome"
  case "$outcome" in decision=accepted*) break;; esac
  [ "$attempt" -eq 3 ] && fail "Die Annahme im Kern ist dreimal gescheitert: $(grep '^checks=' "$record" | cut -c8-200)"
  log="${record%.run}.checks.log"
  finding=$(grep -E 'error(\[|:)|panicked at|^test .* FAILED|assertion|left:|right:|^  -->' "$log" 2>/dev/null | head -30 | cut -c1-300)
  rounds_before=$(ls "$job"/round-[0-9][0-9].run | wc -l | tr -d ' ')
  printf 'Die Übernahme deines angenommenen Stands in den Kern ist gescheitert. Dort laufen `cargo clippy --offline --locked --all-targets -- -D warnings` und die ganze Testsuite, auch die Tests, die das Gate deines Auftrags nicht ausführen kann. Befund: %s\n\nAus dem Protokoll:\n%s\n\nBitte behebe die Ursache. Wenn ein bestehender Test dem neuen Verhalten widerspricht, passe ihn an und nenne ihn in deiner Zusammenfassung. Ändere sonst nichts.' \
    "$(grep '^checks=' "$record" | cut -c8-300)" "$finding" | "$state/taik-seed" reply "$state" "job-$name" >/dev/null || fail "Die Rückmeldung an den Agenten ließ sich nicht ablegen."
  i=0
  while [ $i -lt 400 ]; do
    now=$(ls "$job"/round-[0-9][0-9].run | wc -l | tr -d ' ')
    [ "$now" -gt "$rounds_before" ] && break
    msg=$(ls "$job"/message-*.md | tail -1); case "$msg" in *agent.md) fail "Der Agent fragt zurück: $(tail -n +3 "$msg" | cut -c1-300)";; esac
    sleep 3; i=$((i+1))
  done
  grep -q '^decision=accepted' "$(ls "$job"/round-[0-9][0-9].run | tail -1)" || fail "Die Korrekturrunde des Agenten wurde nicht angenommen."
  attempt=$((attempt + 1))
done
git add -A src tests README.md AGENTS.md tools evidence 2>/dev/null; git reset -q evidence/issues/.open.json 2>/dev/null
git commit -q -m "$subject

Work by an agent session in job-$name, planned by the orchestrator. Promoted
in job-backend after $attempt attempt(s), landed by .taik/land.sh.

Co-Authored-By: Claude Fable 5.2 <noreply@anthropic.com>" || fail "Der Commit ist gescheitert."
commit=$(git rev-parse HEAD)
git push -q origin main 2>/dev/null || echo "push failed: the commit stays local"
rm -f "$job/land-failed.txt"
printf '%s\n' "$commit" > "$job/landed.txt"
git add "evidence/jobs/job-$name/landed.txt" 2>/dev/null; git commit -q -m "Record the landing of job-$name" 2>/dev/null; git push -q origin main 2>/dev/null
rmdir "$lock" 2>/dev/null
launchctl kickstart -k "gui/$(id -u)/local.taik.factory"
git log --oneline -1
