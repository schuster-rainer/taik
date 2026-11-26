#!/bin/sh
set -eu
if ! cargo build --offline --locked; then
  echo "Build fehlgeschlagen"
  exit 1
fi
test -x target/debug/auftragsbericht || { echo "Binary target/debug/auftragsbericht fehlt"; exit 1; }
node - target/debug/auftragsbericht << 'END'
const {spawnSync} = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');
const bin = process.argv[2];
const toml = fs.readFileSync('Cargo.toml', 'utf8');
let section = '';
for (const line of toml.split('\n')) {
  const t = line.trim();
  const m = t.match(/^\[([^\]]+)\]/);
  if (m) { section = m[1]; continue; }
  if ((section === 'dependencies' || section === 'dev-dependencies' || section === 'build-dependencies') && t && !t.startsWith('#')) {
    console.log('Fremde Abhaengigkeit in Cargo.toml: ' + line);
    process.exit(1);
  }
}
const spec = 'a"b\\c\u0001';
function fail(msg, extra) {
  console.log(msg);
  if (extra !== undefined) console.log(String(extra).slice(0, 2000));
  process.exit(1);
}
function write(root, rel, content) {
  const p = path.join(root, rel);
  fs.mkdirSync(path.dirname(p), {recursive: true});
  fs.writeFileSync(p, content);
}
const root = fs.mkdtempSync(path.join(os.tmpdir(), 'ab-'));
write(root, 'job-a/round-01.run', 'decision=pending\nworker_cost_usd=1.25\nsession_listed_usd=0.50\n');
write(root, 'job-a/round-02.run', 'decision=accepted\nworker_cost_usd=2.00\n');
write(root, 'job-a/notes.txt', 'decision=rejected\nworker_cost_usd=9\n');
write(root, 'job-b/round-01.run', 'decision=rejected\nworker_cost_usd=0.50\nworker_cost_usd=nope\n');
write(root, 'job-b/round-1.run', 'decision=accepted\nworker_cost_usd=9.99\n');
write(root, 'job-z/meta.run', 'worker_cost_usd=abc\n');
write(root, 'job-"q/round-01.run', 'decision=a"b\\c\u0001\nworker_cost_usd=0.25\n');
write(root, 'skip-me/round-01.run', 'decision=accepted\nworker_cost_usd=5\n');
write(root, 'loose.run', 'worker_cost_usd=8\n');
const empty = fs.mkdtempSync(path.join(os.tmpdir(), 'ab-empty-'));
function run(args) {
  const r = spawnSync(bin, args, {encoding: 'buffer'});
  if (r.error) fail('Programmstart fehlgeschlagen: ' + r.error.message);
  return r;
}
function keys(o) { return Object.keys(o).join(','); }
function checkJson(buf, jobs, gesamt, label) {
  const s = buf.toString('utf8');
  if (!s.endsWith('\n') || s.slice(0, -1).includes('\n')) fail(label + ': JSON ist nicht genau eine Zeile plus Zeilenende', s);
  const line = s.slice(0, -1);
  if (/\d\.\d{3}/.test(line)) fail(label + ': mehr als zwei Nachkommastellen', line);
  let obj;
  try { obj = JSON.parse(line); } catch (e) { fail(label + ': JSON nicht parsebar: ' + e.message, line); }
  if (!obj || typeof obj !== 'object' || Array.isArray(obj)) fail(label + ': kein Objekt');
  if (keys(obj) !== 'auftraege,gesamt') fail(label + ': Schluessel oben ' + keys(obj));
  if (!Array.isArray(obj.auftraege)) fail(label + ': auftraege keine Liste');
  if (obj.auftraege.length !== jobs.length) fail(label + ': Anzahl Auftraege ' + obj.auftraege.length);
  for (let i = 0; i < jobs.length; i++) {
    const g = obj.auftraege[i];
    const e = jobs[i];
    if (!g || keys(g) !== 'name,runden,entscheidung,kosten') fail(label + ': Schluessel Auftrag', JSON.stringify(g));
    if (g.name !== e.name) fail(label + ': Name', JSON.stringify(g.name));
    if (g.runden !== e.runden || !Number.isInteger(g.runden)) fail(label + ': Runden ' + e.name, String(g.runden));
    if (g.entscheidung !== e.entscheidung) fail(label + ': Entscheidung ' + e.name, JSON.stringify(g.entscheidung));
    if (typeof g.kosten !== 'number' || g.kosten !== e.kosten) fail(label + ': Kosten ' + e.name, String(g.kosten));
  }
  const ge = obj.gesamt;
  if (!ge || keys(ge) !== 'auftraege,angenommen,kosten') fail(label + ': Schluessel gesamt', JSON.stringify(ge));
  if (ge.auftraege !== gesamt[0] || !Number.isInteger(ge.auftraege)) fail(label + ': gesamt.auftraege', String(ge.auftraege));
  if (ge.angenommen !== gesamt[1] || !Number.isInteger(ge.angenommen)) fail(label + ': gesamt.angenommen', String(ge.angenommen));
  if (typeof ge.kosten !== 'number' || ge.kosten !== gesamt[2]) fail(label + ': gesamt.kosten', String(ge.kosten));
}
function checkText(buf, expected, label) {
  const exp = Buffer.from(expected, 'utf8');
  if (buf.compare(exp) !== 0) fail(label + ': Textausgabe weicht ab', buf.toString('utf8'));
}
const allJobs = [
  {name: 'job-"q', runden: 1, entscheidung: spec, kosten: 0.25},
  {name: 'job-a', runden: 2, entscheidung: 'accepted', kosten: 3.75},
  {name: 'job-b', runden: 1, entscheidung: 'rejected', kosten: 0.5},
  {name: 'job-z', runden: 0, entscheidung: '-', kosten: 0}
];
const openJobs = [allJobs[0], allJobs[2], allJobs[3]];
const textAll = 'job-"q\t1\ta"b\\c\u0001\t0.25\njob-a\t2\taccepted\t3.75\njob-b\t1\trejected\t0.50\njob-z\t0\t-\t0.00\ngesamt\t4\t1\t4.50\n';
const textOpen = 'job-"q\t1\ta"b\\c\u0001\t0.25\njob-b\t1\trejected\t0.50\njob-z\t0\t-\t0.00\ngesamt\t3\t0\t0.75\n';
function okRun(args) {
  const r = run(args);
  if (r.status !== 0) fail('Status ' + r.status + ' statt 0 fuer ' + JSON.stringify(args), r.stderr.toString('utf8'));
  return r;
}
let r = okRun([root]);
checkText(r.stdout, textAll, 'ohne Optionen');
r = okRun(['--offen', root]);
checkText(r.stdout, textOpen, '--offen');
r = okRun(['--json', root]);
checkJson(r.stdout, allJobs, [4, 1, 4.5], '--json');
r = okRun(['--json', '--offen', root]);
checkJson(r.stdout, openJobs, [3, 0, 0.75], '--json --offen');
r = okRun(['--offen', '--json', root]);
checkJson(r.stdout, openJobs, [3, 0, 0.75], '--offen --json');
r = okRun([empty]);
checkText(r.stdout, 'gesamt\t0\t0\t0.00\n', 'leerer Ordner');
r = okRun(['--json', empty]);
checkJson(r.stdout, [], [0, 0, 0], '--json leer');
r = okRun(['--offen', '--json', empty]);
checkJson(r.stdout, [], [0, 0, 0], '--offen --json leer');
function assertUsage(args) {
  const u = run(args);
  const both = Buffer.concat([u.stdout, u.stderr]).toString('utf8');
  if (u.status !== 2) fail('Status ' + u.status + ' statt 2 fuer ' + JSON.stringify(args), both);
  if (!both.split(/\r?\n/).includes('Aufruf: auftragsbericht ORDNER')) fail('Aufrufzeile fehlt fuer ' + JSON.stringify(args), both);
}
assertUsage([]);
assertUsage(['--json']);
assertUsage(['--json', '--offen']);
assertUsage(['--foo', root]);
assertUsage(['--json', '--foo', root]);
assertUsage(['--json', root, root]);
r = run([path.join(root, 'kein-ordner')]);
if (r.status !== 1) fail('fehlender Ordner: Status ' + r.status + ' statt 1', r.stderr.toString('utf8'));
r = run([path.join(root, 'loose.run')]);
if (r.status !== 1) fail('Datei statt Ordner: Status ' + r.status + ' statt 1', r.stderr.toString('utf8'));
END
echo "PASS: JSON-Ausgabe und bisherige Textausgabe geprüft"
