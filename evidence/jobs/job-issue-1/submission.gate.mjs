import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const candidatePath = process.argv[2];
assert.ok(candidatePath, 'Aufruf: node verify.mjs <kandidat>');
const candidate = fs.readFileSync(candidatePath, 'utf8');
assert.ok(Buffer.byteLength(candidate, 'utf8') <= 96 * 1024, 'Kandidat ist größer als 96 KiB');

const firstNl = candidate.indexOf('\n');
const firstLine = (firstNl === -1 ? candidate : candidate.slice(0, firstNl)).replace(/\r$/, '');
assert.equal(firstLine, '=== main.rs ===', 'Das Bündel muss mit der Zeile "=== main.rs ===" beginnen');
const source = firstNl === -1 ? '' : candidate.slice(firstNl + 1);
assert.ok(source.trim().length > 0, 'main.rs ist leer');
assert.ok(!/^=== .+ ===$/m.test(source), 'Das Bündel darf nur die Datei main.rs enthalten');

const work = fs.mkdtempSync(path.join(process.cwd(), 'run-diff-gate-'));
const src = path.join(work, 'main.rs');
const bin = path.join(work, 'run-diff');
fs.writeFileSync(src, source);

const rustc = path.join(process.env.HOME || '', '.cargo', 'bin', 'rustc');
assert.ok(fs.existsSync(rustc), `rustc nicht gefunden: ${rustc}`);
const compile = spawnSync(rustc, [src, '-o', bin], { cwd: work, encoding: 'utf8', timeout: 10000 });
assert.equal(compile.status, 0, `rustc fehlgeschlagen (Status ${compile.status}):\n${compile.stderr || ''}\n${compile.error || ''}`);

function write(name, text) { fs.writeFileSync(path.join(work, name), text); }
function run(args) { return spawnSync(bin, args, { cwd: work, encoding: 'utf8', timeout: 2000 }); }
function expect(res, status, stdout, stderr, label) {
  assert.equal(res.error, undefined, `${label}: Prozessfehler: ${res.error}`);
  assert.equal(res.status, status, `${label}: Exit-Status ${res.status}, erwartet ${status}; stderr=${JSON.stringify(res.stderr)}`);
  assert.equal(res.stdout, stdout, `${label}: stdout ${JSON.stringify(res.stdout)}, erwartet ${JSON.stringify(stdout)}`);
  assert.equal(res.stderr, stderr, `${label}: stderr ${JSON.stringify(res.stderr)}, erwartet ${JSON.stringify(stderr)}`);
}

write('a1.run', 'alpha=1\nbeta=zwei\nleer=\nend\n');
write('b1.run', 'beta=zwei\nend\nalpha=1\nleer=\n');
expect(run(['a1.run', 'b1.run']), 0, '', '', 'Gleiche Protokolle');

write('a2.run', 'z=letzter\na=alt\nnurA=ja\ngleich=x\nleer=\ndoppelt=erste\ndoppelt=letzte\n');
write('b2.run', 'a=neu\nnurB=da\ngleich=x\nleer=\nz=letzter\n');
expect(run(['a2.run', 'b2.run']), 1,
  'a: A=alt B=neu\ndoppelt: A=letzte\nnurA: A=ja\nnurB: B=da\n',
  '', 'Abweichende Schlüssel');

write('a3.run', 'k=v=1\r\nleer=\r\nohne Gleichheitszeichen\r\n\r\n');
write('b3.run', 'k=v=2\nleer=x\n');
expect(run(['a3.run', 'b3.run']), 1,
  'k: A=v=1 B=v=2\nleer: A= B=x\n',
  '', 'Protokoll-Parsing');

write('b4.run', 'x=1\n');
expect(run(['fehlt.run', 'b4.run']), 2, '', 'fehlt.run\n', 'Nicht lesbare Datei');
expect(run(['fehltA.run', 'fehltB.run']), 2, '', 'fehltA.run\nfehltB.run\n', 'Zwei nicht lesbare Dateien');

console.log('PASS: run-diff erfüllt die Anforderungen');
