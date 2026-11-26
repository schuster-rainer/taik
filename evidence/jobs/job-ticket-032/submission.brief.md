Erstelle einen eigenständigen Brotteig-Rechner als einzelne HTML-Datei, die ohne Netz läuft.

**Format:** Ein einziges HTML-Dokument, das mit `<!doctype html>` beginnt. Stil, Markup und Skript stehen alle in dieser Datei. Es gibt keine fremden Schriften, Bilder oder Skripte und keine Verweise auf http(s)-Adressen. Der Titel der Seite (`<title>`) lautet genau `Teig`.

**Eingaben:** Fünf `<input type="number">`-Felder mit diesen ids und Standardwerten im Attribut `value`:
- `mehl` = 500 (Gesamtmehl in Gramm, einschließlich des Mehls im Sauerteig)
- `hydration` = 70 (Prozent, bezogen auf das Gesamtmehl)
- `salz` = 2 (Prozent des Gesamtmehls)
- `sauerteig` = 20 (Anteil Sauerteig in Prozent des Gesamtmehls)
- `sauerteigHydration` = 100 (Hydration des Sauerteigs in Prozent)

Jedes Feld hat ein sichtbares Label.

**Ausgabe:** Sie aktualisiert sich sofort bei jeder Änderung eines Feldes und zeigt:
- Sauerteig in Gramm, darin Mehl und Wasser
- Mehl, das zusätzlich abzuwiegen ist
- Wasser, das zusätzlich abzuwiegen ist
- Salz
- Teiggewicht gesamt

Alle Werte sind auf ganze Gramm gerundet. Ist das zusätzliche Mehl oder das zusätzliche Wasser negativ, steht an dieser Stelle ein Hinweis, dass die Rezeptur so nicht aufgeht, statt einer negativen Zahl.

**Rechenfunktion:** Im Skript der Seite steht zwischen den Kommentaren `/*TEIG_BEGIN*/` und `/*TEIG_END*/` eine reine Funktion `teig({ mehl, hydration, salz, sauerteig, sauerteigHydration })`. Der Code zwischen den Markern wird allein, ohne Browserumgebung, ausgewertet. Er darf daher weder `document` noch `window` verwenden und enthält die Deklaration `function teig`. Prozentwerte sind Zahlen, z. B. 70 für 70 %.

Sie rechnet zuerst exakt und rundet dann jeden Ergebniswert einzeln mit `Math.round`:
- Sauerteig S = mehl · sauerteig/100
- sauerteigMehl = S / (1 + sauerteigHydration/100)
- sauerteigWasser = S − sauerteigMehl
- mehlDazu = mehl − sauerteigMehl
- Gesamtwasser = mehl · hydration/100
- wasserDazu = Gesamtwasser − sauerteigWasser
- salz = mehl · salz/100
- gesamt = mehl + Gesamtwasser + salz
- moeglich ist `true`, wenn die exakten Werte mehlDazu und wasserDazu beide ≥ 0 sind, sonst `false`.

Rückgabe: `{ sauerteig, sauerteigMehl, sauerteigWasser, mehlDazu, wasserDazu, salz, gesamt, moeglich }`. Alle Werte außer `moeglich` sind gerundete ganze Zahlen.

Beispiel mit den Standardwerten: sauerteig 100 (50 g Mehl, 50 g Wasser), mehlDazu 450, wasserDazu 300, salz 10, gesamt 860, moeglich true.

**Gestaltung:** Ruhig und gut lesbar. Hell und dunkel richten sich nach der Systemeinstellung (`prefers-color-scheme`).
