# PPL 4.00: Sprachreview nach den Korrekturen

Stand: 2026-09-06, PPL-Basis Commit `1b864888`.

## Korrekturstand 2026-09-06

**N1–N4 sind behoben.** Die folgenden Review-Befunde bleiben als Dokumentation
des zuvor geprüften Zustands erhalten; die Designentscheidungen V1–V5 wurden
nicht umgesetzt.

- **N1:** Gewöhnliche Arrayparameter behalten Elementtyp, Rang und Bounds im
  Parameterheader. Argumentprüfung und VM-Aufrufe übertragen ganze Arraywerte;
  Wertparameter bleiben unabhängig, `VAR` schreibt Wert und Bounds zurück.
  Sprache 400 verlangt dafür Runtime 400. Historische Parameterheader und
  skalares Altverhalten bleiben unverändert.
- **N2:** Bracket-indizierte Record-Zuweisungsziele werden auf denselben
  Speicherpfad wie `()` abgebildet, auch verschachtelt und bei Compound-Updates.
  String-Zeichen und schreibgeschützte API-Snapshots werden nicht beschreibbar.
- **N3/N4:** `SORT` verwendet ab Runtime 400 die wirkliche Vektorlänge und
  erzeugt das Indexarray direkt aus den Ergebnisindizes, einschließlich des
  leeren Ergebnisses. Kein zusätzliches Defaultelement; pre-400 bleibt getrennt.

**36 neue permanente Regressionstests:**
[Arrayparameter](../crates/icy_board_engine/src/vm/tests/array_parameters.rs) (20),
[Record-Bracket-Ziele](../crates/icy_board_engine/src/vm/tests/record_array_brackets.rs) (9),
[SORT](../crates/icy_board_engine/src/vm/tests/array_sort.rs) (7).
Abgesichert sind unter anderem Rang 1–3, Rekursion, direkte und weitergereichte
Callbacks, Record-/String-/Enum-Arrays, feste Record-Felder als Wertargumente,
Snapshot-Werte, Auswertungsreihenfolge, Aliasing, Bounds-Änderungen und
Legacy-Header/Runtime-Verhalten.

Der vollständige Testlauf für `icy_board_engine`, `pplc`, `ppld` und `ppl-lsp`
bestand einschließlich Integrations- und Dokumentationstests; Engine-Library:
**1543 bestanden, 5 ignoriert**. Scoped rustfmt, Whitespace-Prüfung und
Editordiagnosen der geänderten Rust-Dateien sind sauber.

## Ergebnis

**Kein grundlegender Fehlentwurf, aber noch kein konsistenter Array-Wertvertrag.**
Records, Module und geprüfte Routineparameter passen zur Sprache. Die bereits
behobenen L1–L7 werden hier nicht erneut als offene Fehler geführt. Neue
Kontrollfälle zeigen jedoch Lücken beim Übergang zwischen Arraywerten,
Routineparametern, Record-Zuweisungszielen und älteren Arrayoperationen.

Vor einem Sprachfreeze sollten diese Übergänge geschlossen und die wenigen
offenen Designverträge entschieden werden. Weitere Syntax ist derzeit weniger
wichtig als die zuverlässige Kombination der vorhandenen Features.

## Prüfumfang und Evidenz

- 30 Quellvarianten in fünf temporären Beobachtungstests: Arrayparameter,
  Record-Indizierung, leere Arrays und `SORT`, Deklarationen/Enums/`VAR`,
  Formatter-Kontrolle. Sprache und PPE-Runtime jeweils 400.
- Erfolgreich kompilierte Programme liefen durch Compiler, PPE-Serialisierung,
  erneutes Laden und VM. Der Panic wurde im Test aufgefangen, nicht als
  gewöhnlicher PPL-Fehler behandelt. „Test bestanden“ bei diesen Proben bedeutet
  nur, dass die Beobachtung abgeschlossen wurde, nicht dass die Sprache korrekt
  reagiert hat.
- Nach Entfernung der temporären Tests: **123 bestehende Regressionstests
  bestanden** — Arrays 66, Routineparameter 16, verschachtelte Records 31,
  Vorwärtsaufrufe 10. Kein erneuter vollständiger Workspace-Testlauf.
- Keine Implementierung geändert. Unabhängige Upload-Arbeiten ausgeschlossen.
- Keine neue PCBoard-Oracle-Prüfung; insbesondere historische `VAR`- und
  `SORT`-Regeln nicht allein anhand intuitiver Referenzsemantik umdeuten.

## 1. Neu verifizierte Implementierungsprobleme

### N1 · Hoch: Arrayparameter werden wie Skalare behandelt

```PPL
;$LANGVERSION 400
INTEGER data[] = { 7, 8 }
Show(data)

PROCEDURE Show(INTEGER values[])
    PRINT values[0]
ENDPROC
```

Diagnosen: `Not enough arguments passed (data:0:1)` und
`Too many arguments passed (values:1:0)`.

Dasselbe Problem besteht mit `VAR INTEGER values[]`, einem Parameter mit
Anfangsobergrenze sowie Rang 2 und 3. Selbst ein unbenutzter Arrayparameter
akzeptiert den Arrayaufruf nicht. Ein arrayliefernder Funktionsaufruf als
Argument wird ebenfalls als unzulässiger skalarer Gebrauch abgewiesen.

Umgekehrt kompiliert `Show(7)` mit `PROCEDURE Show(INTEGER values[])` und
`PRINT values` im Rumpf und druckt `7`. Das ist also nicht nur eine fehlende
Indexoperation: Der deklarierte Parametervertrag geht verloren.

Ursachen:

- [Parameterregistrierung](../crates/icy_board_engine/src/semantic/mod.rs#L740-L758)
  setzt für gewöhnliche Parameter immer `dim: 0`.
- [Argumentprüfung](../crates/icy_board_engine/src/semantic/mod.rs#L1144-L1155)
  verbietet pauschal das ganze Array als Wert.
- [VM-Aufrufvorbereitung](../crates/icy_board_engine/src/vm/call_stack.rs#L10-L39)
  verwendet die gewöhnliche skalare Ausdrucksauswertung. Eine Reparatur nur
  der Compilerdiagnose reicht daher nicht als abgesicherte Lösung.

**Empfehlung:** Elementtyp und Rang durchgängig in Signaturen tragen, auch bei
Callbacks und Forward-Deklarationen. Für Wertparameter Array-Wertkopien mit
Copy-on-write vorsehen; für `VAR` explizit festlegen, ob Bounds ersetzt werden
dürfen. Positive und negative Call-Site-Tests für Rang 1–3 ergänzen.

### N2 · Hoch: Record-Arrayfelder sind mit `[]` nicht konsistent beschreibbar

```PPL
;$LANGVERSION 400
TYPE Item
    STRING names[1]
ENDTYPE
Item item
item.names[0] = "new"
PRINT item.names[0]
```

Die Zuweisung scheitert mit `Whole arrays cannot be used as scalar values;
index an element first` und `Member not found`.

Kontrollmatrix:

| Operation | Ergebnis |
| :--- | :--- |
| Lesen `item.names[0]` nach Initialisierung mit `()` | funktioniert |
| Schreiben `item.names[0] = "new"` | Compilerfehler |
| Schreiben `item.names(0) = "new"` | druckt anschließend `new` |
| `item.names[0] += "!"` | Compilerfehler |
| `item.names(0) += "!"` | funktioniert |
| INTEGER-Arrayfeld, Schreiben mit `[]` / `()` | ebenfalls Fehler / Erfolg |
| Verschachtelter Record `item.rows[0].value = 7` | `Can't assign value to.` |
| Derselbe Pfad mit `item.rows(0).value = 7` | funktioniert |

Die [Parser-Abbildung von `[]`](../crates/icy_board_engine/src/parser/expression.rs#L275-L302)
auf einen synthetischen Getter und die
[Umwandlung eines indizierten Ziels zum Setter](../crates/icy_board_engine/src/parser/statements.rs#L1182-L1225)
behandeln nicht alle Record-Speicherpfade wie ein Array-Zuweisungsziel.

**Empfehlung:** Lesen und Schreiben aus einem gemeinsamen typisierten
Index-/Member-Ziel ableiten. Beschreibbare Record-Felder, schreibgeschützte
API-Snapshots und String-Zeichenindizes müssen dabei unterscheidbar bleiben.
Die kanonische `[]`-Syntax muss dieselben Record-Ziele unterstützen wie die
noch akzeptierte `()`-Syntax.

### N3 · Hoch: `SORT` auf einem leeren dynamischen Array löst Panic aus

```PPL
;$LANGVERSION 400
INTEGER data[], indices[]
SORT data, indices
PRINT indices.Len()
```

Kompiliert, löst im verwendeten Testprofil aber einen Rust-Panic aus:
`attempt to subtract with overflow`.

[`get_vector_size()`](../crates/icy_board_engine/src/executable/variable_value.rs#L1311-L1317)
berechnet `data.len() - 1`; der
[`SORT`-Aufrufer](../crates/icy_board_engine/src/vm/statements/predefined_procedures.rs#L2084-L2101)
verwendet dies ohne Leerfallprüfung. `Len()` und `FOREACH` auf demselben leeren
Array funktionieren dagegen: Die Kontrollprobe druckt `0 done` und betritt
den Schleifenrumpf nicht.

**Empfehlung:** Leere Arrays als regulären Zustand jeder Arrayoperation
behandeln. `SORT` sollte ein leeres Indexarray liefern oder einen spezifizierten
PPL-Fehler melden, niemals einen Host-Panic. Ein bloßes saturierendes Abziehen
genügt nicht: Der anschließende Ausdruck `+ 1` würde dann fälschlich ein Element
behaupten. Kein Release-Build getestet; der konkrete Panic-Nachweis gilt für
das verwendete Testprofil.

### N4 · Mittel: `SORT` erzeugt einen zusätzlichen Index

```PPL
;$LANGVERSION 400
INTEGER data[] = { 8, 7 }
INTEGER indices[], index
SORT data, indices
PRINT indices.Len(), ": "
FOREACH index IN indices
    PRINT index, " "
NEXT
```

Ergebnis: `3: 1 0 0 ` statt eines Indexarrays mit zwei Elementen. Auch bei
anfänglich begrenzten Arrays `data[1]` und `indices[1]` wächst das Ergebnis
auf drei Elemente.

Die [Implementierung](../crates/icy_board_engine/src/vm/statements/predefined_procedures.rs#L2084-L2101)
berechnet die Elementzahl, übergibt sie dann aber als Obergrenze an `redim`.
Der zusätzliche Defaultindex `0` führt beim Durchlaufen des Ergebnisses zu
einem doppelten Element. Dies ist eine konkrete Auswirkung der Vermischung
von Obergrenze und Elementzahl, nicht nur eine Frage der Benennung.

**Empfehlung:** Für 400 eine Indexfolge mit genau einem Eintrag pro sortiertem
Eingabeelement garantieren; Bounds-/Count-Konvertierung zentralisieren. Bei
Änderungen am alten Opcode vor-400-Verhalten getrennt per Oracle prüfen.

## 2. Designverträge: bewusste Entscheidungen, nicht alles Bugs

### V1 · `DECLARE` ist kein durchgehend verbindlicher Vertrag

Verifiziert: `DECLARE PROCEDURE Show(INTEGER value)` darf eine Implementierung
mit `STRING value` haben; `Show("abc")` kompiliert und druckt `abc`.
Ebenso darf eine Deklaration `VAR INTEGER value` versprechen, während die
Implementierung einen Wertparameter verwendet: Eine Zuweisung im Rumpf wird
nicht zurückgeschrieben.

Dies ist [bewusst beibehaltenes Legacy-Verhalten](../crates/icy_board_engine/src/semantic/symbols.rs#L32-L42),
während Callback-Signaturen streng geprüft werden. Es ist damit eine
Asymmetrie im neuen Sprachvertrag, nicht eine unbemerkte Regression der
vorigen Korrektur.

**Empfehlung:** In Sprache 400 vollständige Übereinstimmung von Typ, Rang,
`VAR`, Routineart und Rückgabe verlangen; historische Sprachversionen getrennt
behandeln. Automatische Vorwärtsdeklarationen sind kein Grund, explizit
widersprüchliche Verträge still zu akzeptieren.

### V2 · Enums sind nominal, aber nicht auf benannte Werte beschränkt

Bei `ENUM Color` mit `Red = 1` und `Green = 3` gilt:

- Ein nicht initialisiertes `Color value` hat den Wert `0`; Vergleiche mit
  beiden Mitgliedern sind falsch. Beobachtete Ausgabe: `0 0 0`.
- `FOR value = Color.Red TO Color.Green` erzeugt `1 2 3`.

Nominale Typisierung bedeutet nicht automatisch eine geschlossene Wertemenge.
Deshalb ist „Enum-Typverwechslung“ nicht die passende Diagnose. Das Design
verbindet allerdings strenge Quelloperationen mit unbenannten Standard- und
Schleifenwerten. Die
[Referenz](new_ppl.md#L1736-L1747) dokumentiert die FOR-Ausnahme, aber keinen
entsprechend klaren Default-Vertrag.

**Empfehlung:** Vor dem Freeze entscheiden: offene nominale Integer-Typen
explizit samt Nullinitialisierung dokumentieren, oder geschlossene Enums mit
passender Initialisierungsregel und ohne numerischen Enum-FOR. Nur `FOR` zu
verbieten würde den unbenannten Defaultwert nicht lösen.

### V3 · Flags sollten eine Typeigenschaft sein

Die [Operatorprüfung](../crates/icy_board_engine/src/semantic/visitor.rs#L186-L192)
erlaubt `&` und `|` speziell für die ID von `RegexOptions`, nicht allgemein
für einen als Flags gekennzeichneten Enumtyp.

**Empfehlung:** Generisches Flags-Merkmal in der Typregistrierung. Falls
benutzerdefinierte Flags gewünscht sind, darauf aufbauend eine Deklarationsform.
Keine weitere Sammlung von Bibliotheks-IDs in der Sprachsemantik.

### V4 · Modulinitialisierung braucht einen vollständigen Vertrag

Der [erste Review](ppl400_language_review_2026-09-05.md) hat Seiteneffekte in
Modul-Variableninitialisierern bereits nachgewiesen; dieser Lauf hat keine
neue Reihenfolge- oder Abhängigkeitsmatrix geprüft. Die
[Referenz](new_ppl.md#L171-L174) sagt weiterhin, Module deklarieren und führen
kein eigenes Programm aus.

**Empfehlung:** Entweder Initialwerte auf konstante Ausdrücke beschränken und
Initialisierung explizit aufrufen, oder Laufzeitinitialisierung samt Reihenfolge,
Abhängigkeiten und Fehlerverhalten spezifizieren. Für PPL ist die erste Lösung
kleiner und leichter nachvollziehbar. Der bestehende Zustand ist vor allem ein
unvollständiger Vertrag, kein Nachweis fehlerhafter Initialisierungsreihenfolge.

### V5 · `VAR` bedeutet Copy-in/Copy-out, nicht echte Referenzaliasierung

Verifiziert mit zwei Parametern, denen dieselbe Variable übergeben wird:
`first = 2` verändert `second` im Rumpf nicht; dieser liest weiterhin `1`.
Nach `second = 3` hat die ursprüngliche Variable nach Rückkehr den Wert `2`.
Ausgabe der Probe: `1 2`.

Die [Aufrufvorbereitung](../crates/icy_board_engine/src/vm/call_stack.rs#L10-L39)
kopiert Werte; beim
[Rücksprung](../crates/icy_board_engine/src/vm/mod.rs#L911-L931)
werden sie in umgekehrter Parameterreihenfolge zurückgeschrieben. Hinzu kommt
die im ersten Review verifizierte erneute Auswertung indizierter Argumentziele.

**Empfehlung:** Nicht als „echte Referenz“ dokumentieren. Aliasierung und
Seiteneffekte im Zielausdruck explizit beschreiben, gegebenenfalls Warnungen
vorsehen. Keine unangekündigte Änderung der alten Semantik; zuerst Oracle.
Für neue Array-`VAR`-Parameter diesen Vertrag bewusst festlegen.

## 3. Was bleiben sollte und was nicht bestätigt wurde

- Nominale Records mit Wertkopien und festen Arrayfeldern sind ein gutes Modell.
- Module mit privaten Implementierungen passen besser als ein zusätzliches
  Klassen-/Vererbungssystem; Closures sind für die vorhandenen Routineparameter
  keine Voraussetzung.
- Snapshot-`FOREACH` und Routinen-Scope ohne neue Scopes durch `BEGIN` sind
  nachvollziehbar. Die Belege dafür stammen aus dem ersten Review.
- Historische Operatoren nicht heimlich modernisieren. Vorrang,
  Assoziativität, Konvertierungen und fehlende Kurzschlussauswertung normativ
  tabellieren; gegebenenfalls Warnungen oder separate Kurzschlussoperatoren.
- Bounds bleiben Obergrenzen, `Len()` liefert Elementzahlen. Für normale
  Arrays mit Anfangsobergrenze nicht denselben Begriff „fixed“ verwenden wie
  für unveränderlich geformte Record-Felder.
- **Kein Formatter-Semantikverlust bestätigt:** Das geprüfte Record-Programm
  druckt vor und nach Formatierung `old`. Der Formatter behält dabei `()` bei.
  Damit ist allerdings die pauschale Zusage in der
  [Referenz](new_ppl.md#L1654-L1657), neu formatierter 400-Code schreibe immer
  `[]`, nicht erfüllt. Keinen automatischen Austausch forcieren, bevor N2
  behoben ist. Der Decompiler wurde in diesem Lauf nicht separat geprüft.

## 4. Priorisierte Empfehlung

1. **N3 und N4:** Leerfall und Elementzahl von `SORT` absichern.
2. **N1 und N2:** Arrayparameter und beschreibbare Index-/Record-Pfade
   durchgängig implementieren, nicht nur einzelne AST-Sonderfälle ergänzen.
3. **400-Verträge entscheiden:** strenges `DECLARE`, Enumwertemenge und
   Defaultwerte, generische Flags, zulässige Modulinitialisierung.
4. **Normative Kurzreferenz:** Wert-/Copy-out-Semantik, Auswertungsreihenfolge,
   Bounds versus Counts, Scope und Initialisierung verbindlich festhalten.
5. **Featurekombinationen als Regressionen:** normale/Callback-Aufrufe,
   Wert-/`VAR`-Parameter, Rang 1–3, leere/nichtleere Arrays, Record-Pfade,
   beide Klammerformen sowie Formatter-/Decompiler-Roundtrips.

Der wichtigste Architekturpunkt ist eine gemeinsame Darstellung von
Elementtyp, Rang und Beschreibbarkeit. Solange diese Eigenschaften je nach
AST-Form und Aufrufpfad erneut hergeleitet werden, entstehen immer neue
Ausnahmen, obwohl die Einzelmerkmale für sich getestet sind.