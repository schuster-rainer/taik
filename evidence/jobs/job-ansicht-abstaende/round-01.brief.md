Runde: Abstände, Scrollbars und Verläufe der Ansicht nach Messung berichtigen.

Du bekommst die installierte Ansicht als angenommenen Ausgangsstand. Liefere einen Patch. Ändere nur die Regeln, die zu den unten genannten Befunden gehören, und was unmittelbar von ihnen abhängt. Alles andere bleibt Zeichen für Zeichen erhalten. Markup, Kennungen und die Funktionen `updateFactory`, `workpieceState`, `tokenFlow` und `highlightRun` bleiben unverändert.

Beobachtung mit WebKit bei 1300x820, jede Zahl gemessen:

1. `#taik-live` hat `height: 100vh` und Innenabstand, aber kein `border-box`. Das Dokument wird 860px hoch in einem 820px hohen Fenster: doppelte Scrollleiste, unterer Rand abgeschnitten.
2. `scrollbar-width: thin` steht nur auf `#taik-live` und wird in WebKit nicht an die scrollenden Bereiche vererbt (`#taik-activity`: `auto`). `scrollbar-color` unterstützt WebKit gar nicht. `::-webkit-scrollbar` wird ignoriert, solange `scrollbar-width` gesetzt ist.
3. Die Karten im Auftragsverlauf sind am Scrollrand beschnitten: der Schatten `--nm-raised` reicht 23px, der Innenabstand war 10px.
4. Der Pfeil der Auswahlfelder bestand aus zwei `linear-gradient`-Dreiecken.
5. Mehrere Bedienelemente waren nur 23–24px hoch, `summary` 24px.
6. `span.flow-detail` war abgeschnitten (sichtbar 102px, Inhalt 141px).
7. In einer Kopfzeile standen Chips mit 32px neben Auswahlfeldern mit 33,5px. Die Ablauf-Knoten waren unterschiedlich hoch (68px bis 80px), ein Name war 14px versetzt.
8. Die kleinste Schrift war 9,6px bei weiter Sperrung.
9. Ein langes ununterbrochenes Wort lief in den Arbeitsschritten über (2736px in 358px).
10. Bei 801px und 1024px Fensterbreite (Spalten nebeneinander) schnürte das Raster die fünf Ablauf-Knoten auf 55px, Text lief über.

Regeln:

- `#taik-live` rechnet mit `border-box`, das Dokument ist nie höher als das Fenster.
- Scrollleisten: eigene `::-webkit-scrollbar`-Regeln mit 9px Breite, durchsichtigem Hintergrund und einem Daumen aus `color-mix(in srgb, var(--quiet) 80%, var(--nm-base))` (Kontrast mindestens 3:1 gegen die Fläche). `scrollbar-width` nur innerhalb von `@supports not selector(::-webkit-scrollbar)`.
- Innenabstand der scrollenden Bereiche mindestens 24px, damit der Schatten nicht beschnitten wird, ohne dass die Karten gegenüber der Kopfzeile verrutschen.
- Kein `gradient(` mehr für den Pfeil der Auswahlfelder; ein gedrehter Rahmen leistet dasselbe.
- Jedes Bedienelement mindestens 28px hoch.
- Kein Text wird abgeschnitten: entweder vollständig sichtbar oder so geklemmt, dass kein Umbruch mitten im Wort entsteht.
- Elemente einer Zeile sind gleich hoch, Ablauf-Knoten gleich hoch und ihre Namen auf einer Linie.
- Schriftgrößen bleiben lesbar: kleinste Schriftgröße mindestens 10,5px.
- Lange Wörter brechen um, nichts läuft waagerecht über.
- Das Ablauf-Raster richtet sich nach der Breite seines Bereichs (`container-type: inline-size` mit `@container`), nicht nur nach dem Fenster.
- Hell und dunkel bleiben lesbar, `prefers-reduced-motion` wird weiter beachtet.

Belegt wird das mit dem mitgelieferten Prüfprogramm `tools/check-factory-design.swift` auf dem vollständigen Ergebnis.
