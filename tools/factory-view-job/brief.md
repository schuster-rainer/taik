Die Factory-Ansicht neumorph, minimalistisch und professionell neu gestalten und `.run`-Prüfprotokolle farblich auszeichnen.

Ausgangsstand ist `tools/factory-view.html`, wie er in der installierten App läuft. Er bleibt als `rollback.factory-view.html` im Auftragsordner.

Verlangt:

- Matte, gleichtonige Flächen. Erhabene und eingelassene Elemente entstehen nur aus einem dunklen und einem hellen Schatten desselben Farbtons. Kein Glow, kein Blur, kein farbiger Schatten.
- Hell und Dunkel mit ausreichendem Textkontrast.
- Der Auftragsverlauf bleibt links sichtbar, die letzte Runde ist geöffnet.
- Rechts bleiben beide Flächen stehen. Unten erscheint während einer Runde der Entwurf als gerenderte Vorschau ohne Skripte und als mitlaufender Quelltext, daneben der Arbeitsfortschritt. Der Arbeitsfortschritt bleibt innerhalb seiner Fläche.
- Das Prüfprotokoll zeichnet Schlüssel, Entscheidung, Prüfergebnis, Prüfsummen, Dateinamen und Mengen aus. Der Text des Protokolls bleibt unverändert.
- Ereignisse eines eingehängten Workers heißen nicht mehr „Codex-Worker".
- Die Ansicht bleibt ein Fragment ohne Netzwerkzugriff und behält alle Anknüpfungspunkte der macOS-App und der WebKit-Prüfung.

Der Entwurf ist diesmal der gerenderte Kandidat selbst. Seine Aufnahme liegt als Nachweis neben der Runde.
