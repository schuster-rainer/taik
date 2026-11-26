Eine geplante Produktänderung bekommt eine eigene Prüfung für das Neue, die auf dem alten Stand scheitern muss.

Beobachtung am 2026-11-26: Vier Änderungen der Ansicht (`job-ticket-004`, `-006`, `-014`, `-017`) bestanden das Gate des Produkts, das von ihrem neuen Verhalten nichts weiß. Geprüft wurde das Neue jeweils erst nachträglich von Hand in der WebKit-Prüfung.

Verlangt:

- Nennt der Autor des Gates für einen geplanten Wunsch ein Produkt, bekommt er in einem zweiten Aufruf den Auftrag und die aktuelle Quelle und schreibt eine Prüfung des neuen Verhaltens samt überarbeitetem Auftrag, der alles nennt, was die Prüfung benutzt.
- Die Prüfung wird an das Gate des Produkts angehängt, wenn sie gültiges JavaScript ist, das Gate allein auf der aktuellen Quelle besteht und das Gate mit der Prüfung auf ihr scheitert. Sonst bleibt es beim Gate des Produkts und beim ersten Auftrag.
- Nach einer Freigabe der Ansicht wird das erweiterte Gate zum Gate des Produkts; das vorige liegt als Rollback im Auftragsordner.

Prüfung: `tools/backend/gate.sh`, dazu der neue Test `a_planned_product_change_gets_a_check_that_is_red_on_the_current_source`.
