Die Antwort eines Gate-Autors wird auch mit Vorsatz oder schließendem Codezaun gelesen.

Beobachtung am 2026-11-26 (`requests/ticket-010`): Der lesende Autor `pruefer` lieferte Auftrag und Gate, begann seine Antwort aber mit einem Satz. Der Dienst meldete „the worker did not return brief.md and gate.sh", und ein Autor ohne Lesezugriff sprang ein. Außerdem nannte das Protokoll bei jedem Ersatz den ersten Autor statt des jeweils versuchten.

Verlangt:

- Was vor der ersten Markierung `=== brief.md ===` steht, wird verworfen, ebenso ein schließender Codezaun am Ende des Gates. Ein leeres Gate bleibt ungültig.
- Das Protokoll nennt bei jedem gescheiterten Versuch den Agenten, der ihn gemacht hat.

Prüfung: `tools/backend/gate.sh`, dazu der neue Modultest in `src/proposal.rs`.
