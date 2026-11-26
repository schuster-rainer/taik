#!/bin/sh
fail() { echo "FAIL: $1"; exit 1; }

test -f tests/request_talk.rs || fail "tests/request_talk.rs fehlt"

out=$(cargo build --offline --locked 2>&1) || { echo "$out" | tail -40; fail "cargo build schlägt fehl"; }
bin=$(pwd)/target/debug/taik-seed
test -x "$bin" || fail "target/debug/taik-seed fehlt nach dem Build"

root=$(mktemp -d /tmp/taik-gate-XXXXXX) || fail "mktemp"
trap 'rm -rf "$root"' EXIT

mkreq() {
  # $1 = state, $2 = name
  mkdir -p "$1/requests" "$1/jobs"
  printf 'decision=observed\nidentity=%s\nintent=Test\nobservation=Etwas Unklares\ncontinuation=Derive one job.\nend\n' "$2" > "$1/requests/$2.run"
}
mktalk() {
  mkdir -p "$1/requests/$2.talk"
  printf 'round=0\n\nWelche Farbe soll es sein?\n' > "$1/requests/$2.talk/message-001.agent.md"
}

# A: Antwort auf eine Frage
s="$root/a"; mkreq "$s" ticket-019; mktalk "$s" ticket-019
res=$(printf 'Blau, bitte.' | "$bin" reply "$s" ticket-019 2>&1); code=$?
[ $code -eq 0 ] || fail "reply auf eine offene Frage endet mit Status $code: $res"
f="$s/requests/ticket-019.talk/message-002.user.md"
test -f "$f" || fail "message-002.user.md wurde nicht angelegt: $res"
grep -q 'Blau, bitte.' "$f" || fail "Antwort steht nicht in message-002.user.md"
test -f "$s/requests/ticket-019.talk/message-001.agent.md" || fail "Frage des Agenten fehlt"

# B: zweite Antwort, letzte Nachricht stammt vom Nutzer
res=$(printf 'Noch etwas.' | "$bin" reply "$s" ticket-019 2>&1); code=$?
[ $code -ne 0 ] || fail "reply ohne offene Frage endet mit Status 0"
test ! -e "$s/requests/ticket-019.talk/message-003.user.md" || fail "reply schrieb trotz fehlender Frage eine Nachricht"

# C: Wunsch ohne Gespräch und unbekannter Wunsch
s="$root/c"; mkreq "$s" ticket-020
res=$(printf 'x' | "$bin" reply "$s" ticket-020 2>&1); code=$?
[ $code -ne 0 ] || fail "reply ohne Gespräch endet mit Status 0"
test ! -e "$s/requests/ticket-020.talk" || fail "reply legte ein Gespräch an, obwohl keine Frage offen war"
res=$(printf 'x' | "$bin" reply "$s" ticket-077 2>&1); code=$?
[ $code -ne 0 ] || fail "reply auf einen unbekannten Wunsch endet mit Status 0"

# D: Vorschlag existiert: Kommentar geht in den Vorschlag, nicht ins Gespräch
s="$root/d"; mkreq "$s" ticket-021; mktalk "$s" ticket-021
mkdir -p "$s/proposals/ticket-021"; printf 'Brief\n' > "$s/proposals/ticket-021/brief.md"
res=$(printf 'Kommentar' | "$bin" reply "$s" ticket-021 2>&1); code=$?
[ $code -eq 0 ] || fail "reply mit Vorschlag endet mit Status $code: $res"
test -f "$s/proposals/ticket-021/message-001.user.md" || fail "Kommentar ging nicht in den Vorschlag"
test ! -e "$s/requests/ticket-021.talk/message-002.user.md" || fail "Kommentar ging trotz Vorschlag ins Gespräch des Wunsches"

# E: orchestrator-prompt zeigt das Gespräch
s="$root/e"; mkreq "$s" ticket-019; mktalk "$s" ticket-019
printf 'Blau, bitte.' | "$bin" reply "$s" ticket-019 >/dev/null 2>&1
p=$("$bin" orchestrator-prompt "$s" "$s/requests/ticket-019.run" 2>&1); code=$?
[ $code -eq 0 ] || { echo "$p" | tail -20; fail "orchestrator-prompt endet mit Status $code"; }
echo "$p" | grep -q 'Welche Farbe soll es sein?' || fail "orchestrator-prompt zeigt die Frage nicht"
echo "$p" | grep -q 'Blau, bitte.' || fail "orchestrator-prompt zeigt die Antwort nicht"
echo "$p" | grep -q 'CONVERSATION ABOUT THIS WORK ORDER' || fail "orchestrator-prompt enthält talk::prompt nicht"
s="$root/f"; mkreq "$s" ticket-019
p=$("$bin" orchestrator-prompt "$s" "$s/requests/ticket-019.run" 2>&1) || fail "orchestrator-prompt ohne Gespräch schlägt fehl: $p"
echo "$p" | grep -q 'CONVERSATION ABOUT THIS WORK ORDER' && fail "orchestrator-prompt zeigt ohne Gespräch einen Gesprächsteil"
echo "$p" | grep -q 'Etwas Unklares' || fail "orchestrator-prompt zeigt den Wunsch nicht mehr"

# Testziel des Auftrags
out=$(cargo test --offline --locked --test request_talk 2>&1); code=$?
[ $code -eq 0 ] || { echo "$out" | tail -40; fail "cargo test --test request_talk schlägt fehl"; }
echo "$out" | grep -Eq 'test result: ok\. [1-9][0-9]* passed' || { echo "$out" | tail -20; fail "request_talk führt keinen bestandenen Test aus"; }

out=$(cargo clippy --offline --locked --all-targets -- -D warnings 2>&1) || { echo "$out" | tail -40; fail "clippy meldet Befunde"; }

echo "PASS: reply und orchestrator-prompt verhalten sich wie gefordert, request_talk und clippy sind sauber"
