#!/bin/sh
set -eu
node <<'NODE'
const { spawnSync } = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');

const root = process.cwd();
const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'taik-adopt-gate-'));
process.on('exit', () => {
  try { fs.rmSync(tmp, { recursive: true, force: true }); } catch (_) {}
});

function fail(msg) {
  console.error('FAIL: ' + msg);
  process.exit(1);
}
function run(cmd, args, opts = {}) {
  const r = spawnSync(cmd, args, { encoding: 'utf8', ...opts });
  if (r.error) fail(`${cmd} konnte nicht gestartet werden: ${r.error.message}`);
  if (r.status !== 0) {
    fail(`${cmd} ${args.join(' ')} schlug fehl (${r.status}): ${((r.stderr || '') + (r.stdout || '')).trim()}`);
  }
  return r;
}
function status(cmd, args, opts = {}) {
  return spawnSync(cmd, args, { encoding: 'utf8', ...opts });
}
function git(cwd, args) {
  return run('git', ['-C', cwd, ...args]).stdout.trim();
}
function write(file, text) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, text);
}
function initRepo(dir) {
  fs.mkdirSync(dir, { recursive: true });
  run('git', ['init', '-q', dir]);
  git(dir, ['config', 'user.email', 'gate@example.invalid']);
  git(dir, ['config', 'user.name', 'Gate']);
  write(path.join(dir, 'file.txt'), 'base\n');
  git(dir, ['add', '.']);
  git(dir, ['commit', '-qm', 'base']);
  return git(dir, ['rev-parse', 'HEAD']);
}
function commit(dir, msg, content) {
  write(path.join(dir, 'file.txt'), content);
  git(dir, ['add', '.']);
  git(dir, ['commit', '-qm', msg]);
  return git(dir, ['rev-parse', 'HEAD']);
}

run('cargo', ['build', '--offline', '--locked'], { cwd: root });
const bin = path.join(root, 'target', 'debug', 'taik-seed');
if (!fs.existsSync(bin)) fail('target/debug/taik-seed fehlt nach dem Build');

const tr = status('cargo', ['test', '--offline', '--locked', '--test', 'adopt'], { cwd: root });
if (tr.status !== 0) {
  fail('cargo test --test adopt ist fehlgeschlagen: ' + ((tr.stderr || '') + (tr.stdout || '')).trim());
}
if (!/test result: ok\. [1-9][0-9]* passed/.test((tr.stdout || '') + (tr.stderr || ''))) {
  fail('tests/adopt.rs enthält keinen bestandenen Test');
}

const readme = fs.readFileSync(path.join(root, 'README.md'), 'utf8');
if (!readme.includes('taik-seed adopt STATE job-NAME')) {
  fail('README.md enthält nicht die Zeile taik-seed adopt STATE job-NAME');
}

function scenario(name, rounds, opts = {}) {
  const state = path.join(tmp, 'state-' + name);
  const origin = path.join(tmp, 'origin-' + name);
  const base = initRepo(origin);
  if (opts.preBranch) git(origin, ['branch', `taik/${name}`, base]);
  const work = path.join(state, 'work', name);
  fs.mkdirSync(path.dirname(work), { recursive: true });
  run('git', ['clone', '-q', origin, work]);

  const jobDir = path.join(state, 'jobs', 'job-' + name);
  fs.mkdirSync(jobDir, { recursive: true });
  if (opts.sessionRepo !== false) write(path.join(jobDir, 'session.repo'), origin + '\n');

  const commits = [];
  for (const r of rounds) {
    const sha = commit(work, `round ${r.n}`, `round ${r.n}\n`);
    commits.push(sha);
    write(path.join(jobDir, `round-${String(r.n).padStart(2, '0')}.run`),
      `decision=${r.decision}\ncommit=${sha}\n`);
  }
  return { state, origin, work, jobDir, base, commits };
}

function adopt(state, job, expectedStatus) {
  const r = status(bin, ['adopt', state, job]);
  if (r.status !== expectedStatus) {
    fail(`adopt ${job}: Status ${r.status}, erwartet ${expectedStatus}; stdout=${(r.stdout || '').trim()}; stderr=${(r.stderr || '').trim()}`);
  }
  return r;
}
function assertNoBranch(origin, branch) {
  const r = status('git', ['-C', origin, 'show-ref', '--verify', '--quiet', `refs/heads/${branch}`]);
  if (r.status === 0) fail(`Zweig ${branch} wurde unerwartet angelegt`);
}

const ok = scenario('demo', [{ n: 1, decision: 'accepted' }]);
const r = adopt(ok.state, 'job-demo', 0);
if ((r.stdout || '').trim() !== 'taik/demo') fail('Erfolg gibt nicht taik/demo aus');
if (git(ok.origin, ['rev-parse', 'refs/heads/taik/demo']) !== ok.commits[0]) {
  fail('Zweig taik/demo zeigt nicht auf den angenommenen Commit');
}
if (git(ok.origin, ['rev-parse', 'HEAD']) !== ok.base) fail('HEAD des Ursprungs-Repositorys wurde bewegt');
if (git(ok.origin, ['status', '--porcelain']) !== '') fail('Arbeitsdateien des Ursprungs-Repositorys wurden geändert');

adopt(ok.state, 'job-demo', 0);
if (git(ok.origin, ['rev-parse', 'refs/heads/taik/demo']) !== ok.commits[0]) {
  fail('Erneuter Aufruf auf denselben Commit hat den Zweig verändert');
}

const lastOk = scenario('lastok', [
  { n: 1, decision: 'rejected' },
  { n: 2, decision: 'accepted' }
]);
adopt(lastOk.state, 'job-lastok', 0);
if (git(lastOk.origin, ['rev-parse', 'refs/heads/taik/lastok']) !== lastOk.commits[1]) {
  fail('Die letzte angenommene Runde wurde nicht verwendet');
}

const lastBad = scenario('lastbad', [
  { n: 1, decision: 'accepted' },
  { n: 2, decision: 'rejected' }
]);
adopt(lastBad.state, 'job-lastbad', 2);
assertNoBranch(lastBad.origin, 'taik/lastbad');

const diff = scenario('diff', [{ n: 1, decision: 'accepted' }], { preBranch: true });
adopt(diff.state, 'job-diff', 2);
if (git(diff.origin, ['rev-parse', 'refs/heads/taik/diff']) !== diff.base) {
  fail('Vorhandener Zweig taik/diff wurde trotz anderem Commit verändert');
}

const noRepo = scenario('norepo', [{ n: 1, decision: 'accepted' }], { sessionRepo: false });
adopt(noRepo.state, 'job-norepo', 2);
assertNoBranch(noRepo.origin, 'taik/norepo');

adopt(path.join(tmp, 'state-missing'), 'job-missing', 2);

console.log('PASS: adopt legt taik/NAME auf den angenommenen Commit, lässt das Ursprungs-Repository sonst unberührt und weist die vier Fälle mit Status 2 ab');
NODE
