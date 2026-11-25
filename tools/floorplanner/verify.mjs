import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

const html = fs.readFileSync(process.argv[2], "utf8");
assert.match(html, /^<!doctype html>/i, "standalone HTML");
assert.doesNotMatch(html, /<(?:script|iframe|link|img|object|embed)\b[^>]*(?:src|href|data|srcdoc)\s*=|https?:\/\//i, "offline artifact");
const scripts = [...html.matchAll(/<script\b([^>]*)>([\s\S]*?)<\/script>/gi)];
assert(scripts.length >= 2, "separate model and interactive layer");
for (const script of scripts) new vm.Script(script[2]);
const pure = scripts.find(script => /\bid=["']layout-model["']/.test(script[1]));
assert(pure, "model script present");
const context = vm.createContext({}, {codeGeneration:{strings:false,wasm:false}});
new vm.Script(pure[2]).runInContext(context, {timeout:1000});
const call = expression => JSON.parse(new vm.Script("JSON.stringify(" + expression + ")").runInContext(context, {timeout:1000}));
const plan = call("floorPlan.initial()");
assert.equal(plan.width, 24);
assert.equal(plan.depth, 18);
assert.deepEqual(plan.entrance, [1,16]);
assert.deepEqual(plan.exit, [22,16]);
assert(plan.fixtures.length < 40 && plan.fixtures.length >= 6);
assert.equal(new Set(plan.fixtures.map(f => f.id)).size, plan.fixtures.length);
for (const kind of ["produce","bakery","shelf","cold","checkout"]) assert(plan.fixtures.some(f => f.kind === kind));
const cell = (x,y) => x + y*plan.width;
const occupied = new Set();
for (const f of plan.fixtures) {
  assert([f.x,f.y,f.w,f.d].every(Number.isInteger));
  assert(f.x >= 0 && f.y >= 0 && f.w > 0 && f.d > 0 && f.x+f.w <= plan.width && f.y+f.d <= plan.depth);
  for(let y=f.y;y<f.y+f.d;y++) for(let x=f.x;x<f.x+f.w;x++) {
    assert(!occupied.has(cell(x,y)), "fixtures overlap");
    occupied.add(cell(x,y));
  }
}
const open = (x,y) => x>=0 && y>=0 && x+1<plan.width && y+1<plan.depth &&
  ![[x,y],[x+1,y],[x,y+1],[x+1,y+1]].some(([a,b])=>occupied.has(cell(a,b)));
const result = call("floorPlan.validate(floorPlan.initial())");
assert.equal(result.valid,true);
assert(result.route.length > 1 && result.route.length <= plan.width*plan.depth);
assert.deepEqual(result.route[0],plan.entrance);
assert.deepEqual(result.route.at(-1),plan.exit);
for (let i=0;i<result.route.length;i++) {
  const [x,y] = result.route[i];
  assert(open(x,y),"cart route crosses a fixture");
  if(i) assert.equal(Math.abs(x-result.route[i-1][0])+Math.abs(y-result.route[i-1][1]),1);
}
const selected = plan.fixtures.find(f=>f.kind==="shelf");
const move = (x,y) => call("floorPlan.move(floorPlan.initial()," + JSON.stringify(selected.id) + "," + x + "," + y + ")");
assert.equal(move(-1,0),null);
const other = plan.fixtures.find(f => f.id !== selected.id);
assert.equal(move(other.x,other.y),null);
const available = plan.fixtures.flatMap(f => [[f.x-1,f.y],[f.x+1,f.y],[f.x,f.y-1],[f.x,f.y+1]]);
assert(available.some(([x,y]) => {
  if(x===selected.x&&y===selected.y) return false;
  const moved=move(x,y);
  return moved && moved.fixtures.some(f=>f.id===selected.id&&f.x===x&&f.y===y);
}),"at least one real legal move");
assert.deepEqual(call("floorPlan.initial()"),plan,"move leaves initial state alone");
for (const [x,y] of [[0,0],[6,4],[23,17]]) {
  const [a,b]=call("floorPlan.unproject(...floorPlan.project("+x+","+y+"))");
  assert(Math.abs(x-a)<1e-6 && Math.abs(y-b)<1e-6,"projection round trip");
}
assert.match(html,/<(?:canvas|svg)\b/i);
assert.match(scripts.map(s=>s[2]).join("\n"),/addEventListener|onpointer|onclick/i);
if (process.argv[3] === "follow-up") {
  for (const [x,y] of available) {
    const changed = move(x,y);
    if (!changed || x===selected.x && y===selected.y) continue;
    const before = JSON.stringify(plan), after = JSON.stringify(changed);
    assert.deepEqual(call("floorPlan.undo("+after+","+before+")"),plan,"undo restores exact previous plan");
    break;
  }
  assert.match(html,/Rückgängig/i,"undo control present");
}
console.log("PASS: offline document, independent fixture geometry, 2×2 cart route across "+(result.route.length-1)+" steps, legal and illegal moves, projection round-trip"+(process.argv[3]==="follow-up"?", undo":""));
