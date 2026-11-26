Ein Wunsch wird abgeleitet, während Aufträge laufen.

Beobachtung am 2026-11-25 und 26: Der Dienst leitete Wünsche nur zu Beginn eines Durchgangs ab. Ein Wunsch, der während einer laufenden Sitzung eintraf, blieb bis zu deren Ende liegen. Sitzungen haben keine Zeitgrenze mehr, also kann das lange dauern.

Verlangt:

- Solange Aufträge laufen, sieht der Dienst zehnmal in der Sekunde nach neuen Wünschen und leitet sie ab. Sind alle Aufträge fertig, endet der Durchgang wie bisher ohne Verzug.

Prüfung: `tools/backend/gate.sh`, dazu der neue Test `a_request_is_derived_while_a_job_runs`. Der bestehende Zeit-Test `a_job_that_arrives_during_work_takes_a_free_slot` blieb unverändert; eine erste Fassung mit einem Takt von einer Sekunde hatte ihn gerissen.
