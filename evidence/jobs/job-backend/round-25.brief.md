Das Gate einer Sitzung darf mit mktemp einen Arbeitsordner anlegen.

Beobachtung am 2026-11-26 in `job-ticket-003`, Runde 1: Der Agent lieferte vier Dateien, das Gate endete mit „FAIL: temporäres Verzeichnis konnte nicht angelegt werden". `mktemp -d` schreibt auf macOS in den temporären Ordner des Nutzers und beachtet `TMPDIR` nicht; die Sandbox des Gates ließ das nicht zu. Der Agent fragte daraufhin in Runde 2 selbst nach einem beschreibbaren temporären Verzeichnis, die erste echte Rückfrage eines Agenten.

Verlangt:

- Das Gate einer Sitzung darf Einträge `tmp.…` im temporären Ordner des Nutzers anlegen. Sonst ändert sich nichts.

Prüfung: `tools/backend/gate.sh`. Das Gate im Test `tests/session.rs` ruft jetzt `mktemp -d`.
