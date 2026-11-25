import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

export const artifact = 'taik-stats.bundle.txt';

const candidate = fs.readFileSync(process.argv[2], 'utf8');
assert.ok(Buffer.byteLength(candidate) <= 96 * 1024, 'Kandidat ist größer als 96 KiB');

const lines = candidate.split('\n');
const header = /^=== (.+) ===\r?$/;
assert.ok(header.test(lines[0]), 'Erste Zeile muss eine Kopfzeile "=== pfad ===" sein');
const files = new Map();
let cur = null;
for (const l of lines) {
  const m = header.exec(l);
  if (m) {
    cur = m[1].trim();
    assert.ok(!files.has(cur), `Pfad doppelt im Bündel: ${cur}`);
    files.set(cur, []);
  } else {
    files.get(cur).push(l.replace(/\r$/, ''));
  }
}
const content = new Map([...files].map(([p, ls]) => [p, ls.join('\n') + '\n']));
for (const p of content.keys()) {
  assert.ok(p === 'Cargo.toml' || /^src\/([A-Za-z0-9_]+\/)*[A-Za-z0-9_]+\.rs$/.test(p),
    `Unerlaubter Pfad im Bündel: ${p}`);
}
assert.ok(content.has('Cargo.toml'), 'Cargo.toml fehlt');
assert.ok(content.has('src/main.rs'), 'src/main.rs fehlt');
const extraRs = [...content.keys()].filter(p => p.endsWith('.rs') && p !== 'src/main.rs');
assert.ok(extraRs.length >= 1, 'Mindestens eine weitere .rs-Datei unter src/ ist nötig');

// Cargo.toml: minimal parse
{
  let section = '';
  const pkg = {};
  const depEntries = [];
  for (const raw of content.get('Cargo.toml').split('\n')) {
    const l = raw.replace(/#.*$/, '').trim();
    if (!l) continue;
    const s = /^\[\[?\s*([^\]]+?)\s*\]\]?$/.exec(l);
    if (s) {
      section = s[1];
      assert.ok(!/^bin$/.test(section) || !l.startsWith('[['), 'Cargo.toml darf keinen [[bin]]-Abschnitt haben');
      continue;
    }
    const kv = /^([A-Za-z0-9_.-]+)\s*=\s*(.+)$/.exec(l);
    if (section === 'package' && kv) pkg[kv[1]] = kv[2].trim();
    if (/(^|\.)(dev-|build-)?dependencies(\.|$)/.test(section)) depEntries.push(l);
  }
  assert.equal(pkg.name, '"taik-stats"', 'Cargo.toml: [package] name muss "taik-stats" sein');
  assert.equal(pkg.version, '"0.1.0"', 'Cargo.toml: [package] version muss "0.1.0" sein');
  assert.equal(pkg.edition, '"2021"', 'Cargo.toml: [package] edition muss "2021" sein');
  assert.deepEqual(depEntries, [], 'Cargo.toml darf keine Abhängigkeiten enthalten');
}

const W = path.resolve('taik-work');
fs.rmSync(W, { recursive: true, force: true });
fs.mkdirSync(W, { recursive: true });
function writeTree(dir, skip) {
  for (const [p, c] of content) {
    if (p === skip) continue;
    const f = path.join(dir, p);
    fs.mkdirSync(path.dirname(f), { recursive: true });
    fs.writeFileSync(f, c);
  }
}
function run(cmd, args, opts = {}) {
  return spawnSync(cmd, args, { encoding: 'utf8', timeout: 12000, ...opts });
}

const binDir = path.join(process.env.HOME || '', '.cargo', 'bin');
const rustc = path.join(binDir, 'rustc');
assert.ok(fs.existsSync(rustc), `rustc nicht gefunden unter ${rustc}`);

const full = path.join(W, 'full');
writeTree(full, null);
fs.mkdirSync(path.join(W, 'bin'));
const exe = path.join(W, 'bin', 'taik-stats');
const t0 = Date.now();
const b = run(rustc, ['--edition', '2021', '-O', '-D', 'warnings', '-F', 'unsafe_code',
  '--crate-name', 'taik_stats', '--crate-type', 'bin', '-o', exe, path.join(full, 'src', 'main.rs')]);
assert.equal(b.status, 0, `rustc-Übersetzung fehlgeschlagen:\n${(b.stderr || '').slice(0, 3000)}${b.error ? String(b.error) : ''}`);

// Each extra module must be needed
extraRs.forEach((p, i) => {
  const d = path.join(W, `rm-${i}`);
  writeTree(d, p);
  const r = run(rustc, ['--edition', '2021', '--emit=metadata', '--crate-name', 'taik_stats',
    '--crate-type', 'bin', '-o', path.join(d, 'out.rmeta'), path.join(d, 'src', 'main.rs')]);
  assert.notEqual(r.status, 0, `Ohne ${p} lässt sich src/main.rs noch übersetzen; die Datei wird nicht gebraucht`);
});

// Test data
const data = path.join(W, 'data');
fs.mkdirSync(data);
const recs = {
  'a.run': 'identity=a\ndecision=accepted\nworker=claude-cli\nworker_cost_usd=0.400\ngate_ms=1200\nnote=decision=rejected worker=local gate_ms=99999999\nend\n',
  'b.run': 'decision=accepted\nintent=x=y=z\nworker=claude-cli\nworker_cost_usd=0.250\ngate_ms=13822\nend\n',
  'c.run': 'decision=rejected\nworker=codex-attached\nworker_cost_usd=0.500\ngate_ms=99999.5\nend\n',
  'd.run': 'decision=accepted\nworker=local\ngate_ms=800\nend\n',
  'e.run': 'decision=observed\nsummary=worker=fake gate_ms=50000 worker_cost_usd=9\ngate_ms=abc\nend\n',
  'f.run': 'decision=rejected\nworker_cost_usd=0.1\nend\n',
  'g.run': 'identity=g\nworker_cost_usd=0.2\ngate_ms=\nend\n',
};
for (const [n, c] of Object.entries(recs)) fs.writeFileSync(path.join(data, n), c);

const exp1 = 'records=5\naccepted=3\nrejected=1\nother=1\ncost_usd=1.150\ngate_ms_max=13822\nworkers=claude-cli:2,codex-attached:1,local:1\n';
const exp2 = 'records=2\naccepted=0\nrejected=1\nother=1\ncost_usd=0.300\ngate_ms_max=0\nworkers=\n';

function checkBinary(bin, label) {
  const r1 = run(bin, ['a.run', 'b.run', 'c.run', 'd.run', 'e.run'], { cwd: data, timeout: 3000 });
  assert.equal(r1.status, 0, `${label}: Exit-Status bei fünf gültigen Protokollen muss 0 sein, war ${r1.status}; stderr: ${r1.stderr}`);
  assert.equal(r1.stdout, exp1, `${label}: falsche Ausgabe für fünf Protokolle`);

  const r1b = run(bin, ['e.run', 'd.run', 'c.run', 'b.run', 'a.run'], { cwd: data, timeout: 3000 });
  assert.equal(r1b.stdout, exp1, `${label}: Ausgabe hängt von der Reihenfolge der Dateien ab`);

  const r2 = run(bin, ['f.run', 'g.run'], { cwd: data, timeout: 3000 });
  assert.equal(r2.status, 0, `${label}: Exit-Status muss 0 sein, war ${r2.status}`);
  assert.equal(r2.stdout, exp2, `${label}: falsche Ausgabe ohne Worker (cost_usd 0.300, gate_ms_max 0, workers leer erwartet)`);

  const r3 = run(bin, [], { cwd: data, timeout: 3000 });
  assert.equal(r3.status, 2, `${label}: ohne Argumente muss der Exit-Status 2 sein, war ${r3.status}`);
  assert.equal(r3.stdout, '', `${label}: ohne Argumente darf nichts auf stdout stehen`);

  const missing = 'fehlt-xyz-404.run';
  const r4 = run(bin, ['a.run', missing, 'b.run'], { cwd: data, timeout: 3000 });
  assert.equal(r4.status, 2, `${label}: bei unlesbarer Datei muss der Exit-Status 2 sein, war ${r4.status}`);
  assert.equal(r4.stdout, '', `${label}: bei unlesbarer Datei darf nichts auf stdout stehen`);
  assert.ok((r4.stderr || '').includes(missing), `${label}: stderr muss den Pfad der unlesbaren Datei nennen`);
}

checkBinary(exe, 'rustc-Build');

let cargoNote = 'cargo nicht vorhanden, übersprungen';
const cargo = path.join(binDir, 'cargo');
if (fs.existsSync(cargo)) {
  const left = 14000 - (Date.now() - t0);
  if (left > 4000) {
    const target = path.join(W, 'target');
    const c = run(cargo, ['build', '--offline', '--quiet', '--manifest-path', path.join(full, 'Cargo.toml')], {
      cwd: full,
      timeout: left - 1500,
      env: { ...process.env, CARGO_HOME: path.join(W, 'cargo-home'), CARGO_TARGET_DIR: target,
        RUSTFLAGS: '-D warnings -F unsafe_code', PATH: binDir + ':' + (process.env.PATH || '') },
    });
    assert.ok(!c.error, `cargo build abgebrochen: ${c.error}`);
    assert.equal(c.status, 0, `cargo build --offline fehlgeschlagen:\n${(c.stderr || '').slice(0, 3000)}`);
    const cexe = path.join(target, 'debug', 'taik-stats');
    assert.ok(fs.existsSync(cexe), 'cargo build erzeugte kein Binärprogramm taik-stats');
    checkBinary(cexe, 'cargo-Build');
    cargoNote = 'cargo build ok';
  } else {
    cargoNote = 'cargo aus Zeitgründen übersprungen';
  }
}

console.log(`PASS: taik-stats aus ${content.size} Dateien übersetzt, alle Module nötig, Zusammenfassungen und Fehlerausgänge korrekt (${cargoNote})`);
