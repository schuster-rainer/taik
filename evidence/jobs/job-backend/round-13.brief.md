Benannte Agenten einführen: Harness, Modell, Effort und Persona je Auftrag, dazu Grok und Pi als Worker.

Beobachtung: Der Kern sprach drei Worker an (Codex, Claude-CLI, Savant), Modell und Effort standen fest im Quelltext, und eine Umgebungsvariable wählte den Worker für den ganzen Dienst. Der Nutzer will am 2026-11-25 Agenten in der Factory konfigurieren und je Ticket umstellen können, damit er T3 nicht mehr braucht. Kein Protokoll nennt bisher Pi oder Grok als Worker.

Qualifiziert vor dem Bau, je ein echter Aufruf in der Sandbox und aus einem launchd-Job: Claude, Grok und Pi laufen ohne Werkzeuge und melden Modell, Tokens und Kosten. Grok und Pi brauchen Schreibrecht auf ihr eigenes Verzeichnis (`~/.grok`, `~/.pi`). Gemini und OpenCode sind nicht angemeldet und bleiben draußen.

Umfang:

- Ein Agent ist eine Datei `STATE/crew/NAME.agent` mit `harness`, `model`, `effort`, `persona`. Unbekannte Werte werden abgelehnt.
- Ein Auftrag nennt seinen Agenten in der Datei `agent` in seinem Ordner. Der Dienst liest sie vor jeder Runde. Ohne die Datei gilt wie bisher die Umgebung.
- Ein Agent, den es nicht gibt, hält den Auftrag an. Es läuft nichts auf einem anderen Konto.
- Die Persona steht vor dem festen Auftragssatz der Factory und ersetzt ihn nie.
- Das Protokoll nennt den Agenten und das Modell, das der Harness gemeldet hat.
- Ein Harness ohne Kostenangabe zählt mit 0,25 USD je Aufruf gegen die Tagesgrenze.
- `taik-seed crew STATE` schreibt die Übersicht der Harnesse (installiert, angemeldet, Modelle, Efforts) und der Agenten nach `STATE/crew/overview.json`. Gelesen wird nur der Anmeldestatus, nie ein Zugangsdatum.
