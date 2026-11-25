# Gate for the similarity job. Pinned per job, written before the candidate exists.
# The installed taik-seed runs this in the checkout with TAIK_CANDIDATE_OUT set.
set -eu
cargo test --offline --locked
cargo clippy --offline --locked --all-targets -- -D warnings
cargo build --release --offline --locked
cp target/release/taik-seed "$TAIK_CANDIDATE_OUT"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cat > "$work/check.mjs" <<'JS'
// Gate for the similarity search in the ticket store. Pinned per job, written before the candidate.
// It reads the candidate binary from TAIK_CANDIDATE_OUT, as tools/backend/gate.sh does.
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, writeFileSync, mkdirSync, existsSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

const binary = process.env.TAIK_CANDIDATE_OUT;
assert(binary && existsSync(binary), 'the candidate binary is missing');
const root = mkdtempSync(join(tmpdir(), 'taik-similar-'));
const state = join(root, 'state');
const inbox = join(state, 'inbox');
mkdirSync(inbox, { recursive: true });
const store = join(state, 'tickets.sqlite');

const ticket = (name, title, text) => writeFileSync(join(inbox, name + '.md'), `# ${title}\n\n${text}\n`);
ticket('t-01', 'Scrollbar im Ansichtsfenster', 'Die Scrollbar der Ansicht ist 9 px breit und ihr Daumen ist gemischt.');
ticket('t-02', 'Kontrast der Flaggen', 'Die Flagge „Frage" hat Kontrast 5,12 im hellen Bild.');
ticket('t-03', 'Paralleler Durchsatz', 'Drei Aufträge laufen parallel, sechs Aufträge brauchen 249 s.');
ticket('t-04', 'Tickets im Board', 'Das Board zeigt die Tickets nach Flagge; ein Klick wählt den Auftrag.');

const run = (args) => spawnSync(binary, args, { encoding: 'utf8' });
const lines = (result) => result.stdout.split('\n').filter(line => line.trim());

// 1. Ingest fills the store.
let result = run(['similar', state, '--ingest']);
assert.equal(result.status, 0, `ingest failed: ${result.stderr}`);
assert(existsSync(store), 'ingest did not create STATE/tickets.sqlite');
const query = (text) => {
  const mine = run(['similar', state, text]);
  assert.equal(mine.status, 0, `search failed: ${mine.stderr}`);
  return lines(mine).map(line => line.split(/\s+/)[1]);
};

// 2. The two tickets that share words come back, closest first.
let hits = query('Scrollbar Daumen Kontrast');
assert(hits.includes('t-01'), `the scrollbar ticket is missing: ${hits}`);
assert(hits.includes('t-02'), `the contrast ticket is missing: ${hits}`);
assert.equal(new Set(hits).size, hits.length, 'a ticket is listed twice');

// 3. A word of one ticket brings that ticket first.
assert.equal(query('Durchsatz')[0], 't-03', 'the closest ticket is not first');

// 4. Stable order.
assert.deepEqual(query('Tickets Board Flagge'), query('Tickets Board Flagge'), 'the order is not stable');

// 5. Nothing found is not an error.
result = run(['similar', state, 'zzzqqq']);
assert.equal(result.status, 0, 'a query without a match failed');
assert.equal(lines(result).length, 0, 'a query without a match printed something');

// 6. Errors are clear.
assert.equal(run(['similar', state, '']).status, 2, 'an empty query is not refused');
assert.equal(run(['similar', state, 'Board', '--limit', '0']).status, 2, 'a limit of zero is not refused');

// 7. Limit.
assert(lines(run(['similar', state, 'Tickets', '--limit', '1'])).length <= 1, 'the limit is not applied');

// 8. A changed ticket is replaced, an unchanged one is skipped, and a removed one stays.
writeFileSync(join(inbox, 't-04.md'), '# Tickets im Board\n\nEin geänderter Text über Flaggen und Board.\n');
rmSync(join(inbox, 't-03.md'));
result = run(['similar', state, '--ingest']);
assert.equal(result.status, 0, `second ingest failed: ${result.stderr}`);
assert(query('geänderter').includes('t-04'), 'the changed ticket was not replaced');
assert(query('Durchsatz').includes('t-03'), 'a removed ticket disappeared from the store');

// 9. Text with quotes, a newline and an umlaut survives the round trip.
ticket('t-05', 'Sonderzeichen', 'Ein „Zitat", ein Zeilenumbruch\nund ein Umlaut: äöü.');
run(['similar', state, '--ingest']);
assert(query('Sonderzeichen').includes('t-05'), 'the special-character ticket is missing');
assert(query('äöü').includes('t-05'), 'the umlaut is not searchable');

console.log('PASS: similarity search stores the inbox, ranks the closest ticket first, keeps a stable order, honours the limit, refuses bad input and survives special characters');
JS
node "$work/check.mjs"
echo "PASS: Rust tests, Clippy with warnings denied, release build, and the similarity search behaves"
