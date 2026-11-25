// Gate for the Galaxy source experiment. Both arms use these bytes.
// The sample records below are held out: they differ from the sample in tools/factory-view-job/verify.mjs.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const html = readFileSync(process.argv[2], 'utf8');
assert.match(html, /^<!doctype html>/i, 'standalone document');
assert.doesNotMatch(html, /https?:\/\/|@import|<link\b|<img\b|<iframe\b|<object\b|<embed\b|\bfetch\(|XMLHttpRequest|WebSocket/i, 'offline and self-contained');
assert.match(html, /<h1[^>]*>\s*Protokoll-Leser\s*<\/h1>/, 'heading');
assert.match(html, /<textarea[^>]*\bid="record"/, 'input #record');
assert.match(html, /<pre[^>]*\bid="view"/, 'output #view');
assert(html.length <= 24 * 1024, 'at most 24 KiB');

const block = html.match(/\/\*RUN_HIGHLIGHT_BEGIN\*\/([\s\S]*?)\/\*RUN_HIGHLIGHT_END\*\//)?.[1];
assert(block, 'marked highlightRun block');
let highlightRun;
try { highlightRun = new Function(`${block}; return highlightRun;`)(); } catch (error) { assert.fail(`highlightRun block does not evaluate on its own: ${error.message}`); }
assert.equal(typeof highlightRun, 'function', 'highlightRun is a function');

const sample = [
  'decision=rejected', 'kind=software', 'tags=', 'identity=round-02',
  'intent=Vergleiche a<b & c>d mit "x" und \'y\'',
  'worker=claude-cli', 'worker_output_tokens=2140', 'worker_duration_ms=23680', 'worker_cost_usd=0.25',
  'limits=3 rounds; verifier 15s; wait 45 µs; share 12.5%; cap 2MiB',
  'gate_sha256=0f3c9a7be2d14c5586a1b0e7d9f24c63a8b5e1d07c4f92ab36de58c1704fa9be',
  'commit=3b9d1c0e5f7a2b4c6d8e0f1a3b5c7d9e1f2a4b6c',
  'checks=FAIL: probes:false, order:true, x=y; state retained after error',
  'baseline=../job-alt/round-01.run', 'rollback=round-09.baseline.tar.gz', 'artifact=round-08-taik-seed',
  'evidence=round-02.checks.log', 'advance=true', 'Lose Zeile ohne Schlüssel: PASS', 'end', '',
].join('\n');
const marked = highlightRun(sample);
assert.equal(typeof marked, 'string', 'highlightRun returns a string');
assert.doesNotMatch(marked, /<(?!\/?span\b)/, 'only span markup is produced');
assert.doesNotMatch(marked, /<span(?! class="run-[a-z]+">)/, 'spans carry exactly one run-* class');
const plain = marked.replace(/<\/?span[^>]*>/g, '')
  .replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&quot;/g, '"').replace(/&#39;/g, "'").replace(/&#x27;/gi, "'").replace(/&amp;/g, '&');
assert.equal(plain, sample, 'highlighting preserves the record text');
const has = (cls, text) => marked.includes(`<span class="run-${cls}">${text}</span>`);
const need = (cls, text) => assert(has(cls, text), `expected ${text} as run-${cls}`);
for (const key of ['decision', 'worker_output_tokens', 'gate_sha256', 'advance']) need('key', key);
need('bad', 'rejected'); need('bad', 'FAIL'); need('bad', 'false'); need('bad', 'error');
need('good', 'true'); need('good', 'retained'); need('good', 'PASS');
need('end', 'end');
need('hash', '0f3c9a7be2d14c5586a1b0e7d9f24c63a8b5e1d07c4f92ab36de58c1704fa9be');
need('hash', '3b9d1c0e5f7a2b4c6d8e0f1a3b5c7d9e1f2a4b6c');
need('path', '../job-alt/round-01.run'); need('path', 'round-09.baseline.tar.gz'); need('path', 'round-08-taik-seed'); need('path', 'round-02.checks.log');
need('number', '2140'); need('number', '23680'); need('number', '0.25'); need('number', '15s'); need('number', '45 µs'.replace(' µs', '')); need('number', '12.5%'); need('number', '2MiB');
assert(!has('key', 'x'), 'only the first = of a line separates key and value');
assert(!has('number', '02') && !has('number', '09'), 'digits inside names are not quantities');
assert(!/<span class="run-key">Lose/.test(marked), 'a line without a key has no key span');

// Readable in both appearances.
const pair = (name) => {
  const match = html.match(new RegExp(`${name}\\s*:\\s*light-dark\\(\\s*(#[0-9a-f]{6})\\s*,\\s*(#[0-9a-f]{6})\\s*\\)`, 'i'));
  assert(match, `${name} is a light-dark pair of six-digit hex colours`);
  return [match[1], match[2]];
};
const luminance = (hex) => {
  const channel = (i) => { const c = parseInt(hex.slice(i, i + 2), 16) / 255; return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4; };
  return 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
};
const contrast = (a, b) => { const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x); return (hi + 0.05) / (lo + 0.05); };
const base = pair('--base'), ink = pair('--ink');
for (const mode of [0, 1]) assert(contrast(ink[mode], base[mode]) >= 7, `--ink on --base ${mode ? 'dark' : 'light'} contrast below 7`);
for (const cls of ['key', 'good', 'bad', 'end', 'hash', 'path', 'number']) assert(html.includes(`.run-${cls}`), `style for .run-${cls}`);

console.log('PASS: offline reader, held-out record highlighted without loss, readable in light and dark');
