#!/bin/sh
# Gate for `taik-seed assign`.
fail() { echo "FAIL: $1"; exit 1; }

[ "$(uname)" = "Darwin" ] || fail "needs macOS for the factory commands"
[ -f tests/assign.rs ] || fail "tests/assign.rs fehlt"
grep -q 'README' /dev/null 2>/dev/null
grep -q 'taik-seed assign STATE NAME AGENT' README.md || fail "README nennt den Befehl nicht"

out=$(cargo test --offline --locked --test assign 2>&1) || { echo "$out" | tail -40; fail "cargo test --test assign schlägt fehl"; }
echo "$out" | grep -Eq 'test result: ok\. [1-9][0-9]* passed' || { echo "$out" | tail -20; fail "tests/assign.rs führt keinen Test aus"; }

out=$(cargo clippy --offline --locked --all-targets -- -D warnings 2>&1) || { echo "$out" | tail -40; fail "clippy hat Befunde"; }

out=$(cargo build --offline --locked 2>&1) || { echo "$out" | tail -30; fail "build schlägt fehl"; }
BIN="$(pwd -P)/${CARGO_TARGET_DIR:-target}/debug/taik-seed"
[ -x "$BIN" ] || BIN="${CARGO_TARGET_DIR}/debug/taik-seed"
[ -x "$BIN" ] || fail "Binary taik-seed nicht gefunden"

T=$(mktemp -d /tmp/taik-assign-gate.XXXXXX) || fail "mktemp"
trap 'rm -rf "$T"' EXIT
S="$T/state"
mkdir -p "$S/jobs" "$S/crew" "$S/proposals"
S=$(cd "$S" && pwd -P)

agent() { printf 'harness=%s\nmodel=m\neffort=low\npersona=\nend\n' "$2" > "$S/crew/$1.agent"; }
agent bert claude
agent anna claude
agent pii pi

R="$T/repo"
mkdir "$R" && (cd "$R" && git init -q && echo x > f && git add f && \
  git -c user.name=t -c user.email=t@t commit -q -m i) || fail "git-Repo anlegen"
R=$(cd "$R" && pwd -P)

prop() { # name kind work gate
  d="$S/proposals/$1"; mkdir -p "$d"
  printf 'Eine Seite mit blauem Titel bauen\n\nDer Titel ist blau.\n' > "$d/brief.md"
  printf "console.log('PASS: ok');\n" > "$d/verify.mjs"
  printf '#!/bin/sh\nexit 0\n' > "$d/gate.sh"
  printf 'derived_from=x\nproposed_by=test\nstate=waiting for approval\n' > "$d/origin.txt"
  if [ "$2" = sitzung ]; then
    printf 'kind=sitzung\nrepo=r\nrepo_path=%s\ngate=%s\nwork=%s\nreason=x\n' "$R" "$4" "$3" > "$d/plan.txt"
  else
    printf 'kind=dokument\nrepo=\nrepo_path=\ngate=%s\nwork=%s\nreason=x\n' "$4" "$3" > "$d/plan.txt"
  fi
}
snap() { (cd "$S" && find . | sort && find . -type f | sort | while read -r f; do cksum "$f"; done); }
run() { "$BIN" "$@" 2>&1; }

expect_refused() { # description args...
  desc=$1; shift
  before=$(snap)
  o=$(run "$@"); c=$?
  [ "$c" = 2 ] || fail "$desc: Status $c statt 2 ($o)"
  [ "$before" = "$(snap)" ] || fail "$desc: hat etwas geschrieben"
}

prop p1 dokument standard anna
prop p2 dokument bert anna
prop p3 dokument bert anna
prop p4 sitzung standard anna
prop p5 dokument standard anna
prop pd dokument standard anna

# write and overwrite
o=$(run assign "$S" p1 bert) || fail "assign p1 bert: $o"
[ "$(cat "$S/proposals/p1/worker")" = bert ] || fail "worker enthält nicht bert"
[ "$(wc -l < "$S/proposals/p1/worker" | tr -d ' ')" = 1 ] || fail "worker ist nicht eine Zeile"
o=$(run assign "$S" p1 standard) || fail "assign p1 standard: $o"
[ "$(cat "$S/proposals/p1/worker")" = standard ] || fail "worker nicht überschrieben"
o=$(run assign "$S" p1 bert) || fail "assign p1 bert (2): $o"

# approve with worker file overriding plan
cp "$S/proposals/p1/plan.txt" "$T/p1.plan"
job=$(run approve "$S" p1) || fail "approve p1: $job"
job=$(echo "$job" | tail -1)
[ -d "$job" ] || fail "approve gab keinen Auftragsordner aus: $job"
[ "$(cat "$job/agent" 2>/dev/null)" = bert ] || fail "p1: agent im Auftrag ist nicht bert"
[ "$(cat "$job/worker" 2>/dev/null)" = bert ] || fail "p1: worker im Auftrag fehlt/falsch"
cmp -s "$job/plan.txt" "$T/p1.plan" || fail "p1: plan.txt im Auftrag weicht ab"
cmp -s "$S/proposals/p1/plan.txt" "$T/p1.plan" || fail "p1: plan.txt des Vorschlags verändert"

# standard in worker file beats work=bert
o=$(run assign "$S" p2 standard) || fail "assign p2 standard: $o"
job=$(run approve "$S" p2) || fail "approve p2: $job"
job=$(echo "$job" | tail -1)
[ ! -e "$job/agent" ] || fail "p2: bei standard darf keine Datei agent existieren"
[ "$(cat "$job/worker" 2>/dev/null)" = standard ] || fail "p2: worker im Auftrag fehlt/falsch"

# no worker file: plan decides
job=$(run approve "$S" p3) || fail "approve p3: $job"
job=$(echo "$job" | tail -1)
[ "$(cat "$job/agent" 2>/dev/null)" = bert ] || fail "p3: ohne worker gilt work= nicht mehr"

# session
expect_refused "Sitzung mit pi-Agent" assign "$S" p4 pii
expect_refused "Sitzung mit Gate-Autor" assign "$S" p4 anna
[ ! -e "$S/proposals/p4/worker" ] || fail "p4: worker trotz Abweisung"
o=$(run assign "$S" p4 bert) || fail "assign p4 bert: $o"
job=$(run approve "$S" p4) || fail "approve p4 (Sitzung): $job"
job=$(echo "$job" | tail -1)
[ "$(cat "$job/agent" 2>/dev/null)" = bert ] || fail "p4: agent fehlt"
[ "$(cat "$job/worker" 2>/dev/null)" = bert ] || fail "p4: worker fehlt"
o=$(run assign "$S" p4 standard) && fail "assign nach Freigabe wurde nicht abgewiesen"
prop p6 sitzung bert anna
o=$(run assign "$S" p6 standard) || fail "Sitzung mit standard muss erlaubt sein: $o"

# document with non-claude agent is allowed
o=$(run assign "$S" p5 pii) || fail "Dokument mit pi-Agent muss erlaubt sein: $o"

# refusals
printf 'Nein.' | "$BIN" decline "$S" pd >/dev/null 2>&1 || fail "decline pd"
expect_refused "abgelehnter Vorschlag" assign "$S" pd bert
expect_refused "freigegebener Vorschlag" assign "$S" p1 standard
expect_refused "Gate-Autor als Arbeiter" assign "$S" p5 anna
expect_refused "unbekannter Agent" assign "$S" p5 nobody
expect_refused "ungültiger Agent" assign "$S" p5 ../bert
expect_refused "leerer Agent" assign "$S" p5 ""
expect_refused "unbekannter Vorschlag" assign "$S" nope bert
expect_refused "ungültiger Name Großbuchstabe" assign "$S" P5 bert
expect_refused "ungültiger Name Pfad" assign "$S" ../p5 bert
expect_refused "ungültiger Name Unterstrich" assign "$S" p_5 bert
[ "$(cat "$S/proposals/p5/worker")" = pii ] || fail "p5: worker durch Abweisung verändert"

echo "PASS: assign schreibt, überschreibt, weist ab und approve folgt der Datei worker"
