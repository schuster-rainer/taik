#!/bin/sh
# Gate of the Mac app, second part: what the app does when the viewer sends a message.
#   tools/check-app-handlers.sh PATH_TO_factory-view
# Each button of the viewer ends in one of these messages. The test sends them to the freshly built
# binary with `--handle`, against a throwaway state folder, and looks at what was written.
set -u
bin=${1:?usage: tools/check-app-handlers.sh PATH_TO_factory-view}
seed="$HOME/.local/state/taik-factory/taik-seed"
root=$(mktemp -d /tmp/taik-app-XXXXXX) || exit 1
trap 'rm -rf "$root"' EXIT
state="$root/state"
mkdir -p "$state/jobs" "$state/proposals/idee" "$state/proposals/weg" "$root/repo" "$root/keinrepo"
ln -s "$seed" "$state/taik-seed"
printf 'Eine Idee\n\nMehr.\n' > "$state/proposals/idee/brief.md"; printf "console.log('PASS');\n" > "$state/proposals/idee/verify.mjs"
cp "$state/proposals/idee/brief.md" "$state/proposals/idee/verify.mjs" "$state/proposals/weg/"
git -C "$root/repo" init -q
fail() { echo "FAIL: $*"; exit 1; }
send() { "$bin" --state "$state" --handle "$1" "$2" >/dev/null 2>&1; }

send newTicket '{"text":"Ein Wunsch aus dem Test\n\nMit einem zweiten Absatz."}'
grep -q '^intent=Ein Wunsch aus dem Test$' "$state/requests/ticket-001.run" 2>/dev/null || fail "newTicket did not record the request"
grep -q 'Mit einem zweiten Absatz' "$state/requests/ticket-001.run" || fail "newTicket lost the text of the request"

send commentTicket '{"name":"idee","text":"Bitte in Blau."}'
grep -q 'Bitte in Blau' "$state/proposals/idee/message-001.user.md" 2>/dev/null || fail "a comment on a proposal was not kept with it"

send declineTicket '{"name":"weg","text":"Brauche ich nicht."}'
grep -q 'Brauche ich nicht' "$state/proposals/weg/declined.md" 2>/dev/null || fail "declineTicket did not record the reason"
send approveProposal '"weg"'
[ ! -e "$state/jobs/job-weg" ] || fail "a declined proposal could still be approved"

send approveProposal '"idee"'
[ -f "$state/jobs/job-idee/submission.brief.md" ] || fail "approveProposal did not queue the job"
grep -q 'Bitte in Blau' "$state/jobs/job-idee/message-001.user.md" 2>/dev/null || fail "the comment did not move into the job"

send commentTicket '{"name":"job-idee","text":"Eine Antwort."}'
grep -q 'Eine Antwort' "$state/jobs/job-idee/message-002.user.md" 2>/dev/null || fail "a reply to a job was not written into it"

send closeTicket '{"job":"job-idee","text":"Erledigt sich."}'
grep -q 'Erledigt sich' "$state/jobs/job-idee/closed.md" 2>/dev/null || fail "closeTicket did not record the reason"

send setParallel '5'
[ "$(cat "$state/parallel" 2>/dev/null)" = 5 ] || fail "setParallel did not store the number"
send setParallel '99'
[ "$(cat "$state/parallel")" = 5 ] || fail "setParallel took a number outside 1 to 8"

send addRepo "{\"name\":\"demo\",\"path\":\"$root/repo\"}"
[ "$(cat "$state/repos/demo.repo" 2>/dev/null)" = "$(cd "$root/repo" && pwd -P)" ] || [ "$(cat "$state/repos/demo.repo" 2>/dev/null)" = "$root/repo" ] || fail "addRepo did not record the repository"
send addRepo "{\"name\":\"nichts\",\"path\":\"$root/keinrepo\"}"
[ ! -e "$state/repos/nichts.repo" ] || fail "addRepo took a folder that is no Git repository"

# Values that try to leave their place are ignored.
before=$(find "$root" | sort | shasum | cut -d' ' -f1)
send commentTicket '{"name":"../idee","text":"x"}'
send commentTicket '{"name":"job-../../x","text":"x"}'
send closeTicket '{"job":"job-../idee","text":"x"}'
send declineTicket '{"name":"IDEE/..","text":"x"}'
send addRepo "{\"name\":\"../x\",\"path\":\"$root/repo\"}"
send addRepo '{"name":"rel","path":"repo"}'
send approveProposal '"../weg"'
send landJob '"job-../x"'
send newTicket '{"text":"   "}'
after=$(find "$root" | sort | shasum | cut -d' ' -f1)
[ "$before" = "$after" ] || fail "a malformed message changed the state"

echo "PASS: 9 messages of the viewer handled, 9 malformed ones ignored"
