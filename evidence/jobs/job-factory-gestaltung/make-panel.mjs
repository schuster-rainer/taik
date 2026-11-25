import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
const [file, title, note] = process.argv.slice(2);
const image = readFileSync(file);
const html = text => text.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');
console.log(`<!doctype html><html lang="de"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>${html(title)}</title><style>:root{color-scheme:light dark;font:15px system-ui,sans-serif;color:light-dark(#142237,#e7eff8);background:light-dark(#f4f7fa,#111b25)}body{margin:0;padding:16px}header{display:flex;gap:6px 20px;flex-wrap:wrap;align-items:baseline;margin-bottom:12px}h1{margin:0;font-size:1rem}p{margin:0;color:light-dark(#425670,#a6b8ca)}img{display:block;width:100%;height:auto;max-height:74vh;object-fit:contain;object-position:left top;border-radius:5px}</style>
<header><h1>${html(title)}</h1><p>${html(note)}</p></header>
<img alt="${html(title)}" data-sha256="${createHash('sha256').update(image).digest('hex')}" src="data:image/jpeg;base64,${image.toString('base64')}"></html>`);
