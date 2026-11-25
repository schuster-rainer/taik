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
for (const hook of ['id="taik-live-job"', 'id="taik-appearance"', 'aria-label="Auftragsverlauf"', 'data-round=',
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

console.log(`PASS: host contract, offline isolation, ${shadows.length} token shadows without glow, contrast ${Math.min(...measured).toFixed(2)}–${Math.max(...measured).toFixed(2)}, lossless .run highlighting, token flow per round, one workpiece pane with live draft, animated flow`);
