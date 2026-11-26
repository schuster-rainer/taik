#!/bin/sh
fail() { echo "FAIL: $1"; exit 1; }
T=$(mktemp -d) || fail "mktemp failed"
trap 'rm -rf "$T"' EXIT

cargo build --offline --locked >"$T/build.log" 2>&1 || { tail -40 "$T/build.log"; fail "cargo build failed"; }
BIN=$PWD/target/debug/taik-seed
test -x "$BIN" || fail "binary target/debug/taik-seed missing after build"

cat > "$T/w.sh" <<EOF
#!/bin/sh
touch "$T/worker-called"
exit 1
EOF
chmod +x "$T/w.sh"

cat > "$T/check.js" <<'JS'
const fs = require('fs'), path = require('path'), cp = require('child_process');
const [bin, tmp] = process.argv.slice(2);
const bad = (m) => { console.log('FAIL: ' + m); process.exit(1); };
const env = { ...process.env, TAIK_LEAD_WORKER: tmp + '/w.sh', TAIK_SECOND_WORKER: tmp + '/w.sh' };
delete env.TAIK_AUTO_PROPOSE;
const run = (args, input) => cp.spawnSync(bin, args, { env, input: input ?? '', encoding: 'utf8', timeout: 60000 });
const mk = (n) => {
  const s = fs.realpathSync(fs.mkdtempSync(path.join(tmp, n)));
  fs.mkdirSync(s + '/jobs/job-c', { recursive: true });
  return s;
};
const age = (p, sec) => { const t = Date.now() / 1000 - sec; fs.utimesSync(p, t, t); };
const obs = (s, job, file, text, sec, decision = 'observed') => {
  const d = `${s}/jobs/${job}`;
  fs.mkdirSync(d, { recursive: true });
  const p = `${d}/${file}`;
  fs.writeFileSync(p, `decision=${decision}\nidentity=x\ncontinuation=${text}\nend\n`);
  age(p, sec);
  return p;
};
const reflect = (s, what) => {
  const r = run(['reflect', s, 'job-c']);
  if (r.status !== 0) bad(`${what}: reflect exit ${r.status}: ${r.stdout}${r.stderr}`);
};
const tickets = (s) => fs.existsSync(s + '/requests')
  ? fs.readdirSync(s + '/requests').filter((f) => /^ticket-\d+\.run$/.test(f)).sort() : [];
const noWorker = (what) => { if (fs.existsSync(tmp + '/worker-called')) bad(what + ': a worker was called'); };

// State 1: order, title, origin lines, numbering, taken detection, limit.
{
  const s = mk('s1-');
  const long = 'Größe' + '0123456789'.repeat(12);
  const A = obs(s, 'job-a', 'summary.run', long, 3000);
  obs(s, 'job-b', 'observation.run', 'Zweites Vorhaben kurz', 2000);
  const D = obs(s, 'job-d', 'summary.run', 'Altes Vorhaben schon aufgegriffen', 5000);
  fs.writeFileSync(s + '/jobs/job-c/reflection-alt.run',
    `decision=observed\nidentity=reflection-alt\nintent_from=${D}\noutcome=stop: x\nend\n`);
  const old = s + '/jobs/job-c/reflection-uralt.run';
  fs.writeFileSync(old, 'decision=observed\nidentity=reflection-uralt\noutcome=stop: x\nend\n');
  age(old, 3 * 86400);
  fs.mkdirSync(s + '/requests');
  fs.writeFileSync(s + '/requests/ticket-001.run',
    'decision=observed\nidentity=ticket-001\nintent=Wunsch\nobservation=x\ncontinuation=y\nend\n');
  reflect(s, 'state 1 run 1');
  let t = tickets(s);
  if (t.join() !== 'ticket-001.run,ticket-002.run') bad('run 1 should add exactly ticket-002, found: ' + t.join(' '));
  const lines = fs.readFileSync(s + '/requests/ticket-002.run', 'utf8').split('\n');
  const title = 'intent=Vorhaben der Factory: ' + [...long].slice(0, 80).join('');
  if (!lines.includes(title)) bad('ticket-002 has no title line "' + title + '":\n' + lines.join('\n'));
  if (!lines.includes('observation=' + long)) bad('ticket-002 does not hold the whole intent as observation:\n' + lines.join('\n'));
  if (!lines.includes('origin=reflection')) bad('ticket-002 has no line origin=reflection');
  const from = lines.find((l) => l.startsWith('intent_from='));
  if (!from || !from.endsWith('/jobs/job-a/summary.run')) bad('ticket-002 intent_from should name the oldest open intent job-a/summary.run, got: ' + from);
  if (!lines.includes('identity=ticket-002')) bad('ticket-002 has wrong identity');
  if (lines.filter((l) => l !== '').pop() !== 'end') bad('ticket-002 does not end with "end"');
  reflect(s, 'state 1 run 2');
  t = tickets(s);
  if (t.join() !== 'ticket-001.run,ticket-002.run,ticket-003.run') bad('run 2 should add exactly ticket-003 (A is taken by its request), found: ' + t.join(' '));
  const r3 = fs.readFileSync(s + '/requests/ticket-003.run', 'utf8');
  if (!/\nintent_from=[^\n]*\/jobs\/job-b\/observation\.run\n/.test(r3) || !r3.includes('\nintent=Vorhaben der Factory: Zweites Vorhaben kurz\n'))
    bad('ticket-003 should come from job-b/observation.run:\n' + r3);
  // Limit: reflection-alt + 2 requests = 3 within 24 h.
  obs(s, 'job-e', 'summary.run', 'Drittes Vorhaben', 1000);
  reflect(s, 'state 1 run 3');
  t = tickets(s);
  if (t.length !== 3) bad('limit of three in 24 hours ignored, found: ' + t.join(' '));
}

// State 2: no open intent; the user request stays as it was.
{
  const s = mk('s2-');
  const r = run(['request', s], 'Mein Wunsch\n\nText dazu.\n');
  if (r.status !== 0 || r.stdout.trim() !== 'ticket-001') bad('taik-seed request broke: ' + r.stdout + r.stderr);
  const rec = fs.readFileSync(s + '/requests/ticket-001.run', 'utf8');
  if (/origin=|intent_from=/.test(rec) || !rec.includes('\nintent=Mein Wunsch\n') || !rec.includes('\nobservation=Text dazu.\n'))
    bad('a request of the user changed:\n' + rec);
  obs(s, 'job-x', 'summary.run', 'Schon entschieden', 100, 'accepted');
  reflect(s, 'state 2');
  if (tickets(s).join() !== 'ticket-001.run') bad('no open intent but reflect created: ' + tickets(s).join(' '));
  if (fs.existsSync(s + '/proposals')) bad('reflect created a proposal');
}

// State 3: an intent taken by a request alone is not taken again.
{
  const s = mk('s3-');
  const X = obs(s, 'job-x', 'summary.run', 'Vorhaben mit Wunsch', 100);
  fs.mkdirSync(s + '/requests');
  fs.writeFileSync(s + '/requests/ticket-001.run',
    `decision=observed\nidentity=ticket-001\nintent=Vorhaben der Factory: Vorhaben mit Wunsch\nobservation=Vorhaben mit Wunsch\ncontinuation=y\norigin=reflection\nintent_from=${X}\nend\n`);
  reflect(s, 'state 3');
  if (tickets(s).join() !== 'ticket-001.run') bad('an intent with a request was taken again: ' + tickets(s).join(' '));
}
noWorker('all states');
console.log('scenarios ok');
JS

node "$T/check.js" "$BIN" "$T" || exit 1

cargo test --offline --locked --test reflect >"$T/test.log" 2>&1 || { tail -60 "$T/test.log"; fail "cargo test --test reflect failed"; }
grep -E 'test result: ok\. ([5-9]|[1-9][0-9]+) passed' "$T/test.log" >/dev/null || { tail -20 "$T/test.log"; fail "tests/reflect.rs must run at least five passing tests"; }

cargo clippy --offline --locked --all-targets -- -D warnings >"$T/clippy.log" 2>&1 || { tail -60 "$T/clippy.log"; fail "clippy reports findings"; }

echo "PASS: reflect creates requests through the orchestrator path, limits and tests hold"
