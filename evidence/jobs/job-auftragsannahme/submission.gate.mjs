import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const html = readFileSync(process.argv[2], 'utf8');
assert.match(html, /^<!doctype html>/i);
assert.match(html, /<h1[^>]*>\s*Auftrag einreichen\s*<\/h1>/i);
const example = html.match(/<pre[^>]*>([\s\S]+?)<\/pre>/i)?.[1]
  .replace(/<\/?code[^>]*>/gi, '')
  .replaceAll('&quot;', '"');
assert.ok(example, 'A preformatted command example is required');
assert.ok(example.includes('state="$HOME/.local/state/taik-factory"'));
assert.ok(example.includes('"$state/taik-seed" queue-source "$state" mein-auftrag ./brief.md ./verify.mjs'));
for (const term of ['evidence/jobs/job-mein-auftrag', 'round-01.run', 'round-01.html']) {
  assert.ok(html.includes(term), `Missing output location ${term}`);
}
assert.doesNotMatch(html, /<(?:script|img|iframe|link|object|embed|a)\b|https?:\/\//i);
console.log('PASS: offline HTML, heading, runnable example and named output paths');
