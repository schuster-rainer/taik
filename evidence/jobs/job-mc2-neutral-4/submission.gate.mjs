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

  ['let x = 1; set x = 2; print x;', '2\n', 0],
  ['let x = 1; set x = x + 4; print x;', '5\n', 0],
  ['let x = 1; if 1 { set x = 7; } print x;', '7\n', 0],
  ['let x = 1; if 1 { let y = 2; } print y;', '2\n', 0],
  ['if 0 { print 1; } else { print 2; }', '2\n', 0],
  ['if 2 { print 1; }', '1\n', 0],
  ['if 0 { print 1; }', '', 0],
  ['if 1 { print 1; } else { print 2; }', '1\n', 0],
  ['if 1 - 1 { print 1; } else { print 2; }', '2\n', 0],
  ['if 1 < 2 { print 3; }', '3\n', 0],
  ['let i = 0; while i < 3 { print i; set i = i + 1; }', '0\n1\n2\n', 0],
  ['let i = 3; while i { set i = i - 1; } print i;', '0\n', 0],
  ['while 0 { print 1; } print 2;', '2\n', 0],
  ['let i = 0; let s = 0; while i < 4 { set s = s + i; set i = i + 1; } print s;', '6\n', 0],
  ['let n = 5; let f = 1; while n > 0 { set f = f * n; set n = n - 1; } print f;', '120\n', 0],
  ['let x = 0; if 1 { if 0 { set x = 1; } else { set x = 2; } } print x;', '2\n', 0],
  ['let i = 0; while i < 4 { if i % 2 == 0 { print i; } set i = i + 1; }', '0\n2\n', 0],
  ['let i = 0; while i < 2 { if i == 0 { print 10; } else { print 20; } set i = i + 1; }', '10\n20\n', 0],
  ['let x = 1; if 0 { set x = 2; } else { if 1 { set x = 3; } } print x;', '3\n', 0],
  ['if 1 { if 0 { print 1; } else { print 2; } }', '2\n', 0],

  ['print 2 == 2;', '1\n', 0],
  ['print 2 != 2;', '0\n', 0],
  ['print 2 < 3;', '1\n', 0],
  ['print 3 <= 3;', '1\n', 0],
  ['print 4 > 5;', '0\n', 0],
  ['print 5 >= 5;', '1\n', 0],
  ['print -1 < 0;', '1\n', 0],
  ['print 1 < -2;', '0\n', 0],
  ['print 1 + 2 < 4;', '1\n', 0],
  ['print 1 < 2 + 3;', '1\n', 0],
  ['print 1 + 2 * 3 == 7;', '1\n', 0],
  ['print (1 < 2) + 1;', '2\n', 0],
  ['print 1 == 1 + 0;', '1\n', 0],
  ['print 1 != 1 + 0;', '0\n', 0],
  ['print 0 == 1 - 1;', '1\n', 0],
  ['print (1 < 2) < 3;', '1\n', 0],
  ['print 1 < (2 < 3);', '0\n', 0],

  ['set x = 1;', 'ERROR 1:5 UNDEFINED\n', 1],
  ['let x = 1; set y = 2;', 'ERROR 1:16 UNDEFINED\n', 1],
  ['let x = 1; set x = 1 / 0;', 'ERROR 1:22 DIV_ZERO\n', 1],
  ['set x = 1 / 0;', 'ERROR 1:11 DIV_ZERO\n', 1],
  ['if 1 / 0 { print 1; }', 'ERROR 1:6 DIV_ZERO\n', 1],
  ['while 1 / 0 { }', 'ERROR 1:9 DIV_ZERO\n', 1],
  ['let i = 0;\nwhile i < 3 {\n  print i;\n  set i = i + 1;\n  print 10 / (i - 2);\n}', '0\n-10\n1\nERROR 5:12 DIV_ZERO\n', 1],

  ['while 1 { }', 'ERROR 1:1 STEP_LIMIT\n', 1],
  ['print 9; while 1 { }', '9\nERROR 1:10 STEP_LIMIT\n', 1],
  ['let i = 0;\nwhile 1 {\n  set i = i + 1;\n}', 'ERROR 3:3 STEP_LIMIT\n', 1],

  ['let set = 1;', 'ERROR 1:5 SYNTAX\n', 1],
  ['let while = 1;', 'ERROR 1:5 SYNTAX\n', 1],
  ['set x 1;', 'ERROR 1:7 SYNTAX\n', 1],
  ['set = 1;', 'ERROR 1:5 SYNTAX\n', 1],
  ['set x = ;', 'ERROR 1:9 SYNTAX\n', 1],
  ['set x = 1', 'ERROR 1:10 SYNTAX\n', 1],
  ['set x = 1 + ;', 'ERROR 1:13 SYNTAX\n', 1],
  ['if { }', 'ERROR 1:4 SYNTAX\n', 1],
  ['if 1 print 2;', 'ERROR 1:6 SYNTAX\n', 1],
  ['if 1 {', 'ERROR 1:7 SYNTAX\n', 1],
  ['if 1 { print 2; } else print 3;', 'ERROR 1:24 SYNTAX\n', 1],
  ['if 1 { } else', 'ERROR 1:14 SYNTAX\n', 1],
  ['if 1 { } else {', 'ERROR 1:16 SYNTAX\n', 1],
  ['else { }', 'ERROR 1:1 SYNTAX\n', 1],
  ['if 1 { } else { } else { }', 'ERROR 1:19 SYNTAX\n', 1],
  ['while 1 { } else { }', 'ERROR 1:13 SYNTAX\n', 1],
  ['while { }', 'ERROR 1:7 SYNTAX\n', 1],
  ['while 1 print 2;', 'ERROR 1:9 SYNTAX\n', 1],
  ['while 1 {', 'ERROR 1:10 SYNTAX\n', 1],
  ['while 1 { print 2;', 'ERROR 1:19 SYNTAX\n', 1],
  ['print 1 < 2 < 3;', 'ERROR 1:13 SYNTAX\n', 1],
  ['print 1 == 2 == 3;', 'ERROR 1:14 SYNTAX\n', 1],
  ['print 1 < ;', 'ERROR 1:11 SYNTAX\n', 1],
  ['print 1 == ;', 'ERROR 1:12 SYNTAX\n', 1],
  ['print 1 <= ;', 'ERROR 1:12 SYNTAX\n', 1],
  ['print 1 >= ;', 'ERROR 1:12 SYNTAX\n', 1],
  ['print 1 != ;', 'ERROR 1:12 SYNTAX\n', 1],
  ['print 1 = 2;', 'ERROR 1:9 SYNTAX\n', 1],
  ['if 1 { } else if 0 { }', 'ERROR 1:15 SYNTAX\n', 1],
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
