Einen eigenständigen Protokoll-Leser für `.run`-Entscheidungsprotokolle bauen.

Liefere ein HTML-Dokument, das mit `<!doctype html>` beginnt, ohne Netzwerk auskommt und höchstens 24 KiB groß ist. Keine externen Ressourcen, keine Bilder, keine Frames.

Aufbau: Überschrift `<h1>Protokoll-Leser</h1>`, ein `<textarea id="record">` für den Protokolltext und ein `<pre id="view">`, das den Text bei jeder Eingabe ausgezeichnet zeigt. Farben als CSS-Variablen `--base` und `--ink` in der Form `light-dark(#rrggbb, #rrggbb)`, Kontrast von `--ink` auf `--base` mindestens 7 in Hell und Dunkel.

Die Auszeichnung leistet eine Funktion `highlightRun(text)`. Sie steht zwischen den Kommentaren `/*RUN_HIGHLIGHT_BEGIN*/` und `/*RUN_HIGHLIGHT_END*/`, braucht nichts außerhalb dieses Blocks und gibt HTML als Zeichenkette zurück. Regeln:

- Ausgabe ist der unveränderte Text, HTML-maskiert (`&`, `<`, `>`, `"`, `'`), nur ergänzt um `<span class="run-…">…</span>` mit genau einer Klasse.
- Eine Zeile, die genau `end` lautet: `run-end`.
- Eine Zeile `schlüssel=wert`, deren Schlüssel bis zum ersten `=` aus Kleinbuchstaben, Ziffern und `_` besteht und nicht mit einer Ziffer beginnt: Schlüssel als `run-key`, das `=` bleibt unmarkiert. Weitere `=` gehören zum Wert.
- Im Wert und in Zeilen ohne Schlüssel:
  - Prüfsummen aus 40 bis 64 Hex-Kleinbuchstaben und Ziffern: `run-hash`.
  - Die Wörter `accepted`, `PASS`, `true`, `retained`: `run-good`. Die Wörter `rejected`, `FAIL`, `false`, `error`: `run-bad`.
  - Dateinamen mit den Endungen `.run .html .mjs .md .log .json .txt .png .jpg .gz` samt vorangestelltem relativem Verzeichnis, außerdem Namen, die auf `-taik-seed` enden: als Ganzes `run-path`. Beispiele: `../job-x/round-01.run`, `round-09.baseline.tar.gz`, `round-08-taik-seed`.
  - Zahlen mit optionalem Dezimalteil und optional direkt angehängter Einheit `KiB`, `MiB`, `ms`, `s`, `%`: `run-number`. Steht die Einheit `µs` nach einem Leerzeichen, wird nur die Zahl markiert. Ziffern, die Teil eines Namens sind wie in `round-02`, sind keine Zahl.
- Für jede Klasse `run-key`, `run-good`, `run-bad`, `run-end`, `run-hash`, `run-path`, `run-number` gibt es eine CSS-Regel.

Wiederverwendbares Ergebnis aus einem anderen Auftrag: Der folgende Block wurde in `job-factory-neomorph/round-01` von einem unabhängigen Gate angenommen und läuft in der Factory-Ansicht. Du darfst ihn unverändert übernehmen oder anpassen. Das Gate dieses Auftrags prüft mit anderen Protokollen als das damalige.

```js
/*RUN_HIGHLIGHT_BEGIN*/
function highlightRun(text) {
  const esc = value => value.replace(/[&<>"']/g, character => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[character]);
  const span = (kind, value) => '<span class="run-' + kind + '">' + esc(value) + '</span>';
  const tokens = /\b[a-f0-9]{40,64}\b|(?:\.\.?\/)?[\w.\/-]*\.(?:run|html|mjs|md|log|json|txt|png|jpg|gz)\b|[\w.-]+-taik-seed\b|\b(?:accepted|PASS|true|retained)\b|\b(?:rejected|FAIL|false|error)\b|(?<![\w.-])\d+(?:\.\d+)?(?:KiB|MiB|ms|µs|s|%)?(?![\w-])/g;
  const kinds = match => /^[a-f0-9]{40,64}$/.test(match) ? 'hash'
    : /^(?:accepted|PASS|true|retained)$/.test(match) ? 'good'
    : /^(?:rejected|FAIL|false|error)$/.test(match) ? 'bad'
    : /^\d/.test(match) && !/[\/]|\.[a-z]{2,4}$/.test(match) ? 'number' : 'path';
  const value = line => {
    let cursor = 0;
    let markup = '';
    for (const token of line.matchAll(tokens)) {
      markup += esc(line.slice(cursor, token.index)) + span(kinds(token[0]), token[0]);
      cursor = token.index + token[0].length;
    }
    return markup + esc(line.slice(cursor));
  };
  return String(text ?? '').split('\n').map(line => {
    if (line === 'end') return span('end', line);
    const at = line.indexOf('=');
    const key = line.slice(0, Math.max(at, 0));
    if (!/^[a-z_][a-z0-9_]*$/.test(key)) return value(line);
    return span('key', key) + '=' + value(line.slice(at + 1));
  }).join('\n');
}
/*RUN_HIGHLIGHT_END*/
```
