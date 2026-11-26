Ändere die Ansicht so, dass ein wartendes Ticket geschlossen werden kann und geschlossene Aufträge als entschieden gelten.
Die App liefert für jeden Auftrag das Feld job.closed: den Text der Begründung, wenn der Auftrag geschlossen wurde, sonst ist das Feld nicht vorhanden. Sie nimmt die Nachricht closeTicket mit { job: "job-…", text: "Begründung" } an.
Im Ticket eines Auftrags, der auf mich wartet (letzte Runde abgelehnt oder der Agent hat gefragt), steht neben „Senden" ein Knopf „Schließen". Er nimmt den Text aus dem Antwortfeld als Begründung. Ist das Feld leer, bekommt es den Fokus und es passiert nichts. Sonst sendet der Knopf window.webkit.messageHandlers.closeTicket.postMessage({ job, text }) mit der Auftrags-ID und diesem Text.
Ein geschlossener Auftrag (job.closed vorhanden) steht in der Ticketliste unter „Entschieden" mit dem Kennzeichen „Geschlossen" und zählt nicht mehr zu „warten auf dich". Das gilt auch dann, wenn seine letzte Runde abgelehnt war oder der Agent gefragt hatte. boardTickets muss das leisten.
Im Stand des Tickets (ticketState) gilt für einen geschlossenen Auftrag: waits ist leer, next lautet „Geschlossen: " gefolgt von der Begründung. Antwortfeld und Knöpfe entfallen im geschlossenen Ticket.
Ein Eintrag des Bestands mit decision "closed" trägt ebenfalls „Geschlossen".
Alles andere bleibt, wie es ist.
