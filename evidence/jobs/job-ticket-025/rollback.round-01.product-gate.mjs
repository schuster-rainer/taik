// TAIK: release tools/release-view.sh
// Independent gate for the Factory viewer fragment. Pinned per job before the candidate exists.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const html = readFileSync(process.argv[2], 'utf8');
const count = (pattern) => (html.match(pattern) ?? []).length;

// 1. Contract with the macOS host and the WebKit design check.
assert.match(html, /^<style>/, 'the viewer is a fragment that starts with its style sheet');
assert.equal(count(/\/\*FACTORY_DATA\*\/null/g), 1, 'exactly one FACTORY_DATA placeholder');
assert.equal(count(/window\.updateFactory\s*=/g), 1, 'one updateFactory entry point');
for (const id of ['taik-live', 'taik-activity', 'taik-preview', 'taik-flow', 'taik-result-viewport', 'taik-draft-viewport',
  'taik-result-source', 'taik-result-diff']) {
  assert.equal(count(new RegExp(`id="${id}"`, 'g')), 1, `static element #${id}`);
}
for (const hook of ['id="taik-appearance"', 'aria-label="Auftragsverlauf"', 'data-round=',
  'class="rounds"', 'class="round', 'taik-appearance', 'openArtifact', 'setAppearance']) {
  assert(html.includes(hook), `viewer hook ${hook}`);
}

// 2. Offline and isolated. Verified artifacts may run scripts; an unverified draft may not.
assert.doesNotMatch(html, /https?:\/\/|@import|<link\b|<object\b|<embed\b|\bfetch\(|XMLHttpRequest|WebSocket/i, 'no network or external resource');
assert.doesNotMatch(html, /url\((?!\s*['"]?data:)/i, 'only data: URLs in styles');
assert(html.includes('sandbox="allow-scripts"'), 'accepted results stay in a script-only sandbox');
assert.match(html, /setAttribute\('sandbox',\s*''\)/, 'the live draft frame gets an empty sandbox');
assert.match(html, /script-src \$\{[^}]+\}/, 'the draft policy switches script-src');

// 3. Neumorphic surfaces without glow: every shadow is a token built from one dark and one light tone.
const css = html.slice(0, html.indexOf('</style>'));
assert.doesNotMatch(css, /text-shadow|drop-shadow|\bfilter\s*:|backdrop-filter/i, 'no glow, blur or filter effects');
const shadows = [...css.matchAll(/box-shadow\s*:\s*([^;}]+)/g)].map(match => match[1].trim());
assert(shadows.length >= 6, 'surfaces use shadows');
for (const value of shadows) {
  assert.match(value, /^(?:none|var\(--nm-(?:raised|raised-sm|inset|inset-sm|pressed)\))$/, `shadow outside the token set: ${value}`);
}
const tokens = Object.fromEntries([...css.matchAll(/(--nm-(?:raised|raised-sm|inset|inset-sm|pressed))\s*:\s*([^;]+);/g)].map(m => [m[1], m[2]]));
for (const name of ['--nm-raised', '--nm-raised-sm', '--nm-inset', '--nm-inset-sm']) {
  const value = tokens[name];
  assert(value, `shadow token ${name}`);
  const layers = value.split(/,(?![^(]*\))/).map(layer => layer.trim());
  assert.equal(layers.length, 2, `${name} pairs two layers`);
  assert(layers.some(layer => layer.endsWith('var(--nm-shade)')) && layers.some(layer => layer.endsWith('var(--nm-light)')),
    `${name} pairs the dark and the light tone`);
  const offsets = layers.map(layer => parseFloat(layer.replace(/^inset\s+/, '')));
  assert(offsets[0] * offsets[1] < 0, `${name} offsets oppose each other`);
  assert.equal(layers.every(layer => layer.startsWith('inset ')), name.includes('inset'), `${name} direction`);
}

// 4. Readable in both appearances: WCAG contrast of the text tokens on the base surface.
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
const measured = [];
for (const [name, minimum] of [['--ink', 7], ['--quiet', 4.5], ['--active', 4.5], ['--good', 4.5], ['--bad', 4.5]]) {
  const tone = pair(name);
  for (const mode of [0, 1]) {
    const ratio = contrast(tone[mode], base[mode]);
    assert(ratio >= minimum, `${name} ${mode ? 'dark' : 'light'} contrast ${ratio.toFixed(2)} < ${minimum}`);
    measured.push(ratio);
  }
}
for (const name of ['--nm-shade', '--nm-light']) {
  const tone = pair(name);
  for (const mode of [0, 1]) {
    const ratio = contrast(tone[mode], base[mode]);
    assert(ratio > 1.04 && ratio < 1.9, `${name} ${mode ? 'dark' : 'light'} stays a same-tone relief (${ratio.toFixed(2)})`);
  }
}
assert.notEqual(base[0], base[1], 'light and dark use different surfaces');

// 5. The .run highlighter is a self-contained, lossless, escaping function.
const block = html.match(/\/\*RUN_HIGHLIGHT_BEGIN\*\/([\s\S]*?)\/\*RUN_HIGHLIGHT_END\*\//)?.[1];
assert(block, 'marked highlightRun block');
const highlightRun = new Function(`${block}; return highlightRun;`)();
const sample = [
  'decision=accepted', 'kind=software', 'procedure=', 'identity=round-04',
  'intent=Prüfe <script>alert(1)</script> & "Zitate"', 'limits=4 rounds; worker 150s; verifier 15s; 96KiB source',
  'gate_sha256=d4feb1f67cf6a0db364dbde521301794bc734be22fab94f6eae09519703760ef',
  'checks=PASS: artifact:true, probes:false, a=b', 'artifact=round-04.html', 'baseline=../job-x/round-02.run',
  'advance=false', 'end', '',
].join('\n');
const marked = highlightRun(sample);
assert.doesNotMatch(marked, /<(?!\/?span\b)/, 'only span markup is produced');
assert.doesNotMatch(marked, /<span(?! class="run-[a-z]+">)/, 'spans carry one run-* class and nothing else');
const plain = marked.replace(/<\/?span[^>]*>/g, '')
  .replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&quot;/g, '"').replace(/&#39;/g, "'").replace(/&amp;/g, '&');
assert.equal(plain, sample, 'highlighting preserves the record text');
const has = (cls, text) => marked.includes(`<span class="run-${cls}">${text}</span>`);
assert(has('key', 'decision') && has('key', 'gate_sha256') && has('key', 'advance'), 'keys are marked');
assert(has('good', 'accepted') && has('good', 'PASS') && has('good', 'true'), 'positive outcomes are marked');
assert(has('bad', 'false'), 'negative outcomes are marked');
assert(has('end', 'end'), 'the terminator is marked');
assert(has('hash', 'd4feb1f67cf6a0db364dbde521301794bc734be22fab94f6eae09519703760ef'), 'digests are marked');
assert(has('path', 'round-04.html') && has('path', '../job-x/round-02.run'), 'record and artifact names are marked');
assert(has('number', '150s') && has('number', '96KiB'), 'quantities are marked');
assert(!marked.includes('<span class="run-key">a</span>'), 'only the first = on a line separates key and value');
assert(highlightRun('decision=rejected\n').includes('<span class="run-bad">rejected</span>'), 'a rejection is marked');
assert.equal(highlightRun('FAIL: x').includes('<span class="run-bad">FAIL</span>'), true, 'a failed check is marked');
for (const cls of ['key', 'good', 'bad', 'end', 'hash', 'path', 'number']) {
  assert(css.includes(`.run-${cls}`), `style for .run-${cls}`);
}
assert.match(html, /highlightRun\(round\.detail\)/, 'the Prüfprotokoll renders through highlightRun');

// 6. Token flow per round: a pure function turns round fields into bar segments.
const flowBlock = html.match(/\/\*TOKEN_FLOW_BEGIN\*\/([\s\S]*?)\/\*TOKEN_FLOW_END\*\//)?.[1];
assert(flowBlock, 'marked tokenFlow block');
const tokenFlow = new Function(`${flowBlock}; return tokenFlow;`)();
const flow = tokenFlow([
  { worker_input_tokens: '1000', worker_output_tokens: '3000', worker_cost_usd: '0.0016', worker_duration_ms: '25000', gate_ms: '90' },
  { worker_output_tokens: '2000', worker_cost_usd: '0.1', worker_duration_ms: '500' },
  { decision: 'accepted' },
  { worker_input_tokens: 'x', worker_output_tokens: '8000', worker_cost_usd: '', worker_duration_ms: '61000' },
]);
assert.equal(flow.length, 4, 'one entry per round');
const near = (value, expected, what) => assert(Math.abs(value - expected) < 0.06, `${what}: ${value} instead of ${expected}`);
assert.deepEqual([flow[0].input, flow[0].output, flow[1].input, flow[1].output, flow[2].input, flow[2].output, flow[3].input, flow[3].output],
  [1000, 3000, 0, 2000, 0, 0, 0, 8000], 'token counts, missing or unreadable values count as 0');
near(flow[0].widthIn, 12.5, 'input width of round 1 relative to the largest round');
near(flow[0].widthOut, 37.5, 'output width of round 1');
near(flow[1].widthOut, 25, 'output width of round 2');
near(flow[3].widthOut, 100, 'the largest round fills the bar');
assert.equal(flow[2].widthIn + flow[2].widthOut, 0, 'a round without tokens has no bar');
near(flow[0].seconds, 25, 'seconds of round 1');
near(flow[1].seconds, 0.5, 'seconds of round 2');
near(flow[0].cost, 0.0016, 'cost of round 1');
assert.equal(flow[3].cost, 0, 'an empty cost counts as 0');
assert.deepEqual(tokenFlow([]), [], 'no rounds, no entries');
assert.deepEqual(tokenFlow([{}]).map(entry => [entry.input, entry.output, entry.widthIn, entry.widthOut]), [[0, 0, 0, 0]], 'no division by zero');
assert(css.includes('.token-flow'), 'style for .token-flow');
assert.match(html, /tokenFlow\(/, 'the history renders through tokenFlow');
assert.match(html, /class="token-flow/, 'the bar is part of the round markup');

// 7. One workpiece pane. The second pane and its duplicate of the work steps are gone.
assert.doesNotMatch(html, /taik-job-content|taik-job-observation|taik-job-viewport|Zum Auftrag/, 'the pane "Zum Auftrag" is removed');
for (const mode of ['view', 'source', 'diff']) assert(html.includes(`data-workpiece-mode="${mode}"`), `mode button ${mode}`);
const pieceBlock = html.match(/\/\*WORKPIECE_BEGIN\*\/([\s\S]*?)\/\*WORKPIECE_END\*\//)?.[1];
assert(pieceBlock, 'marked workpieceState block');
const workpieceState = new Function(`${pieceBlock}; return workpieceState;`)();
const doc = '<!doctype html><p>x</p>', program = '=== main.rs ===\nfn main() {}\n';
const cases = [
  [null, 'view', 'empty', false],
  [{}, 'view', 'empty', false],
  [{ preview: doc }, 'view', 'accepted', false],
  [{ preview: doc }, 'source', 'source', false],
  [{ preview: doc }, 'diff', 'source', false],
  [{ preview: doc, previous: doc }, 'diff', 'diff', false],
  [{ preview: program }, 'view', 'source', false],
  [{ preview: doc, draft: '  <html><body>y</body></html>' }, 'view', 'draft', true],
  [{ preview: doc, draft: '<style>p{}</style><p>z</p>' }, 'view', 'source', true],
  [{ preview: '<style>p{}</style><div id="x"></div>' }, 'view', 'source', false],
  [{ preview: '\n <!DOCTYPE html><p>x</p>' }, 'view', 'accepted', false],
  [{ preview: doc, draft: 'fn main() {}' }, 'view', 'source', true],
  [{ preview: doc, draft: '<html>' }, 'source', 'source', true],
  [{ preview: doc, draft: '<html>' }, 'diff', 'diff', true],
  [{ draft: '<html>' }, 'diff', 'source', true],
  [{ draft: '' }, 'view', 'empty', false],
];
for (const [job, mode, show, unverified] of cases) {
  const state = workpieceState(job, mode);
  assert.equal(state.show, show, `workpieceState(${JSON.stringify(job)}, ${mode}).show`);
  assert.equal(state.unverified, unverified, `workpieceState(${JSON.stringify(job)}, ${mode}).unverified`);
}
assert.match(html, /workpieceState\(/, 'the pane renders through workpieceState');

// 8. Visible activity without glow: motion comes from keyframes and stops for reduced motion.
const keyframes = [...css.matchAll(/@keyframes\s+([\w-]+)/g)].map(match => match[1]);
assert(new Set(keyframes).size >= 4, `at least four animations (found ${new Set(keyframes).size})`);
const reduced = css.match(/@media\s*\(prefers-reduced-motion:\s*reduce\)\s*\{([\s\S]*?\})\s*\}/g) ?? [];
assert(reduced.some(block => /animation:\s*none/.test(block)), 'animations stop for reduced motion');
assert(css.includes('.flow-node.active'), 'the active station has its own style');

// 9. Ticket board. The logic is pure and marked, so this gate can run it.
assert.equal(count(/id="taik-board"/g), 1, 'one board element');
for (const hook of ['id="taik-board-waiting"', 'commentTicket', 'approveProposal', 'boardTickets(snapshot)']) {
  assert(html.includes(hook), `board hook ${hook}`);
}
for (const group of ['open', 'run', 'decision']) {
  assert(html.includes(`id="taik-board-${group}"`) || html.includes('id="taik-board-${'), `board group ${group}`);
}
const boardBlock = html.match(/\/\*BOARD_BEGIN\*\/([\s\S]*?)\/\*BOARD_END\*\//)?.[1];
assert(boardBlock, 'the board logic sits between BOARD_BEGIN and BOARD_END');
assert.doesNotMatch(boardBlock, /document|window\.|innerHTML|querySelector/, 'the board logic touches no DOM');
const { boardTickets, boardTab, ticketTitle } = new Function(boardBlock + '\nreturn { boardTickets, boardTab, ticketTitle };')();
const round = (decision, advance = 'false') => ({ fields: { decision, advance } });
const job = (overrides) => ({ id: 'job-x', cost: 0, baseline: { fields: { kind: 'software' } }, rounds: [], sourceActive: false,
  sourceQueued: false, progress: {}, ...overrides });
const board = boardTickets({
  proposals: [{ name: 'vorschlag-1', intent: '# Ein Vorschlag', origin: 'job-x' }],
  jobs: [
    job({ id: 'job-a', sourceActive: true }),
    job({ id: 'job-b', rounds: [round('rejected')], goal: '# Board view: every waiting thing', cost: 0.0371 }),
    job({ id: 'job-c', rounds: [round('accepted')] }),
    job({ id: 'job-d', sourceQueued: true }),
    job({ id: 'job-e', rounds: [round('accepted', 'true')] }),
  ],
});
const everyTicket = [...board.open, ...board.run, ...board.decision];
assert.equal(everyTicket.length, 6, 'six things, six tickets');
assert.equal(new Set(everyTicket.map(ticket => ticket.key)).size, 6, 'no ticket is listed twice');
assert(board.open.some(t => t.proposal === 'vorschlag-1' && t.flag === 'Freigabe'), 'a waiting proposal asks for approval');
assert(board.open.some(t => t.job === 'job-b' && t.flag === 'Abgelehnt'), 'a rejected round is named as rejected, not as a question');
assert(board.open.some(t => t.job === 'job-d' && t.flag === 'Antwort'), 'a queued revision asks for an Antwort');
assert(board.run.some(t => t.job === 'job-a' && t.flag === 'In Arbeit'), 'an active software job is In Arbeit');
assert(board.run.some(t => t.job === 'job-e' && t.flag === 'Warteschlange'), 'a pending next round is Warteschlange');
assert(board.decision.some(t => t.job === 'job-c' && t.flag === 'Angenommen'), 'a finished accepted job is Angenommen');
assert.equal(board.decision.find(t => t.job === 'job-c')?.title, 'job-c', 'a ticket without a goal carries its job name');
assert.deepEqual(boardTickets({ proposals: [], jobs: [] }), { open: [], run: [], decision: [] }, 'an empty snapshot gives empty groups');

// 10. Titles are plain text, costs read like everywhere else in the viewer.
assert.equal(ticketTitle('# Board view'), 'Board view', 'a leading heading mark is dropped');
assert.equal(ticketTitle('##   Zwei  '), 'Zwei', 'several marks and outer space are dropped');
assert.equal(ticketTitle('Nummer # 5'), 'Nummer # 5', 'a mark inside the text stays');
assert.equal(ticketTitle(undefined), '', 'no text gives an empty title');
assert.equal(board.open.find(t => t.job === 'job-b').title, 'Board view: every waiting thing', 'a ticket title has no heading mark');
assert.equal(board.open.find(t => t.proposal === 'vorschlag-1').title, 'Ein Vorschlag', 'a proposal title has no heading mark');
assert(board.open.find(t => t.job === 'job-b').meta.includes('0,037 $'), 'a ticket shows its cost with a decimal comma');

// 11. One group at a time. Without a choice the first group that holds something is shown.
const groups = (open, run, decision) => ({ open: Array(open).fill(0), run: Array(run).fill(0), decision: Array(decision).fill(0) });
assert.equal(boardTab(groups(2, 0, 11), ''), 'open', 'waiting tickets come first');
assert.equal(boardTab(groups(0, 1, 11), ''), 'run', 'then running work');
assert.equal(boardTab(groups(0, 0, 11), ''), 'decision', 'then what was decided');
assert.equal(boardTab(groups(0, 0, 0), ''), 'open', 'an empty board shows the waiting group');
assert.equal(boardTab(groups(2, 0, 11), 'decision'), 'decision', 'the choice of the user wins');
assert.equal(boardTab(groups(2, 0, 11), 'nonsense'), 'open', 'an unknown choice is ignored');
for (const tab of ['open', 'run', 'decision']) assert(html.includes(`'${tab}'`) && html.includes('data-board-tab='), `tab ${tab}`);
assert.doesNotMatch(html, /data-ticket-first|aria-label="Vorschläge"|class="proposals"/, 'the jump button and the duplicate proposal list are gone');
assert.match(css, /#taik-live \.ticket \{[^}]*width:\s*100%/, 'a ticket spans its list');
assert.match(css, /#taik-live \.board-list \{[^}]*max-height:[^}]*overflow-y:\s*auto/, 'a long group scrolls inside the board');
assert.match(css, /#taik-live \.board-column\[hidden\]\s*\{\s*display:\s*none/, 'hidden groups take no space');

// 12. Crew: harnesses, agents and the agent of a ticket. The logic is pure and marked.
assert.equal(count(/id="taik-crew"/g), 1, 'one crew element');
for (const hook of ['saveAgent', 'deleteAgent', 'setJobAgent', 'refreshCrew', 'id="taik-agent-name"', 'id="taik-agent-harness"',
  'id="taik-agent-model"', 'id="taik-agent-effort"', 'id="taik-agent-persona"', 'data-ticket-agent=', 'data-agent-new', 'data-agent-save']) {
  assert(html.includes(hook), `crew hook ${hook}`);
}
const crewBlock = html.match(/\/\*CREW_BEGIN\*\/([\s\S]*?)\/\*CREW_END\*\//)?.[1];
assert(crewBlock, 'the crew logic sits between CREW_BEGIN and CREW_END');
assert.doesNotMatch(crewBlock, /document|window\.|innerHTML|querySelector/, 'the crew logic touches no DOM');
const { crewOf, mentioned, agentLine, validAgent } = new Function(crewBlock + '\nreturn { crewOf, mentioned, agentLine, validAgent };')();
const overviewText = JSON.stringify({ checked_at: 5, harnesses: [
  { name: 'grok', worker: 'grok-cli', login: 'angemeldet', billing: 'subscription', models: ['grok-4.7'], efforts: ['low', 'high'] },
  { name: 'gemini', worker: null, login: 'nicht eingebunden', billing: 'subscription', models: [], efforts: [] },
], agents: [{ name: 'grok-schnell', harness: 'grok', model: 'grok-4.7', effort: 'low', persona: 'Knapp.' }, { name: 'kaputt', error: 'unknown harness' }] });
const crew = crewOf({ crew: overviewText });
assert.equal(crew.harnesses.length, 2, 'harnesses come from the overview text');
assert.deepEqual(crew.agents.map(agent => agent.name), ['grok-schnell', 'kaputt'], 'agents come from the overview text');
assert.deepEqual(crewOf({ crew: JSON.parse(overviewText) }).agents.length, 2, 'an overview object is read as well');
for (const broken of [{}, { crew: '' }, { crew: '{not json' }, { crew: '[]' }, { crew: null }]) {
  assert.deepEqual(crewOf(broken), { harnesses: [], agents: [], checkedAt: 0 }, `a missing or broken overview gives an empty crew: ${JSON.stringify(broken)}`);
}
assert.equal(mentioned('bitte @grok-schnell übernehmen', crew.agents), 'grok-schnell', 'a mention names a defined agent');
assert.equal(mentioned('@grok-schnell: mach das', crew.agents), 'grok-schnell', 'a mention may open the text');
assert.equal(mentioned('frag @niemand', crew.agents), '', 'an unknown name is no mention');
assert.equal(mentioned('mail an a@grok-schnell.de', crew.agents), '', 'an address is no mention');
assert.equal(mentioned('@kaputt soll es tun', crew.agents), '', 'a broken agent cannot be mentioned');
assert.equal(mentioned('', crew.agents), '', 'no text, no mention');
assert.equal(agentLine(crew.agents[0]), 'grok · grok-4.7 · low', 'an agent reads as harness, model and effort');
assert.equal(agentLine({ name: 'x', harness: 'savant', model: '', effort: '' }), 'savant · Standardmodell', 'defaults are named');
const draft = (overrides) => ({ name: 'neu', harness: 'grok', model: 'grok-4.7', effort: 'low', persona: '', ...overrides });
assert.equal(validAgent(draft({}), crew.harnesses), '', 'a complete draft is valid');
assert.notEqual(validAgent(draft({ name: 'Mit Leerzeichen' }), crew.harnesses), '', 'a name with spaces is refused');
assert.notEqual(validAgent(draft({ name: '' }), crew.harnesses), '', 'an empty name is refused');
assert.notEqual(validAgent(draft({ harness: 'gemini' }), crew.harnesses), '', 'a harness that is not wired is refused');
assert.notEqual(validAgent(draft({ harness: 'unbekannt' }), crew.harnesses), '', 'an unknown harness is refused');
assert.notEqual(validAgent(draft({ model: 'a b;c' }), crew.harnesses), '', 'a model with other characters is refused');
assert.notEqual(validAgent(draft({ effort: 'extreme' }), crew.harnesses), '', 'an effort the harness does not offer is refused');
assert.notEqual(validAgent(draft({ persona: 'x'.repeat(601) }), crew.harnesses), '', 'a persona over 600 characters is refused');
assert.doesNotMatch(html, /api[_-]?key|bearer|password/i, 'the viewer shows and asks for no credential');

// 13. The conversation of a job is shown in its ticket, in the order things happened.
const threadBlock = html.match(/\/\*THREAD_BEGIN\*\/([\s\S]*?)\/\*THREAD_END\*\//)?.[1];
assert(threadBlock, 'the thread logic sits between THREAD_BEGIN and THREAD_END');
assert.doesNotMatch(threadBlock, /document|window\.|innerHTML|querySelector/, 'the thread logic touches no DOM');
const { ticketThread } = new Function(threadBlock + '\nreturn { ticketThread };')();
const told = ticketThread({ rounds: [
    { name: 'round-01.run', fields: { decision: 'rejected', checks: 'AssertionError: heading', worker_cost_usd: '0.01' } },
    { name: 'round-02.run', fields: { decision: 'accepted', checks: 'PASS: all' } } ],
  messages: [
    { author: 'agent', round: 0, text: 'Welche Farbe?' }, { author: 'user', round: 0, text: 'Blau.' },
    { author: 'user', round: 1, text: 'Bitte größer.' }, { author: 'user', round: 2, text: 'Danke, jetzt rot.' } ] });
assert.deepEqual(told.map(entry => entry.kind === 'round' ? `round ${entry.number} ${entry.decision}` : `${entry.author}: ${entry.text}`), [
  'agent: Welche Farbe?', 'user: Blau.', 'round 1 rejected', 'user: Bitte größer.', 'round 2 accepted', 'user: Danke, jetzt rot.',
], 'messages and rounds are told in the order they happened');
assert.equal(told.find(entry => entry.kind === 'round').checks, 'AssertionError: heading', 'a round carries what the gate said');
assert.deepEqual(ticketThread({ rounds: [], messages: undefined }), [], 'a job without rounds and messages has an empty thread');
assert.deepEqual(ticketThread(undefined), [], 'no job, no thread');
const asking = boardTickets({ proposals: [], jobs: [
  job({ id: 'job-fragt', rounds: [round('accepted')], messages: [{ author: 'agent', round: 1, text: 'Was nun?' }] }),
  job({ id: 'job-beantwortet', rounds: [round('accepted')], messages: [{ author: 'user', round: 1, text: 'Mach weiter.' }] }),
  job({ id: 'job-erledigt', rounds: [round('accepted'), round('accepted')], messages: [{ author: 'user', round: 1, text: 'Mach weiter.' }] }),
] });
assert(asking.open.some(t => t.job === 'job-fragt' && t.flag === 'Frage'), 'a question of the worker waits for the user');
assert(asking.run.some(t => t.job === 'job-beantwortet' && t.flag === 'Warteschlange'), 'an unanswered reply of the user waits for its round');
assert(asking.decision.some(t => t.job === 'job-erledigt'), 'a reply that a round has followed is settled');
for (const hook of ['class="thread"', 'attention', 'ticketThread(']) assert(html.includes(hook), `thread hook ${hook}`);
assert(css.includes('#taik-live .thread'), 'style for the thread');

// 14. One place for a job: the ticket. It holds the description, the conversation and the full history.
assert.equal(count(/id="taik-ticket"/g) >= 1 && count(/<section id="taik-ticket"/g) <= 2, true, 'the ticket element');
// Each removed building block is named in the finding, so that a worker who cannot read this gate knows what to avoid.
for (const gone of ['taik-live-job', 'aria-labelledby="taik-goal"', 'Software-Auftrag', 'data-ticket-close']) {
  assert(!html.includes(gone), `the document must not contain the removed name ${gone}`);
}
assert(html.indexOf('id="taik-ticket"') > 0 && html.indexOf('id="taik-ticket"') < html.indexOf('aria-label="Auftragsverlauf"'), 'the history is part of the ticket');
assert.doesNotMatch(html, /<section aria-label="Auftragsverlauf"/, 'the history is no card of its own');
assert(html.includes('class="ticket-brief"') && html.includes('class="round-checks"'), 'description and the full finding of the gate are shown');
assert.doesNotMatch(css, /#taik-live \.round-checks \{[^}]*(?:text-overflow|white-space:\s*nowrap|line-clamp)/, 'the finding of the gate is not cut off');
assert.doesNotMatch(css, /#taik-live \.thread-message \{[^}]*(?:text-overflow|white-space:\s*nowrap|line-clamp)/, 'a message is not cut off');

// 15. A ticket is opened in the viewer, in the user's words. A request shows where it stands.
for (const hook of ['id="taik-new-ticket"', 'newTicket', 'firstTicket(', 'foldSteps(']) assert(html.includes(hook), `request hook ${hook}`);
const { firstTicket } = new Function(boardBlock + '\nreturn { firstTicket };')();
const requested = boardTickets({ jobs: [], proposals: [
  { name: 'ticket-001', intent: 'Wird gerade abgeleitet', origin: '', state: 'deriving' },
  { name: 'ticket-002', intent: 'Unklarer Wunsch', origin: '', state: 'declined', note: 'zu vage' },
  { name: 'ticket-003', intent: 'Fehler beim Ableiten', origin: '', state: 'failed', note: 'worker' },
  { name: 'ticket-004', intent: 'Fertiger Vorschlag', origin: 'requests' },
] });
assert.deepEqual(requested.run.map(entry => [entry.key, entry.flag]), [['proposal:ticket-001', 'Wird abgeleitet']], 'a request being derived is running');
assert.deepEqual(requested.decision.map(entry => [entry.key, entry.flag]), [['proposal:ticket-002', 'Abgelehnt'], ['proposal:ticket-003', 'Fehler']], 'a declined or failed request is decided');
assert(requested.decision[0].meta.includes('zu vage'), 'the reason of a declined request is told');
assert.deepEqual(requested.open.map(entry => entry.key), ['proposal:ticket-004'], 'only a finished proposal waits for the user');
// The ticket shown first belongs to the group shown first.
assert.equal(firstTicket(requested, ''), 'proposal:ticket-004');
assert.equal(firstTicket(requested, 'run'), 'proposal:ticket-001');
assert.equal(firstTicket({ open: [], run: [], decision: [] }, ''), null);
// A stream of byte counts is one step, not fifty.
const stepsBlock = html.match(/\/\*STEPS_BEGIN\*\/([\s\S]*?)\/\*STEPS_END\*\//)?.[1];
assert(stepsBlock, 'the step logic sits between STEPS_BEGIN and STEPS_END');
assert.doesNotMatch(stepsBlock, /document|window\.|innerHTML|querySelector/, 'the step logic touches no DOM');
const { foldSteps } = new Function(stepsBlock + '\nreturn { foldSteps };')();
const steps = [
  { elapsed: 1, kind: 'worker', detail: 'Worker liefert Quellcode' },
  { elapsed: 2, kind: 'worker', detail: '4096 Bytes Quellcode empfangen' },
  { elapsed: 3, kind: 'worker', detail: '8192 Bytes Quellcode empfangen' },
  { elapsed: 4, kind: 'worker', detail: '12288 Bytes Quellcode empfangen' },
  { elapsed: 5, kind: 'evaluate', detail: 'Gate' },
  { elapsed: 6, kind: 'worker', detail: '100 Bytes Quellcode empfangen' },
];
const before = JSON.stringify(steps);
assert.deepEqual(foldSteps(steps), [steps[0], { elapsed: 4, kind: 'worker', detail: '12288 Bytes Quellcode empfangen', folded: 3 }, steps[4], steps[5]], 'consecutive byte counts fold into the last one');
assert.equal(JSON.stringify(steps), before, 'folding leaves the events untouched');
assert.deepEqual(foldSteps([]), []);

// 16. The board holds the whole stock. Jobs that are not loaded appear as light entries and can be searched.
for (const hook of ['id="taik-board-search"', 'showJob', 'filterTickets(']) assert(html.includes(hook), `stock hook ${hook}`);
const { filterTickets } = new Function(boardBlock + '\nreturn { filterTickets };')();
const stock = boardTickets({ proposals: [], jobs: [
  { id: 'job-neu', goal: 'Neuer Auftrag', cost: 0, rounds: [{ fields: { decision: 'accepted', advance: 'false' } }] },
], index: [
  { id: 'job-neu', title: 'doppelt', decision: 'accepted', rounds: 1 },
  { id: 'job-alt', title: 'Alter Auftrag', decision: 'accepted', rounds: 3 },
  { id: 'job-gescheitert', title: 'Gescheitert', decision: 'rejected', rounds: 4 },
  { id: 'job-leer', title: '', decision: '', rounds: 0 },
] });
assert.deepEqual(stock.decision.map(entry => entry.key), ['job-neu', 'job-alt', 'job-gescheitert', 'job-leer'], 'the stock follows the loaded jobs, each job once');
assert.deepEqual(stock.open, [], 'an old rejected job does not wait for the user');
assert.deepEqual(stock.decision.slice(1).map(entry => [entry.flag, entry.archived]), [['Angenommen', true], ['Abgelehnt', true], ['Ohne Runde', true]], 'the stock tells how each job ended');
assert(stock.decision[1].meta.includes('3 Runden') && stock.decision[3].title === 'job-leer', 'a stock entry names its rounds and falls back to its id');
const found = filterTickets(stock, '  ALTER ');
assert.deepEqual([found.open.length, found.run.length, found.decision.map(entry => entry.key)], [0, 0, ['job-alt']], 'the search matches titles without regard to case');
assert.deepEqual(filterTickets(stock, 'gescheitert').decision.map(entry => entry.key), ['job-gescheitert'], 'the search matches the id');
assert.equal(filterTickets(stock, '').decision.length, 4, 'an empty search shows everything');
assert.equal(stock.decision.length, 4, 'searching leaves the groups untouched');

// 17. Top down: where a job stands, what each round changed, then the changed lines.
for (const hook of ['class="ticket-state"', 'class="round-change"', 'ticketState(', 'changeLine(']) assert(html.includes(hook), `state hook ${hook}`);
const { ticketState, changeLine } = new Function(threadBlock + '\nreturn { ticketState, changeLine };')();
const stood = (decision, more = {}) => ({ name: 'round.run', fields: { decision, intent: 'Eine Seite bauen', advance: 'false', ...more } });
assert.deepEqual(ticketState({ rounds: [], messages: [] }), { done: 'Noch keine Runde gelaufen.', problem: '', question: '', next: 'Wartet auf die erste Runde.', analysis: [], waits: '' });
assert.deepEqual(ticketState({ rounds: [stood('accepted')], messages: [] }),
  { done: 'Runde 1 angenommen: Eine Seite bauen', problem: '', question: '', next: 'Fertig. Eine Antwort von dir gibt eine weitere Runde.', analysis: [], waits: '' });
const failed = ticketState({ rounds: [stood('accepted'), stood('rejected', { checks: 'FAIL: kein Titel', worker_failure: '' })], messages: [] });
assert.equal(failed.done, 'Runde 2 abgelehnt. Der Stand aus Runde 1 gilt weiter.');
assert.equal(failed.problem, 'FAIL: kein Titel');
assert.equal(ticketState({ rounds: [stood('rejected', { checks: '', worker_failure: 'timeout' })], messages: [] }).problem, 'Der Agent hat nichts geliefert: timeout');
assert.equal(ticketState({ rounds: [stood('rejected', { checks: 'x' })], messages: [] }).done, 'Runde 1 abgelehnt. Es gibt noch keinen angenommenen Stand.');
const asked = ticketState({ rounds: [stood('accepted')], messages: [{ author: 'user', round: 0, text: 'a' }, { author: 'agent', round: 1, text: 'Welche Farbe?' }] });
assert.deepEqual([asked.question, asked.next], ['Welche Farbe?', 'Deine Antwort im Feld unten.'], 'an open question of the agent is the next step');
assert.equal(ticketState({ rounds: [stood('accepted')], messages: [{ author: 'user', round: 1, text: 'Blau' }] }).next, 'Deine Antwort wartet auf ihre Runde.');
assert.equal(ticketState({ rounds: [stood('accepted')], messages: [], sourceActive: true }).next, 'Der Agent arbeitet an Runde 2.');
assert.equal(ticketState(undefined).done, 'Noch keine Runde gelaufen.');
assert.equal(changeLine({ file: 'round-02.html', added: 12, removed: 3 }), 'round-02.html · +12 −3 Zeilen');
assert.equal(changeLine({ file: 'round-01.html', added: 1, removed: 0, fresh: true }), 'round-01.html · neu, 1 Zeile');
assert.equal(changeLine({ file: 'round-03.rejected.txt', added: 0, removed: 0 }), 'round-03.rejected.txt · unverändert');
assert.equal(changeLine(undefined), '');
assert.doesNotMatch(css, /#taik-live \.ticket-state[^{]*\{[^}]*(?:text-overflow|white-space:\s*nowrap|line-clamp)/, 'the state is not cut off');

// 18. A rejected round is explained: the analysis is read into labelled lines and stands in the state.
for (const hook of ['class="analysis"', 'analysisLines(']) assert(html.includes(hook), `analysis hook ${hook}`);
const { analysisLines } = new Function(threadBlock + '\nreturn { analysisLines };')();
assert.deepEqual(analysisLines('Verlangt: ein Titel.\nGescheitert an: keiner da.\n\nUrsache: Kandidat: zu kurz.\nNächster Schritt: Agent.\n'),
  [['Verlangt', 'ein Titel.'], ['Gescheitert an', 'keiner da.'], ['Ursache', 'Kandidat: zu kurz.'], ['Nächster Schritt', 'Agent.']], 'an analysis is read as labelled lines');
assert.deepEqual(analysisLines('ohne Aufbau'), [['', 'ohne Aufbau']], 'a line without a label is kept as it is');
assert.deepEqual(analysisLines(undefined), []);
const explained = ticketState({ rounds: [stood('rejected', { checks: 'FAIL: kein Titel' })], messages: [], analyses: { 'round': 'Verlangt: ein Titel.\nUrsache: fehlt.' } });
assert.deepEqual(explained.analysis, [['Verlangt', 'ein Titel.'], ['Ursache', 'fehlt.']], 'the state carries the analysis of the last round');
assert.equal(explained.problem, 'FAIL: kein Titel', 'the finding of the gate stays beside the analysis');
assert.deepEqual(ticketState({ rounds: [stood('accepted')], messages: [], analyses: { 'round': 'Verlangt: x' } }).analysis, [], 'an accepted round has no analysis in the state');
assert.doesNotMatch(css, /#taik-live \.analysis[^{]*\{[^}]*(?:text-overflow|white-space:\s*nowrap|line-clamp)/, 'the analysis is not cut off');

// 19. An agent session: each round shows what the agent said and did, and its own summary.
for (const hook of ['class="round-dialog"', 'class="round-summary"', 'dialogLine(']) assert(html.includes(hook), `session hook ${hook}`);
const { dialogLine } = new Function(threadBlock + '\nreturn { dialogLine };')();
assert.deepEqual(dialogLine({ who: 'agent', text: 'Ich lese die Datei.' }), { who: 'Agent', text: 'Ich lese die Datei.', tool: false });
assert.deepEqual(dialogLine({ who: 'user', text: 'Bitte kürzer.' }), { who: 'Du', text: 'Bitte kürzer.', tool: false });
assert.deepEqual(dialogLine({ tool: 'Edit', target: 'src/main.rs' }), { who: 'Edit', text: 'src/main.rs', tool: true });
assert.deepEqual(dialogLine({ tool: 'Glob', target: '' }), { who: 'Glob', text: '', tool: true });
assert.equal(dialogLine(undefined), null);
assert.doesNotMatch(css, /#taik-live \.round-(?:dialog|summary)[^{]*\{[^}]*(?:text-overflow|white-space:\s*nowrap|line-clamp)/, 'the dialog is not cut off');

// 20. Every waiting ticket says why it waits, on the board and in the ticket.
const waiting = boardTickets({ proposals: [{ name: 'idee', intent: 'Eine Idee', origin: '' }], jobs: [
  { id: 'job-fragt', goal: 'A', cost: 0, rounds: [], messages: [{ author: 'agent', round: 0, text: 'Welche Farbe?' }] },
  { id: 'job-erklaert', goal: 'B', cost: 0, rounds: [{ name: 'round-01.run', fields: { decision: 'rejected', checks: 'FAIL: kein Titel' } }], messages: [],
    analyses: { 'round-01': 'Verlangt: ein Titel.\nUrsache: fehlt.\nNächster Schritt: Du stellst den Auftrag klar.' } },
  { id: 'job-roh', goal: 'C', cost: 0, rounds: [{ name: 'round-01.run', fields: { decision: 'rejected', checks: 'FAIL: kein Titel' } }], messages: [] },
] });
const why = Object.fromEntries(waiting.open.map(entry => [entry.key, entry.why]));
assert.deepEqual(why, {
  'proposal:idee': 'Die Factory schlägt diesen Auftrag vor. Er startet erst mit deiner Freigabe.',
  'job-fragt': 'Der Agent fragt: Welche Farbe?',
  'job-erklaert': 'Du stellst den Auftrag klar.',
  'job-roh': 'Runde 1 wurde abgelehnt: FAIL: kein Titel',
}, 'each waiting ticket carries its reason');
assert(waiting.open.every(entry => entry.why), 'no ticket waits without a reason');
assert(html.includes('class="why"') && html.includes('class="waits"'), 'the reason is shown on the board and in the ticket');
assert.equal(ticketState({ rounds: [], messages: [{ author: 'agent', round: 0, text: 'Welche Farbe?' }] }).waits, 'Der Agent hat eine Frage gestellt und arbeitet erst nach deiner Antwort weiter.');
assert.equal(ticketState({ rounds: [stood('rejected', { checks: 'x' })], messages: [] }).waits, 'Die letzte Runde wurde abgelehnt. Es läuft nichts weiter, bis du im Feld unten antwortest.');
assert.equal(ticketState({ rounds: [stood('accepted')], messages: [] }).waits, '', 'an accepted job does not wait');
assert.equal(ticketState({ rounds: [stood('rejected', { checks: 'x' })], messages: [], sourceActive: true }).waits, '', 'a job at work does not wait');
assert.doesNotMatch(css, /#taik-live \.ticket \.why[^{]*\{[^}]*(?:text-overflow|white-space:\s*nowrap|line-clamp)/, 'the reason is not cut off');

// 21. The orchestrator: the user speaks to it, and it shows for each request what it planned and why.
for (const hook of ['id="taik-orchestrator"', 'class="plans"', 'planLine(', 'orchestratorFeed(']) assert(html.includes(hook), `orchestrator hook ${hook}`);
const { planLine, orchestratorFeed } = new Function(boardBlock + '\nreturn { planLine, orchestratorFeed };')();
assert.equal(planLine({ kind: 'sitzung', repo: 'taik', gate: 'savant', work: 'standard' }), 'Sitzung auf taik · Gate @savant · Arbeit @standard');
assert.equal(planLine({ kind: 'dokument', repo: '', gate: 'standard', work: 'grok-schnell' }), 'Dokument · Gate @standard · Arbeit @grok-schnell');
assert.equal(planLine(undefined), '');
const feed = orchestratorFeed({ proposals: [
  { name: 'ticket-001', intent: 'Alt und ohne Plan', origin: 'requests' },
  { name: 'ticket-002', intent: 'Geplant', origin: 'requests', plan: { kind: 'dokument', repo: '', gate: 'savant', work: 'standard', reason: 'Ein neues Dokument.' } },
  { name: 'ticket-003', intent: 'Wird geplant', origin: '', state: 'deriving' },
  { name: 'ticket-004', intent: 'In Arbeit beim Gate', origin: '', state: 'deriving', plan: { kind: 'sitzung', repo: 'taik', gate: 'savant', work: 'standard', reason: 'Dateien ändern.' } },
] });
assert.deepEqual(feed.map(entry => [entry.key, entry.line, entry.reason]), [
  ['proposal:ticket-004', 'Sitzung auf taik · Gate @savant · Arbeit @standard', 'Dateien ändern. Das Gate wird gerade geschrieben.'],
  ['proposal:ticket-003', 'Der Orchestrator plant.', ''],
  ['proposal:ticket-002', 'Dokument · Gate @savant · Arbeit @standard', 'Ein neues Dokument.'],
], 'the feed shows planned and pending requests, newest first, and leaves out what has no plan');
assert.deepEqual(orchestratorFeed({ jobs: [] }), []);
assert.doesNotMatch(css, /#taik-live \.plans[^{]*\{[^}]*(?:text-overflow|white-space:\s*nowrap|line-clamp)/, 'a plan is not cut off');

console.log(`PASS: host contract, offline isolation, ${shadows.length} token shadows without glow, contrast ${Math.min(...measured).toFixed(2)}–${Math.max(...measured).toFixed(2)}, lossless .run highlighting, token flow per round, one workpiece pane with live draft, animated flow, ticket board in one group at a time, crew with agents and harnesses, conversation in the ticket, one ticket with the full history, tickets opened in the viewer, the whole stock on the board, top-down state and changes, analysis of a rejection, dialog of an agent session, a reason for every waiting ticket, the orchestrator and its plans`);
// TAIK: check added by job ticket-018
await (async () => {
  const { default: assert } = await import('node:assert/strict');
  const { readFileSync } = await import('node:fs');
  const html = readFileSync(process.argv[2], 'utf8');
  void readFileSync;
const m = html.match(/\/\*BOARD_BEGIN\*\/([\s\S]*?)\/\*BOARD_END\*\//);
assert.ok(m, 'The BOARD block between /*BOARD_BEGIN*/ and /*BOARD_END*/ is missing.');
const { orchestratorFeed, planLine } = new Function(m[1] + '\nreturn { orchestratorFeed, planLine };')();

const plan = { kind: 'dokument', gate: 'g', work: 'w', reason: 'weil' };
const pl = planLine(plan);
const data = { proposals: [
  { name: 'a', intent: 'Alpha', state: 'declined', note: 'zu vage' },
  { name: 'b', intent: 'Beta', state: 'failed', note: '   ' },
  { name: 'c', intent: 'Gamma', state: 'declined', note: 'nein', plan },
  { name: 'd', intent: 'Delta', state: 'failed', plan },
  { name: 'e', intent: 'Eps', plan },
  { name: 'f', intent: 'Phi', state: 'deriving' },
  { name: 'g', intent: 'Gamma2' },
  { name: 'h', intent: 'Eta', state: 'declined' },
] };
const feed = orchestratorFeed(data);
assert.deepEqual(feed.map(e => e.key), ['proposal:h', 'proposal:f', 'proposal:e', 'proposal:d', 'proposal:c', 'proposal:b', 'proposal:a'],
  'Declined and failed requests must appear in the feed, in the same newest-first order as the other entries, without a separate group.');
const by = k => feed.find(e => e.key === 'proposal:' + k);
assert.equal(by('a').line, 'Abgelehnt', 'A declined request must have the line "Abgelehnt".');
assert.equal(by('a').reason, 'zu vage', 'The reason of a declined request must be the text of its note.');
assert.equal(by('a').title, 'Alpha', 'The title of the entry must stay the intent.');
assert.equal(by('b').line, 'Fehlgeschlagen', 'A failed request must have the line "Fehlgeschlagen".');
assert.equal(by('b').reason, 'ohne Begründung', 'A whitespace-only note must give the reason "ohne Begründung".');
assert.equal(by('h').reason, 'ohne Begründung', 'A missing note must give the reason "ohne Begründung".');
assert.equal(by('c').line, 'Abgelehnt', 'A declined request with a plan must keep the line "Abgelehnt".');
assert.equal(by('c').reason, pl + ' · nein', 'A declined request with a plan must have reason = plan line + " · " + note.');
assert.equal(by('d').line, 'Fehlgeschlagen', 'A failed request with a plan must keep the line "Fehlgeschlagen".');
assert.equal(by('d').reason, pl + ' · ohne Begründung', 'A failed planned request without note must have reason = plan line + " · ohne Begründung".');
assert.equal(by('e').line, pl, 'A planned request in another state must keep the plan line as its line.');
assert.equal(by('e').reason, 'weil', 'A planned request in another state must keep the plan reason.');
assert.equal(by('f').line, 'Der Orchestrator plant.', 'A deriving request must stay unchanged.');
assert.equal(by('f').reason, '', 'A deriving request without plan must keep an empty reason.');
const derivingPlanned = orchestratorFeed({ proposals: [{ name: 'x', intent: 'X', state: 'deriving', plan }] })[0];
assert.equal(derivingPlanned.reason, 'weil Das Gate wird gerade geschrieben.', 'A deriving request with a plan must keep its reason unchanged.');
assert.equal(derivingPlanned.line, pl, 'A deriving request with a plan must keep its plan line.');
})();
// TAIK: check added by job ticket-020
await (async () => {
  const { default: assert } = await import('node:assert/strict');
  const { readFileSync } = await import('node:fs');
  const html = readFileSync(process.argv[2], 'utf8');
  void readFileSync;
const grab = (name) => {
  const m = html.match(new RegExp('/\\*' + name + '_BEGIN\\*/([\\s\\S]*?)/\\*' + name + '_END\\*/'));
  assert.ok(m, 'Block ' + name + ' fehlt im Dokument.');
  return m[1];
};
const { boardTickets, orchestratorFeed } = new Function(grab('BOARD') + '\nreturn { boardTickets, orchestratorFeed };')();
const { flowStations } = new Function(grab('FLOW') + '\nreturn { flowStations };')();

const proposal = {
  name: 'farbe', intent: 'Mach etwas Buntes', state: 'asks',
  messages: [
    { author: 'user', round: 0, text: 'Mach etwas Buntes' },
    { author: 'agent', round: 1, text: 'Welche Farbe?' },
  ],
};
const data = { jobs: [], proposals: [proposal] };

const board = boardTickets(data);
const hit = board.open.find(t => t.key === 'proposal:farbe');
assert.ok(hit, 'Ein Vorschlag mit state "asks" fehlt in der Gruppe open (Wartet) von boardTickets, Schlüssel "proposal:farbe".');
assert.equal(hit.flag, 'Frage', 'Das Kennzeichen des Vorschlags mit Rückfrage muss „Frage" sein.');
assert.equal(hit.why, 'Der Orchestrator fragt: Welche Farbe?', 'Der Grund muss „Der Orchestrator fragt: " plus Text der letzten Agent-Nachricht sein.');
assert.ok(!board.run.some(t => t.key === 'proposal:farbe') && !board.decision.some(t => t.key === 'proposal:farbe'),
  'Der Vorschlag mit Rückfrage darf nur unter Wartet stehen.');

const feed = orchestratorFeed(data);
const row = feed.find(e => e.key === 'proposal:farbe');
assert.ok(row, 'orchestratorFeed listet den Vorschlag mit state "asks" nicht.');
assert.equal(row.line, 'Der Orchestrator fragt.', 'Die Zeile (line) muss „Der Orchestrator fragt." sein.');
assert.equal(row.reason, 'Welche Farbe?', 'Der Grund (reason) muss der Text der letzten Agent-Nachricht sein.');

const st = flowStations(null, proposal);
assert.equal(st.length, 6, 'flowStations muss sechs Stationen liefern.');
assert.equal(st[0].state, 'erledigt', 'Bei einer Rückfrage muss „Wunsch" erledigt sein.');
assert.equal(st[1].name, 'Plan des Orchestrators', 'Station 2 muss „Plan des Orchestrators" heißen.');
assert.equal(st[1].state, 'aktiv', 'Bei einer Rückfrage muss „Plan des Orchestrators" aktiv sein.');
for (let i = 2; i < 6; i++) assert.equal(st[i].state, 'offen', 'Bei einer Rückfrage müssen alle späteren Stationen offen sein (Station ' + (i + 1) + ').');
})();
// TAIK: check added by job ticket-023
await (async () => {
  const { default: assert } = await import('node:assert/strict');
  const { readFileSync } = await import('node:fs');
  const html = readFileSync(process.argv[2], 'utf8');
  void readFileSync;
const m = html.match(/\/\*BOARD_BEGIN\*\/[\s\S]*?\/\*BOARD_END\*\//);
assert.ok(m, 'Der Block BOARD (/*BOARD_BEGIN*/ bis /*BOARD_END*/) fehlt.');
const api = new Function(m[0] + '\nreturn { ticketActions: typeof ticketActions === "function" ? ticketActions : undefined };')();
assert.equal(typeof api.ticketActions, 'function', 'Die Funktion ticketActions fehlt im Block BOARD.');
const f = api.ticketActions;
const waiting = ["Freigeben", "Ablehnen", "Senden"];
assert.deepEqual(f({ name: 'ticket-1' }), waiting, 'Ein Vorschlag ohne state soll ["Freigeben","Ablehnen","Senden"] liefern.');
assert.deepEqual(f({ name: 'ticket-1', state: undefined }), waiting, 'state undefined soll wie ohne state behandelt werden.');
assert.deepEqual(f({ name: 'ticket-1', state: null }), waiting, 'state null soll wie ohne state behandelt werden.');
assert.deepEqual(f({ name: 'ticket-1', state: '' }), waiting, 'Ein leerer state soll wie ohne state behandelt werden.');
assert.deepEqual(f({ state: 'asks' }), ["Zurückziehen", "Senden"], 'Bei state "asks" sollen "Zurückziehen" und "Senden" kommen.');
for (const s of ['deriving', 'declined', 'failed', 'other']) {
  assert.deepEqual(f({ state: s }), [], `Bei state "${s}" soll die Liste leer sein.`);
}
assert.deepEqual(f(undefined), [], 'Bei undefined soll die Liste leer sein.');
assert.deepEqual(f(null), [], 'Bei null soll die Liste leer sein.');
const frozen = Object.freeze({ name: 'ticket-2', state: 'asks', messages: Object.freeze([]) });
assert.deepEqual(f(frozen), ["Zurückziehen", "Senden"], 'Die Funktion soll ihr Argument nur lesen und auch bei eingefrorenen Daten arbeiten.');
const a = f({});
a.push('x');
assert.deepEqual(f({}), waiting, 'Jeder Aufruf soll eine neue Liste liefern, ohne gemeinsamen Zustand.');
})();
// TAIK: check added by job ticket-024
await (async () => {
  const { default: assert } = await import('node:assert/strict');
  const { readFileSync } = await import('node:fs');
  const html = readFileSync(process.argv[2], 'utf8');
  void readFileSync;
const m = html.match(/\/\*CREW_BEGIN\*\/([\s\S]*?)\/\*CREW_END\*\//);
assert.ok(m, 'The CREW block between /*CREW_BEGIN*/ and /*CREW_END*/ is missing.');
const api = new Function(m[1] + '\nreturn { validRepo: typeof validRepo === "function" ? validRepo : null };')();
assert.ok(api.validRepo, 'The CREW block does not define a function validRepo(name, path, repos).');
const v = api.validRepo;
const NAME = 'Name: Kleinbuchstaben, Ziffern und Bindestrich, höchstens 32 Zeichen';
const DUP = 'Diesen Namen gibt es schon';
const PATH = 'Pfad: absolut oder mit ~ beginnend';

assert.equal(v('mein-repo', '/srv/x', []), '', 'A valid name and absolute path should give an empty string.');
assert.equal(v('mein-repo', '  ~/x  ', []), '', 'A path starting with ~ after trimming should be valid.');
assert.equal(v('mein-repo', '/srv/x'), '', 'Missing repos should mean no name conflict.');
assert.equal(v('a1-b', '~', undefined), '', 'A path that is just ~ should be valid.');
assert.equal(v('Mein', '/x', []), NAME, 'An uppercase name should give the exact name message.');
assert.equal(v('', '/x', []), NAME, 'An empty name should give the name message.');
assert.equal(v(' a', '/x', []), NAME, 'A name with a leading space should be invalid (name is not trimmed).');
assert.equal(v('a ', '/x', []), NAME, 'A name with a trailing space should be invalid.');
assert.equal(v('a_b', '/x', []), NAME, 'An underscore in the name should be invalid.');
assert.equal(v('a'.repeat(32), '/x', []), '', 'A name of 32 characters should be valid.');
assert.equal(v('a'.repeat(33), '/x', []), NAME, 'A name of 33 characters should be invalid.');
assert.equal(v('a', 'x', [{ name: 'a' }]), DUP, 'An existing name should give the duplicate message.');
assert.equal(v('a', '/x', [{ name: 'b' }]), '', 'A different existing name should not conflict.');
assert.equal(v('a', 'rel/pfad', []), PATH, 'A relative path should give the path message.');
assert.equal(v('a', '', []), PATH, 'An empty path should give the path message.');
assert.equal(v('a', '   ', []), PATH, 'A blank path should give the path message.');
assert.equal(v('A', 'x', [{ name: 'A' }]), NAME, 'The name rule should win over the duplicate rule.');
assert.equal(v('a', 'x', [{ name: 'a' }]), DUP, 'The duplicate rule should win over the path rule.');

const repos = [{ name: 'q', path: '/q', protected: [] }];
const copy = JSON.stringify(repos);
v('z', '/z', repos);
assert.equal(JSON.stringify(repos), copy, 'validRepo must not change repos.');
})();
