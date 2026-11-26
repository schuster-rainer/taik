#!/bin/sh
fail() { echo "FAIL: $1"; exit 1; }

[ "$(uname -s)" = "Darwin" ] || fail "Das Modul orchestrator gibt es nur auf macOS"

out=$(cargo build --offline --locked 2>&1) || { echo "$out" | tail -30; fail "cargo build ist gescheitert"; }
BIN="$PWD/target/debug/taik-seed"
[ -x "$BIN" ] || fail "Binärdatei $BIN fehlt"

grep -q 'taik-seed orchestrator-prompt STATE REQUEST.run' README.md || fail "README.md nennt den Befehl nicht"
grep -q 'orchestrator-prompt' src/main.rs || fail "src/main.rs kennt orchestrator-prompt nicht"

T=$(mktemp -d) || fail "mktemp"
S="$T/state"
mkdir -p "$S/crew" "$S/products" "$S/repos" "$T/repo1/.git" "$T/repo1/.taik" "$T/repo2/.git"
printf '# Kommentar\n\nsrc/alpha.rs\nAGENTS.md\n# noch einer\ntools/x\n' > "$T/repo1/.taik/protected"
printf 'readme eins\n' > "$T/repo1/README.md"
printf '%s\n' "$T/repo1" > "$S/repos/eins.repo"
printf '%s\n' "$T/repo2" > "$S/repos/zwei.repo"
for n in anna bert; do
  printf 'harness=claude\nmodel=m\neffort=low\npersona=\nend\n' > "$S/crew/$n.agent"
done
printf 'x\n' > "$S/crew/anna.failed"
printf 'x\n' > "$S/crew/bert.failed"
printf 'seed=/tmp/s.html\ngate=/tmp/g.mjs\nabout=Ein Rechner fuer Zahlen\nend\n' > "$S/products/rechner.product"
printf 'decision=observed\nidentity=ticket-1\nintent=Teste\nobservation=Wunschtext-XYZ\ncontinuation=x\nend\n' > "$T/req.run"
# bert.failed ist alt (8 Stunden)
node -e 'const t=(Date.now()-8*3600*1000)/1000;require("fs").utimesSync(process.argv[1],t,t)' "$S/crew/bert.failed" || fail "utimes"

cat > "$T/worker" <<EOF
#!/bin/sh
echo called > "$T/worker-called"
EOF
chmod +x "$T/worker"

P="$T/prompt.txt"
TAIK_LEAD_WORKER="$T/worker" "$BIN" orchestrator-prompt "$S" "$T/req.run" > "$P" 2> "$T/err.txt"
rc=$?
[ "$rc" -eq 0 ] || { cat "$T/err.txt"; fail "orchestrator-prompt endete mit Status $rc"; }
[ ! -e "$T/worker-called" ] || fail "orchestrator-prompt hat einen Worker aufgerufen"

grep -q 'Wunschtext-XYZ' "$P" || fail "Der Wunsch fehlt im Auftragssatz"
grep -q '^AGENTS:$' "$P" || fail "Abschnitt AGENTS: fehlt"
grep -q '^REPOSITORIES:$' "$P" || fail "Abschnitt REPOSITORIES: fehlt"

# Produkte
grep -A1 '^PRODUCTS:$' "$P" | tail -1 | grep -qx -- '- rechner: Ein Rechner fuer Zahlen' \
  || { cat "$P"; fail "Zeile '- rechner: ...' unter PRODUCTS: fehlt"; }

# geschützte Pfade
grep -qx '  protected: src/alpha.rs, AGENTS.md, tools/x' "$P" \
  || { cat "$P"; fail "Zeile '  protected: src/alpha.rs, AGENTS.md, tools/x' fehlt"; }
grep -qx '  protected: (none)' "$P" || fail "Zeile '  protected: (none)' für Repo ohne Datei fehlt"
n=$(grep -c '^  protected: ' "$P")
[ "$n" -eq 2 ] || fail "Erwartet 2 protected-Zeilen, gefunden $n"
# protected steht direkt unter seinem Repository
grep -A1 '^- eins: ' "$P" | tail -1 | grep -q '^  protected: src/alpha.rs' || fail "protected steht nicht unter Repo eins"
grep -A1 '^- zwei: ' "$P" | tail -1 | grep -qx '  protected: (none)' || fail "protected steht nicht unter Repo zwei"

# ausgefallene Agenten
grep -q '^- anna: .* (failed recently)$' "$P" || { cat "$P"; fail "anna fehlt mit ' (failed recently)'"; }
grep -q '^- bert: ' "$P" || fail "bert fehlt"
grep '^- bert: ' "$P" | grep -q 'failed recently' && fail "bert (Marke älter als 6 Stunden) darf nicht markiert sein"
grep '^- standard: ' "$P" | grep -q 'failed recently' && fail "standard darf nicht markiert sein"

# Leerer Zustand
S2="$T/state2"; mkdir -p "$S2"
"$BIN" orchestrator-prompt "$S2" "$T/req.run" > "$T/p2.txt" 2>/dev/null || fail "orchestrator-prompt mit leerem STATE scheitert"
grep -A1 '^PRODUCTS:$' "$T/p2.txt" | tail -1 | grep -qx '(none)' || fail "PRODUCTS: ohne Produkte braucht (none)"

# Sprechende Hinweise im Auftragssatz
grep -qi 'protected' "$P" && grep -qi 'dokument' "$P" || fail "Auftragssatz fehlt Hinweis auf Produkte/geschützte Pfade"

# Tests
run_test() {
  o=$("$@" 2>&1) || { echo "$o" | tail -40; fail "Testlauf scheitert: $*"; }
  echo "$o"
}
o=$(run_test cargo test --offline --locked --test orchestrator) || { echo "$o"; exit 1; }
echo "$o" | grep -q 'test result: ok\. [1-9]' || { echo "$o" | tail -20; fail "tests/orchestrator.rs führt keinen Test aus"; }

for name in a_failed_gate_agent_is_replaced the_failed_marker_is_written_and_removed \
            a_plan_is_read_checked_and_kept another_agent_steps_in_for_a_gate_author_that_fails; do
  o=$(cargo test --offline --locked --bin taik-seed -- "$name" 2>&1) || { echo "$o" | tail -40; fail "Unit-Test $name scheitert"; }
  echo "$o" | grep -q "$name ... ok" || { echo "$o" | tail -20; fail "Unit-Test $name nicht gefunden oder nicht ok"; }
done

rm -rf "$T"
echo "PASS: orchestrator-prompt, Produkte, geschützte Pfade, Ausfallmarken und Tests sind in Ordnung"
