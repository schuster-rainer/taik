Die Crew-Karte der Ansicht bekommt einen aufklappbaren Bereich „Repositories“, in dem man Repositories sehen und über die Nachricht addRepo bekannt machen kann.

Ändere nur das Folgende am Produkt ansicht. Alles andere bleibt, wie es ist.

Datenquelle: snapshot.repos ist eine Liste von Objekten { name, path, protected }. protected ist eine Liste von Pfaden, die eine Sitzung im Repository nicht ändern darf. Fehlt snapshot.repos, gilt eine leere Liste.

1. Bereich „Repositories“ in der Karte „Crew“
- Der Bereich ist aufklappbar und trägt als Titel „Repositories“ mit der Zahl der Einträge in Klammern, zum Beispiel „Repositories (2)“.
- Er ist zunächst zugeklappt.
- Er merkt sich wie die anderen aufklappbaren Bereiche der Crew, ob der Nutzer ihn geöffnet hat. Dieser Zustand bleibt erhalten, wenn die Ansicht neue Daten bekommt.
- Geöffnet zeigt er je Repository:
  - den Namen,
  - den Pfad in einem code-Element,
  - darunter den Text „geschützt: “ gefolgt von den geschützten Pfaden, durch Komma getrennt. Ist die Liste leer, steht dort „geschützt: nichts“.
- Nichts wird abgeschnitten: Namen, Pfade und geschützte Pfade erscheinen vollständig, ohne Auslassungspunkte und ohne verdeckenden Überlauf. Lange Pfade dürfen umbrechen.
- Alle angezeigten Werte werden als Text eingesetzt, nicht als HTML.

2. Eingabe unter der Repository-Liste im selben Bereich
- Zwei Eingabefelder mit den Beschriftungen „Name“ und „Pfad“ und ein Knopf mit dem Text „Bekannt machen“.
- Der Inhalt der Felder bleibt erhalten, wenn die Ansicht neue Daten bekommt, auch wenn dabei die Crew neu gezeichnet wird.

3. Reine Funktion validRepo(name, path, repos) im Block CREW
- Die Funktion steht zwischen den Markierungen /*CREW_BEGIN*/ und /*CREW_END*/ als Funktionsdeklaration `function validRepo(name, path, repos)`, sodass sie sich mit dem Rest des Blocks auswerten lässt, ohne auf Dokument oder andere Namen außerhalb des Blocks zuzugreifen.
- Sie hat keine Nebenwirkungen (sie verändert insbesondere repos nicht) und liefert eine Zeichenkette. Eine leere Zeichenkette bedeutet: Eingabe in Ordnung.
- Geprüft wird in dieser Reihenfolge, die erste Verletzung gewinnt:
  a) Der Name muss aus 1 bis 32 Zeichen bestehen, nur Kleinbuchstaben a–z, Ziffern 0–9 und Bindestrich. Sonst lautet die Meldung genau: Name: Kleinbuchstaben, Ziffern und Bindestrich, höchstens 32 Zeichen
  b) Enthält repos einen Eintrag mit genau diesem Namen, lautet die Meldung genau: Diesen Namen gibt es schon
  c) Der Pfad wird zuerst von Leerzeichen am Rand befreit. Er ist gültig, wenn er danach mit / oder mit ~ beginnt. Ein leerer Pfad ist ungültig. Sonst lautet die Meldung genau: Pfad: absolut oder mit ~ beginnend
- Der Name wird nicht getrimmt: ein Name mit Leerzeichen am Rand ist ungültig.
- repos kann fehlen oder leer sein; dann gibt es keinen Namenskonflikt.
- Beispiele: validRepo("mein-repo", "/srv/x", []) ergibt ""; validRepo("mein-repo", "  ~/x  ", []) ergibt ""; validRepo("Mein", "/x", []) ergibt die Namensmeldung; validRepo("a", "x", [{name:"a"}]) ergibt „Diesen Namen gibt es schon“; validRepo("a", "rel/pfad", []) ergibt die Pfadmeldung.

4. Verhalten des Knopfs „Bekannt machen“
- Liefert validRepo für die aktuellen Eingaben und die aktuelle snapshot.repos eine leere Zeichenkette, sendet der Knopf genau einmal window.webkit.messageHandlers.addRepo.postMessage({ name, path }). Dabei ist name der Wert des Namensfelds und path der von Leerzeichen am Rand befreite Wert des Pfadfelds. Danach werden beide Felder geleert.
- Andernfalls wird nichts gesendet. Die Meldung von validRepo erscheint als Text neben dem Knopf, und die Felder bleiben unverändert.
- Bei einem erfolgreichen Senden verschwindet eine zuvor angezeigte Meldung.

Alles andere an der Ansicht bleibt unverändert: die übrigen Karten, Bereiche, Namen, Nachrichten und Funktionen, und das Format als Fragment mit Stil, Markup und Skript.
