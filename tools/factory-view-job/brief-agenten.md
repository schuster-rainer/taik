Zeigen, welcher Agent gerade woran arbeitet, ohne dafür Tokens zu verbrauchen.

Beobachtung des Nutzers am 2026-11-25: Im Dashboard ist nicht zu sehen, welcher Agent was tut. Ein Claude-Code-Hook (`tools/agent-hook.sh`, eingetragen in `.claude/settings.json`) schreibt bei jedem Werkzeugaufruf eine kleine Statusdatei in den Zustand der Factory. Die macOS-App liefert die Sitzungen der letzten zehn Minuten mit. Die Ansicht zeigt je Sitzung Werkzeug, Ziel und Alter der Meldung.
