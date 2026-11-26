Der Orchestrator soll seinen Auftragssatz sichtbar machen, Produkte und geschützte Pfade nennen, ausgefallene Agenten berücksichtigen und Fehlschläge benannter Gate-Autoren als Zustand führen.

Ziel: `src/orchestrator.rs` plant jeden Wunsch. Ergänze den Befehl `taik-seed orchestrator-prompt STATE REQUEST.run`. Er gibt genau den Text auf stdout aus, den der Orchestrator für diesen Wunsch an seinen Worker schicken würde, und ruft keinen Worker auf. `plan()` verwendet denselben Text.

Auftragssatz:
- Der Abschnitt beginnt mit einer Zeile `PRODUCTS:`. Für jede Datei `STATE/products/NAME.product` folgt `- NAME: <Text des Feldes about=>`. Gibt es keine, folgt `(none)`. Der Text sagt dem Worker, dass eine Änderung an einem gelisteten Produkt die Art `dokument` ist.
- Unter jedem Repository in `REPOSITORIES:` steht eingerückt `  protected: ` mit den nicht leeren Zeilen ohne `#` aus dessen `.taik/protected`, durch Komma und Leerzeichen getrennt, oder `  protected: (none)`. Der Text sagt, dass eine Sitzung diese Pfade nicht ändern darf.
- Existiert `STATE/crew/NAME.failed` und ist jünger als sechs Stunden, hängt an der Zeile dieses Agenten in `AGENTS:` ` (failed recently)`. Wählt der Worker so einen Agenten für `Gate:`, obwohl ein anderer nicht ausgefallener Agent existiert, der nicht die Arbeit macht, ersetzt der Orchestrator ihn durch den ersten solchen Agenten und hängt an den Grund einen Satz an, der beide Namen mit vorangestelltem @ nennt.

Dienst:
- Scheitert der Aufruf eines benannten Gate-Autors, schreibt der Dienst `STATE/crew/NAME.failed` mit einer Zeile Fehlermeldung. Liefert derselbe Agent später ein Gate, wird die Datei entfernt. Für den Standard-Worker wird keine solche Datei geschrieben.

Gate-Vorgaben, genau diese Werte prüft das Gate:
- Build: `cargo build --offline --locked`, Binary `target/debug/taik-seed`.
- Zustand: `STATE/products/kasse.product` mit `about=Kasse`, `STATE/products/lager.product` mit `about=Lager`; `STATE/repositories/alpha/.taik/protected` mit `# Kommentar`, `src/secret.rs`, Leerzeile, `Cargo.toml`; `STATE/repositories/beta/.taik/protected` leer; `STATE/crew/gate-author` und `STATE/crew/other` als normale Agenten; `STATE/crew/gate-author.failed` mit `kaputt`; `STATE/crew/other.failed` mit Zeitstempel vor sieben Stunden.
- Request: `decision=pending`, `identity=gate-check`, `intent=Prüfe den Auftragssatz`, `observation=Ein Wunsch`, `continuation=Test`.
- Erwartet im Auftragssatz: `PRODUCTS:`, `- kasse: Kasse`, `- lager: Lager`, `  protected: src/secret.rs, Cargo.toml`, `  protected: (none)`, `gate-author` mit ` (failed recently)` am Zeilenende und `other` ohne diesen Zusatz. Ohne Produkte erscheint `(none)`.
- `cargo test --offline --locked` läuft. In `README.md` steht unter den Befehlen genau `taik-seed orchestrator-prompt STATE REQUEST.run`.

Bestehendes Verhalten, bestehende Ausgaben und bestehende Tests bleiben unverändert. Neue Tests zeigen jeden Punkt: Befehl ohne Worker, Produkte, leere Produkte, geschützte Pfade, leere Pfade, junger und alter Failed-Marker, Gate-Autor-Ersatz, Schreiben und Entfernen von `.failed`, kein `.failed` für den Standard-Worker.
