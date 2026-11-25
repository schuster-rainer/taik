#!/bin/sh
# Releases the latest accepted viewer of a job: WebKit check on the accepted bytes with live data,
# install of exactly those bytes, check of the installed app, and a record beside the round.
#
#   tools/release-view.sh evidence/jobs/job-NAME
#
# Nothing is installed unless the check passes. The previous viewer is kept in the job folder.
set -eu
job=${1:?usage: tools/release-view.sh JOB_FOLDER}
cd "$(dirname "$0")/.."
app="$HOME/Applications/TAIK Factory.app/Contents"
round=$(grep -l '^decision=accepted' "$job"/round-*.run | tail -1)
[ -n "$round" ] || { echo "no accepted round in $job" >&2; exit 2; }
stem=$(basename "$round" .run)
artifact="$job/$stem.html"
record="$job/release-$stem.run"
[ ! -e "$record" ] || { echo "$record exists: this round was already released" >&2; exit 2; }
grep -q "^artifact=$stem.html$" "$round" || { echo "the record names another artifact" >&2; exit 2; }
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
digest=$(shasum -a 256 "$artifact" | cut -d' ' -f1)
finish() {
  printf 'decision=%s\nkind=release\nidentity=release-%s\nartifact=%s.html\nartifact_sha256=%s\nreleased_by=tools/release-view.sh\nchecks=%s\nrollback=%s\nend\n' \
    "$1" "$stem" "$stem" "$digest" "$2" "$3" > "$record"
  cat "$record"
}
# Live data of the running Factory, shown by the candidate instead of the installed viewer.
"$app/MacOS/factory-view" --snapshot > "$work/installed.html"
python3 - "$work" "$artifact" <<'PY'
import sys
work, artifact = sys.argv[1:3]
old = open('tools/factory-view.html').read()
snap = open(work + '/installed.html').read().rstrip('\n')
pre, post = old.split('/*FACTORY_DATA*/null')
post = post.rstrip('\n')
assert snap.startswith(pre) and snap.endswith(post), 'the installed viewer differs from tools/factory-view.html'
data = snap[len(pre):len(snap) - len(post)]
open(work + '/candidate.html', 'w').write(open(artifact).read().replace('/*FACTORY_DATA*/null', data))
PY
before=$(swift tools/check-factory-design.swift --require-open < "$work/candidate.html" 2>&1 | tail -1)
case "$before" in
  PASS*) ;;
  *) finish rejected "candidate: $before" ""; exit 1 ;;
esac
cp tools/factory-view.html "$job/rollback.$stem.html"
cp "$artifact" tools/factory-view.html
pkill -x factory-view 2>/dev/null || true
sleep 1
./tools/open-factory-view.sh
sleep 4
cmp -s "$app/Resources/factory-view.html" "$artifact" || { finish rejected "the installed bytes differ from the accepted artifact" "rollback.$stem.html"; exit 1; }
"$app/MacOS/factory-view" --snapshot > "$work/after.html"
after=$(swift tools/check-factory-design.swift --require-open < "$work/after.html" 2>&1 | tail -1)
case "$after" in
  PASS*) finish accepted "candidate: $before installed: $after" "rollback.$stem.html" ;;
  *) cp "$job/rollback.$stem.html" tools/factory-view.html
     pkill -x factory-view 2>/dev/null || true; sleep 1; ./tools/open-factory-view.sh
     finish rejected "installed: $after. The previous viewer was restored." "rollback.$stem.html"; exit 1 ;;
esac
