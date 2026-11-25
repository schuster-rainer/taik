Ein Markenblatt für TAIK als eigenständiges HTML-Dokument bauen.

Das Blatt zeigt die Marke so, dass jemand sie ohne Rückfrage einsetzen kann. Es beginnt mit `<!doctype html>`, kommt ohne Netzwerk, ohne Skripte und ohne Bilddateien aus und ist höchstens 24 KiB groß.

Inhalt:

- Überschrift `<h1>TAIK</h1>` und ein Satz, was TAIK ist: eine Software-Factory, die ihren eigenen Prozess auf sich selbst anwendet.
- Die Bildmarke als Inline-SVG mit `viewBox="0 0 64 64"` in mindestens drei Größen (etwa 128, 48 und 20 Pixel), jeweils auf hellem und auf dunklem Grund. Die beiden Pfade unten unverändert übernehmen. Der zweite Pfad ist der Kern und trägt die Akzentfarbe.
- Die Wortmarke: `TAIK` halbfett und `FACTORY` normal und gedämpft, beide in Versalien mit weitem Zeichenabstand.
- Die Farben mit Hex-Wert: Grund hell `#e4e8ee`, Grund dunkel `#21262e`, Schrift hell `#1b2330`, Schrift dunkel `#e9eef5`, Akzent hell `#0d6b5f`, Akzent dunkel `#6fd0bf`.
- Der Name in drei kurzen Punkten: ein Wortbild zu *talk* wie in Smalltalk, nur nicht small. Der Klang des deutschen Wortes *Teig*. Und *talk* mit **AI** in der Mitte.
- Regeln: flach und matt, kein Glow, kein Verlauf, kein Schlagschatten auf der Marke, Schutzraum von einer halben Markenbreite.

Gestaltung: ruhig, präzise, viel Weißraum. CSS-Variablen `--base` und `--ink` in der Form `light-dark(#rrggbb, #rrggbb)`, damit das Blatt in Hell und Dunkel lesbar ist.

Pfad 1 (Kammern):

```
M26.22 14.55A17.35 17.35 0 1 1 18.13 43.58A15.72 15.72 0 1 0 26.22 14.55Z M16.43 31.11A10.96 10.96 0 1 1 36.03 35.93A10.1 10.1 0 1 0 16.43 31.11Z M22.27 37.25A6.47 6.47 0 1 1 32.15 29.67A7.04 7.04 0 0 0 22.27 37.25Z
```

Pfad 2 (Kern):

```
M26.6 39.95A4.95 4.95 0 1 1 34.1 34.52A5.32 5.32 0 0 0 26.6 39.95Z
```
