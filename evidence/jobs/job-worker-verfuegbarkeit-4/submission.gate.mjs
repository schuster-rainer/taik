import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
const html = readFileSync(process.argv[2], 'utf8');
assert.match(html, /^<!doctype html>/i, 'standalone document');
assert.match(html, /<h1[^>]*>\s*Worker bereit\s*<\/h1>/, 'heading');
assert.match(html, /<p[^>]*>[^<]{20,}<\/p>/, 'one explanatory paragraph');
assert.doesNotMatch(html, /<(?:script|img|iframe|link|object|embed)\b|https?:\/\//i, 'inert and offline');
console.log('PASS: inert probe document with heading and paragraph');
