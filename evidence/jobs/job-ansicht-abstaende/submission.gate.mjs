// TAIK: release tools/release-view.sh
// Independent gate for the Factory viewer fragment. Pinned per job before the candidate exists.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const html = readFileSync(process.argv[2], 'utf8');
const count = (pattern) => (html.match(pattern) ?? []).length;
const css = html.slice(0, html.indexOf('</style>'));

// The declarations of every rule whose selector list contains `selector`, in document order.
const rules = [...css.matchAll(/([^{}]+)\{([^{}]*)\}/g)]
  .map(match => ({ selector: match[1].replace(/\/\*[\s\S]*?\*\//g, '').trim(), body: match[2] }));
const matches = (part, selector) => part.trim() === selector || (selector.startsWith('::') && part.trim().endsWith(selector));
const rulesFor = (selector) => rules.filter(rule => rule.selector.split(',').some(part => matches(part, selector)));
const declared = (selector, property) => rulesFor(selector)
  .flatMap(rule => [...rule.body.matchAll(new RegExp(`(?:^|;)\\s*${property}\\s*:\\s*([^;]+)`, 'g'))].map(match => match[1].trim()));
// Blocks of at-rules (@media, @container, @supports) with balanced braces, so the desktop rules can be read alone.
const stripAtRules = (text) => {
  let out = '';
  let index = 0;
  while (index < text.length) {
    const at = text.slice(index).search(/@(media|container|supports)\b/);
    if (at < 0) { out += text.slice(index); break; }
    out += text.slice(index, index + at);
    let open = text.indexOf('{', index + at);
    let depth = 0;
    let end = open;
    for (; end < text.length; end++) {
      if (text[end] === '{') depth++;
      else if (text[end] === '}' && --depth === 0) break;
    }
    index = end + 1;
  }
  return out;
};
const desktopRules = [...stripAtRules(css).matchAll(/([^{}]+)\{([^{}]*)\}/g)]
  .map(match => ({ selector: match[1].replace(/\/\*[\s\S]*?\*\//g, '').trim(), body: match[2] }));
const desktop = (selector, property) => desktopRules.filter(rule => rule.selector.split(',').some(part => matches(part, selector)))
  .flatMap(rule => [...rule.body.matchAll(new RegExp(`(?:^|;)\\s*${property}\\s*:\\s*([^;]+)`, 'g'))].map(match => match[1].trim()));
const rem = (value) => { const match = /([\d.]+)rem/.exec(value); return match ? parseFloat(match[1]) * 16 : null; };
const px = (value) => { const match = /([\d.]+)px/.exec(value); return match ? parseFloat(match[1]) : null; };

// 1. Contract with the macOS host and the WebKit design check.
assert.match(html, /^<style>/, 'the viewer is a fragment that starts with its style sheet');
assert.equal(count(/\/\*FACTORY_DATA\*\/null/g), 1, 'exactly one FACTORY_DATA placeholder');
assert.equal(count(/window\.updateFactory\s*=/g), 1, 'one updateFactory entry point');
for (const id of ['taik-live', 'taik-activity', 'taik-preview', 'taik-flow', 'taik-result-viewport', 'taik-draft-viewport',
  'taik-result-source', 'taik-result-diff']) {
  assert.equal(count(new RegExp(`id="${id}"`, 'g')), 1, `static element #${id}`);
}
for (const hook of ['id="taik-live-job"', 'id="taik-appearance"', 'aria-label="Auftragsverlauf"', 'data-round=',
  'class="rounds"', 'class="round', 'openArtifact', 'setAppearance']) {
  assert(html.includes(hook), `viewer hook ${hook}`);
}
for (const name of ['workpieceState', 'tokenFlow', 'highlightRun']) {
  assert(new RegExp(`function ${name}\\b`).test(html), `the function ${name} is unchanged in name`);
}
assert.equal(count(/\/\*WORKPIECE_BEGIN\*\//g), 1, 'one workpieceState block');
assert.equal(count(/\/\*TOKEN_FLOW_BEGIN\*\//g), 1, 'one tokenFlow block');
assert.equal(count(/\/\*RUN_HIGHLIGHT_BEGIN\*\//g), 1, 'one highlightRun block');

// 2. Offline and isolated.
assert.doesNotMatch(html, /https?:\/\/|@import|<link\b|<object\b|<embed\b|\bfetch\(|XMLHttpRequest|WebSocket/i, 'no network or external resource');
assert.doesNotMatch(html, /url\((?!\s*['"]?data:)/i, 'only data: URLs in styles');
assert(html.includes('sandbox="allow-scripts"'), 'accepted results stay in a script-only sandbox');
assert.match(html, /setAttribute\('sandbox',\s*''\)/, 'the live draft frame gets an empty sandbox');

// 3. Neumorphic surfaces without glow: every shadow is a token built from one dark and one light tone.
assert.doesNotMatch(css, /text-shadow|drop-shadow|\bfilter\s*:|backdrop-filter/i, 'no glow, blur or filter effects');
const shadows = [...css.matchAll(/box-shadow\s*:\s*([^;}]+)/g)].map(match => match[1].trim());
assert(shadows.length >= 6, 'surfaces use shadows');
for (const value of shadows) {
  assert.match(value, /^(?:none|var\(--nm-(?:raised|raised-sm|inset|inset-sm|pressed)\))$/, `shadow outside the token set: ${value}`);
}

// 4. The window box includes its padding: no second scrollbar on the document.
assert(declared('#taik-live', 'box-sizing').includes('border-box'), '#taik-live counts its padding inside the window height');
assert.equal(px(declared('#taik-live', 'height')[0] ?? '0px'), null, '#taik-live keeps its viewport-height rule');
assert.match(rulesFor('#taik-live').map(rule => rule.body).join(';'), /height:\s*100vh/, '#taik-live still fills the window');

// 5. Scrollbars: WebKit's own pseudo-elements, a thumb that is not a relief tone, no scrollbar-width outside the fallback.
const supportsFallback = css.match(/@supports\s+not\s+selector\(::-webkit-scrollbar\)\s*\{([\s\S]*?)\n\s*\}/)?.[1] ?? '';
const outsideFallback = css.replace(supportsFallback, '');
assert.doesNotMatch(outsideFallback, /scrollbar-width\s*:|scrollbar-color\s*:/, 'scrollbar-width only inside the @supports fallback');
assert.match(supportsFallback, /scrollbar-width\s*:/, 'other engines keep a thin native bar');
assert.doesNotMatch(css, /scrollbar-color\s*:\s*var\(--nm-shade\)/, 'the relief tone is not used as the scrollbar colour');
const scrollbarWidth = declared('::-webkit-scrollbar', 'width')[0];
assert(scrollbarWidth, '::-webkit-scrollbar has a width');
assert(Math.abs((px(scrollbarWidth) ?? 0) - 9) < 0.01, `::-webkit-scrollbar is 9px wide (${scrollbarWidth})`);
const thumb = declared('::-webkit-scrollbar-thumb', 'background')[0];
assert(thumb, '::-webkit-scrollbar-thumb has a background');
assert.match(thumb, /color-mix\(in srgb, var\(--quiet\) 80%, var\(--nm-base\)\)/, `the thumb is mixed from quiet and the surface: ${thumb}`);
assert(/transparent/.test(declared('::-webkit-scrollbar-track', 'background')[0] ?? ''), 'the scrollbar track is transparent');

// 6. Room for the shadow reach inside the scrollport, without shifting the cards.
const activityPadding = desktop('#taik-activity', 'padding').at(-1);
assert(activityPadding, '#taik-activity declares its padding');
const padValues = activityPadding.split(/\s+/).map(value => px(value) ?? 0);
assert(Math.min(...padValues) >= 24, `every padding of #taik-activity is at least 24px: ${activityPadding}`);
const activityMargin = desktop('#taik-activity', 'margin').at(-1) ?? '';
// A negative margin keeps its sign: px() reads the number, the sign is applied here.
const marginValues = activityMargin.split(/\s+/).map(value => (px(value) ?? 0) * (value.trim().startsWith('-') ? -1 : 1));
// Left side only: the cards must keep their horizontal alignment with the header on the left.
const left = padValues[3] ?? padValues[1] ?? padValues[0];
const leftMargin = marginValues[3] ?? marginValues[1] ?? 0;
assert(Math.abs(left + leftMargin) < 0.5, `the left margin cancels the left padding (${left} vs ${leftMargin}): padding ${activityPadding} margin ${activityMargin}`);
const previewPadding = desktop('#taik-live .preview', 'padding').at(-1);
assert(previewPadding && Math.min(...previewPadding.split(/\s+/).map(value => px(value) ?? 0)) >= 24,
  `#taik-live .preview has the same room: ${previewPadding}`);

// 7. No gradient as a control arrow. The flow connector may keep its repeating gradient.
assert(count(/gradient\(/g) <= 1, `at most the flow connector gradient remains (${count(/gradient\(/g)})`);
assert.doesNotMatch(css, /linear-gradient\((?:45|135)deg/, 'the select arrow is not drawn with two gradient triangles');
assert.match(css, /\.appearance::after|\.job-control::after/, 'the select arrow is drawn by a pseudo element');

// 8. Controls are big enough to hit.
const controlSelectors = ['#taik-live summary', '#taik-live select', '#taik-live .status', '#taik-live .modes button'];
for (const selector of controlSelectors) {
  const heights = desktop(selector, 'min-height').map(px);
  assert(heights.length && Math.min(...heights) >= 28, `${selector} is at least 28px high (${heights})`);
}

// 9. Nothing is clipped in the middle of a word, and long words wrap.
const detail = declared('#taik-live .flow-detail', '-webkit-line-clamp')[0];
assert(detail && parseInt(detail, 10) >= 2, 'the flow detail is clamped to at least two lines');
assert.match(rulesFor('#taik-live .events li').map(rule => rule.body).join(';'), /overflow-wrap:\s*anywhere|word-break:\s*break-all/,
  'long words in the work steps wrap');

// 10. The smallest text is still readable.
const fontSizes = [...css.matchAll(/font-size\s*:\s*([^;}]+)|font\s*:\s*([^;}]+)/g)]
  .flatMap(match => (match[1] ?? match[2]).match(/[\d.]+rem/g) ?? []).map(rem);
assert(fontSizes.length, 'the sheet declares text sizes');
assert(Math.min(...fontSizes) >= 0.65, `the smallest text is at least 0.65rem (${Math.min(...fontSizes)})`);
assert(rem(declared('#taik-live .flow-op', 'font').at(-1).match(/[\d.]+rem/)?.[0]) >= 0.66, 'the flow operation word is at least 0.66rem');

// 11. The flow grid follows its own pane, not only the window.
assert(declared('#taik-live .flow-pane', 'container-type').includes('inline-size'), 'the flow pane is an inline-size container');
const containerRule = css.match(/@container\s*\(max-width:\s*\d+px\)\s*\{([\s\S]*?)\n\s*\}/)?.[1] ?? '';
assert(containerRule, 'a container query exists');
assert(/\.flow\s*\{[^}]*grid-template-columns:\s*repeat\(2/.test(containerRule), 'the container query narrows the flow to two columns');

// 12. Motion still stops for reduced motion.
const reduced = css.match(/@media\s*\(prefers-reduced-motion:\s*reduce\)\s*\{([\s\S]*?)\n\s*\}/g) ?? [];
assert(reduced.some(block => /animation:\s*none/.test(block)), 'animations stop for reduced motion');

// 13. Readable in both appearances.
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

console.log(`PASS: host contract, ${shadows.length} token shadows, window box, 9px WebKit scrollbar with a mixed thumb, 24px shadow room, no gradient arrow, 28px controls, readable type, pane-driven flow, reduced motion, contrast ${Math.min(...measured).toFixed(2)}`);
