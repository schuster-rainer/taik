Bei der Freigabe eines Vorschlags wird dessen plan.txt unverändert in den Auftragsordner übernommen.

## Ausgangslage

`taik-seed approve STATE NAME` (in `src/proposal.rs`, Funktion `approve`) legt aus `STATE/proposals/NAME` den Auftragsordner `STATE/jobs/job-NAME` an. Dokument-Aufträge gehen über `service::queue_source`, Sitzungsaufträge über `session::queue`. Die Datei `STATE/proposals/NAME/plan.txt` bleibt dabei beim Vorschlag zurück. Später weiß dadurch niemand am Auftrag, wer das Gate geschrieben hat.

## Gewünschtes Verhalten

- Hat der Vorschlag eine Datei `plan.txt`, liegt nach `approve` im Auftragsordner `STATE/jobs/job-NAME/plan.txt` mit byte-genau demselben Inhalt. Der Inhalt wird kopiert, nicht aus dem geparsten Plan neu erzeugt. Zeilen, die `Plan::read` nicht kennt, bleiben also erhalten.
- Das gilt für beide Arten: Dokument-Auftrag (`kind=dokument`, Gate `verify.mjs`) und Sitzungsauftrag (`kind=sitzung` mit Repo, Gate `gate.sh`).
- Die Datei entsteht im vorbereiteten Ordner, zusammen mit den übrigen Dateien und vor dem Umbenennen zum fertigen Auftrag. Der Dienst darf den Auftrag nie ohne `plan.txt` sehen. Sie wird nicht nachträglich in den fertigen Ordner geschrieben.
- Hat der Vorschlag keine `plan.txt`, wird wie bisher freigegeben, und der Auftrag hat dann keine `plan.txt`.
- Die Anpassung bleibt klein. Die Signaturen von `service::queue_source` und `session::queue` darfst du ändern, wenn du alle Aufrufer in `src/` und `tests/` mitziehst. Prüfe das mit Grep.

## Neuer Test

Lege `tests/plan_job.rs` als eigenes Testziel an. Cargo.toml braucht dafür keinen Eintrag. Wie in `tests/proposal.rs` beginnt die Datei mit `#![cfg(target_os = "macos")]` und nutzt `env!("CARGO_BIN_EXE_taik-seed")`, `tempfile` und `std::process::Command`. Der Test kommt ohne Worker und ohne Sandbox aus. Er legt die Vorschlagsordner selbst an und ruft `taik-seed approve STATE NAME` auf. Es gibt mindestens drei Testfunktionen:

1. Dokument-Auftrag mit `plan.txt`. Der Vorschlagsordner enthält `brief.md`, `verify.mjs` und `plan.txt`. Der Test prüft, dass `job-NAME/plan.txt` denselben Inhalt hat.
2. Sitzungsauftrag mit `plan.txt`. Der Vorschlagsordner enthält `brief.md`, `gate.sh` und `plan.txt` mit `kind=sitzung`, `repo=` und `repo_path=`. Der Test legt dafür ein echtes kleines Git-Repository an (`git init`, eine Datei, ein Commit mit gesetztem `user.name` und `user.email`). Er prüft denselben Inhalt in `job-NAME/plan.txt`.
3. Vorschlag ohne `plan.txt`. Die Freigabe gelingt, und `job-NAME/plan.txt` existiert nicht.

Format der `plan.txt` (Zeilen `key=value`, siehe `Plan::text` in `src/orchestrator.rs`): `kind=dokument` oder `kind=sitzung`, `repo=NAME`, `repo_path=PFAD`, `gate=standard`, `work=standard`, `reason=...`. Die erste Zeile von `brief.md` ist ein Satz ohne Markdown, zum Beispiel `Eine Seite mit Titel bauen`. Der Test darf eine zusätzliche, unbekannte Zeile in `plan.txt` aufnehmen, um das byte-genaue Kopieren zu zeigen.

## Muss bleiben

- Bestehendes Verhalten, bestehende Tests und deren Dateien (`tests/*.rs`, `src/**`, soweit nicht für die Änderung nötig) bleiben unverändert.
- Die Freigabe schreibt weiterhin `approved.txt` und liefert weiterhin den Pfad des Auftrags.
- Der Auftrag bekommt keine zusätzlichen Dateien außer `plan.txt`.

## Wie geprüft wird

Das Gate baut die Binärdatei (`cargo build --offline --locked`) und führt `approve` selbst auf eigenen Vorschlägen aus, jeweils für beide Arten und für den Fall ohne Plan. Es prüft den Inhalt von `plan.txt` im Auftrag. Außerdem führt es `cargo test --offline --locked --test plan_job` aus und verlangt, dass mindestens drei Tests laufen und bestehen.
