#!/bin/sh
set -u

fail() {
    printf 'FAIL: %s\n' "$1"
    exit 1
}

command -v cargo >/dev/null 2>&1 || fail "cargo fehlt"
command -v node >/dev/null 2>&1 || fail "node fehlt"

tmp="/tmp/taik-gate-$$"
node -e 'require("fs").mkdirSync(process.argv[1], {recursive:true})' "$tmp" \
    || fail "temporäres Verzeichnis konnte nicht angelegt werden"

if ! cargo build --offline --locked >"$tmp/build.log" 2>&1; then
    fail "cargo build --offline --locked ist fehlgeschlagen"
fi

bin="target/debug/taik-seed"
test -x "$bin" || fail "target/debug/taik-seed fehlt"

node - "$tmp" <<'NODE'
const fs = require("fs");
const path = require("path");
const t = process.argv[2];

function w(p, s) {
  fs.mkdirSync(path.dirname(p), {recursive: true});
  fs.writeFileSync(p, s);
}

w(path.join(t, "state/products/kasse.product"), "about=Kasse\n");
w(path.join(t, "state/products/lager.product"), "about=Lager\n");
w(path.join(t, "state/repositories/alpha/.taik/protected"),
  "# Kommentar\nsrc/secret.rs\n\nCargo.toml\n");
w(path.join(t, "state/repositories/beta/.taik/protected"), "");
w(path.join(t, "state/crew/gate-author"), "gate-author\n");
w(path.join(t, "state/crew/other"), "other\n");
w(path.join(t, "state/crew/gate-author.failed"), "kaputt\n");
w(path.join(t, "state/crew/other.failed"), "alt\n");

const old = new Date(Date.now() - 7 * 60 * 60 * 1000);
fs.utimesSync(path.join(t, "state/crew/other.failed"), old, old);

fs.mkdirSync(path.join(t, "state2/products"), {recursive: true});
NODE
test $? -eq 0 || fail "Testzustand konnte nicht angelegt werden"

printf '%s\n' \
    'decision=pending' \
    'identity=gate-check' \
    'intent=Prüfe den Auftragssatz' \
    'observation=Ein Wunsch' \
    'continuation=Test' >"$tmp/request.run"

if ! "$bin" orchestrator-prompt "$tmp/state" "$tmp/request.run" >"$tmp/out.txt" 2>"$tmp/err.txt"; then
    fail "taik-seed orchestrator-prompt ist fehlgeschlagen"
fi

node - "$tmp/out.txt" <<'NODE'
const fs = require("fs");
const out = fs.readFileSync(process.argv[2], "utf8");
const lines = out.split(/\r?\n/);

function bad(m) {
  console.error("FAIL: " + m);
  process.exit(1);
}
function need(s) {
  if (!out.includes(s)) bad("Auftragssatz enthält nicht: " + s);
}

need("PRODUCTS:");
need("- kasse: Kasse");
need("- lager: Lager");
need("  protected: src/secret.rs, Cargo.toml");
need("  protected: (none)");

if (!lines.some(l => /^gate-author.* \(failed recently\)$/.test(l))) {
  bad("gate-author ist nicht als kürzlich ausgefallen markiert");
}
if (lines.some(l => /^other.* \(failed recently\)$/.test(l))) {
  bad("other ist trotz altem Fehlschlag als kürzlich ausgefallen markiert");
}
NODE
test $? -eq 0 || exit 1

if ! "$bin" orchestrator-prompt "$tmp/state2" "$tmp/request.run" >"$tmp/out2.txt" 2>"$tmp/err2.txt"; then
    fail "taik-seed orchestrator-prompt ohne Produkte ist fehlgeschlagen"
fi

node - "$tmp/out2.txt" <<'NODE'
const fs = require("fs");
const out = fs.readFileSync(process.argv[2], "utf8");
if (!out.includes("PRODUCTS:") || !out.includes("(none)")) {
  console.error("FAIL: ohne Produkte fehlt PRODUCTS: mit (none)");
  process.exit(1);
}
NODE
test $? -eq 0 || exit 1

grep -F 'taik-seed orchestrator-prompt STATE REQUEST.run' README.md >/dev/null 2>&1 \
    || fail "README-Zeile für orchestrator-prompt fehlt"

if ! cargo test --offline --locked >"$tmp/test.log" 2>&1; then
    fail "cargo test --offline --locked ist fehlgeschlagen"
fi

printf 'PASS: Auftragssatz, Zustandsregeln und Tests erfüllt\n'
