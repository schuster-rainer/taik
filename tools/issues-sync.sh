#!/bin/sh
# Intake from GitHub issues and report back. Files stay the record, issues are the front door.
#
#   tools/issues-sync.sh            show what would happen
#   tools/issues-sync.sh apply      do it
#
# Only open issues with the label "taik" that the repository owner wrote are read. An issue becomes
# a proposal that waits. The label "freigegeben" on the issue approves it. The decision of the job is
# posted back and an accepted job closes the issue. Nobody else's text reaches a worker.
set -eu
mode=${1:-dry-run}
cd "$(dirname "$0")/.."
state=${TAIK_STATE:-$HOME/.local/state/taik-factory}
seed="$state/taik-seed"
plist="$HOME/Library/LaunchAgents/local.taik.factory.plist"
setting() { /usr/libexec/PlistBuddy -c "Print :EnvironmentVariables:$1" "$plist" 2>/dev/null || true; }
export TAIK_LEAD_WORKER="${TAIK_LEAD_WORKER:-$(setting TAIK_LEAD_WORKER)}"
export TAIK_SUBQ_KEYCHAIN="${TAIK_SUBQ_KEYCHAIN:-$(setting TAIK_SUBQ_KEYCHAIN)}"
repo=$(gh repo view --json nameWithOwner -q .nameWithOwner)
owner=${repo%%/*}
inbox=evidence/issues
mkdir -p "$inbox"
say() { printf '%s\n' "$*"; }
act() { if [ "$mode" = apply ]; then "$@"; else say "  would run: $*"; fi; }
line() { tr '\r\n\t' '   ' | sed 's/  */ /g' | cut -c1-"$1"; }

gh issue list --repo "$repo" --state open --label taik --author "$owner" --limit 20 \
  --json number,title,body,labels > "$inbox/.open.json"
count=$(jq length "$inbox/.open.json")
say "$count open issue(s) by $owner with the label taik in $repo"
index=0
while [ "$index" -lt "$count" ]; do
  number=$(jq -r ".[$index].number" "$inbox/.open.json")
  title=$(jq -r ".[$index].title" "$inbox/.open.json" | line 300)
  approved=$(jq -r ".[$index].labels | map(.name) | index(\"freigegeben\") != null" "$inbox/.open.json")
  index=$((index + 1))
  name="issue-$number"
  request="$inbox/$number.run"
  proposal="$state/proposals/$name"
  job="evidence/jobs/job-$name"
  say "#$number $title"
  if [ ! -e "$request" ]; then
    say "  new request"
    if [ "$mode" = apply ]; then
      body=$(jq -r ".[$((index - 1))].body // \"\"" "$inbox/.open.json" | line 3000)
      printf 'decision=observed\nidentity=%s\nintent=%s\nobservation=%s\ncontinuation=Derive one job that fulfils this request of the repository owner. Stop if the request is unclear or cannot be checked by a gate.\nend\n' \
        "$name" "$title" "$body" > "$request"
      outcome=$("$seed" propose "$state" "$name" "$request")
      if [ "$outcome" = stop ]; then
        reason=$(grep '^outcome=' "$inbox/reflection-$name.run" | cut -c9-)
        gh issue comment "$number" --repo "$repo" --body "Die Factory hat daraus keinen Auftrag abgeleitet: $reason"
      else
        intent=$(head -1 "$proposal/brief.md")
        gh issue comment "$number" --repo "$repo" --body "Die Factory schlägt diesen Auftrag vor: **$intent**

Auftrag und Gate warten auf Freigabe. Das Label \`freigegeben\` an diesem Issue gibt sie frei."
      fi
    else
      say "  would write $request, let the Factory propose a job and comment on the issue"
    fi
  elif [ -d "$proposal" ] && [ ! -e "$proposal/approved.txt" ]; then
    if [ "$approved" = true ]; then
      say "  approved by label"
      act "$seed" approve "$state" "$name"
      act gh issue comment "$number" --repo "$repo" --body "Freigegeben. Der Auftrag ist eingereiht."
    else
      say "  proposal waits for the label freigegeben"
    fi
  elif [ -d "$job" ] && [ ! -e "$inbox/$number.reported" ]; then
    last=$(ls "$job"/round-*.run 2>/dev/null | tail -1)
    if [ -n "$last" ] && grep -q '^advance=false' "$last"; then
      decision=$(grep '^decision=' "$last" | cut -d= -f2)
      checks=$(grep '^checks=' "$last" | cut -c8- | line 400)
      cost=$(cat "$job"/*.run | awk -F= '$1 == "worker_cost_usd" { sum += $2 } END { printf "%.4f", sum }')
      rounds=$(ls "$job"/round-*.run | wc -l | tr -d ' ')
      say "  finished: $decision after $rounds round(s), $cost USD"
      if [ "$mode" = apply ]; then
        gh issue comment "$number" --repo "$repo" --body "Ergebnis: **$decision** nach $rounds Runde(n), Worker-Kosten $cost USD zum Listenpreis.

Prüfung: $checks

Protokoll: \`$last\`"
        [ "$decision" = accepted ] && gh issue close "$number" --repo "$repo" >/dev/null
        date > "$inbox/$number.reported"
      fi
    else
      say "  job is running"
    fi
  else
    say "  nothing to do"
  fi
done
rm -f "$inbox/.open.json"
