Die Kosten unbeaufsichtigter Claude-Runden auf einen Tagesbetrag begrenzen und die Auslastung des Kontos festhalten.

Beobachtung: Die CLI meldete am 2026-11-25 eine Wochenauslastung von 96 Prozent. Nichts in der Factory begrenzte, was eingereihte Aufträge verbrauchen (Spec Abschnitt 8, `../job-prozess-reflexion/summary.run`). Vor jedem Aufruf summiert der Worker jetzt die in den letzten 24 Stunden protokollierten Kosten und lehnt ab, wenn `TAIK_DAILY_BUDGET_USD` erreicht ist (Vorgabe 3 USD zum Listenpreis).
