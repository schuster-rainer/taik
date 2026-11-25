Der Auftrag ergänzt im Repo taik den Befehl taik-seed close STATE job-NAME, der einen Auftrag mit einer Begründung dauerhaft schließt.

Verhalten, das danach gelten muss:
- `taik-seed close STATE job-NAME` liest die Begründung aus der Standardeingabe. Gültig sind 1 bis 2000 Bytes.
- Eine leere Begründung oder eine Begründung mit mehr als 2000 Bytes wird mit Exit-Status 2 abgewiesen; es entsteht keine Datei.
- Für dieses Gate ist ein Auftragsordner das Verzeichnis `STATE/jobs/job-NAME`. Existiert es nicht, endet der Befehl mit Status 2. Der Befehl darf für den Gate nicht mehr als die Existenz dieses Ordners verlangen.
- Bei gültiger Eingabe entsteht `STATE/jobs/job-NAME/closed.md` mit genau den gelesenen Bytes, ohne zusätzliche Ausgabe. Exit-Status ist 0.
- `closed.md` ist einmalig und nicht überschreibbar: Ein zweites Schließen desselben Auftrags ändert die Datei nicht und endet mit Status 2.
- Ein Auftrag mit `closed.md` bekommt vom Dienst keine weitere Runde mehr, auch nicht nach einer Antwort.
- Der Test liegt als `tests/close.rs` vor und besteht mit `cargo test --offline --locked --test close`.
- In `README.md` steht unter den Befehlen genau die Zeile `taik-seed close STATE job-NAME`.
- Die Ansicht (`tools/factory-view*`) wird nicht geändert. Bestehende Befehle und Ausgaben bleiben unverändert.

Namen und Werte, die das Gate prüft:
Binary `target/release/taik-seed`; Unterbefehl `close`; Argumente `STATE` und `job-NAME`; Auftragsordner `STATE/jobs/job-NAME`; Datei `closed.md`; Exit 0 bei Erfolg; Exit 2 bei unbekanntem Auftrag, leerer Begründung, Begründung über 2000 Bytes und zweitem Schließen; Begründungslänge 1 bis 2000 Bytes; Testziel `close`; README-Zeile `taik-seed close STATE job-NAME`.
