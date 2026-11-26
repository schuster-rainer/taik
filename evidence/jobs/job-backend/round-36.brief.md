Der Zweig eines Auftrags rückt bei einer Folgerunde nach.

Von der Factory hervorgebracht. Beobachtung in `job-ticket-013`: Nach der zweiten angenommenen Runde wies die Übernahme ab, weil `taik/ticket-013` schon auf den Commit der ersten Runde zeigte (`adopt-failed.txt`). Wunsch `requests/ticket-016.run`. Die erste Fassung (`ticket-015`) lehnte der lesende Gate-Autor ab, weil sie einem bestehenden Test widersprach und zugleich alle bestehenden Tests unverändert lassen wollte; die zweite benennt diesen Test. Gate von `pruefer` (Sonnet 5.5, mit Clippy), Arbeit von Opus 5.5, `job-ticket-016` in Runde 1 angenommen.

Der Kandidat dieser Runde ist der Merge jenes Commits; von `claude-fable` stammt nur `cargo fmt`.

Verhalten, in `job-ticket-016` geprüft: Zeigt `taik/NAME` auf einen Vorfahren des angenommenen Commits, rückt der Zweig vor. Sonst bleibt die Ablehnung. Ist der Zweig im Ursprungs-Repository ausgecheckt, wird nichts verändert.

Prüfung: `tools/backend/gate.sh`, mit `tests/adopt_forward.rs` des Agenten.
