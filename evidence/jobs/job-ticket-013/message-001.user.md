round=1

Dein Stand hat das Gate des Auftrags bestanden, aber die Übernahme in den Kern ist gescheitert: `cargo clippy --all-targets -- -D warnings` meldet „this function has too many arguments (8/7)" für `queue_source` in src/service.rs und für `queue` in src/session.rs. Bitte beide Funktionen so ändern, dass sie höchstens sieben Parameter haben (zum Beispiel die zusammengehörigen Angaben in einer kleinen Struktur bündeln), ohne ein allow-Attribut. Alle Aufrufer und Tests anpassen. Das Verhalten bleibt gleich.
