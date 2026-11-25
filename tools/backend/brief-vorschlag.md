Die Factory ihren nächsten Auftrag vorschlagen lassen und ihn erst nach Freigabe einreihen.

Beobachtung aus `../job-prozess-reflexion/summary.run`: Aufträge entstanden bisher nur durch Nutzer oder Agent, jedes Gate war von Hand geschrieben. Der Nutzer hat am 2026-11-25 entschieden, dass die Factory Auftrag und Gate vorschlagen darf und er freigibt. `propose` lässt den Worker beides aus der Fortsetzung eines Protokolls schreiben und legt es in `STATE/proposals`. `approve` reiht es ein. Der Worker, der später den Kandidaten schreibt, sieht das Gate nicht.
