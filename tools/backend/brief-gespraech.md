Die Unterhaltung eines Auftrags in den Auftrag legen und Aufrufe ohne Kostenangabe aus einer Preisliste berechnen.

Beobachtung des Nutzers am 2026-11-25: Ein Auftrag hat eine Beschreibung, und die Antworten von Agent und Nutzer gehören als Verlauf an denselben Auftrag. Gebaut war etwas anderes: Eine Antwort aus der Ansicht landete als Datei in `STATE/inbox`, das kein Teil des Kerns las, und ein Worker konnte nie etwas zurückfragen.

Umfang Unterhaltung:

- Eine Nachricht ist eine Datei im Auftragsordner, `message-NNN.user.md` oder `message-NNN.agent.md`, einmal geschrieben und nie ersetzt.
- `taik-seed reply STATE JOB` schreibt die Antwort des Nutzers in den Auftrag und weckt den Dienst.
- Hat der Nutzer seit der letzten Runde geantwortet, läuft ein abgeschlossener Auftrag genau eine weitere Runde. Der Worker liest die ganze Unterhaltung. Jede Antwort gewährt eine Runde über das Budget von vier hinaus, höchstens acht.
- Ein Worker darf statt eines Kandidaten eine Rückfrage liefern (`=== FRAGE ===`). Dann wird nichts beurteilt, die Frage steht im Auftrag, ihre Kosten stehen in einem eigenen Protokoll, und der Auftrag wartet, bis der Nutzer geantwortet hat.

Umfang Preise: Der Nutzer fragte, welcher Harness keine Kosten melde, und verlangte eine Preisliste. Es war nur Codex, dessen CLI Tokens meldet, aber keinen Betrag. `taik-seed crew` liefert jetzt Listenpreise je Modell aus dem Modellkatalog von Pi und der SubQ-Dokumentation. Ein Codex-Aufruf mit gemeldeten Tokens und bekanntem Modell wird daraus berechnet. Nur ohne diese Angaben zählt weiter der feste Betrag. Am echten Codex ist das nicht geprüft, weil das Konto kein Kontingent hat.
