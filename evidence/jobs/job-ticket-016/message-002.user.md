round=2

Noch ein Test mit derselben Annahme lässt die Übernahme scheitern: `a_branch_on_another_commit_is_a_failure` in tests/settle.rs legt den Zweig auf einen Vorfahren und erwartet Status 2, bekommt jetzt aber 0. Bitte passe ihn wie die beiden anderen an (Zweig auf einem Commit, der kein Vorfahr ist). Durchsuche danach alle Dateien in tests/ und die Modultests in src/ nach weiteren Stellen, die einen schon vorhandenen Zweig taik/… auf einen Vorfahren legen und eine Ablehnung erwarten, und passe jede an. Nenne in deiner Zusammenfassung alle Stellen, die du geprüft hast.
