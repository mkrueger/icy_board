# PPL 4.00: Sprachreview

Stand: 2026-09-05. Gegenstand: Sprache einschließlich der geerbten
3.50-Erweiterungen, **nicht** nochmals die Board-/Standardbibliotheks-API.
Ergänzt das [API-Review](ppl400_api_review_2026-09-05.md).

## Korrekturstand 2026-09-06

**Die sieben nachgewiesenen Implementierungsprobleme L1–L7 sind behoben.**
Die folgenden ursprünglichen Befunde bleiben als Fehlerbeschreibung erhalten;
ihre Ist-Ausgaben beziehen sich auf den Stand vor der Korrektur.

- **L1/L2:** Dynamische Speicherkennzeichnung und `STATIC` verwenden getrennte
  Bits. Lokale Arrays und Array-Rückgaben sind aufruflokal, einschließlich
  Rekursion. Ganzarray-Zuweisungen kopieren auch in begrenzte Ziele; dynamische
  Brace-Initializer behalten ihre Dynamik und werden bei jeder Ausführung
  initialisiert, auch `{}`. Leere dynamische Recordarrays bleiben leer.
  [Regressionen](../crates/icy_board_engine/src/vm/tests/array_values.rs)
- **L3/L4:** Automatische Deklarationen und rekursive Callback-Signaturen
  enthalten den Rückgaberang. Array-Callbacks unterstützen Rang 1–3; AST-Ausgabe,
  Formatierung und LSP-Signaturen erhalten ihn.
  [Forward-Aufrufe](../crates/icy_board_engine/src/vm/tests/forward_calls.rs),
  [Callbacks](../crates/icy_board_engine/src/vm/tests/routine_parameters.rs),
  [LSP](../crates/ppl-lsp/tests/array_return_signatures.rs)
- **L5/L6:** Array-Ergebnisse werden in skalaren Kontexten unabhängig von der
  Ausdrucksform abgewiesen. Beide `REDIM`-Schreibweisen verlangen ab Sprache
  400 eine Arrayvariable und genau eine Obergrenze pro deklarierter Dimension.
  Klassisches rangänderndes `REDIM` bleibt für ältere Sprachversionen erhalten.
  [Regressionen](../crates/icy_board_engine/src/vm/tests/array_type_checks.rs)
- **L7:** Zielindizes werden vor dem Lesen und vor der rechten Seite einmalig
  von links nach rechts erfasst; Objektempfänger behalten ihre Identität.
  Temporäre Werte bleiben auch bei Rekursion erhalten. Zusätzlich wurden
  dateiübergreifende Kollisionen der Member-Typinformationen behoben.
  [Regressionen](../crates/icy_board_engine/src/vm/tests/nested_records.rs),
  [Module](../crates/icy_board_engine/src/compiler/modules.rs)

Validierung: Engine-Bibliothek **1507 bestanden, 5 ignoriert**; vollständige
Tests einschließlich Integrations- und Dokumentationstests für
`icy_board_engine`, `pplc`, `ppld` und `ppl-lsp` bestanden. Die neuen Tests prüfen
auch optimierte/unoptimierte Compound-Zuweisungen und Runtime 340 sowie
unveränderte klassische Headerflags und Array-Decay-Semantik.

Die Designentscheidungen D1–D6 und die noch nicht per PCBoard-Oracle geklärte
`VAR`-Index-Neuauswertung wurden nicht umgestaltet. Durch das getrennte
Dynamik-Bit müssen vorhandene **unveröffentlichte 4.00-PPEs mit dynamischen
Arrays neu kompiliert** werden; klassische PPE-Formate sind davon nicht betroffen.
Keine Upload-Änderungen angefasst; kein Commit/Push in diesem Korrekturschritt.

## Urteil

**Die Richtung stimmt, aber die Sprache ist noch nicht konsistent genug zum
Einfrieren.** Prozedurales BASIC, nominale Records, Module und typisierte
Routineparameter ergeben ein brauchbares, überschaubares Modell. Es braucht
keine Klassen, Vererbung oder Closures, um dieses Modell abzurunden.

Der größte Fehler ist nicht ein einzelnes Syntaxdetail: **Arrays wurden noch
nicht durchgängig als Werte mit Elementtyp und Rang integriert.** Parser,
Signaturerfassung, Typprüfung, Codegenerierung und VM verwenden dafür teilweise
unterschiedliche Kriterien. Das erzeugt echte Fehlübersetzungen und nicht nur
ungewohnte Schreibweisen.

Vor einem Freeze würde ich zuerst die sieben nachgewiesenen Probleme unten
beheben und danach wenige Sprachverträge ausdrücklich festlegen. Dieses Review
ändert diese Verträge und die Implementierung noch nicht.

## Methode und Grenzen

- Quellprüfung von Parser, AST-Transformation, Semantik, Modul-Lowering,
  Variablentabelle, Aufrufrahmen und VM.
- 28 gezielte Beobachtungsprogramme in fünf temporären Rust-Tests, ausgeführt
  über den vorhandenen Compiler → PPE-Serialisierung → Laden → VM-Testpfad.
  Die Programme protokollierten Ergebnisse und Compilerdiagnosen; ein grüner
  Testlauf bedeutet hier **nicht**, dass das beobachtete Verhalten korrekt ist.
- Zusätzlich bestanden: 16 Modultests, 13 Enumtests und vier Formatierungstests.
- Die temporären Proben wurden nach der Auswertung entfernt. Reproduktionen
  der zentralen Befunde stehen unten. Keine Produktionsänderung und kein
  Commit/Push in diesem Review; der Git-Index blieb unverändert.
- Kein neuer PCBoard-Oracle-Abgleich. Insbesondere die historischen `VAR`-
  Details dürfen nicht auf Grundlage moderner Erwartungen umdefiniert werden.
- Kein vollständiger LSP-/Decompiler-End-to-End-Audit in dieser Runde.

## 1. Nachgewiesene Implementierungsprobleme

Alle Beispiele in diesem Abschnitt verwenden `;$LANGVERSION 400`.
P1 bedeutet: vor dem Sprachfreeze beheben; keine Behauptung eines Sicherheitsfehlers.

### L1 · P1: Dynamische lokale Arrays sind nicht aufruflokal

```PPL
;$LANGVERSION 400
Work(2)

PROCEDURE Work(INTEGER depth)
    INTEGER values[]
    values.Redim(0)
    values[0] = depth
    IF depth > 0 Work(depth - 1)
    PRINT values[0], " "
ENDPROC
```

**Ist:** `0 0 0 `. Bei aufruflokalen Variablen wäre `0 1 2 ` zu erwarten.
Innere Aufrufe überschreiben das Array des äußeren Aufrufs.

Weitere Kontrollen:

- Zwei Aufrufe derselben Prozedur mit `INTEGER values[]`, wobei nur der erste
  `values.Redim(2)` ausführt, drucken `3 3 ` statt eines frischen leeren Arrays
  im zweiten Aufruf.
- Eine deklarierte Array-Funktion mit bedingtem `RETURN values` liefert beim
  zweiten Aufruf ohne ausgeführtes `RETURN` nochmals das vorherige Array.
  Die entsprechende skalare Funktion liefert dagegen `7 0 7` bei
  wahr/falsch/wahr; ein generell veralteter skalarer Rückgabewert wurde nicht bestätigt.
- Mit `INTEGER values[] = { 0 }` ergibt die Rekursionsprobe `0 1 2 `.
  Der Initializer ändert somit sogar die Aufrufsemantik, nicht nur den Startwert.

Ursache: [VARIABLE_FLAG_DYNAMIC_ARRAY](../crates/icy_board_engine/src/executable/variable_table.rs#L19)
verwendet `0x01`. Genau dieses Bit überspringen
[save_call_frame](../crates/icy_board_engine/src/vm/call_stack.rs#L50-L59) und
[die Wiederherstellung beim Return](../crates/icy_board_engine/src/vm/mod.rs#L890-L901).
Der Brace-Initializer wird dagegen in der
[Variablendeklaration](../crates/icy_board_engine/src/semantic/visitor.rs#L1721-L1751)
wie ein Array mit konkreter Obergrenze erfasst.

**Empfehlung:** Array-Dynamik und Lebensdauer/Frame-Verhalten getrennt
repräsentieren; lokale Arrays und Rückgabespeicher korrekt initialisieren und
sichern. Rekursion, Folgeaufrufe und Initializer-Varianten als Pflichtregressionen.

### L2 · P1: Akzeptierte Ganzarray-Zuweisungen können vollständig verschwinden

```PPL
;$LANGVERSION 400
INTEGER source[] = { 7, 8, 9 }
INTEGER target[] = { 1 }
target = source
PRINT target.Len(), " ", target[0]
```

**Ist:** `1 1`, ohne Compilerfehler. Nach dem dokumentierten Kopiervertrag wäre
`3 7` zu erwarten. Mit einem tatsächlich dynamisch erfassten Ziel ohne
Brace-Initializer funktioniert die Kopie.

Kontrollprobe: Ein befülltes `INTEGER source[5]` wird einem
`INTEGER target[1]` zugewiesen. Das Ziel bleibt bei zwei Nullelementen.
Mit `INTEGER target[]` werden hingegen sechs Elemente samt Werten übernommen.

Die [Formprüfung](../crates/icy_board_engine/src/semantic/arrays.rs#L17-L24)
behandelt normale Arrayvariablen als größenveränderlich. Die
[Codegenerierung](../crates/icy_board_engine/src/compiler/mod.rs#L558-L563)
lässt eine unindizierte Arrayzuweisung aber nur mit gesetztem Dynamik-Bit zu;
sonst protokolliert sie einen Fehler und gibt keinen Befehl zurück. Deshalb
verschwindet hier die Anweisung, statt eine Compilerdiagnose zu erzeugen.

**Empfehlung:** Erst den Vertrag festlegen: entweder alle normalen Arrays
übernehmen die Form, oder nur ausdrücklich dynamische Ziele dürfen das.
Semantik und Codegenerierung müssen denselben Vertrag verwenden. Eine
akzeptierte Zuweisung darf niemals bloß wegfallen. `[] = { ... }` darf die
explizit angegebene Dynamik nicht unbemerkt verlieren.

### L3 · P1: Optionale DECLAREs funktionieren nicht für Array-Rückgaben

```PPL
;$LANGVERSION 400
INTEGER values[] = MakeValues()
PRINT values.Len()

FUNCTION MakeValues() INTEGER[]
    INTEGER result[] = { 17, 23 }
    RETURN result
ENDFUNC
```

**Ist:** unter anderem `FUNCTION return type does not match with declaration
(MakeValues)`, obwohl keine explizite Deklaration existiert.
`DECLARE FUNCTION MakeValues() INTEGER[]` voranzustellen beseitigt diese
Signaturabweichung.

Die [automatische Voraberfassung](../crates/icy_board_engine/src/semantic/mod.rs#L972-L992)
übernimmt den Elementtyp, nicht den Rückgaberang. Die spätere Implementierung
wird dagegen [einschließlich Rang verglichen](../crates/icy_board_engine/src/semantic/visitor.rs#L1901-L1906).

**Empfehlung:** Eine vollständige Signaturrepräsentation für explizite und
automatische Deklarationen; kein spezieller `DECLARE`-Zwang für Arrayfunktionen.

### L4 · P1: Callback-Typprüfung verliert den Rückgaberang

```PPL
;$LANGVERSION 400
DECLARE FUNCTION MakeValues() INTEGER[]
Apply(MakeValues)

PROCEDURE Apply(FUNCTION callback() INTEGER)
    PRINT callback()
ENDPROC

FUNCTION MakeValues() INTEGER[]
    INTEGER values[] = { 17, 23 }
    RETURN values
ENDFUNC
```

**Ist:** kompiliert und druckt `0`. Ein Callback, der einen Integer erwartet,
akzeptiert eine Funktion, die ein Integerarray zurückgibt.

Umgekehrt lässt sich die gewünschte Callback-Signatur
`FUNCTION callback() INTEGER[]` nicht ausdrücken: Der Parser meldet unter
anderem `Expected type ([)`.

Die [Callback-Prüfung](../crates/icy_board_engine/src/semantic/mod.rs#L1085-L1115)
vergleicht Elementtyp und Parameter, nicht den Rückgaberang.
[FunctionParameterSpecifier-Parsing](../crates/icy_board_engine/src/parser/mod.rs#L641-L655)
liest nach dem Rückgabetyp keinen Array-Rang.

**Empfehlung:** Den Rang durch Parser, AST und rekursive Signaturvergleiche
führen. Solange Array-Callbacks nicht unterstützt sind, müssen solche
Funktionen als Callback wenigstens gezielt abgewiesen werden.

### L5 · P1: Array-Ausdrücke werden nicht einheitlich als Skalare abgewiesen

Mit explizitem `DECLARE FUNCTION MakeValues() INTEGER[]` und derselben
Implementierung wie oben gilt:

- `PRINT MakeValues(), " ", MakeValues() + 1` kompiliert und druckt `0 1`.
- `INTEGER values[] = { 17, 23 }` gefolgt von `PRINT values` wird korrekt
  abgewiesen: `Not enough arguments passed (values:0:1)`.

Die [Array-Erkennung](../crates/icy_board_engine/src/semantic/arrays.rs#L49-L72)
kennt funktionswertige Arrays bereits. Die anschließende
[Skalar-Abweisung](../crates/icy_board_engine/src/semantic/arrays.rs#L106-L129)
meldet aber nur bestimmte Formen, insbesondere Identifier und Felder.

**Empfehlung:** Skalar-/Arrayprüfung vom Typ und Rang des Ausdrucks abhängig
machen, nicht von seiner syntaktischen Form. Dasselbe gilt für Klammerung,
Funktionsergebnisse, Member-Ergebnisse, Argumente und Rückgaben.

### L6 · P1: REDIM ändert den Laufzeitrang, die Typprüfung nicht

```PPL
;$LANGVERSION 400
INTEGER values[]
values.Redim(1, 2)
values[1] = 7
PRINT values.Len(0), " ", values.Len(1), " ", values[1]
```

**Ist:** `2 3 7`: Das deklarierte eindimensionale Array ist zur Matrix geworden.
Die dafür passende Schreibweise `values[1, 2] = 7` wird hingegen abgewiesen:
`Too many arguments passed (values:2:1)`.

`REDIM values, 1, 2` und `values.Redim(1, 2)` stimmen dabei überein. Es gibt
**keinen** nachgewiesenen Unterschied zwischen den beiden Schreibweisen.
Die [VM setzt den Rang aus der Argumentzahl](../crates/icy_board_engine/src/vm/statements/predefined_procedures.rs#L1939-L1946);
die Semantik behält die deklarierte Dimension.

**Empfehlung:** Für 400 rangstabile Arrays: `REDIM` verändert Obergrenzen,
nicht den Rang. Alternativ wäre ein dynamischer Rang möglich, erforderte aber
eine andere Typprüfung überall. Das passt schlechter zur expliziten
Syntax `[]`, `[,]`, `[,,]` und zu geprüften Rückgabesignaturen.

### L7 · P1: Zusammengesetzte Zuweisungen werten Zielausdrücke mehrfach aus

```PPL
;$LANGVERSION 400
INTEGER calls
INTEGER values[] = { 10, 20, 30, 40 }
values[NextIndex()] += 1
PRINT calls, " ", values[0], " ", values[1], " ", values[2]

FUNCTION NextIndex() INTEGER
    calls += 1
    RETURN calls - 1
ENDFUNC
```

**Ist:** `2 10 11 30`. Gelesen wird Element 0, geschrieben Element 1.
Ein einmal ausgewertetes Zuweisungsziel würde `1 11 20 30` ergeben.

Die [Compound-Transformation](../crates/icy_board_engine/src/compiler/ast_transform.rs#L449-L514)
kopiert den Zielausdruck in die rechte Seite. Die VM wertet seine Indizes
beim Lesen und nochmals [beim Schreiben](../crates/icy_board_engine/src/vm/mod.rs#L736-L748) aus.

**Empfehlung:** Einen zugewiesenen Speicherort samt Indizes einmal bestimmen;
danach lesen, Operation ausführen und an denselben Ort schreiben. Das ist ein
Vertrag für die 350/400-Erweiterung, kein Grund, alte PPEs umzudeuten.

**Separat zu klären:** `Increment(values[NextIndex()])` mit einem `VAR`
Parameter erzeugt ebenfalls `2 10 11 30`. Der Aufruf speichert den
[Argumentausdruck für das Zurückschreiben](../crates/icy_board_engine/src/vm/call_stack.rs#L10-L40).
Das Verhalten ist belegt; ob es von PCBoard übernommen werden muss, wurde
hier nicht per Oracle entschieden. Nicht ungeprüft zusammen mit `+=` ändern.

## 2. Designentscheidungen, die verbessert werden sollten

### D1 · Array-Größe, Obergrenze und feste Form klar unterscheiden

`INTEGER values[10]` enthält elf Elemente, während `{ 1, 2, 3 }` drei
Elemente liefert. `Redim(20)` führt zu `Len() = 21`. Das ist historisch
erklärbar, aber die neue Klammernotation erzeugt andere Erwartungen.

Zusätzlich sind normale begrenzte Arrays resizebar, Record-Arrayfelder dagegen
formfest. „Fixed array“ ist deshalb als Bezeichnung für beide ungeeignet.

**Empfehlung:** Vor dem Freeze ausdrücklich zwischen Anfangsobergrenze,
dynamischer Anfangsform und fester Record-Form unterscheiden. Wenn 400 bei
Obergrenzen bleibt, überall „upper bound“ statt „size“ schreiben und
`Len()` konsequent als Elementzahl dokumentieren. Nicht dieselbe Syntax ohne
Versionsgrenze nachträglich von Obergrenze auf Elementzahl umstellen.

### D2 · Nominale Enums und numerische FOR-Schleifen widersprechen sich

Für `ENUM Color` mit `Red = 1` und `Green = 3` druckt
`FOR value = Color.Red TO Color.Green` die Werte `1 2 3`.
Die Schleife erzeugt also einen unbenannten Wert eines ansonsten nominal und
arithmetisch eingeschränkten Typs. Die [Referenz](new_ppl.md#L1723-L1729)
beschreibt die Ausnahme; sie ist kein versehentlicher Schleifenfehler.

**Empfehlung:** Entweder Enums ausdrücklich als offene nominale Integer-Typen
definieren oder den numerischen `FOR` über Enums untersagen. Für eine Sprache,
die sonst nur benannte Enumwerte zulässt, würde ich Letzteres bevorzugen.
Eine spätere Iteration über deklarierte Mitglieder wäre etwas anderes als
eine Integer-Schleife; dafür nicht vorschnell neue Syntax einführen.

### D3 · Flags brauchen eine allgemeine Regel, keinen Bibliotheksnamen

Benutzerdefinierte Enums erlauben kein `|`/`&`. Der Compiler nimmt dagegen
[RegexOptions ausdrücklich aus](../crates/icy_board_engine/src/semantic/visitor.rs#L185-L192).
Das ist eine echte Design-Asymmetrie: Eine Bibliotheksidentität bestimmt
Sprachoperatoren.

**Empfehlung:** Allgemeines Flags-Merkmal für Enumtypen in der Typregistrierung;
falls Benutzer Flags deklarieren dürfen, eine dazu passende Deklaration.
Normale Enums bleiben nominal und ohne Bitoperationen. Alternativ die
Kombination über benannte Operationen anbieten, statt Compiler-Sonderwissen.

### D4 · Ausdrucksregeln explizit machen, nicht still modernisieren

Beobachtet:

- `"5" + 1`, `"5" = 5`, `5 = "5"`, `"5" - 1` ergeben `6 1 1 4`.
  Die Vermutung „Arithmetik konvertiert, skalare Vergleiche lehnen dieselben
  Typen ab“ wurde **nicht bestätigt**.
- `FALSE & Touch()` und `TRUE | Touch()` rufen beide `Touch()` auf.
- `TRUE | TRUE & FALSE` ergibt `0`: `&` und `|` sind gleichrangig und
  linksassoziativ. Vergleiche binden stärker als beide, nicht gleich stark.
- Potenzierung wird im Parser linksassoziativ gelesen; unäre Operatoren binden
  davor. Siehe [Ausdrucksparser](../crates/icy_board_engine/src/parser/expression.rs#L41-L167).

**Empfehlung:** Eine verbindliche Tabelle für Vorrang, Assoziativität,
Konvertierung und Auswertungsreihenfolge; Warnungen für überraschende
Mischungen und Vergleichsketten. Die bestehenden Operatoren nicht heimlich
auf Kurzschlussauswertung umstellen. Bei Bedarf später separate, eindeutig
benannte Kurzschlussoperatoren statt einer Umdeutung alter Programme.

### D5 · Modulinitialisierung ist ein unvollständiger Vertrag

Die Referenz sagt, Module deklarieren nur und hätten kein eigenes Programm.
Explizite Statements werden auch abgewiesen. Allerdings kompiliert ein Modul
mit `INTEGER value = Initialize()`, dessen Funktion `PRINT "module init"`
ausführt, und die Probe druckt tatsächlich `module init`.

Das ist nicht automatisch falsch: Deklarationsinitialisierung darf Arbeit
ausführen. Es widerspricht aber einer uneingeschränkten Zusage „Module laufen
nicht“. Die [Modulprüfung](../crates/icy_board_engine/src/compiler/modules.rs#L230-L245)
lässt Variablendeklarationen als Ganzes zu.

**Empfehlung:** Für Module entweder nur konstante Initialwerte zulassen und
Laufzeitinitialisierung explizit aufrufen, oder Ausführungszeitpunkt,
Reihenfolge, Abhängigkeiten und Fehlerverhalten von Initializern spezifizieren.
Für PPL würde ich die erste, kleinere Lösung bevorzugen.

`PUBLIC` als Standard ist konsistent implementiert, aber für wiederverwendbare
Bibliotheken fehleranfällig. `PRIVATE` als Standard wäre vor dem Freeze eine
sinnvolle Option, keine notwendige Fehlerkorrektur. Abschnittsbasierte
Sichtbarkeit und quelllokale Import-Aliase können ansonsten bleiben.

### D6 · FOREACH ist ein Snapshot; BEGIN erzeugt keinen neuen Scope

- Die Probe läuft über `{ 1, 2, 3 }` und setzt im Schleifenrumpf Element 1 auf
  `99`. Sie druckt weiterhin `1 2 3`, danach zeigt das Originalarray `99`.
  Die VM hält [die Array-Sammlung im Iteratorrahmen](../crates/icy_board_engine/src/vm/mod.rs#L1004-L1038).
  Das ist Snapshot-Verhalten, nicht ein erneutes Lesen der aktuellen
  Quellvariable pro Schritt. Die Formulierung in der
  [Referenz](new_ppl.md#L1601-L1604) sollte das ausdrücklich sagen.
- Ein in einem inneren `BEGIN ... END` deklariertes `INTEGER item = 1` ist
  danach im äußeren Block sichtbar. Das ist Gruppierung, kein Block-Scope.

**Empfehlung:** Beides beibehalten und klar benennen. Snapshot-Iteration ist
gut vorhersehbar; reine Gruppierungsblöcke passen zu PPLs Routinen-Scope.
Lexikalische Blockscopes wären ein größeres neues Feature samt Regeln für
Shadowing und Sprünge, keine kleine kosmetische Korrektur.

## 3. Was konsistent und erhaltenswert ist

- **Nominale Records:** gleiche Struktur bedeutet nicht gleicher Typ;
  Wertkopien, verschachtelte Records und feste Arrayfelder ergeben ein
  verständliches Modell. Gleichheit ohne Record-Arithmetik ist sinnvoll.
- **Module statt Klassen:** qualifizierte Namen, private Implementierung,
  quelllokale Aliase und ein einzelnes resultierendes PPE passen zum Einsatz.
  Die 16 vorhandenen Modulregressionen bestanden.
- **Routineparameter statt eines vollständigen Closure-Systems:** gute
  Umfangsbegrenzung. Vollständige Signaturen einschließlich Array-Rang fehlen
  noch, nicht ein ganz anderes Funktionsmodell.
- **Sprachversion getrennt von PPE-Runtime:** sinnvolle Trennung zwischen
  Quellsyntax und Speicherformat. Vorhandene PPE-Kompatibilität bleibt ein
  eigenständiger Vertrag.
- **Zuweisung als Statement:** keine Assignment-Ausdrücke einzuführen ist kein
  Designfehler. Auch `VAR` ohne tatsächliche Mutation muss nicht verboten werden.
- **Kontrollfluss:** `REPEAT`, `LOOP`, `BREAK`, `CONTINUE` und das getrennte
  Schließen/Beenden durch `END` und `EXIT` sind nachvollziehbare Erweiterungen.

## 4. Empfohlene Reihenfolge

1. **Arrays durchgängig modellieren:** Elementtyp, Rang, Größenveränderlichkeit
   und Lebensdauer getrennt; nicht aus AST-Form oder einem überladenen Headerbit
   erraten. L1/L2 zuerst, dann Signaturen und Ausdrucksprüfung L3–L5.
2. **Rang- und Zielauswertungsvertrag festlegen:** rangstabiles `REDIM`, einmalige
   Auswertung zusammengesetzter Zuweisungsziele. Legacy-`VAR` separat prüfen.
3. **Drei kleine Designentscheidungen:** Enum-Iteration, generische Flags,
   zulässige Modulinitialisierung.
4. **Kurze normative Sprachreferenz:** Typ-/Konvertierungsregeln,
   Operatorentabelle, Arrayformen, Routinen-Scope, Snapshot-Iteration und
   Rückgabewerte ohne ausgeführtes `RETURN`.
5. **Regressionen als Matrix:** skalare Werte/Arrays/Records; lokale/globale
   Variablen; Initializer/kein Initializer; direkte/Callback-Aufrufe;
   explizite/automatische Deklaration; Rekursion; Rang 1–3; anschließend
   Formatter, Decompiler und LSP auf denselben Signaturvertrag prüfen.

**Fazit:** Kein grundlegender Fehlentwurf. Aber ein zu wenig durchgezogener
Array-Werttyp und einige nicht vollständig entschiedene Sprachverträge.
Jetzt konsolidieren, statt vor dem Freeze weitere Syntax hinzuzufügen.