# MiniCalc 2

Entwickle in Rust einen Interpreter für die kleine Ausdruckssprache MiniCalc 2 als einzelne Datei main.rs.

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
- Schlüsselwörter: `let`, `print`, `set`, `if`, `else`, `while`. Sie sind keine `IDENT`-Token und dürfen nicht als Variablenname verwendet werden.
- Symbole: `+ - * / % = == != < <= > >= ; ( ) { }`
- Für mehrzeichenige Symbole wird die längste passende Folge gelesen (`<=` vor `<`, `>=` vor `>`, `==` vor `=`, `!=` vor einem einzelnen `!`).
- Jedes andere Zeichen ist `BAD_CHAR` an genau dieser Position.

## Grammatik

```
program := stmt* EOF
stmt    := "let" IDENT "=" expr ";"
         | "set" IDENT "=" expr ";"
         | "print" expr ";"
         | "if" expr block ("else" block)?
         | "while" expr block
block   := "{" stmt* "}"
expr    := cmp
cmp     := add (("==" | "!=" | "<" | "<=" | ">" | ">=") add)?
add     := mul (("+" | "-") mul)*
mul     := unary (("*" | "/" | "%") unary)*
unary   := "-" unary | primary
primary := INT | IDENT | "(" expr ")"
```

Ein `block` ist keine Anweisung und kann nicht allein als `stmt` auftreten; er folgt nur auf `if` bzw. `while` und optional auf `else`.

`*`, `/` und `%` haben dieselbe Priorität und sind linksassoziativ. `+` und `-` haben niedrigere Priorität und sind linksassoziativ. Unäres `-` bindet am stärksten und ist rechtsassoziativ. Vergleichsoperatoren haben niedrigere Priorität als `+` und `-`, dieselbe Priorität untereinander und sind nicht assoziativ: In einem `expr` ist höchstens ein Vergleich erlaubt, außer er wird geklammert. Ein leerer Quelltext ist gültig: keine Ausgabe, Exit 0. Zeilenumbrüche sind nur Whitespace; `let`, `set` und `print` werden durch `;` beendet.

## Auswertung

- Werte sind vorzeichenbehaftete 64-Bit-Ganzzahlen (`i64`).
- Variablen sind global und haben keine Blöcke; `if` und `while` öffnen keinen Scope. `let` bindet einen Namen, `set` weist einem bereits gebundenen Namen einen neuen Wert zu.
- Bei `let NAME = expr;` wird zuerst `expr` ausgewertet. Erst wenn das gelingt, wird geprüft, ob `NAME` schon gebunden ist (`DUPLICATE` am Namens-Token), und dann gebunden.
- Bei `set NAME = expr;` wird zuerst `expr` ausgewertet. Erst wenn das gelingt, wird geprüft, ob `NAME` gebunden ist (`UNDEFINED` am Namens-Token), und dann der Wert zugewiesen.
- Ein nicht gebundener Name ist `UNDEFINED` am Identifier-Token.
- Bei `if expr block1 else block2` wird `expr` ausgewertet. Ist der Wert ungleich 0, wird `block1` ausgeführt, sonst `block2` (falls vorhanden). Ohne `else` wird bei 0 nichts ausgeführt.
- Bei `while expr block` wird `expr` wiederholt ausgewertet. Solange der Wert ungleich 0 ist, wird der Block ausgeführt und danach die Bedingung erneut geprüft.
- Alle binären Operatoren, auch Vergleiche, werten zuerst den linken und dann den rechten Operanden aus.
- Die Vergleichsoperatoren `== != < <= > >=` liefern 1 bei wahr und 0 bei falsch. `==` und `!=` vergleichen auf Gleichheit, `< <= > >=` vergleichen numerisch.
- `+`, `-`, `*`, `/`, `%` und unäres `-` rechnen mit Überlaufprüfung. Das mathematische Ergebnis muss in `i64` passen; sonst `OVERFLOW` am Operator-Token.
- `/` schneidet zur Null hin ab. `%` hat das Vorzeichen des Dividenden (Rust-Semantik).
- Division oder Rest durch 0 ist `DIV_ZERO` am Operator-Token.
- `i64::MIN / -1` und `i64::MIN % -1` sind `OVERFLOW` am Operator-Token.
- Unäres `-` auf `i64::MIN` ist `OVERFLOW` am `-`-Token.
- In Bedingungen von `if` und `while` gilt jeder Wert ungleich 0 als wahr.

## Schrittlimit

Vor jeder Ausführung einer Anweisung wird geprüft, ob bereits 100000 Anweisungen ausgeführt wurden. Ist das der Fall, endet das Programm mit `ERROR <zeile>:<spalte> STEP_LIMIT` an der ersten Token-Position der Anweisung, die als nächste ausgeführt würde; diese Anweisung wird nicht mehr ausgeführt. Andernfalls wird der Zähler um 1 erhöht und die Anweisung ausgeführt. Ein Programm, das nach genau 100000 Schritten endet, ist erfolgreich.

Als Anweisung zählen alle `stmt`-Knoten: `let`, `print`, `set`, `if` und `while`. Ein Block `{ ... }` ist keine Anweisung. `if` zählt einmal. `while` zählt bei jeder Auswertung seiner Bedingung einmal, also vor jeder Iteration und auch für die abschließende falsche Bedingung. Die Anweisungen im ausgewählten Block bzw. im Schleifenrumpf zählen einzeln.

## Fehlerpositionen

Zeilen und Spalten sind 1-basiert. Die Spalte zählt Zeichen seit dem letzten `\n`, beginnend bei 1. Das EOF-Token steht direkt hinter dem letzten Zeichen; nach einem abschließenden `\n` ist es in der nächsten Zeile bei Spalte 1. Leerer Quelltext: 1:1.

- `SYNTAX`: an dem Token, das die Grammatik nicht erwartet (auch EOF). Fehlendes `;`, `)`, `=`, `{`, `}` usw. meldet das aktuelle Token.
- `BAD_CHAR`: an dem ungültigen Zeichen.
- `LITERAL_RANGE`: an der ersten Ziffer des Literals.
- `UNDEFINED`/`DUPLICATE`: am ersten Zeichen des Identifier-Tokens.
- `DIV_ZERO`/`OVERFLOW`: am Operator-Token (`+ - * / %` oder unäres `-`).
- `STEP_LIMIT`: an der ersten Token-Position der Anweisung, die als nächste ausgeführt würde.

Es zählt immer der erste Fehler in Lesereihenfolge bzw. bei Laufzeit in Auswertungsreihenfolge.
