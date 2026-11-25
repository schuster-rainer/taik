// Independent gate for the ticket board in the Factory view. Pinned per job before the candidate exists.
// The ticket logic is a pure function in the fragment, so this gate runs it in Node on a fixed snapshot.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const html = readFileSync(process.argv[2], 'utf8');
const count = (pattern) => (html.match(pattern) ?? []).length;
const css = html.slice(0, html.indexOf('</style>'));

// 1. The fragment contract is unchanged.
assert.match(html, /^<style>/, 'the viewer is a fragment that starts with its style sheet');
assert.equal(count(/\/\*FACTORY_DATA\*\/null/g), 1, 'exactly one FACTORY_DATA placeholder');
assert.equal(count(/window\.updateFactory\s*=/g), 1, 'one updateFactory entry point');
assert.equal(count(/id="taik-board"/g), 1, 'one board element');
for (const hook of ['id="taik-board-waiting"', 'id="taik-board-open"', 'id="taik-board-run"', 'id="taik-board-decision"',
  'commentTicket']) {
  assert(html.includes(hook), `board hook ${hook}`);
}
assert(html.includes('approveProposal'), 'the approve path is kept');

// 2. Every ticket comes from the snapshot that is already there.
assert.doesNotMatch(html, /fetch\(|XMLHttpRequest|WebSocket/, 'the board adds no data source');
assert.doesNotMatch(html, /https?:\/\/|@import|<link\b/i, 'no network or external resource');
assert(html.includes('boardTickets(snapshot)'), 'the board is drawn from the snapshot');

// 3. The ticket logic is pure and marked, so this gate can run it.
const block = html.match(/\/\*BOARD_BEGIN\*\/([\s\S]*?)\/\*BOARD_END\*\//)?.[1];
assert(block, 'the board logic sits between BOARD_BEGIN and BOARD_END');
assert.equal(count(/\/\*BOARD_BEGIN\*\//g), 1, 'one board logic block');
assert.doesNotMatch(block, /document|window\.|innerHTML|querySelector/, 'the board logic touches no DOM');
const boardTickets = new Function(block + '\nreturn boardTickets;')();

// 4. A ticket appears in exactly one column, and the columns follow the table in the brief.
const round = (decision, advance = 'false') => ({ fields: { decision, advance } });
const job = (overrides) => ({ id: 'job-x', cost: 0, baseline: { fields: { kind: 'software' } }, rounds: [], sourceActive: false,
  sourceQueued: false, progress: {}, ...overrides });
const snapshot = {
  proposals: [{ name: 'vorschlag-1', intent: 'Ein Vorschlag', origin: 'job-x' }],
  jobs: [
    job({ id: 'job-a', sourceActive: true }),
    job({ id: 'job-b', rounds: [round('rejected')] }),
    job({ id: 'job-c', rounds: [round('accepted')] }),
    job({ id: 'job-d', sourceQueued: true }),
    job({ id: 'job-e', rounds: [round('accepted', 'true')] }),
  ],
};
const columns = boardTickets(snapshot);
const all = [...columns.open, ...columns.run, ...columns.decision];
const keys = all.map(ticket => ticket.key);
assert.equal(new Set(keys).size, keys.length, `a ticket is listed twice: ${keys}`);
assert(columns.open.some(t => t.proposal === 'vorschlag-1' && t.flag === 'Entscheidung'), 'a waiting proposal is not an Entscheidung ticket');
assert(columns.open.some(t => t.job === 'job-b' && t.flag === 'Frage'), 'a rejected round does not ask a Frage');
assert(columns.open.some(t => t.job === 'job-d' && t.flag === 'Antwort'), 'a queued revision does not ask for an Antwort');
assert(columns.run.some(t => t.job === 'job-a' && t.flag === 'In Arbeit'), 'an active software job is not In Arbeit');
assert(columns.run.some(t => t.job === 'job-e' && t.flag === 'Warteschlange'), 'a pending next round is not Warteschlange');
assert(columns.decision.some(t => t.job === 'job-c' && t.flag === 'Angenommen'), 'a finished accepted job is not Angenommen');
assert.equal(columns.decision.find(t => t.job === 'job-c')?.title, 'job-c', 'a ticket carries its job title');
assert.deepEqual(boardTickets({ proposals: [], jobs: [] }), { open: [], run: [], decision: [] }, 'an empty snapshot fails');

// 5. Scoped style, shared shapes, no new shadow.
const rules = css.replace(/^<style>/, '');
for (const rule of rules.matchAll(/([^{}]+)\{/g)) {
  for (const part of rule[1].replace(/\/\*[\s\S]*?\*\//g, '').split(',')) {
    const selector = part.trim();
    if (!selector || selector.startsWith('@') || selector === ':root' || selector.startsWith('html') || selector.startsWith('::-webkit')) continue;
    assert(selector.startsWith('#taik-live'), `a rule is not scoped to #taik-live: ${selector}`);
  }
}
const shadows = [...css.matchAll(/box-shadow\s*:\s*([^;}]+)/g)].map(match => match[1].trim());
for (const value of shadows) {
  assert.match(value, /^(?:none|var\(--nm-(?:raised|raised-sm|inset|inset-sm|pressed)\))$/, `shadow outside the token set: ${value}`);
}

// 6. Readable text.
const rem = (value) => { const match = /([\d.]+)rem/.exec(value); return match ? parseFloat(match[1]) * 16 : null; };
const fontSizes = [...css.matchAll(/font-size\s*:\s*([^;}]+)|font\s*:\s*([^;}]+)/g)]
  .flatMap(match => (match[1] ?? match[2]).match(/[\d.]+rem/g) ?? []).map(rem);
assert(Math.min(...fontSizes) >= 0.65, `the smallest text is at least 0.65rem (${Math.min(...fontSizes)})`);

// 7. Readable in both appearances: the tones keep their contrast on the surface.
const pair = (name) => {
  const match = css.match(new RegExp(`${name}\\s*:\\s*light-dark\\(\\s*(#[0-9a-f]{6})\\s*,\\s*(#[0-9a-f]{6})\\s*\\)`, 'i'));
  assert(match, `${name} is a light-dark pair of hex colours`);
  return [match[1], match[2]];
};
const luminance = (hex) => {
  const channel = (i) => { const c = parseInt(hex.slice(i, i + 2), 16) / 255; return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4; };
  return 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
};
const contrast = (a, b) => { const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x); return (hi + 0.05) / (lo + 0.05); };
const base = pair('--nm-base');
for (const name of ['--ink', '--quiet', '--active', '--bad']) {
  const tone = pair(name);
  for (const mode of [0, 1]) {
    assert(contrast(tone[mode], base[mode]) >= 4.5, `${name} ${mode ? 'dark' : 'light'} contrast is below 4.5`);
  }
}

console.log(`PASS: fragment contract, no new data source, ${keys.length} tickets in one column each, flags from the table, scoped style, ${shadows.length} token shadows, readable type and contrast`);
