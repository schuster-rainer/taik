Agenten in der Ansicht konfigurieren: Übersicht der Harnesse und Abos, Agenten anlegen und ändern, Agent je Ticket wählen.

Beobachtung: Der Nutzer will am 2026-11-25 in der Factory sehen, welche Abos vorhanden sind, Agenten mit Modell, Effort und Persona einrichten, den Agenten eines Tickets umstellen und mit `@name` einen dazuholen. Der Kern kann benannte Agenten seit den Runden 13 bis 16 von `job-backend`. In der Ansicht fehlte jede Bedienung dafür.

Verlangt:

- Ein Feld „Crew" unter den Tickets. Es listet die Agenten mit Harness, Modell und Effort. Ein Klick öffnet das Formular: Name, Harness, Modell mit den Vorschlägen des Harness, Effort aus dem Angebot des Harness, Persona. Speichern, Löschen, Abbrechen.
- Ein aufklappbarer Teil „Harnesse und Abos": je Harness der Anmeldestatus, die Anmeldeart, Abo oder API, die Zahl der Modelle und das Ablaufdatum der Anmeldung. Kein Zugangsdatum wird gezeigt oder abgefragt. Nicht eingebundene Harnesse stehen sichtbar dabei.
- Ein Entwurf im Formular und der Fokus überleben das Auffrischen der Ansicht.
- Im geöffneten Ticket eines Auftrags eine Auswahl „Agent der nächsten Runde" mit dem Standard des Dienstes und allen Agenten. Der Agent steht auch in der Zeile der Karte.
- Ein `@name` in einer Ticket-Antwort übergibt die nächste Runde an diesen Agenten, wenn es ihn gibt. Eine Mail-Adresse ist keine Erwähnung.
- Die Entscheidungen treffen reine Funktionen im Block `CREW`: `crewOf`, `mentioned`, `agentLine`, `validAgent`.
- Das bisherige Feld „Agenten" mit den gemeldeten Claude-Code-Sitzungen heißt „Sitzungen".

Die macOS-App liefert die Übersicht aus `STATE/crew/overview.json` und den Agenten je Auftrag mit. Sie schreibt Agentendateien nur nach `STATE/crew` und die Datei `agent` nur in einen vorhandenen Auftragsordner, jeden Wert gegen eine feste Form geprüft.
