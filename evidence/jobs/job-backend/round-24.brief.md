Das Gate einer Sitzung kann einen Checkout bauen und testen.

Beobachtung am 2026-11-26, vor der Freigabe von `ticket-003`: Das von Savant geschriebene Gate ruft `cargo build` und `cargo test`. In der Sandbox des Gates scheiterte der Bau, weil ein Build-Skript auf das Null-Gerät schreibt, und ein Test, weil die Tests dieses Repos ihre Arbeitsordner unter `/tmp/taik-…` anlegen. Gemessen in einem Klon: ohne die Freigaben „Operation not permitted", mit dem Null-Gerät baut `cargo build --release --offline --locked` in 8 s.

Verlangt:

- Das Gate einer Sitzung darf zusätzlich auf das Null-Gerät und auf Pfade unter `/private/tmp/taik-` schreiben. Netz, Home-Verzeichnis, Haupt-Checkout und Auftragsordner bleiben zu.

Prüfung: `tools/backend/gate.sh`. Das Gate im Test `tests/session.rs` schreibt jetzt auf das Null-Gerät, legt einen Ordner unter `/tmp/taik-` an und prüft, dass es nicht ins Home schreiben kann.
