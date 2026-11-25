In der Factory-Ansicht den Token- und Zeitfluss jeder Runde als Balken zeigen.

Du bekommst die installierte Ansicht als angenommenen Ausgangsstand. Ändere nur, was dieser Auftrag verlangt. Liefere bevorzugt einen Patch. Alles andere an der Ansicht bleibt Zeichen für Zeichen erhalten, insbesondere die Stilregeln (Schatten nur über die vorhandenen `--nm-*`-Variablen, kein Glow, kein Filter), alle vorhandenen IDs und Funktionen und der Platzhalter `/*FACTORY_DATA*/null`.

## Was dazukommt

1. Eine reine Funktion `tokenFlow(rounds)` zwischen den Kommentaren `/*TOKEN_FLOW_BEGIN*/` und `/*TOKEN_FLOW_END*/`. Der Block braucht nichts außerhalb von sich.
   - `rounds` ist ein Array von Objekten mit den Feldern eines Rundenprotokolls. Alle Werte sind Zeichenketten und können fehlen.
   - Ergebnis ist ein Array gleicher Länge mit je einem Objekt `{ input, output, cost, seconds, widthIn, widthOut }`.
   - `input` ist `worker_input_tokens`, `output` ist `worker_output_tokens`, `cost` ist `worker_cost_usd`, jeweils als Zahl. `seconds` ist `worker_duration_ms` geteilt durch 1000. Fehlende, leere oder nicht lesbare Werte zählen als 0.
   - `widthIn` und `widthOut` sind Prozentwerte: `input` bzw. `output` geteilt durch die größte Summe `input + output` aller Runden, mal 100. Die größte Runde füllt also zusammen 100. Gibt es keine Tokens, sind alle Breiten 0.

2. In `renderRound` unter der Zeile mit Rundennummer und Entscheidung ein Balken für Runden mit Tokens:
   `<div class="token-flow" title="…">` mit zwei `<span>`-Segmenten, deren `style="width:…%"` aus `widthIn` und `widthOut` kommt, das erste mit Klasse `in`, das zweite mit Klasse `out`. Darunter eine Zeile mit Klasse `sub` in der Form `1 431 → 13 598 Tokens · 25 s · 0,007 $`. Tausender mit schmalem Leerzeichen, Sekunden ohne Nachkommastelle ab 10 s und mit einer darunter, Kosten über die vorhandene Funktion `money`. Das `title` nennt die Protokolldatei der Runde, etwa `round-01.run`.
   Damit `renderRound` die Breiten kennt, berechne `tokenFlow(job.rounds.map(round => round.fields))` einmal je Auftrag in `render` und reiche den Eintrag der Runde durch.

3. Im Kopf des Auftragsverlaufs, direkt unter der Auftragsauswahl, eine Summenzeile mit Klasse `sub` für den gewählten Auftrag: Summe der Eingabe-Tokens, Summe der Ausgabe-Tokens, Summe der Sekunden und die Zahl der Runden, etwa `Gesamt 5 455 → 22 871 Tokens · 60 s · 1 Runde`. Ohne Tokens entfällt die Zeile.

4. CSS im vorhandenen Stilblock:
   - `#taik-live .token-flow`: Höhe 8px, `display: flex`, Abstand nach oben 8px, Radius 4px, `box-shadow: var(--nm-inset-sm)`, `overflow: hidden`.
   - `.token-flow .in` mit `background: var(--quiet)`, `.token-flow .out` mit `background: var(--active)`.
