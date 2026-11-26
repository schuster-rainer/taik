#!/bin/sh
fail() { echo "FAIL: $1"; exit 1; }

command -v cargo >/dev/null 2>&1 || fail "cargo nicht gefunden"
command -v node >/dev/null 2>&1 || fail "node nicht gefunden"

OUT=$(cargo build --offline --locked 2>&1) || { echo "$OUT" | tail -30; fail "cargo build fehlgeschlagen"; }
BIN="$PWD/target/debug/auftragsbericht"
test -x "$BIN" || fail "Binärdatei $BIN fehlt"

# keine Abhängigkeiten
n=$(grep -c '^name = ' Cargo.lock)
test "$n" = 1 || fail "Cargo.lock enthält fremde Pakete"
sed -n '/^\[dependencies\]/,/^\[/p' Cargo.toml | grep -v '^\[' | grep -q '[a-zA-Z]' && fail "Cargo.toml enthält Abhängigkeiten"

grep -q -- '--json' README.md || fail "README.md erwähnt --json nicht"

T=$(mktemp -d) || fail "mktemp"
W="$T/w"
mkdir -p "$W/job-a" "$W/job-b" "$W/job-c" "$W/anders" "$W/job-q\"b\\c" || fail "mkdir"
printf 'decision=accepted\nworker_cost_usd=3.5\n' > "$W/job-a/round-01.run"
printf 'session_listed_usd=0.25\n' > "$W/job-a/extra.run"
printf 'decision=rejected\nworker_cost_usd=0.5\n' > "$W/job-b/round-01.run"
printf 'worker_cost_usd=1\n' > "$W/job-c/sonst.run"
printf 'decision=rej"\\\001x\nworker_cost_usd=0.1\nsession_listed_usd=0.2\n' > "$W/job-q\"b\\c/round-01.run"
printf 'decision=accepted\n' > "$W/anders/round-01.run"
mkdir "$T/leer"

# bisheriges Verhalten
"$BIN" "$W" > "$T/tab.out" 2>"$T/tab.err" || fail "Aufruf ohne --json schlug fehl"
head -3 "$T/tab.out" | grep -q '^job-a	1	accepted	3.75$' || fail "Tabellenausgabe ohne --json verändert"
tail -1 "$T/tab.out" | grep -q '^gesamt	4	1	5.55$' || fail "gesamt-Zeile ohne --json verändert"

cat > "$T/check.js" <<'EOF'
const fs = require('fs');
const [file, modus] = process.argv.slice(2);
const raw = fs.readFileSync(file, 'utf8');
function bad(m) { console.log('FAIL: ' + m); process.exit(1); }
if (!raw.endsWith('\n')) bad('Ausgabe endet nicht mit Zeilenende');
let o;
try { o = JSON.parse(raw); } catch (e) { bad('kein gültiges JSON: ' + e.message + ' / ' + JSON.stringify(raw.slice(0, 200))); }
function keys(x, k, w) {
  if (x === null || typeof x !== 'object' || Array.isArray(x)) bad(w + ' ist kein Objekt');
  if (Object.keys(x).sort().join(',') !== k.sort().join(',')) bad('Schlüssel von ' + w + ': ' + Object.keys(x).join(','));
}
function twoDec(x, w) {
  if (typeof x !== 'number' || !isFinite(x)) bad(w + ' ist keine Zahl');
  if (Math.abs(x * 100 - Math.round(x * 100)) > 1e-6) bad(w + ' hat mehr als zwei Nachkommastellen: ' + x);
}
function near(a, b, w) { if (Math.abs(a - b) > 1e-6) bad(w + ': erwartet ' + b + ', erhalten ' + a); }
keys(o, ['auftraege', 'gesamt'], 'Wurzel');
if (!Array.isArray(o.auftraege)) bad('auftraege ist keine Liste');
keys(o.gesamt, ['auftraege', 'angenommen', 'kosten'], 'gesamt');
const sonder = 'job-q"b\\c';
const dec = 'rej"\\\u0001x';
let alle = [
  ['job-a', 1, 'accepted', 3.75],
  ['job-b', 1, 'rejected', 0.5],
  ['job-c', 0, '-', 1],
  [sonder, 1, dec, 0.3],
];
let erw = modus === 'offen' ? alle.filter(a => a[2] !== 'accepted') : modus === 'leer' ? [] : alle;
if (o.auftraege.length !== erw.length) bad('Anzahl Aufträge ' + o.auftraege.length + ', erwartet ' + erw.length);
erw.forEach((e, i) => {
  const a = o.auftraege[i];
  keys(a, ['name', 'runden', 'entscheidung', 'kosten'], 'Auftrag ' + i);
  if (a.name !== e[0]) bad('name ' + JSON.stringify(a.name) + ' erwartet ' + JSON.stringify(e[0]));
  if (a.runden !== e[1]) bad('runden von ' + e[0] + ': ' + a.runden);
  if (a.entscheidung !== e[2]) bad('entscheidung von ' + e[0] + ': ' + JSON.stringify(a.entscheidung));
  twoDec(a.kosten, 'kosten von ' + e[0]);
  near(a.kosten, e[3], 'kosten von ' + e[0]);
});
const summe = erw.reduce((s, e) => s + e[3], 0);
const ang = erw.filter(e => e[2] === 'accepted').length;
if (o.gesamt.auftraege !== erw.length) bad('gesamt.auftraege ' + o.gesamt.auftraege);
if (o.gesamt.angenommen !== ang) bad('gesamt.angenommen ' + o.gesamt.angenommen);
twoDec(o.gesamt.kosten, 'gesamt.kosten');
near(o.gesamt.kosten, summe, 'gesamt.kosten');
EOF

lauf() { # name modus args...
  nm=$1; modus=$2; shift 2
  "$BIN" "$@" > "$T/$nm.out" 2> "$T/$nm.err"
  rc=$?
  test $rc -eq 0 || { cat "$T/$nm.err"; fail "$nm: Rückgabewert $rc"; }
  node "$T/check.js" "$T/$nm.out" "$modus" || exit 1
}
lauf voll voll --json "$W"
lauf offen1 offen --json --offen "$W"
lauf offen2 offen --offen --json "$W"
lauf leer leer --json "$T/leer"

# Fehlerfälle
"$BIN" --json "$T/gibt-es-nicht" > "$T/f.out" 2>"$T/f.err"; rc=$?
test $rc -eq 1 || fail "--json mit fehlendem Ordner: Rückgabewert $rc statt 1"
test -s "$T/f.out" && fail "--json mit fehlendem Ordner schrieb auf stdout"
"$BIN" --json > "$T/f.out" 2>"$T/f.err"; rc=$?
test $rc -eq 2 || fail "--json ohne Ordner: Rückgabewert $rc statt 2"
grep -q 'Aufruf: auftragsbericht ORDNER' "$T/f.err" || fail "Aufrufzeile fehlt bei --json ohne Ordner"

rm -rf "$T"
echo "PASS: --json funktioniert, auch mit --offen, Maskierung und unverändertem Standardverhalten"
