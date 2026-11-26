Der Befehl `taik-seed decline STATE NAME` lehnt einen wartenden Vorschlag ab oder zieht einen Wunsch zurück.

# Ablehnen und Zurückziehen

Bisher kann der Nutzer einen Vorschlag nur freigeben (`taik-seed approve`). Ablehnen kann er ihn nicht. Einen Wunsch, zu dem der Orchestrator eine Rückfrage stellt, kann er nicht zurückziehen. Das soll der neue Befehl `taik-seed decline STATE NAME` leisten. Er liest die Begründung von der Standardeingabe (1 bis 2000 Bytes, wie bei `close`).

## Verhalten

1. Gibt es den Vorschlagsordner `STATE/proposals/NAME`, und ist er weder freigegeben (`approved.txt`) noch abgelehnt (`declined.md`), legt der Befehl die Begründung unverändert als `declined.md` in diesen Ordner. Das geschieht einmalig, es wird nie überschrieben. Der Status ist 0.
2. Gibt es keinen Vorschlag, aber die Datei `STATE/requests/NAME.run`, und ist der Wunsch noch nicht zurückgezogen, legt der Befehl die Begründung unverändert als `STATE/requests/NAME.withdrawn` ab (einmalig). Der Status ist 0.
3. Ein abgelehnter Vorschlag kann nicht mehr freigegeben werden: `taik-seed approve STATE NAME` endet dann mit Status 2 und legt keinen Auftrag an (`STATE/jobs` bleibt ohne neuen Eintrag, `approved.txt` entsteht nicht).
4. Einen zurückgezogenen Wunsch plant der Dienst nicht mehr und leitet nichts aus ihm ab, auch nicht nach einer Antwort auf eine Rückfrage (`derive_requests` in `src/service.rs`). `taik-seed reply STATE NAME` auf einen zurückgezogenen Wunsch endet mit Status 2 und schreibt nichts.
5. Mit Status 2 und ohne etwas zu schreiben werden abgewiesen: ein unbekannter Name, ein ungültiger Name (Regel von `worker_name`: nur a-z, 0-9 und `-`, höchstens 32 Zeichen), eine leere oder zu lange Begründung (über 2000 Bytes), ein schon freigegebener Vorschlag, ein zweites Ablehnen und ein zweites Zurückziehen.

Wie `approve` und `reply` darf `decline` auf macOS beschränkt sein (`#[cfg(target_os = "macos")]`). Nehmt den Befehl auch in den `USAGE`-Text in `src/main.rs` auf.

## Tests

- Neues Testziel `tests/decline.rs`. Es kommt ohne Worker und ohne Sandbox aus: Vorschlag und Wunsch legt der Test selbst im Zustandsordner an (Vorbild für Aufbau und Hilfsfunktionen: `tests/request_question.rs`, mit `#![cfg(target_os = "macos")]`). Es zeigt die Punkte 1, 2, 3 und 5 sowie die Abweisung von `reply` aus Punkt 4. Es enthält mindestens 4 Tests.
- Der Teil von Punkt 4, der den Dienst betrifft (`drain` plant einen zurückgezogenen Wunsch nicht, auch nicht nach einer Antwort), bekommt einen Test nach dem Vorbild von `tests/request_question.rs`, in einer eigenen Datei, zum Beispiel `tests/decline_service.rs`. Er läuft mit einem Stand-in-Worker und in der Sandbox des Dienstes. Das Gate des Auftrags führt ihn nicht aus. Er muss aber kompilieren und clippy-rein sein.

## README

Im Abschnitt „Commands“ der `README.md` steht die Zeile `taik-seed decline STATE NAME`.

## Muss bleiben

- Bestehende Tests bleiben unverändert (keine Änderung an den vorhandenen Dateien unter `tests/`).
- Das Verhalten von `approve`, `reply`, `close` und `request` für nicht abgelehnte und nicht zurückgezogene Fälle bleibt gleich.
- `cargo clippy --offline --locked --all-targets -- -D warnings` bleibt ohne Befund.
