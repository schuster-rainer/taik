#!/bin/sh
set -eu

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

TMP=$(mktemp -d) || fail "temporäres Verzeichnis konnte nicht angelegt werden"
trap 'rm -rf "$TMP" || true' 0 1 2 15

if ! cargo build --release --offline --locked >/dev/null 2>&1; then
    fail "cargo build ist fehlgeschlagen"
fi

BIN=target/release/taik-seed
[ -x "$BIN" ] || fail "Binary $BIN fehlt"

grep -Fq 'taik-seed close STATE job-NAME' README.md || fail "README-Zeile fehlt"

if ! cargo test --offline --locked --test close >/dev/null 2>&1; then
    fail "Testziel close fehlt oder schlägt fehl"
fi

STATE="$TMP/state"
JOB=job-gate
mkdir -p "$STATE/jobs/$JOB" || fail "Auftragsordner konnte nicht angelegt werden"
CLOSED="$STATE/jobs/$JOB/closed.md"

close_expect() {
    expected=$1
    input=$2
    job=$3
    set +e
    printf '%s' "$input" | "$BIN" close "$STATE" "$job" >/dev/null 2>&1
    got=$?
    set -e
    [ "$got" -eq "$expected" ] || fail "close $job: erwartet $expected, war $got"
}

close_expect 2 'Grund' job-unbekannt
[ ! -e "$STATE/jobs/job-unbekannt/closed.md" ] || fail "unbekannter Auftrag hat closed.md erzeugt"

close_expect 2 '' "$JOB"
[ ! -e "$CLOSED" ] || fail "leere Begründung hat closed.md erzeugt"

LONG=$(node -e 'process.stdout.write("x".repeat(2001))')
close_expect 2 "$LONG" "$JOB"
[ ! -e "$CLOSED" ] || fail "zu lange Begründung hat closed.md erzeugt"

close_expect 0 'Grund' "$JOB"
[ -f "$CLOSED" ] || fail "closed.md fehlt nach gültigem Schließen"
node -e 'const fs=require("fs"); const b=fs.readFileSync(process.argv[1]); if (b.length!==5 || b.toString()!=="Grund") process.exit(1)' "$CLOSED" || fail "closed.md enthält nicht genau die Begründung"

close_expect 2 'Anders' "$JOB"
node -e 'const fs=require("fs"); const b=fs.readFileSync(process.argv[1]); if (b.toString()!=="Grund") process.exit(1)' "$CLOSED" || fail "closed.md wurde beim zweiten Schließen geändert"

echo "PASS: close schließt Aufträge und weist unbekannte, leere, zu lange und zweite Begründungen mit Status 2 ab"
