#!/bin/sh
fail() { echo "FEHLER: $1"; [ -n "$2" ] && echo "$2" | tail -n 30; exit 1; }

test -f Cargo.toml || fail "Cargo.toml fehlt"
test -f Cargo.lock || fail "Cargo.lock fehlt"
grep -Eq '^name *= *"auftragsbericht"' Cargo.toml || fail "Paketname auftragsbericht fehlt in Cargo.toml"
grep -Eq '^edition *= *"2021"' Cargo.toml || fail "Edition 2021 fehlt"
if sed -n '/^\[dependencies\]/,/^\[/p' Cargo.toml | grep -Eq '^[a-zA-Z_-]+ *='; then
  fail "Cargo.toml enthaelt Abhaengigkeiten"
fi

out=$(cargo build --offline --locked 2>&1) || fail "cargo build fehlgeschlagen" "$out"
BIN=target/debug/auftragsbericht
test -x "$BIN" || fail "Programm $BIN nicht gebaut"

out=$(cargo test --offline --locked 2>&1) || fail "cargo test fehlgeschlagen" "$out"
echo "$out" | grep -Eq 'test result: ok\. [1-9][0-9]* passed' || fail "keine laufenden Tests gefunden" "$out"

T=$(mktemp -d) || fail "mktemp fehlgeschlagen"
trap 'rm -rf "$T"' EXIT
D="$T/in"
mkdir -p "$D/job-a" "$D/job-b" "$D/job-c" "$D/andere" "$T/leer"
printf 'decision=rejected\nworker_cost_usd=1.5\n' > "$D/job-a/round-01.run"
printf 'decision=accepted\nworker_cost_usd=0.25\n' > "$D/job-a/round-02.run"
printf 'session_listed_usd=2\n' > "$D/job-a/extra.run"
printf 'worker_cost_usd=100\n' > "$D/job-a/notes.txt"
printf 'decision=accepted\nworker_cost_usd=abc\nsession_listed_usd=1\n' > "$D/job-b/round-01.run"
printf 'session_listed_usd=3\ndecision=accepted\n' > "$D/job-c/other.run"
printf 'decision=accepted\nworker_cost_usd=50\n' > "$D/andere/round-01.run"
printf 'x\n' > "$D/job-datei"

printf 'job-a\t2\taccepted\t3.75\njob-b\t1\taccepted\t1.00\njob-c\t0\t-\t3.00\ngesamt\t3\t2\t7.75\n' > "$T/erwartet"
"$BIN" "$D" > "$T/aus" 2> "$T/err"; rc=$?
test $rc -eq 0 || fail "Status $rc statt 0 bei gueltigem Ordner" "$(cat "$T/err")"
cmp -s "$T/aus" "$T/erwartet" || fail "Ausgabe weicht ab. Erhalten:
$(cat -A "$T/aus" 2>/dev/null || cat "$T/aus")"

"$BIN" "$T/leer" > "$T/aus" 2> "$T/err"; rc=$?
test $rc -eq 0 || fail "leerer Ordner: Status $rc statt 0"
printf 'gesamt\t0\t0\t0.00\n' > "$T/erwartet"
cmp -s "$T/aus" "$T/erwartet" || fail "leerer Ordner: falsche Ausgabe: $(cat "$T/aus")"

"$BIN" > "$T/aus" 2> "$T/err"; rc=$?
test $rc -eq 2 || fail "ohne Argument: Status $rc statt 2"
test "$(cat "$T/err")" = "Aufruf: auftragsbericht ORDNER" || fail "ohne Argument: falsche Fehlerausgabe: $(cat "$T/err")"

"$BIN" a b > "$T/aus" 2> "$T/err"; rc=$?
test $rc -eq 2 || fail "zwei Argumente: Status $rc statt 2"
test "$(cat "$T/err")" = "Aufruf: auftragsbericht ORDNER" || fail "zwei Argumente: falsche Fehlerausgabe"

"$BIN" "$T/gibtsnicht" > "$T/aus" 2> "$T/err"; rc=$?
test $rc -eq 1 || fail "fehlender Ordner: Status $rc statt 1"
test -s "$T/err" || fail "fehlender Ordner: keine Meldung auf Fehlerausgabe"

"$BIN" "$D/job-datei" > "$T/aus" 2> "$T/err"; rc=$?
test $rc -eq 1 || fail "Datei als ORDNER: Status $rc statt 1"
test -s "$T/err" || fail "Datei als ORDNER: keine Meldung auf Fehlerausgabe"

test -f README.md || fail "README.md fehlt"
grep -q 'auftragsbericht' README.md || fail "README nennt den Programmnamen nicht"
grep -qi 'Aufruf' README.md || fail "README ohne Abschnitt Aufruf"
grep -q 'gesamt' README.md || fail "README ohne Ausgabebeispiel (gesamt-Zeile)"

echo "PASS: auftragsbericht baut, Tests laufen, Verhalten und README stimmen"
