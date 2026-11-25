Den Freigabeschritt vom Dienst ausführen lassen, wenn das Gate ihn nennt.

Beobachtung aus `../job-ansicht-werkstueck/observation.run`: Nach einer angenommenen Ansichtsrunde prüfte und installierte weiterhin ein Agent. Nennt ein Gate die Zeile `// TAIK: release tools/NAME.sh`, ruft der Dienst dieses Skript nach der Annahme einmal mit dem Auftragsordner auf, höchstens 300 s, mit Ausgabe nach `release.log`. Das Gate steht vor dem Kandidaten fest, und die Freigabe des Auftrags deckt den Schritt. Skripte außerhalb von `tools/` werden abgelehnt.
