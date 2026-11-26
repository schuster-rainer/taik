#!/bin/sh
# Gate: settle command (part 1), black-box on real git repos.
fail() { echo "FAIL: $1"; exit 1; }

export CARGO_TARGET_DIR="$PWD/target"
out=$(cargo build --offline --locked 2>&1) || { echo "$out" | tail -30; fail "cargo build scheitert"; }
BIN="$CARGO_TARGET_DIR/debug/taik-seed"
test -x "$BIN" || fail "Binary $BIN fehlt"

grep -qx 'taik-seed settle STATE job-NAME' README.md || fail "README.md enthaelt die Zeile 'taik-seed settle STATE job-NAME' nicht"
test -f tests/settle.rs || fail "tests/settle.rs fehlt"

T=$(mktemp -d) || fail "mktemp"
STATE="$T/state"
mkdir -p "$STATE/jobs" "$STATE/work"

g() { d=$1; shift; git -C "$d" -c user.name=T -c user.email=t@example.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null "$@"; }

# mk NAME: origin with one commit, work copy with one more, accepted job round.
mk() {
  n=$1
  o="$T/origin-$n"
  mkdir "$o"
  g "$o" init --quiet >/dev/null 2>&1 || fail "git init"
  echo a > "$o/a.txt"
  g "$o" add a.txt && g "$o" commit --quiet -m start || fail "git commit origin"
  git clone --quiet --local --no-hardlinks "$o" "$STATE/work/$n" >/dev/null 2>&1 || fail "git clone"
  g "$STATE/work/$n" remote remove origin
  echo b > "$STATE/work/$n/b.txt"
  g "$STATE/work/$n" add b.txt && g "$STATE/work/$n" commit --quiet -m agent || fail "git commit work"
  c=$(g "$STATE/work/$n" rev-parse HEAD)
  mkdir "$STATE/jobs/job-$n"
  printf '%s\n' "$o" > "$STATE/jobs/job-$n/session.repo"
  printf 'decision=accepted\nkind=software\ncommit=%s\nend\n' "$c" > "$STATE/jobs/job-$n/round-01.run"
}

# 1. success
mk demo
J="$STATE/jobs/job-demo"
res=$("$BIN" settle "$STATE" job-demo 2>"$T/err"); rc=$?
test $rc -eq 0 || { cat "$T/err"; fail "settle Erfolg: Status $rc statt 0"; }
test "$res" = "taik/demo" || fail "settle Erfolg: stdout ist '$res' statt 'taik/demo'"
test -f "$J/adopted.txt" || fail "adopted.txt wurde nicht geschrieben"
test "$(cat "$J/adopted.txt")" = "taik/demo" || fail "adopted.txt hat falschen Inhalt"
test "$(grep -c '' "$J/adopted.txt")" -eq 1 || fail "adopted.txt hat nicht genau eine Zeile"
test "$(g "$T/origin-demo" rev-parse refs/heads/taik/demo)" = "$(g "$STATE/work/demo" rev-parse HEAD)" || fail "Zweig taik/demo zeigt nicht auf den angenommenen Commit"
test ! -e "$J/adopt-failed.txt" || fail "adopt-failed.txt bei Erfolg angelegt"

# 2. existing adopted.txt stays unchanged
printf 'alt\n' > "$J/adopted.txt"
"$BIN" settle "$STATE" job-demo >/dev/null 2>&1; rc=$?
test $rc -eq 0 || fail "zweiter settle-Aufruf: Status $rc statt 0"
test "$(cat "$J/adopted.txt")" = "alt" || fail "vorhandene adopted.txt wurde veraendert"

# 3. failure: branch already on another commit
mk bad
JB="$STATE/jobs/job-bad"
g "$T/origin-bad" branch taik/bad "$(g "$T/origin-bad" rev-parse HEAD)" || fail "git branch"
printf 'old\nold\n' > "$JB/adopt-failed.txt"
"$BIN" settle "$STATE" job-bad >"$T/o" 2>"$T/err"; rc=$?
test $rc -eq 2 || fail "settle Fehlschlag: Status $rc statt 2"
test -f "$JB/adopt-failed.txt" || fail "adopt-failed.txt fehlt"
test "$(grep -c '' "$JB/adopt-failed.txt")" -eq 1 || fail "adopt-failed.txt hat nicht genau eine Zeile (alte Datei nicht ueberschrieben?)"
test -n "$(tr -d ' \n' < "$JB/adopt-failed.txt")" || fail "adopt-failed.txt ist leer"
test "$(cat "$JB/adopt-failed.txt")" != "old" || fail "adopt-failed.txt nicht ueberschrieben"
test ! -e "$JB/adopted.txt" || fail "adopted.txt trotz Fehlschlag angelegt"

# 4. job without session.repo: nothing written, status 0
mkdir "$STATE/jobs/job-norepo"
printf 'decision=accepted\nkind=software\ncommit=%s\nend\n' "$(g "$STATE/work/demo" rev-parse HEAD)" > "$STATE/jobs/job-norepo/round-01.run"
before=$(ls -A "$STATE/jobs/job-norepo")
"$BIN" settle "$STATE" job-norepo >/dev/null 2>&1; rc=$?
test $rc -eq 0 || fail "Auftrag ohne session.repo: Status $rc statt 0"
test "$(ls -A "$STATE/jobs/job-norepo")" = "$before" || fail "Auftrag ohne session.repo: es wurde etwas geschrieben"

# 5. unknown job: status 2, nothing created
before=$(ls -A "$STATE/jobs")
"$BIN" settle "$STATE" job-missing >/dev/null 2>&1; rc=$?
test $rc -eq 2 || fail "unbekannter Auftrag: Status $rc statt 2"
test "$(ls -A "$STATE/jobs")" = "$before" || fail "unbekannter Auftrag: es wurde etwas angelegt"

# 6. adopt unchanged
mk plain
res=$("$BIN" adopt "$STATE" job-plain 2>/dev/null); rc=$?
test $rc -eq 0 && test "$res" = "taik/plain" || fail "adopt verhaelt sich anders (Status $rc, Ausgabe '$res')"
test ! -e "$STATE/jobs/job-plain/adopted.txt" || fail "adopt schreibt adopted.txt, soll es nicht"

# 7. own test target
out=$(cargo test --offline --locked --test settle 2>&1); rc=$?
if [ $rc -ne 0 ]; then echo "$out" | tail -40; fail "cargo test --test settle scheitert"; fi
echo "$out" | grep -Eq 'test result: ok\. [1-9][0-9]* passed' || { echo "$out" | tail -20; fail "tests/settle.rs enthaelt keinen bestandenen Test"; }

rm -rf "$T"
echo "PASS: settle verhaelt sich wie verlangt und tests/settle.rs besteht"
