Der Beleg einer Analyse zählt nicht als Runde.

Beobachtung am 2026-11-26 in `job-ticket-003`: Nach der Antwort des Nutzers lief keine Runde, der Dienst meldete „No such file or directory". `rounds_done` zählte `round-01.analysis.run` als zweite Runde und suchte danach `round-02.run`. Der Fehler trifft jeden Auftrag, der nach einer analysierten Ablehnung weiterlaufen soll.

Verlangt:

- Als Runde zählt nur `round-NN.run`.

Prüfung: `tools/backend/gate.sh`, der Modultest in `src/talk.rs` zeigt den Fall.
