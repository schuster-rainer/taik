import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

assert.ok(process.argv[2], 'Pfad zum Kandidaten fehlt');
const doc = fs.readFileSync(process.argv[2], 'utf8');
assert.ok(Buffer.byteLength(doc) <= 96 * 1024, 'Kandidat ist größer als 96 KiB');
const marks = [...doc.matchAll(/^=== (.+) ===$/gm)];
assert.deepEqual(marks.map(x => x[1]), ['main.rs'], 'Bundle muss genau main.rs enthalten');
assert.equal(marks[0].index, 0, 'Erste Zeile muss === main.rs === sein');
const main = doc.slice(marks[0].index + marks[0][0].length).replace(/^\r?\n/, '');

const T = [
  ['', '', 0],
  ['print 42;', '42\n', 0],
  ['let x = 5; print x;', '5\n', 0],
  ['let _a1 = 3; print _a1;', '3\n', 0],
  ['print 007;', '7\n', 0],
  ['print 2 + 3 * 4;', '14\n', 0],
  ['print (2 + 3) * 4;', '20\n', 0],
  ['print 10 - 3 - 2;', '5\n', 0],
  ['print 20 / 3 / 2;', '3\n', 0],
  ['print -5 + 2;', '-3\n', 0],
  ['print --5;', '5\n', 0],
  ['print 7 % 4;', '3\n', 0],
  ['print -7 % 4;', '-3\n', 0],
  ['print 7 % -4;', '3\n', 0],
  ['print -7 / 2;', '-3\n', 0],
  ['print 7 / -2;', '-3\n', 0],
  ['print 1 + 2 * 3 - 4 / 2;', '5\n', 0],
  ['let\nx=1;\nprint x;', '1\n', 0],
  ['let x = 1;', '', 0],
  ['print 1; print 2;', '1\n2\n', 0],
  ['let x = 1; let x = 2;', 'ERROR 1:16 DUPLICATE\n', 1],
  ['print y;', 'ERROR 1:7 UNDEFINED\n', 1],
  ['let x = x;', 'ERROR 1:9 UNDEFINED\n', 1],
  ['print 1 / 0;', 'ERROR 1:9 DIV_ZERO\n', 1],
  ['print 5 % 0;', 'ERROR 1:9 DIV_ZERO\n', 1],
  ['print 9223372036854775807 + 1;', 'ERROR 1:27 OVERFLOW\n', 1],
  ['print -9223372036854775807 - 2;', 'ERROR 1:28 OVERFLOW\n', 1],
  ['print 3037000500 * 3037000500;', 'ERROR 1:18 OVERFLOW\n', 1],
  ['print (-9223372036854775807 - 1) / -1;', 'ERROR 1:34 OVERFLOW\n', 1],
  ['print (-9223372036854775807 - 1) % -1;', 'ERROR 1:34 OVERFLOW\n', 1],
  ['print -(-9223372036854775807 - 1);', 'ERROR 1:7 OVERFLOW\n', 1],
  ['print 9223372036854775808;', 'ERROR 1:7 LITERAL_RANGE\n', 1],
  ['print 1', 'ERROR 1:8 SYNTAX\n', 1],
  ['print ;', 'ERROR 1:7 SYNTAX\n', 1],
  ['print 1 $;', 'ERROR 1:9 BAD_CHAR\n', 1],
  ['print (1;', 'ERROR 1:9 SYNTAX\n', 1],
  ['print 1;\nprint 2 / 0;', '1\nERROR 2:9 DIV_ZERO\n', 1],
  ['print 1; print ;', 'ERROR 1:16 SYNTAX\n', 1],
  ['let x = 1; let x = 1 / 0;', 'ERROR 1:22 DIV_ZERO\n', 1],
  ['let print = 1;', 'ERROR 1:5 SYNTAX\n', 1],
];

const dir = fs.mkdtempSync(path.join(process.cwd(), 'taik-'));
try {
  fs.writeFileSync(path.join(dir, 'main.rs'), main);
  const rustc = process.env.HOME + '/.cargo/bin/rustc';
  assert.ok(fs.existsSync(rustc), 'rustc nicht gefunden');
  const bin = path.join(dir, 'prog');
  const c = spawnSync(rustc, ['--edition=2021', '-O', 'main.rs', '-o', 'prog'], { cwd: dir, encoding: 'utf8', timeout: 10000 });
  assert.equal(c.status, 0, 'rustc ist fehlgeschlagen:\n' + (c.stderr || ''));
  for (const [input, expected, code] of T) {
    const r = spawnSync(bin, [], { input, encoding: 'utf8', timeout: 1000 });
    assert.equal(r.status, code, `Exit-Code für ${JSON.stringify(input)} ist ${r.status}, erwartet ${code}\nstdout=${JSON.stringify(r.stdout)}\nstderr=${JSON.stringify(r.stderr)}`);
    assert.equal(r.stdout, expected, `Ausgabe für ${JSON.stringify(input)} ist ${JSON.stringify(r.stdout)}, erwartet ${JSON.stringify(expected)}`);
  }
  console.log(`PASS: ${T.length} Verhaltensprüfungen bestanden`);
} finally {
  fs.rmSync(dir, { recursive: true, force: true });
}
