node --input-type=module -e "
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
const { greet } = await import(process.cwd() + '/greet.mjs');
assert.equal(greet('Rainer'), 'Hallo, Rainer!');
assert.equal(greet('  Ada '), 'Hallo, Ada!');
for (const empty of [undefined, null, '', '   ']) assert.equal(greet(empty), 'Hallo, Welt!');
const readme = readFileSync('README.md', 'utf8');
assert.match(readme, /Hallo, Welt!/, 'the README names the fallback');
assert.doesNotMatch(readme, /English greeting/, 'the README no longer describes the old behaviour');
console.log('PASS: German greeting, fallback for missing names, README updated');
"
