import assert from "node:assert/strict";
import fs from "node:fs";

const src = fs.readFileSync(process.argv[2], "utf8");
assert.ok(/^\s*<!doctype html>/i.test(src), "Dokument muss mit <!doctype html> beginnen");
assert.ok(src.length <= 512 * 1024, "Dokument zu groß");
assert.match(src, /<title>\s*Teig\s*<\/title>/, "Titel muss 'Teig' sein");
assert.ok(!/(?:src|href)\s*=\s*["']?\s*(?:https?:)?\/\//i.test(src), "Keine externen Verweise erlaubt");
assert.ok(!/@import|url\(\s*["']?https?:/i.test(src), "Keine externen Ressourcen in CSS");
assert.match(src, /prefers-color-scheme/, "Hell/Dunkel nach Systemeinstellung fehlt");

const defaults = { mehl: 500, hydration: 70, salz: 2, sauerteig: 20, sauerteigHydration: 100 };
for (const [id, v] of Object.entries(defaults)) {
  const m = [...src.matchAll(/<input\b[^>]*>/gi)].map((x) => x[0]).find((t) => new RegExp(`\\bid\\s*=\\s*["']${id}["']`).test(t));
  assert.ok(m, `Eingabefeld mit id ${id} fehlt`);
  assert.match(m, /type\s*=\s*["']?number/i, `${id} muss type=number sein`);
  const val = m.match(/\bvalue\s*=\s*["']?([0-9.]+)/i);
  assert.ok(val && Number(val[1]) === v, `${id} Standardwert muss ${v} sein`);
}

const b = src.indexOf("/*TEIG_BEGIN*/");
const e = src.indexOf("/*TEIG_END*/");
assert.ok(b >= 0 && e > b, "TEIG-Marker fehlen oder falsche Reihenfolge");
const code = src.slice(b + 14, e);
let teig;
try {
  teig = new Function('"use strict";\n' + code + "\n;return teig;")();
} catch (err) {
  assert.fail("Code zwischen den Markern nicht allein auswertbar: " + err.message);
}
assert.equal(typeof teig, "function", "teig ist keine Funktion");

function ref(p) {
  const S = (p.mehl * p.sauerteig) / 100;
  const sm = S / (1 + p.sauerteigHydration / 100);
  const sw = S - sm;
  const md = p.mehl - sm;
  const tw = (p.mehl * p.hydration) / 100;
  const wd = tw - sw;
  const salz = (p.mehl * p.salz) / 100;
  return {
    sauerteig: Math.round(S), sauerteigMehl: Math.round(sm), sauerteigWasser: Math.round(sw),
    mehlDazu: Math.round(md), wasserDazu: Math.round(wd), salz: Math.round(salz),
    gesamt: Math.round(p.mehl + tw + salz), moeglich: md >= 0 && wd >= 0,
  };
}

assert.deepEqual(
  { ...teig(defaults) },
  { sauerteig: 100, sauerteigMehl: 50, sauerteigWasser: 50, mehlDazu: 450, wasserDazu: 300, salz: 10, gesamt: 860, moeglich: true },
  "Standardbeispiel falsch"
);

const cases = [
  { mehl: 500, hydration: 70, salz: 2, sauerteig: 80, sauerteigHydration: 100 },
  { mehl: 333, hydration: 65, salz: 1.8, sauerteig: 15, sauerteigHydration: 80 },
  { mehl: 1000, hydration: 75, salz: 2.2, sauerteig: 25, sauerteigHydration: 60 },
  { mehl: 500, hydration: 10, salz: 2, sauerteig: 40, sauerteigHydration: 100 },
  { mehl: 500, hydration: 70, salz: 2, sauerteig: 120, sauerteigHydration: 0 },
  { mehl: 750, hydration: 68, salz: 2, sauerteig: 0, sauerteigHydration: 100 },
];
for (const c of cases) {
  const got = teig(c);
  const want = ref(c);
  assert.deepEqual({ ...got }, want, "Falsches Ergebnis für " + JSON.stringify(c));
  for (const k of Object.keys(want)) {
    if (k !== "moeglich") assert.ok(Number.isInteger(got[k]), `${k} muss ganzzahlig sein`);
  }
  assert.equal(typeof got.moeglich, "boolean", "moeglich muss Wahrheitswert sein");
}
assert.equal(teig(cases[3]).moeglich, false, "negatives Wasser muss moeglich=false ergeben");
assert.equal(teig(cases[4]).moeglich, false, "negatives Mehl muss moeglich=false ergeben");

console.log("PASS: Teig-Rechner erfüllt Format und Rechnung");
