Die Zahl der parallelen Plätze aus der Ansicht einstellbar machen.

Beobachtung: Der Nutzer fragte am 2026-11-25, ob er im Frontend einstellen kann, wie viele Aufträge parallel laufen. Der Dienst liest `STATE/parallel` bei jedem Durchlauf, vor der Umgebungsvariable `TAIK_PARALLEL`. Ohne beides bleiben es drei Plätze, höchstens acht.
