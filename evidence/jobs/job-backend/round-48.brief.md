Der Durchgang endet sofort, wenn der letzte Auftrag fertig ist.

Beobachtung am 2026-11-26: Runde 45 von `job-backend` wurde vom Zeit-Test `a_job_that_arrives_during_work_takes_a_free_slot` abgelehnt, Runde 46 nahm dieselbe Änderung an. Seit Runde 40 schaut ein eigener Faden zehnmal in der Sekunde nach Wünschen; nach dem letzten Auftrag wartete er bis zu 100 ms, bevor der Durchgang endete. Das hat den Spielraum dieses Tests verkleinert.

Verlangt:

- Ein Arbeitsfaden weckt den Faden für Wünsche, wenn er endet. Der Takt von 100 ms bleibt für das Nachsehen während der Arbeit.

Prüfung: `tools/backend/gate.sh`, mit unverändertem Zeit-Test.
