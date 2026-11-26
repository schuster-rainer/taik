#!/bin/sh
# Gate: README section "Working through the orchestrator"
test -f README.md || { echo "FAIL: README.md fehlt"; exit 1; }
out=$(node -e '
const fs=require("fs");
const t=fs.readFileSync("README.md","utf8");
const L=t.split("\n");
const H="## Working through the orchestrator";
const idx=[];L.forEach((l,i)=>{if(l.trimEnd()===H)idx.push(i)});
if(idx.length!==1){console.log("Ueberschrift \""+H+"\" kommt "+idx.length+" mal vor, erwartet genau 1");process.exit(1)}
const s=idx[0];
let e=-1;for(let i=s+1;i<L.length;i++){if(/^## /.test(L[i])){e=i;break}}
if(e<0||L[e].trimEnd()!=="## Status"){console.log("Der Abschnitt steht nicht direkt vor \"## Status\"; naechste Ueberschrift: "+(e<0?"keine":L[e]));process.exit(1)}
let end=e;while(end>s+1&&L[end-1].trim()==="")end--;
const sec=L.slice(s,end);
if(sec.length>60){console.log("Abschnitt hat "+sec.length+" Zeilen, erlaubt sind 60");process.exit(1)}
if(sec.length<12){console.log("Abschnitt ist mit "+sec.length+" Zeilen zu kurz fuer den verlangten Inhalt");process.exit(1)}
const body=sec.join("\n");
if(/[äöüÄÖÜß]/.test(body)){console.log("Abschnitt enthaelt deutsche Zeichen, er soll englisch sein");process.exit(1)}
const first=(c)=>{const re=new RegExp("`[^`\n]*"+c.replace(/[.*+?^${}()|[\]\\\/]/g,"\\$&")+"[^`\n]*`");const m=re.exec(body);return m?m.index:-1};
const need=["taik-seed request","taik-seed reply","taik-seed approve","taik-seed decline","taik-seed settle","taik-seed adopt","taik-seed close","taik-seed orchestrator-prompt","taik-seed reflect",".taik/land.sh",".taik/protected","taik/NAME"];
const pos={};
for(const c of need){pos[c]=first(c);if(pos[c]<0){console.log("Nicht in Code-Schreibweise genannt: "+c);process.exit(1)}}
const order=["taik-seed request","taik-seed reply","taik-seed approve",".taik/protected","taik/NAME"];
for(let i=1;i<order.length;i++)if(pos[order[i]]<pos[order[i-1]]){console.log("Reihenfolge verletzt: "+order[i]+" steht vor "+order[i-1]);process.exit(1)}
for(const c of ["taik-seed close","taik-seed orchestrator-prompt","taik-seed reflect"])if(pos[c]<pos["taik/NAME"]){console.log(c+" steht vor der Beschreibung von taik/NAME");process.exit(1)}
if(!/sandbox/i.test(body)){console.log("Grenze mit Sandbox fehlt im Abschnitt");process.exit(1)}
if(!/shell/i.test(body)){console.log("Grenze ohne Shell fehlt im Abschnitt");process.exit(1)}
// rest of README
const heads=["## Status","## How a job runs","## Build and try","## Software jobs","## Workers","## Agents","## The conversation of a job","## The service","## Jobs the Factory proposes","## Requests from GitHub issues","## Changing the backend","## The viewer","## What has been measured","## Limits","## Commands","## License"];
for(const h of heads){const n=L.filter(l=>l.trimEnd()===h).length;if(n!==1){console.log("Ueberschrift \""+h+"\" kommt "+n+" mal vor, erwartet 1");process.exit(1)}}
const spec="The specification is [TAIK-FACTORY-SEED-v0.4.md](TAIK-FACTORY-SEED-v0.4.md). It is frozen.";
if(!t.includes(spec)||!t.includes("# TAIK\n")||!t.includes("This is a running seed, not a product.")){console.log("Urspruengliche README-Teile wurden veraendert");process.exit(1)}
if(!t.includes("taik-seed close STATE job-NAME")||!t.includes("[MIT](LICENSE)")){console.log("Bestehende Befehlsliste oder Lizenz veraendert");process.exit(1)}
console.log("OK");
' 2>&1) || { echo "FAIL: $out"; exit 1; }
echo "PASS: README-Abschnitt 'Working through the orchestrator' ist vorhanden, kurz genug, an der richtigen Stelle und nennt alle Befehle und Grenzen"
