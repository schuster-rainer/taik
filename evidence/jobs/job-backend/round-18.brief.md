Der Reflexion die offenen Vorhaben der Factory geben und sie Änderungen an einem bestehenden Produkt vorschlagen lassen.

Beobachtung am 2026-11-25: Alle sechs Reflexionen des Dienstes der letzten 24 Stunden endeten mit „Stopp", jedes Mal mit derselben Begründung. Sie lasen nur das letzte Rundenprotokoll, und dessen Standardsatz lautet „No further work was authorized by this round". Die Schleife hielt sich an ihrem eigenen Textbaustein auf. Was tatsächlich offen ist, steht als `continuation=` in den Auswertungen (`summary*.run`, `observation*.run`), und die bekam die Reflexion nie zu sehen. Außerdem konnte ein Vorschlag nur ein neues, leeres Artefakt beginnen, also weder die Ansicht noch ein anderes vorhandenes Ergebnis ändern.

Umfang:

- Die Reflexion bekommt die offenen Vorhaben: die Fortsetzung jeder Auswertung, die noch keine Reflexion aufgegriffen hat, neueste zuerst, höchstens 25. Greift sie eines auf, steht dessen Herkunft im Reflexionsprotokoll, und es gilt als vergeben.
- Der Auftragssatz sagt ausdrücklich, dass der Standardsatz eines Rundenprotokolls kein Grund zum Anhalten ist. „Stopp" bleibt erlaubt, wenn kein Vorhaben als ein Auftrag machbar ist.
- Ein Produkt ist eine Datei `STATE/products/NAME.product` mit Beschreibung, angenommenem Stand und fixiertem Gate. Ein Vorschlag darf ein Produkt ändern: Er liefert dann nur den Auftrag. Gate und Ausgangsstand kommen vom Produkt, der Worker schreibt kein Gate.
- Die Freigabe bleibt beim Nutzer. Ein freigegebener Produktauftrag startet vom angenommenen Stand.
