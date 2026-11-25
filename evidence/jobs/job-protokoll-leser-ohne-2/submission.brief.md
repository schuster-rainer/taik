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
