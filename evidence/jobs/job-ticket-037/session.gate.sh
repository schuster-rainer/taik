#!/bin/sh
fail() { echo "FAIL: $1"; exit 1; }

out=$(cargo build --offline --locked 2>&1) || { echo "$out"; fail "Build fehlgeschlagen"; }
BIN=./target/debug/auftragsbericht
test -x "$BIN" || fail "Binärdatei $BIN fehlt"

T=$(mktemp -d) || fail "mktemp"
mkdir -p "$T/leer"

"$BIN" --hilfe >"$T/o1" 2>"$T/e1"; rc=$?
test $rc -eq 0 || fail "--hilfe: Status $rc statt 0"
test ! -s "$T/e1" || fail "--hilfe: Standardfehlerausgabe nicht leer: $(cat "$T/e1")"
n=$(wc -l <"$T/o1" | tr -d ' ')
test "$n" -eq 4 || fail "Hilfe hat $n Zeilen statt 4: $(cat "$T/o1")"
l1=$(sed -n 1p "$T/o1")
test "$l1" = "Aufruf: auftragsbericht [--offen] [--json] ORDNER" || fail "Erste Zeile falsch: $l1"
sed -n 2p "$T/o1" | grep -q -- '--offen' || fail "Zeile 2 nennt --offen nicht"
sed -n 3p "$T/o1" | grep -q -- '--json' || fail "Zeile 3 nennt --json nicht"
sed -n 4p "$T/o1" | grep -q -- '--hilfe' || fail "Zeile 4 nennt --hilfe nicht"
for i in 2 3 4; do
  c=$(sed -n ${i}p "$T/o1" | wc -c | tr -d ' ')
  test "$c" -gt 20 || fail "Zeile $i hat keine Erklärung"
done

k=0
for args in "--offen --hilfe" "--hilfe --json $T/leer" "$T/leer --hilfe" "--bogus --hilfe" "--hilfe $T/nix $T/leer" "--json --offen --hilfe $T/leer"; do
  k=$((k+1))
  "$BIN" $args >"$T/o$k" 2>"$T/e$k"; rc=$?
  test $rc -eq 0 || fail "Aufruf '$args': Status $rc statt 0"
  cmp -s "$T/o1" "$T/o$k" || fail "Aufruf '$args': Ausgabe weicht ab: $(cat "$T/o$k")"
  test ! -s "$T/e$k" || fail "Aufruf '$args': Standardfehlerausgabe nicht leer"
done

"$BIN" >"$T/o" 2>"$T/e"; rc=$?
test $rc -eq 2 || fail "ohne Argumente: Status $rc statt 2"
test ! -s "$T/o" || fail "ohne Argumente: Standardausgabe nicht leer"
test "$(cat "$T/e")" = "Aufruf: auftragsbericht ORDNER" || fail "Fehlermeldung verändert: $(cat "$T/e")"
"$BIN" --bogus "$T/leer" >"$T/o" 2>"$T/e"; rc=$?
test $rc -eq 2 || fail "unbekannte Option: Status $rc statt 2"
test "$(cat "$T/e")" = "Aufruf: auftragsbericht ORDNER" || fail "Fehlermeldung bei unbekannter Option verändert"

"$BIN" "$T/leer" >"$T/o" 2>&1; rc=$?
test $rc -eq 0 || fail "leerer Ordner: Status $rc"
printf 'gesamt\t0\t0\t0.00\n' | cmp -s - "$T/o" || fail "leerer Ordner: Ausgabe verändert"

grep -q -- '--hilfe' README.md || fail "README erwähnt --hilfe nicht"
grep -q 'fn [a-z_]*hilfe[a-z_]*' tests/cli.rs || fail "Kein Test mit 'hilfe' im Namen in tests/cli.rs"

out=$(cargo test --offline --locked --test cli hilfe 2>&1) || { echo "$out"; fail "Hilfe-Tests fehlgeschlagen"; }
echo "$out" | grep -Eq 'test [a-z_0-9]*hilfe[a-z_0-9]* \.\.\. ok' || { echo "$out"; fail "Kein Hilfe-Test wurde ausgeführt"; }

rm -rf "$T"
echo "PASS: --hilfe gibt die Hilfe aus, Fehlerfall unverändert, Tests vorhanden"
