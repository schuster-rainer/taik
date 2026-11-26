Ein Wunsch wird vollständig festgehalten.

Beobachtung am 2026-11-26 (`requests/ticket-005.run`): Ein Wunsch von rund 1500 Zeichen kam beim Orchestrator nach 1024 Zeichen abgeschnitten an. Der Gate-Autor lehnte ihn zu Recht ab: „bricht mitten im Satz ab". Die Hilfsfunktion für einzeilige Felder kürzt auf 1024 Zeichen.

Verlangt:

- `taik-seed request` schreibt den ganzen Text des Wunsches in das Feld `observation`, ohne Kürzung. Die Grenze von 4096 Bytes für einen Wunsch bleibt.

Prüfung: `tools/backend/gate.sh`. Der bestehende Test für Wünsche prüft jetzt zusätzlich einen langen Wunsch.
