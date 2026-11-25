// Gate for a Rust command-line tool delivered as a one-file bundle. Compiles and runs the candidate.
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { spawnSync } from 'node:child_process';

const text = readFileSync(process.argv[2], 'utf8');
const parts = text.split(/^=== (.+) ===\n/m);
assert.equal(parts[0].trim(), '', 'the candidate starts with a "=== main.rs ===" line');
assert.equal(parts.length, 3, 'exactly one file in the bundle');
assert.equal(parts[1], 'main.rs', 'the file is main.rs');
const source = parts[2];
assert.doesNotMatch(source, /\bunsafe\b|std::process|std::net|include_(?:str|bytes)!|extern\s+crate|#!\[feature/, 'no unsafe code, processes, network or includes');
mkdirSync('build'); mkdirSync('data');
writeFileSync('build/main.rs', source);
const rustc = `${process.env.HOME}/.cargo/bin/rustc`;
const built = spawnSync(rustc, ['--edition', '2021', '-O', '-D', 'warnings', 'build/main.rs', '-o', 'build/taik-stats'], { encoding: 'utf8', timeout: 12000 });
assert.equal(built.status, 0, `the program compiles without warnings: ${(built.stderr || built.error?.message || '').split('\n').slice(0, 6).join(' | ')}`);

const record = (fields) => Object.entries(fields).map(([k, v]) => `${k}=${v}`).join('\n') + '\nend\n';
writeFileSync('data/a.run', record({ decision: 'accepted', kind: 'software', worker: 'claude-cli', worker_cost_usd: '0.109856', gate_ms: '91', checks: 'PASS: x=y' }));
writeFileSync('data/b.run', record({ decision: 'rejected', kind: 'software', worker: 'claude-cli', worker_cost_usd: '0.0402', gate_ms: '13822', checks: 'FAIL: decision=accepted inside a value' }));
writeFileSync('data/c.run', record({ decision: 'accepted', kind: 'search', worker: 'local' }));
writeFileSync('data/d.run', record({ decision: 'observed', identity: 'summary', gate_ms: 'n/a' }));
writeFileSync('data/e.run', record({ decision: 'accepted', kind: 'software', worker: 'codex-attached', worker_cost_usd: '1' }));
const run = (...args) => spawnSync('build/taik-stats', args, { encoding: 'utf8', timeout: 3000 });

const all = run('data/a.run', 'data/b.run', 'data/c.run', 'data/d.run', 'data/e.run');
assert.equal(all.status, 0, `exit 0 for readable records: ${all.stderr}`);
assert.equal(all.stdout, 'records=5\naccepted=3\nrejected=1\nother=1\ncost_usd=1.150\ngate_ms_max=13822\nworkers=claude-cli:2,codex-attached:1,local:1\n', 'summary of five records');
const one = run('data/c.run');
assert.equal(one.stdout, 'records=1\naccepted=1\nrejected=0\nother=0\ncost_usd=0.000\ngate_ms_max=0\nworkers=local:1\n', 'summary of a record without cost or gate time');
const none = run();
assert.equal(none.status, 2, 'exit 2 without arguments');
assert.equal(none.stdout, '', 'nothing on stdout without arguments');
const missing = run('data/a.run', 'data/missing.run');
assert.equal(missing.status, 2, 'exit 2 for an unreadable file');
assert(missing.stderr.includes('data/missing.run'), 'the unreadable file is named on stderr');
assert.equal(missing.stdout, '', 'no partial summary when a file is unreadable');
console.log(`PASS: compiled without warnings, five-record summary, empty-field summary and both error exits (${source.split('\n').length} lines of Rust)`);
