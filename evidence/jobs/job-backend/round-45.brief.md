Die Antwort eines Gate-Autors, der ein Produkt nennt, wird auch mit Vorsatz gelesen.

Beobachtung am 2026-11-26 (`requests/ticket-028`): Der lesende Autor `pruefer` nannte das Produkt, begann seine Antwort aber mit einem Satz. Die Markierung `=== product NAME ===` wurde nur am Anfang der Antwort erkannt, der Dienst meldete „the worker did not return brief.md and verify.mjs", und ein anderer Autor sprang ein. Runde 32 hatte dasselbe für `=== brief.md ===` behoben.

Verlangt:

- Die Markierung eines Produkts gilt am Anfang jeder Zeile der Antwort.

Prüfung: `tools/backend/gate.sh`. Der Stellvertreter im Test `a_planned_product_change_gets_a_check_that_is_red_on_the_current_source` beginnt seine Antwort jetzt mit einem Satz.
