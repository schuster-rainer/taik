const assert = require('node:assert/strict');
const { spawn, spawnSync } = require('node:child_process');
const { createHash } = require('node:crypto');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { performance } = require('node:perf_hooks');
const { setTimeout: sleep } = require('node:timers/promises');

const archives = [0, 500, 2000];
const rounds = ['round-01.run', 'round-02.run', 'round-03.run'];
const sampleCount = Number(process.argv[4] ?? 12);
if (process.argv.length < 4 || !Number.isSafeInteger(sampleCount) || sampleCount < 2) {
  console.error('Usage: node queue-latency.cjs BASELINE_BINARY CANDIDATE_BINARY [SAMPLES]');
  process.exit(2);
}
const binaries = process.argv.slice(2, 4).map(pathToBinary => {
  const location = fs.realpathSync(pathToBinary);
  return {
    path: location,
    sha256: createHash('sha256').update(fs.readFileSync(location)).digest('hex'),
  };
});
const root = fs.mkdtempSync(path.join(os.tmpdir(), 'taik-archive-perf-'));
const seed = path.join(root, 'seed.run');
const daemons = [];

function run(binary, ...arguments_) {
  const result = spawnSync(binary.path, arguments_, { encoding: 'utf8' });
  assert.equal(result.status, 0, result.error?.message ?? result.stderr);
  return result.stdout;
}

function field(report, name) {
  const prefix = `${name}=`;
  return report.split('\n').find(line => line.startsWith(prefix))?.slice(prefix.length);
}

function checkOutcome(report) {
  assert.equal(field(report, 'decision'), 'accepted');
  assert.equal(field(report, 'advance'), 'false');
  assert.equal(field(report, 'tags'), 'ash,cedar');
  assert.equal(field(report, 'procedure'), 'trim,drop-blank,lower,sort,dedup');
  assert.equal(field(report, 'checks'), 'artifact_matches:true, probe_matches:true, size_within_limit:true');
  assert(report.endsWith('end\n'));
}

function template() {
  run(binaries[0], 'seed', seed);
  const state = path.join(root, 'template');
  const job = run(binaries[0], 'queue', state, seed).trim();
  run(binaries[0], 'drain', state);
  checkOutcome(fs.readFileSync(path.join(job, 'round-03.run'), 'utf8'));
  const names = ['baseline.run', ...rounds, 'origin.txt'];
  return names.map(name => [name, fs.readFileSync(path.join(job, name))]);
}

function jobRoot(state) {
  return path.join(state, 'jobs');
}

function archiveJob(state, index) {
  return path.join(jobRoot(state), `job-a-${String(index).padStart(4, '0')}`);
}

async function deliver(workload, index) {
  const { state, child } = workload;
  const name = `job-zz-fresh-${String(index).padStart(3, '0')}`;
  const queued = path.join(jobRoot(state), name);
  const staged = path.join(state, `${name}.pending`);
  const terminal = path.join(queued, 'round-03.run');
  fs.mkdirSync(staged);
  fs.copyFileSync(seed, path.join(staged, 'baseline.run'));
  fs.writeFileSync(path.join(staged, 'origin.txt'), 'observation=isolated archive benchmark\n');
  const start = performance.now();
  fs.renameSync(staged, queued);
  while (!fs.existsSync(terminal)) {
    assert.equal(child.exitCode, null, `Factory process exited: ${workload.stderr}`);
    assert(performance.now() - start < 15_000, `Timed out: ${workload.stderr}`);
    await sleep(1);
  }
  const elapsed = performance.now() - start;
  checkOutcome(fs.readFileSync(terminal, 'utf8'));
  return elapsed;
}

async function setup(version, archived, files) {
  const state = path.join(root, `version-${version}-${archived}`);
  fs.mkdirSync(jobRoot(state), { recursive: true });
  for (let index = 0; index < archived; index++) {
    const job = archiveJob(state, index);
    fs.mkdirSync(job);
    for (const [name, bytes] of files) fs.writeFileSync(path.join(job, name), bytes);
  }
  const child = spawn(binaries[version].path, ['serve', state], { stdio: ['ignore', 'ignore', 'pipe'] });
  daemons.push(child);
  const workload = { version, archived, state, child, stderr: '', samples: [] };
  child.stderr.on('data', chunk => { workload.stderr += String(chunk); });
  const started = performance.now();
  while (!workload.stderr.includes('watching for jobs')) {
    assert.equal(child.exitCode, null, `Factory process exited: ${workload.stderr}`);
    assert(performance.now() - started < 10_000, `Failed to start: ${workload.stderr}`);
    await sleep(1);
  }
  await deliver(workload, 0);
  await sleep(100);
  return workload;
}

function checkArchive(workload, files) {
  for (let index = 0; index < workload.archived; index++) {
    const job = archiveJob(workload.state, index);
    for (const [name, bytes] of files) {
      assert(fs.readFileSync(path.join(job, name)).equals(bytes), `Changed archive: ${job}/${name}`);
    }
  }
}

function summarize(workload) {
  const sorted = [...workload.samples].sort((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  return {
    version: workload.version,
    completed_before_warmup: workload.archived,
    median_ms: sorted.length % 2 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2,
    empirical_p95_ms: sorted[Math.ceil(sorted.length * 0.95) - 1],
    samples_ms: workload.samples,
  };
}

async function main() {
  try {
    const files = template();
    const workloads = [];
    for (const archived of archives) {
      for (const version of archived === 500 ? [1, 0] : [0, 1]) {
        workloads.push(await setup(version, archived, files));
      }
    }
    for (let index = 1; index <= sampleCount; index++) {
      const order = index % 2 ? workloads : [...workloads].reverse();
      for (const workload of order) {
        workload.samples.push(await deliver(workload, index));
        await sleep(5);
      }
    }
    for (const workload of workloads) checkArchive(workload, files);
    const results = workloads.map(summarize).sort((a, b) => a.completed_before_warmup - b.completed_before_warmup || a.version - b.version);
    console.log(JSON.stringify({
      machine: { platform: os.platform(), architecture: os.arch(), release: os.release(), cpu: os.cpus()[0]?.model, node: process.version },
      binaries,
      process: 'Warm persistent serve. Atomic local job publication to readable terminal .run. One warmup plus balanced serial trials. Output and archived bytes checked.',
      limits: 'One 3-round local tag workload, warm filesystem, approximately 1 ms observation interval, no submission fsync or remote software jobs. Archive grows by one job per trial.',
      results,
    }, null, 2));
  } finally {
    for (const child of daemons) child.kill();
    await sleep(80);
    fs.rmSync(root, { recursive: true, force: true });
  }
}

main().catch(error => {
  console.error(error);
  process.exitCode = 1;
});
