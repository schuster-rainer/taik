// Independent gate for the Factory viewer fragment. Pinned per job before the candidate exists.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const html = readFileSync(process.argv[2], 'utf8');
const count = (pattern) => (html.match(pattern) ?? []).length;

// 1. Contract with the macOS host and the WebKit design check.
assert.match(html, /^<style>/, 'the viewer is a fragment that starts with its style sheet');
assert.equal(count(/\/\*FACTORY_DATA\*\/null/g), 1, 'exactly one FACTORY_DATA placeholder');
assert.equal(count(/window\.updateFactory\s*=/g), 1, 'one updateFactory entry point');
for (const id of ['taik-live', 'taik-activity', 'taik-preview', 'taik-result-viewport', 'taik-job-content',
  'taik-job-viewport', 'taik-draft-viewport', 'taik-job-observation']) {
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

console.log(`PASS: host contract, offline isolation, ${shadows.length} token shadows without glow, contrast ${Math.min(...measured).toFixed(2)}–${Math.max(...measured).toFixed(2)}, lossless .run highlighting`);
