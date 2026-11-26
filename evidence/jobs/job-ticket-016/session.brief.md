Der Zweig taik/NAME eines Auftrags rückt bei `adopt` und `settle` auf den neu angenommenen Commit vor, wenn er auf einen Vorfahren zeigt.

## Problem

In `src/adopt.rs` weist `adopt` die Übernahme ab, sobald `taik/NAME` im Ursprungs-Repository schon auf einen anderen Commit zeigt. Das trifft jeden Auftrag, der nach einer angenommenen Runde durch eine Antwort des Nutzers eine weitere angenommene Runde bekommt. Der neue Commit baut auf dem alten auf und wird trotzdem nicht übernommen. `settle` und der Dienst sind mit betroffen, weil sie `adopt` aufrufen.

## Gewünschtes Verhalten

`taik-seed adopt STATE job-NAME` und `taik-seed settle STATE job-NAME` verhalten sich in vier Fällen so:

1. **Kein Zweig `taik/NAME`:** Er wird angelegt und zeigt auf den angenommenen Commit. Das ist der heutige Weg und bleibt.
2. **Der Zweig zeigt auf den angenommenen Commit:** Das ist weiter ein Erfolg, und nichts ändert sich.
3. **Der Zweig zeigt auf einen Vorfahren des angenommenen Commits:** Er rückt auf den neuen Commit vor, nur vorwärts wie ein Fast-Forward. Die Übernahme gelingt mit Status 0 und gibt `taik/NAME` aus.
4. **Der Zweig zeigt auf einen Commit, der kein Vorfahr ist:** Die Übernahme wird wie bisher mit Status 2 abgelehnt, und der Zweig bleibt unverändert. Der Zweig darf nie rückwärts oder seitwärts bewegt werden.

Ist im Ursprungs-Repository gerade der Zweig `taik/NAME` ausgecheckt (HEAD ist `refs/heads/taik/NAME`), weist `adopt` die Übernahme mit Status 2 ab und verändert nichts. Das gilt auch, wenn der Zweig auf einen Vorfahren zeigt. So geraten keine Arbeitsdateien des Nutzers hinter ihren Zweig. Beim Anlegen oder beim unveränderten Zweig (Fälle 1 und 2) bleibt alles wie bisher.

`settle` schreibt `adopted.txt` weiterhin nur einmal und ersetzt eine vorhandene Datei nie. Ihr Inhalt, der Zweigname, ändert sich durch das Nachrücken nicht.

Nach der Übernahme bleiben HEAD, ausgecheckter Stand und Arbeitsdateien des Ursprungs-Repositorys unverändert. Das gilt nicht für den ausgecheckten Fall, der abgewiesen wird.

## Tests

- **Neues Testziel `tests/adopt_forward.rs`:** Es nutzt ein echtes kleines Git-Repository und keine Sandbox, wie `tests/adopt.rs`. Es zeigt alle vier Fälle: kein Zweig, Vorfahr, kein Vorfahr, ausgecheckt. Bei Vorfahr erwartet es Erfolg und den vorgerückten Zweig. Bei kein Vorfahr und bei ausgecheckt erwartet es Status 2 und einen unveränderten Zweig.
- **`a_branch_on_another_commit_is_refused` in `tests/adopt.rs`:** Der Test widerspricht dem neuen Verhalten, weil er einen Vorfahren verwendet. Passe ihn an. Der Zweig soll auf einen Commit zeigen, der kein Vorfahr des angenommenen Commits ist, zum Beispiel ein eigener Commit im Ursprung neben `base`. Der Test erwartet weiter Status 2 und einen unveränderten Zweig.
- **Übrige Tests:** Alle anderen bestehenden Tests bleiben unverändert.

## Muss bleiben

- `cargo clippy --offline --locked --all-targets -- -D warnings` meldet keinen Befund.
- Die Namen und das Format von `adopted.txt` und `adopt-failed.txt` bleiben, ebenso die Kommandozeile von `adopt` und `settle`.
- Es gibt keine neuen Abhängigkeiten.
