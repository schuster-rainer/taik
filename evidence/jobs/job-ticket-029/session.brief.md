Die README bekommt einen neuen englischen Abschnitt `## Working through the orchestrator`, der beschreibt, wie ein Wunsch heute durch die Factory läuft.

## Ziel

In `README.md` steht direkt vor `## Status` ein neuer Abschnitt mit genau der Überschrift `## Working through the orchestrator`. Er ist auf Englisch geschrieben wie der Rest der README. Die Überschrift kommt in der Datei genau einmal vor.

Lies vor dem Schreiben den Code, damit der Abschnitt nur beschreibt, was wirklich so ist:

- `src/orchestrator.rs`
- `src/proposal.rs`
- `src/session.rs`
- `src/service.rs`
- `src/adopt.rs`
- `.taik/land.sh`
- `tools/factory/`
- `.taik/protected`

Behaupte nichts, was der Code nicht tut.

## Inhalt in dieser Reihenfolge

1. **Der Weg eines Wunsches.**
   - `taik-seed request` nimmt den Wunsch entgegen.
   - Der Plan des Orchestrators nennt Dokument oder Sitzung, den Autor des Gates, den Arbeiter und den Grund.
   - Es gibt Rückfrage und Antwort mit `taik-seed reply`.
   - Der Vorschlag enthält Auftrag und Gate.
   - Danach folgt `taik-seed approve` oder `taik-seed decline`.
2. **Die zwei Arten von Aufträgen.**
   - Dokument: ein Textdokument. Bei einem Produkt gilt dessen Gate, dazu eine zusätzliche Prüfung, die auf dem alten Stand scheitern muss.
   - Sitzung: eigene Kopie, Datei-Werkzeuge ohne Shell, Urteil über den Commit. Geschützte Pfade stehen in `.taik/protected`.
3. **Nach der Annahme.**
   - Der Zweig `taik/NAME` entsteht im Ursprungs-Repository (`taik-seed settle`, `taik-seed adopt`).
   - Für dieses Repository folgt die Übernahme mit `.taik/land.sh`.
4. **Vier Befehle in je einem Satz:** `taik-seed close`, `taik-seed decline`, `taik-seed orchestrator-prompt`, `taik-seed reflect`.
5. **Zwei Grenzen, ehrlich benannt:**
   - Das Gate einer Sitzung kann keine weitere Sandbox starten.
   - Ein Sitzungsagent hat keine Shell.
   Die Wörter „sandbox“ und „shell“ kommen im Abschnitt vor.

## Form

- Der Abschnitt ist von seiner Überschrift bis vor `## Status` höchstens 60 Zeilen lang.
- Jeder Befehl steht in Code-Schreibweise (Backticks), also zum Beispiel `` `taik-seed request` ``, `` `taik-seed reply` ``, `` `taik-seed approve` ``, `` `taik-seed decline` ``, `` `taik-seed settle` ``, `` `taik-seed adopt` ``, `` `taik-seed close` ``, `` `taik-seed orchestrator-prompt` ``, `` `taik-seed reflect` ``. Auch `` `.taik/land.sh` ``, `` `.taik/protected` `` und `` `taik/NAME` `` stehen in Backticks.
- Die Reihenfolge der ersten Erwähnungen: `taik-seed request`, `taik-seed reply`, `taik-seed approve`, dann `.taik/protected`, dann `taik/NAME`. `taik-seed close`, `taik-seed orchestrator-prompt` und `taik-seed reflect` stehen danach.
- Im Abschnitt kommen keine deutschen Umlaute und kein ß vor.

## Was bleibt

- Die übrige README bleibt unverändert, bis auf Verweise, die durch den neuen Abschnitt falsch würden. Alle bisherigen `##`-Überschriften bleiben je einmal erhalten, ebenso die ersten Zeilen bis zum Satz über die Spezifikation.
- Es wird kein Rust-Code und keine andere Datei geändert. Das gilt besonders für `src/`, `tests/`, `.taik/` und `tools/`.
