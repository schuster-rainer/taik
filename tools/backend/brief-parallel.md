Eingereihte Software-Aufträge nebeneinander abarbeiten.

Beobachtung: Der Dienst arbeitete strikt nacheinander. Sechs Aufträge des letzten Versuchs brauchten zusammen rund sieben Minuten, obwohl jede Runde fast nur auf den Worker wartet. Bis zu drei Software-Aufträge laufen jetzt gleichzeitig, einstellbar über `TAIK_PARALLEL` bis höchstens acht. Jeder Auftrag behält seine eigene Sperre, seinen Ordner und seine Protokolle.
