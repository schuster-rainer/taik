Entwickle in Rust einen Interpreter für die kleine Ausdruckssprache MiniCalc als einzelne Datei main.rs.

## Kandidatendokument

Das Dokument ist höchstens 96 KiB groß und ein Bundle aus genau einer Datei. Die erste Zeile lautet exakt `=== main.rs ===`; danach folgt der vollständige Inhalt von `main.rs` bis zum Ende. Es gibt keine weiteren Dateimarker und keine anderen Dateien.

`main.rs` muss ohne Cargo und ohne externe Crates mit `rustc --edition=2021 main.rs -o prog` übersetzbar sein und nur die Rust-Standardbibliothek verwenden.

## Aufruf, Ausgabe und Exit-Code

Das Programm liest den gesamten Quelltext von stdin bis EOF. Es schreibt nur nach stdout; stderr wird nicht bewertet.

- Eine `print`-Anweisung gibt den Wert als Dezimalzahl zur Basis 10 (optional mit führendem `-`) und danach genau `\n` aus.
- Ein Fehler gibt genau `ERROR <zeile>:<spalte> <CODE>\n` aus.
- Bei Erfolg ist der Exit-Code 0.
- Beim ersten Fehler ist der Exit-Code 1 und die Ausführung endet sofort.

Das ganze Programm wird zuerst vollständig lexikalisiert und geparst. Bei einem lexikalischen oder syntaktischen Fehler wird nichts ausgewertet: Es erscheinen keine `print`-Zeilen, nur die eine Fehlerzeile. Bei einem Laufzeitfehler bleiben bereits ausgegebene `print`-Zeilen stehen; danach folgt die Fehlerzeile.

## Lexik

Whitespace sind Leerzeichen, Tabulator und `\n`. Ein `\n` erhöht die Zeile; alle anderen Zeichen zählen innerhalb der Zeile eine Spalte weiter.

- `IDENT`: `[A-Za-z_][A-Za-z0-9_]*`
- `INT`: eine oder mehrere ASCII-Ziffern. Führende Nullen sind erlaubt. Der Wert muss in `0..=9223372036854775807` liegen; sonst `LITERAL_RANGE` an der ersten Ziffer.
- Schlüsselwörter: `let`, `print`. Sie sind keine `IDENT`-Token und dürfen nicht als Variablenname verwendet werden.
- Symbole: `+ - * / % = ; ( )`
- Jedes andere Zeichen ist `BAD_CHAR` an genau dieser Position.

## Grammatik

```
program := stmt* EOF
stmt    := "let" IDENT "=" expr ";"
         | "print" expr ";"
expr    := add
add     := mul (("+" | "-") mul)*
mul     := unary (("*" | "/" | "%") unary)*
unary   := "-" unary | primary
primary := INT | IDENT | "(" expr ")"
```

`*`, `/` und `%` haben dieselbe Priorität und sind linksassoziativ. `+` und `-` haben niedrigere Priorität und sind linksassoziativ. Unäres `-` bindet am stärksten und ist rechtsassoziativ. Ein leerer Quelltext ist gültig: keine Ausgabe, Exit 0. Zeilenumbrüche sind nur Whitespace; Anweisungen werden durch `;` beendet.

## Auswertung

- Werte sind vorzeichenbehaftete 64-Bit-Ganzzahlen (`i64`).
- Variablen sind global, unveränderlich und haben keine Blöcke. Es gibt nur `let`.
- Bei `let NAME = expr;` wird zuerst `expr` ausgewertet. Erst wenn das gelingt, wird geprüft, ob `NAME` schon gebunden ist (`DUPLICATE` am Namens-Token), und dann gebunden.
- Ein nicht gebundener Name ist `UNDEFINED` am Identifier-Token.
- `+`, `-`, `*`, `/`, `%` und unäres `-` rechnen mit Überlaufprüfung. Das mathematische Ergebnis muss in `i64` passen; sonst `OVERFLOW` am Operator-Token.
- `/` schneidet zur Null hin ab. `%` hat das Vorzeichen des Dividenden (Rust-Semantik).
- Division oder Rest durch 0 ist `DIV_ZERO` am Operator-Token.
- `i64::MIN / -1` und `i64::MIN % -1` sind `OVERFLOW` am Operator-Token.
- Unäres `-` auf `i64::MIN` ist `OVERFLOW` am `-`-Token.

## Fehlerpositionen

Zeilen und Spalten sind 1-basiert. Die Spalte zählt Zeichen seit dem letzten `\n`, beginnend bei 1. Das EOF-Token steht direkt hinter dem letzten Zeichen; nach einem abschließenden `\n` ist es in der nächsten Zeile bei Spalte 1. Leerer Quelltext: 1:1.

- `SYNTAX`: an dem Token, das die Grammatik nicht erwartet (auch EOF). Fehlendes `;`, `)`, `=` usw. meldet das aktuelle Token.
- `BAD_CHAR`: an dem ungültigen Zeichen.
- `LITERAL_RANGE`: an der ersten Ziffer des Literals.
- `UNDEFINED`/`DUPLICATE`: am ersten Zeichen des Identifier-Tokens.
- `DIV_ZERO`/`OVERFLOW`: am Operator-Token (`+ - * / %` oder unäres `-`).

Es zählt immer der erste Fehler in Lesereihenfolge bzw. bei Laufzeit in Auswertungsreihenfolge.
