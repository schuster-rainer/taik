Die Analyse einer abgelehnten Runde im Ticket zeigen.

Beobachtung des Nutzers am 2026-11-25: „ich möchte im ticket dann eine analyse sehen, wenn etwas abgelehnt wird. alles muss logisch und verständlich sein. aber keinen roman." Der Kern schreibt sie seit Runde 20 von `job-backend` als `round-NN.analysis.md`.

Verlangt:

- Der Stand oben im Ticket zeigt nach einer Ablehnung die Zeilen der Analyse mit ihren Bezeichnungen (`analysisLines`), neben dem Befund des Gates.
- Frühere abgelehnte Runden zeigen ihre Analyse an der Runde.
- Kein Text wird abgeschnitten.

Prüfung: `tools/factory-view-analyse/verify.mjs`. Es übernimmt das Gate von `job-ansicht-stand`; zwei Vergleiche dort erwarten jetzt zusätzlich das leere Feld `analysis`.
