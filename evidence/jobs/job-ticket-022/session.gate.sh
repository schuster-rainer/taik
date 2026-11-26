#!/bin/sh
# Gate: taik-seed decline
fail() { echo "FAIL: $1"; exit 1; }

[ -f tests/decline.rs ] || fail "tests/decline.rs fehlt"

out=$(cargo build --offline --locked 2>&1) || { echo "$out" | tail -40; fail "cargo build schlaegt fehl"; }

out=$(cargo clippy --offline --locked --all-targets -- -D warnings 2>&1) || { echo "$out" | tail -60; fail "clippy hat Befunde"; }

grep -q '^taik-seed decline STATE NAME$' README.md || fail "README enthaelt die Zeile 'taik-seed decline STATE NAME' nicht"

out=$(cargo test --offline --locked --test decline 2>&1) || { echo "$out" | tail -60; fail "cargo test --test decline schlaegt fehl"; }
if [ "$(uname)" = "Darwin" ]; then
  n=$(echo "$out" | sed -n 's/^test result: ok\. \([0-9][0-9]*\) passed.*/\1/p' | head -1)
  [ -n "$n" ] && [ "$n" -ge 4 ] || { echo "$out" | tail -30; fail "tests/decline.rs fuehrt weniger als 4 Tests aus"; }
fi

if [ "$(uname)" != "Darwin" ]; then
  echo "PASS: Build, clippy, README und Tests ok (Verhaltenspruefung nur auf macOS)"
  exit 0
fi

bin="$PWD/target/debug/taik-seed"
[ -x "$bin" ] || fail "Binary $bin fehlt"
root=$(mktemp -d /tmp/taik-gate-XXXXXX) || fail "mktemp"
s="$root/state"
mkdir -p "$s/jobs" "$s/proposals" "$s/requests"

# Ausfuehren mit Eingabe; Status in $rc
run() { # input args...
  in="$1"; shift
  printf '%s' "$in" | "$bin" "$@" >"$root/o" 2>"$root/e"; rc=$?
}
snap() { (cd "$s" && find . | LC_ALL=C sort); }
expect2() { # label input args...
  label="$1"; before=$(snap); shift
  run "$@"
  [ "$rc" -eq 2 ] || { cat "$root/e"; fail "$label: Status $rc statt 2"; }
  [ "$before" = "$(snap)" ] || fail "$label: hat etwas geschrieben"
}

# 1 Vorschlag ablehnen
mkdir -p "$s/proposals/vor-a" "$s/proposals/vor-b" "$s/proposals/vor-c"
for p in vor-a vor-b vor-c; do echo "Titel" > "$s/proposals/$p/brief.md"; done
echo "job=x" > "$s/proposals/vor-c/approved.txt"
run "Zu vage." decline "$s" vor-a
[ "$rc" -eq 0 ] || { cat "$root/e"; fail "decline eines Vorschlags: Status $rc"; }
[ "$(cat "$s/proposals/vor-a/declined.md")" = "Zu vage." ] || fail "declined.md hat falschen Inhalt"
[ ! -e "$s/proposals/vor-a/approved.txt" ] || fail "approved.txt entstand"

# 5 Abweisungen
expect2 "zweites Ablehnen" "Nochmal." decline "$s" vor-a
[ "$(cat "$s/proposals/vor-a/declined.md")" = "Zu vage." ] || fail "declined.md wurde ueberschrieben"
expect2 "freigegebener Vorschlag" "Nein." decline "$s" vor-c
expect2 "unbekannter Name" "Nein." decline "$s" gibt-es-nicht
expect2 "ungueltiger Name" "Nein." decline "$s" "Ung%ltig"
expect2 "Grossbuchstaben" "Nein." decline "$s" VOR-B
expect2 "leere Begruendung" "" decline "$s" vor-b
long=$(head -c 2001 /dev/zero | tr '\0' 'a')
expect2 "zu lange Begruendung" "$long" decline "$s" vor-b
ok2000=$(head -c 2000 /dev/zero | tr '\0' 'b')
run "$ok2000" decline "$s" vor-b
[ "$rc" -eq 0 ] || { cat "$root/e"; fail "2000 Bytes muessen angenommen werden: Status $rc"; }

# 3 approve nach decline
expect2 "approve nach decline" "" approve "$s" vor-a
[ -z "$(ls "$s/jobs")" ] || fail "approve nach decline legte einen Auftrag an"

# 2 Wunsch zurueckziehen
for t in ticket-001 ticket-002 ticket-003; do
  printf 'decision=observed\nidentity=%s\nintent=X\nobservation=Y\ncontinuation=Z\nend\n' "$t" > "$s/requests/$t.run"
done
for t in ticket-001 ticket-002; do
  mkdir -p "$s/requests/$t.talk"
  printf 'round=0\n\nWelche Farbe?\n' > "$s/requests/$t.talk/message-001.agent.md"
done
run "Brauche ich nicht mehr." decline "$s" ticket-001
[ "$rc" -eq 0 ] || { cat "$root/e"; fail "Wunsch zurueckziehen: Status $rc"; }
[ "$(cat "$s/requests/ticket-001.withdrawn")" = "Brauche ich nicht mehr." ] || fail ".withdrawn hat falschen Inhalt"
expect2 "zweites Zurueckziehen" "Nochmal." decline "$s" ticket-001

# 4 reply auf zurueckgezogenen Wunsch; Kontrolle: auf offenen Wunsch geht es
expect2 "reply auf zurueckgezogenen Wunsch" "Blau." reply "$s" ticket-001
run "Blau." reply "$s" ticket-002
[ "$rc" -eq 0 ] || { cat "$root/e"; fail "Kontrolle: reply auf offenen Wunsch Status $rc"; }

rm -rf "$root"
echo "PASS: decline, approve-Sperre, withdrawn, reply-Sperre, Tests und clippy ok"
