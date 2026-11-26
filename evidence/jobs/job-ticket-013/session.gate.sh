#!/bin/sh
# Gate: approve copies plan.txt of the proposal into the job folder.
fail() { echo "FAIL: $1"; exit 1; }

ROOT=$(pwd)
OUT=$(mktemp -d "${TMPDIR:-/tmp}/plan-gate-XXXXXX") || fail "kein Temp-Ordner"
trap 'rm -rf "$OUT"' EXIT

cargo build --offline --locked >"$OUT/build.log" 2>&1 || {
  tail -n 40 "$OUT/build.log"
  fail "cargo build schlägt fehl"
}
BIN="$ROOT/target/debug/taik-seed"
test -x "$BIN" || fail "Binärdatei $BIN fehlt"

PLAN_TAIL='gate=standard
work=standard
reason=Testplan
extra=ä unbekannte Zeile'

# --- 1. Dokument mit Plan ---------------------------------------------------
S1="$OUT/s1"
mkdir -p "$S1/jobs" "$S1/proposals/doc"
printf 'Eine Seite mit Titel bauen\n\nMehr Text.\n' >"$S1/proposals/doc/brief.md"
printf "console.log('PASS: ok');\n" >"$S1/proposals/doc/verify.mjs"
printf 'kind=dokument\nrepo=\nrepo_path=\n%s\n' "$PLAN_TAIL" >"$S1/proposals/doc/plan.txt"
"$BIN" approve "$S1" doc >"$OUT/a1.log" 2>&1 || { cat "$OUT/a1.log"; fail "approve (Dokument mit Plan) schlägt fehl"; }
J1="$S1/jobs/job-doc"
test -f "$J1/plan.txt" || fail "Dokument-Auftrag hat keine plan.txt"
cmp -s "$S1/proposals/doc/plan.txt" "$J1/plan.txt" || fail "plan.txt im Dokument-Auftrag weicht vom Vorschlag ab"
test -f "$J1/submission.brief.md" || fail "Dokument-Auftrag unvollständig (submission.brief.md fehlt)"
test -f "$S1/proposals/doc/approved.txt" || fail "approved.txt fehlt (Dokument)"

# --- 2. Dokument ohne Plan --------------------------------------------------
S2="$OUT/s2"
mkdir -p "$S2/jobs" "$S2/proposals/doc"
printf 'Eine Seite mit Titel bauen\n' >"$S2/proposals/doc/brief.md"
printf "console.log('PASS: ok');\n" >"$S2/proposals/doc/verify.mjs"
"$BIN" approve "$S2" doc >"$OUT/a2.log" 2>&1 || { cat "$OUT/a2.log"; fail "approve (Dokument ohne Plan) schlägt fehl"; }
test -f "$S2/jobs/job-doc/submission.brief.md" || fail "Dokument-Auftrag ohne Plan unvollständig"
test ! -e "$S2/jobs/job-doc/plan.txt" || fail "Auftrag ohne Plan hat trotzdem eine plan.txt"

# --- 3. Sitzung mit Plan ----------------------------------------------------
REPO="$OUT/repo"
mkdir -p "$REPO"
(
  cd "$REPO" &&
  git init -q . &&
  echo hallo >a.txt &&
  git add a.txt &&
  git -c user.name=t -c user.email=t@example.com commit -q -m init
) >"$OUT/git.log" 2>&1 || { cat "$OUT/git.log"; fail "Test-Repository konnte nicht angelegt werden"; }

S3="$OUT/s3"
mkdir -p "$S3/jobs" "$S3/proposals/sit"
printf 'Eine Datei hello.txt anlegen\n' >"$S3/proposals/sit/brief.md"
printf "test -f hello.txt && echo 'PASS: da'\n" >"$S3/proposals/sit/gate.sh"
printf 'kind=sitzung\nrepo=demo\nrepo_path=%s\n%s\n' "$REPO" "$PLAN_TAIL" >"$S3/proposals/sit/plan.txt"
"$BIN" approve "$S3" sit >"$OUT/a3.log" 2>&1 || { cat "$OUT/a3.log"; fail "approve (Sitzung mit Plan) schlägt fehl"; }
J3="$S3/jobs/job-sit"
test -f "$J3/session.gate.sh" || fail "Sitzungsauftrag unvollständig (session.gate.sh fehlt)"
test -f "$J3/plan.txt" || fail "Sitzungsauftrag hat keine plan.txt"
cmp -s "$S3/proposals/sit/plan.txt" "$J3/plan.txt" || fail "plan.txt im Sitzungsauftrag weicht vom Vorschlag ab"

# --- 4. Sitzung ohne Plan (Repo-Angabe nur über den Plan möglich: Dokumentpfad) ---
# Ein Vorschlag ohne plan.txt wird als Dokument freigegeben; geprüft in Fall 2.

# --- 5. Das neue Testziel ---------------------------------------------------
test -f tests/plan_job.rs || fail "tests/plan_job.rs fehlt"
cargo test --offline --locked --test plan_job >"$OUT/test.log" 2>&1 || {
  tail -n 60 "$OUT/test.log"
  fail "cargo test --test plan_job schlägt fehl"
}
PASSED=$(sed -n 's/^test result: ok\. \([0-9][0-9]*\) passed.*/\1/p' "$OUT/test.log" | head -n 1)
test -n "$PASSED" || { tail -n 30 "$OUT/test.log"; fail "keine Testergebnisse von plan_job gefunden"; }
test "$PASSED" -ge 3 || { tail -n 30 "$OUT/test.log"; fail "plan_job führt nur $PASSED Tests aus, erwartet mindestens 3"; }

echo "PASS: approve übernimmt plan.txt in beide Auftragsarten, ohne Plan bleibt sie aus, plan_job hat $PASSED Tests"
