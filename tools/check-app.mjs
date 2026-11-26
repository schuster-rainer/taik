// Gate of the Mac app: what the freshly built host hands to the viewer.
// Reads the output of `factory-view --snapshot` on stdin. The viewer contract is checked separately
// by tools/check-factory-design.swift on the same snapshot.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

// The snapshot is the viewer with its data put in the place of the marker. The viewer is argv[2].
const page = readFileSync(0, 'utf8').replace(/\n+$/, '');
const [before, after] = readFileSync(process.argv[2], 'utf8').replace(/\n+$/, '').split('/*FACTORY_DATA*/null');
assert(after !== undefined, 'the viewer has no place for data');
assert(page.startsWith(before) && page.endsWith(after), 'the snapshot is not the viewer with data in it');
const text = page.slice(before.length, page.length - after.length);
let data;
try { data = JSON.parse(text); } catch (error) { assert.fail('the data of the snapshot is not JSON: ' + String(error.message).slice(0, 120)); }
assert(!/<\/script|<!--/i.test(text), 'the data can end the script early');

const kinds = { repos: 'array', index: 'array', parallel: 'number', agents: 'array', proposals: 'array', spent: 'number', jobs: 'array', pending: 'number' };
for (const [key, kind] of Object.entries(kinds)) {
  const value = data[key];
  assert(kind === 'array' ? Array.isArray(value) : typeof value === kind, `snapshot.${key} is not ${kind === 'array' ? 'a list' : 'a ' + kind}`);
}
assert(data.parallel >= 1 && data.parallel <= 8, 'snapshot.parallel is outside 1 to 8');

const jobKeys = ['id', 'cost', 'messages', 'changes', 'analyses', 'dialogs', 'baseline', 'rounds', 'progress', 'sourceQueued', 'sourceActive'];
for (const job of data.jobs) {
  for (const key of jobKeys) assert(key in job, `job ${job.id} lacks ${key}`);
  assert(/^job-[A-Za-z0-9-]+$/.test(job.id), `job id ${job.id}`);
  assert(Array.isArray(job.rounds) && job.rounds.every(round => round.name && round.fields && typeof round.detail === 'string'), `job ${job.id}: a round lacks name, fields or detail`);
  assert(Array.isArray(job.messages) && job.messages.every(message => ['user', 'agent'].includes(message.author) && typeof message.text === 'string'), `job ${job.id}: a message is malformed`);
}
for (const proposal of data.proposals) {
  for (const key of ['name', 'intent', 'origin', 'brief', 'messages']) assert(key in proposal, `proposal ${proposal.name} lacks ${key}`);
  assert(proposal.state === undefined || ['deriving', 'declined', 'failed', 'asks'].includes(proposal.state), `proposal ${proposal.name}: unknown state ${proposal.state}`);
}
for (const entry of data.index) assert(entry.id && 'title' in entry && 'decision' in entry && Number.isInteger(entry.rounds), `stock entry ${entry.id} is malformed`);
for (const repo of data.repos) assert(repo.name && repo.path && Array.isArray(repo.protected), `repository ${repo.name} is malformed`);
// Sign-in state may be shown, credentials never.
assert(!/sk-[A-Za-z0-9]{20,}|"(?:api[_-]?key|token|secret|password)"\s*:\s*"[^"]{8,}/i.test(text), 'the snapshot carries something that looks like a credential');

console.log(`PASS: ${data.jobs.length} jobs, ${data.index.length} stock entries, ${data.proposals.length} proposals, ${data.repos.length} repositories`);
