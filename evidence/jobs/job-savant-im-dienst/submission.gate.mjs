// Gate for the TAIK brand sheet.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const html = readFileSync(process.argv[2], 'utf8');
assert.match(html, /^<!doctype html>/i, 'standalone document');
assert(html.length <= 24 * 1024, 'at most 24 KiB');
assert.doesNotMatch(html, /<script\b|<img\b|<iframe\b|<link\b|<object\b|<embed\b|https?:\/\/|@import|url\(/i, 'inert, offline, no image files');
assert.doesNotMatch(html, /text-shadow|drop-shadow|box-shadow|\bfilter\s*:|gradient\(/i, 'flat and matte: no shadow, glow or gradient');
assert.match(html, /<h1[^>]*>\s*TAIK\s*<\/h1>/, 'heading');
const chambers = 'M26.22 14.55A17.35 17.35 0 1 1 18.13 43.58A15.72 15.72 0 1 0 26.22 14.55Z M16.43 31.11A10.96 10.96 0 1 1 36.03 35.93A10.1 10.1 0 1 0 16.43 31.11Z M22.27 37.25A6.47 6.47 0 1 1 32.15 29.67A7.04 7.04 0 0 0 22.27 37.25Z';
const core = 'M26.6 39.95A4.95 4.95 0 1 1 34.1 34.52A5.32 5.32 0 0 0 26.6 39.95Z';
const count = (text) => html.split(text).length - 1;
assert(count(chambers) >= 1 && count(core) >= 1, 'both mark paths are present unchanged');
const marks = (html.match(/<svg\b[^>]*viewBox="0 0 64 64"/g) ?? []).length + (html.match(/<use\b/g) ?? []).length;
assert(marks >= 6, `the mark appears in three sizes on two grounds (found ${marks})`);
for (const colour of ['#e4e8ee', '#21262e', '#1b2330', '#e9eef5', '#0d6b5f', '#6fd0bf']) {
  assert(html.toLowerCase().includes(colour), `colour ${colour} is shown`);
}
const text = html.replace(/<style[\s\S]*?<\/style>/gi, '').replace(/<[^>]+>/g, ' ');
for (const word of ['FACTORY', 'Teig', 'Smalltalk', 'AI', 'Schutzraum']) assert(text.includes(word), `the sheet mentions ${word}`);
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
console.log(`PASS: inert brand sheet with the unchanged mark ${marks} times, six colours and readable in light and dark`);
