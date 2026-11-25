Eine abgelehnte Runde im Ticket kurz erklären, die Zeitgrenze von 900 s streichen und die Größengrenze anheben.

Beobachtungen des Nutzers am 2026-11-25: „ich möchte im ticket dann eine analyse sehen, wenn etwas abgelehnt wird. alles muss logisch und verständlich sein. aber keinen roman." Und: „ich hab doch schon mal gesagt die 900 sekunden sind banane", „diese harten constraints sind nicht von mir. die hat ein agent aufgestellt."

Verlangt:

- Nach einer abgelehnten Runde schreibt der Dienst `round-NN.analysis.md`: vier Zeilen „Verlangt", „Gescheitert an", „Ursache", „Nächster Schritt". Der Worker des Dienstes bekommt dafür Auftrag, Protokoll und Gate. Eine Antwort ohne diese vier Zeilen wird verworfen. Die Kosten stehen in `round-NN.analysis.run`.
- Hat der Worker gar nichts geliefert, steht die Analyse fest und kostet keinen Aufruf.
- `taik-seed analyse JOB` schreibt die Analyse der letzten Runde nach.
- Ein Aufruf hat keine eigene Zeitgrenze mehr. Die Lease von 60 s ohne Ausgabe bleibt. `TAIK_WORKER_CEILING_MS` setzt für Tests weiter eine Grenze.
- Ein Dokument darf 512 KiB groß sein statt 96. Messung: Die Ansicht ist mit 95 KiB an die alte Grenze gestoßen, ihr Gate läuft in 60 ms.

Prüfung: `tools/backend/gate.sh`, dazu der neue Test `a_rejected_round_gets_a_short_analysis_once`. Die bestehenden Tests blieben unverändert.
