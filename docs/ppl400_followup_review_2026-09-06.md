# PPL 400: Follow-up nach Enum-Bitoperationen und Has

Stand: 2026-09-06, geprüfter PPL-Commit `aeac4ece`.

## Korrekturstand

**F1–F12 sind implementiert.** Die folgenden Befunde dokumentieren den zuvor
geprüften Zustand, nicht verbleibende offene Fehler.

- F1–F3: Compiler und VM sichern Arrayquelle, skalares FOREACH-Ziel, nominale
  Recordtypen und tatsächliche feste Feldformen; fehlerhafte Feldzuweisungen
  lassen den Zielwert unverändert.
- F4/F7/F8: Gemeinsame Deklarationskonvertierung für Compiler/LSP, typisierte
  Substitution, getrennte Enum-Namensräume und gleiche Enum-Argumentregeln.
  **Nachträgliche Benutzerentscheidung:** `CONST BYTE N = 257` wird in Sprache
  400 als Bereichsfehler abgelehnt. Das ursprüngliche Review-Beispiel sollte
  also nicht mehr zweimal 1 ausgeben. Konvertierte gültige Initialwerte, etwa
  `CONST INTEGER N = 1.5`, ergeben für `N` und abhängige Konstanten jeweils 1.
  Vor-400-Konvertierungen und gewöhnliche Variablen bleiben unverändert.
- F5/F6: Recordresultate bleiben bei Rekursion aufruflokal; direkte und
  Callback-Arrayresultate lassen sich in Rang 1–3 indizieren. Der Decompiler
  rekonstruiert Callbacksignaturen aus konkreten Aufrufstellen.
- F9/F10: Split-Auswertung erfolgt links nach rechts; FindAll prüft Limits
  unabhängig vom Startwert.
- F11: Nur der bereits getrennte moderne StripATX-Member entfernt vollständige
  `@Xhh`-Codes und erhält ungültigen Text. Klassischer Opcode unverändert;
  PCBoard-Quellcode als Referenz geprüft, kein neuer DOS-Oracledurchlauf.
- F12: Gemeinsame typisierte Empfänger-/Aufrufbeschreibung für LSP-Completion
  und Signaturhilfe, einschließlich Rang, Mutierbarkeit und skalaren Methoden.
  DE/EN-Texte für unbeschränktes STRING korrigiert.
- Fehlerlebensdauer präzisiert: Reines Ordinal (implizit/ explizit) erhält alte
  Fehler, fallibles IgnoreCase meldet Erfolg weiterhin über den vorhandenen
  Fehlermechanismus. Fehler aus demselben Statement bleiben geschützt.

Permanente Regressionen:
[Sprache und Laufzeit](../crates/icy_board_engine/tests/ppl400_followup_language.rs),
[Konstanten](../crates/icy_board_engine/tests/ppl400_followup_constants.rs),
[API und Legacy-Kontrollen](../crates/icy_board_engine/tests/ppl400_followup_api.rs),
[LSP](../crates/ppl-lsp/tests/receiver_signatures.rs) sowie VM-Invariantentests.

### Abschließende Validierung

Die vollständige Suite für `icy_board_engine`, `pplc`, `ppld` und `ppl-lsp`
besteht auf einer isolierten Kopie des vorgemerkten Commit-Stands, ohne
Testausschlüsse (`CARGO_INCREMENTAL=0 cargo test-low -q -p icy_board_engine
-p pplc -p ppld -p ppl-lsp`). Die Engine-Library meldet **1592 bestanden,
5 ignoriert**; sämtliche Integrations-, Compiler-, Decompiler-, LSP- und
Dokumentationstests bestehen ebenfalls. Die vier neuen Regressionstestdateien
enthalten **39 bestandene Tests** (API 8, Konstanten 10, Sprache 14, LSP 7),
ergänzt durch zwei VM-Invariantentests.

Die Isolation grenzt gleichzeitig laufende Upload- und Decompiler-Arbeiten ab:
Im gemischten Arbeitsverzeichnis schlug zeitweise der bestehende
`while_break_continue`-Decompilervergleich fehl; im isolierten PPL-Stand
besteht er. Diese parallelen Änderungen sind nicht Teil dieses Fixes.
Die Rust-Formatierung wurde gezielt geprüft; der vorgemerkte Diff ist frei von
Whitespacefehlern.

## Ursprünglicher Review

## Nachtrag: Umsetzung F9–F12 (2026-09-06)

Die Befunde unten dokumentieren den ursprünglichen Reviewzustand. Der separate
API-/LSP-Follow-up korrigiert F9–F12; F1–F8 und deren Compiler-/VM-Integration
werden parallel bearbeitet und sind nicht Gegenstand dieses Nachtrags.

- **F9:** Beide Split-Schreibweisen werten Text/Empfänger, Separator und optionales
  Limit genau einmal von links nach rechts aus, vor der Argumentprüfung.
- **F10:** `Regex.FindAll` prüft negative und über 100.000 liegende Limits auch
  dann, wenn die Startposition keine Suche zulässt. Fehler aus Argumenten bleiben
  innerhalb derselben Anweisung erhalten.
- **F11:** Nur der bereits vorhandene interne Runtime-400-Opcode
  `StringStripAtx` entfernt vollständige `@Xhh`-Codes; gewöhnlicher Text,
  unvollständige/ungültige Codes und UTF-8 bleiben erhalten. Klassisches
  `STRIPATX` (-60), dessen Scanner und Speicherverhalten bleiben unverändert.
  PCBoard-Quellcode bestätigt die vollständige Tokenprüfung; **kein neuer
  DOS-Oracle-Lauf** und keine Behauptung vollständiger Legacy-Parität.
- **F12:** Completion, Kontext-Enumargumente und Member-Signaturhilfe verwenden
  gemeinsame Empfänger-/Aufrufmetadaten mit Elementtyp, Rang, Namespace und
  Redim-Fähigkeit. Arrays, Recordfelder, berechnete Ergebnisse und gewöhnliche
  sowie Callback-Rückgaben behalten ihre Ränge. Skalare STRING-/BYTES-Signaturen
  beachten die Sprachversionsgrenze. Regex-Rückgaben erscheinen als
  `RegexMatch[]` bzw. `STRING[]`; EN/DE-Hilfen nennen unbeschränktes `STRING`
  statt `BIGSTR` für neue Stringergebnisse.
- **Fehlerlebensdauer:** Erfolgreiche explizite `StringComparison.Ordinal`-
  Vergleiche erhalten wie die Default-Overloads einen älteren Fehler. Ungültige
  Modi melden weiterhin einen Fehler. Für die falliblen IgnoreCase-Overloads
  bleibt der bereits getestete Vertrag erhalten: Erfolg löscht ältere Fehler,
  aber keinen Fehler derselben Anweisung; Regex-Ressourcenfehler werden gemeldet.
  Keine pauschale Änderung anderer reiner Funktionen.
- **Integrationsgrenze:** Die bereits schmutzigen Opcode-Ergänzungen
  `ArrayValueAt2` (-357, drei Argumente) und `ArrayValueAt3` (-358, vier Argumente)
  gehören zu F6. Sie benötigen die parallelen Semantic-/Compiler-/VM-/Decompiler-
  Änderungen; der veröffentlichte `ArrayValueAt`-Vertrag bleibt erhalten.
  Die skalaren Parametertypen/-namen und Split-Rückgaberänge liegen vorerst in
  genau einem LSP-Adapter, bis die Engine diese Metadaten vollständig exportiert.

Permanente Regressionen:
[serialisierte API-Proben](../crates/icy_board_engine/tests/ppl400_followup_api.rs),
[LSP-Empfänger und Signaturen](../crates/ppl-lsp/tests/receiver_signatures.rs).
Validierungsergebnisse werden nach dem abschließenden Testlauf ergänzt.

## Ergebnis und Evidenz

Die bestehenden Features brauchen noch Absicherung an ihren Übergängen,
insbesondere `FOREACH`, Record-Zuweisungen und Konstanten-Lowering. Eine weitere
Spracherweiterung ist weniger wichtig als das Schließen dieser Typ-/Wertlücken.

- Sprache 400, Runtime 400; kompilierte Proben wurden als PPE serialisiert,
  erneut geladen und ausgeführt. LSP-Befunde zusätzlich über echte JSON-RPC-
  Anfragen an den frisch gebauten Sprachserver überprüft.
- 14 temporäre Beobachtungstests bestanden: 3 Array-/Recordtests mit 44
  Beobachtungen, 3 Konstantentests und 8 API-/LSP-Tests. Diese Tests behaupteten
  das **beobachtete fehlerhafte Verhalten**, nicht die gewünschte Korrektheit.
  Der Rust-Panic wurde aufgefangen. Die temporären Tests wurden danach entfernt.
- Kein erneuter vollständiger Workspace-Testlauf. Keine Produktivkorrekturen,
  keine Commits und keine Änderungen an paralleler Upload-/Corpus-Arbeit.
- Frühere behobene N1–N4/API-Befunde werden nicht erneut als offen geführt.
  Das alte Sprachreview enthält noch historische, inzwischen überholte
  Zusammenfassungen zu `DECLARE`, Modulinitialisierung und Enum-Regeln.

## Hoch: vor einem Freeze korrigieren

### F1: FOREACH umgeht Typ- und Rangprüfung, bis zum Host-Panic

```PPL
ENUM Color
    Red = 1
ENDENUM
INTEGER values[] = {99}
Color item
FOREACH item IN values
    PRINT TOINTEGER(item)
NEXT
```

Akzeptiert; bei Ausführung Rust-Panic `Unsupported type: UserData(...)` statt
einer PPL-Typdiagnose. Auch ein Array als Schleifenziel wird durch einen Skalar
ersetzt; ein Skalar als Quelle wird einmal durchlaufen. Die Kontrolle mit
passendem Enumarray funktioniert.

Ursache: [SemanticVisitor](../crates/icy_board_engine/src/semantic/visitor.rs#L1764-L1775)
prüft nur den Zielnamen; die [VM](../crates/icy_board_engine/src/vm/mod.rs#L1036-L1075)
schreibt Elemente direkt mit `set_value`, ohne den normalen typisierten
Zuweisungspfad. Quelle als Array und Ziel als kompatiblen Skalar prüfen;
Enum-/Recordinvarianten zusätzlich in der VM erhalten.

### F2: Feste Record-Arrayfelder verlieren ihre deklarierte Form

```PPL
TYPE Boxed
    INTEGER items[1]
ENDTYPE
Boxed box
INTEGER a[1]
a.Redim(4)
a[4] = 9
box.items = a
PRINT box.items.Len(), " ", box.items[4]
```

Ausgabe `5 9`: Das feste Feld mit zwei Elementen enthält nun fünf. Dasselbe
geschieht bei `Boxed { items = a }`. Die deklarierte Form des Quellarrays wird
verglichen, nicht dessen nach `Redim` tatsächlich vorhandene Form.

[Statische Formprüfung](../crates/icy_board_engine/src/semantic/arrays.rs#L19-L24)
und [VM-Feldzuweisung](../crates/icy_board_engine/src/vm/mod.rs#L831-L848).
Vor Feldersetzung/Recordkonstruktion aktuelle Dimensionen und Bounds prüfen;
bei Fehler den bisherigen Record unverändert lassen.

### F3: Indizierte Record-Ziele umgehen nominale Typprüfung

```PPL
TYPE First
    INTEGER value
ENDTYPE
TYPE Second
    STRING text
ENDTYPE
TYPE Boxed
    First entry
ENDTYPE
Boxed boxes[0]
boxes[0].entry = Second { text = "wrong" }
PRINT boxes[0].entry.value
```

Kompiliert, endet aber mit `Type not found in registry`. Die entsprechende
nichtindizierte Zuweisung `box.entry = Second { ... }` wird korrekt abgelehnt.
Der [explizite Zuweisungszielpfad](../crates/icy_board_engine/src/semantic/visitor.rs#L1469-L1518)
prüft Enumkonflikte, aber nicht alle nominalen Recordkonflikte.
Beide Zielschreibweisen müssen dieselben Zuweisungsregeln verwenden.

### F4: Abhängige Konstanten verwenden unkonvertierte Initialwerte

```PPL
CONST BYTE N = 257
CONST INTEGER M = N
PRINT N, "|", M
```

Ausgabe `1|257` statt `1|1`. Auch Modulkonstanten und daraus initialisierte
Modulvariablen sind betroffen (`1|257|257`). Schwerere Variante:

```PPL
ENUM Bits
    One = 1
    Two = 2
ENDENUM
CONST INTEGER N = 1.5
CONST Bits Value = Bits(N)
PRINT Value
```

Compiler und direkte Semantik akzeptieren dies; die VM liefert `Internal VM
error`. Dagegen druckt `PRINT Bits(N)` ohne die abhängige Enumkonstante korrekt
`1`. [Konstantensammlung](../crates/icy_board_engine/src/compiler/ast_transform.rs#L225-L243)
speichert rohe Initialwerte, während die
[semantische Deklaration](../crates/icy_board_engine/src/semantic/visitor.rs#L1846-L1856)
zum deklarierten Typ konvertiert. Konstante Werte und Typen müssen aus einem
gemeinsamen Auswertungspfad kommen; nicht aufgelöste Konstanten dürfen nicht als
ungültiger HIR-Ausdruck bis zur Bytecodeemission gelangen.

## Mittel: Konsistenz und nutzbare Sprachoberfläche

### F5: Rekursive Record-Rückgaben überschreiben das äußere Ergebnis

```PPL
TYPE Item
    INTEGER value
ENDTYPE
PRINT Build(2).value
FUNCTION Build(INTEGER depth) Item
    Build = Item { value = depth }
    IF depth > 0 THEN
        Item child = Build(depth - 1)
    ENDIF
ENDFUNC
```

Ausgabe `0`, obwohl der äußere Aufruf seinen Rückgaberecord auf `2` gesetzt
hat. Über Callback derselbe Effekt; die entsprechende Record-Arrayrückgabe
bewahrt `2`. [Wiederherstellung](../crates/icy_board_engine/src/vm/mod.rs#L668-L685)
schützt nur Arrayresultate. Für neue Recordwerte aufruflokale Ergebnisse
festlegen und absichern; klassische skalare Rückgabesemantik nicht ungeprüft
mitverändern. Workaround: Ergebnis nach dem rekursiven Aufruf explizit setzen.

### F6: Mehrdimensionale Funktionsresultate nicht direkt indizierbar

`Make()[1, 1]` bei Rückgabetyp `INTEGER[,]` sowie der Rang-3-Fall werden mit
Whole-array-/Member-Diagnosen abgelehnt. Auch Callbackresultate sind betroffen.
Zuweisung an ein lokales Array und anschließendes Indizieren funktioniert bei
allen drei Rängen. Der [Getterpfad](../crates/icy_board_engine/src/semantic/visitor.rs#L833-L845)
erkennt berechnete Arrays nur bei Rang 1. Alle unterstützten Rückgaberänge
durchgängig behandeln; nicht über zusätzliche Syntax umgehen.

### F7: Konstantenersetzung verändert Enum-Typnamen

Nach `ENUM Bits ... ENDENUM` akzeptiert die direkte Semantik
`CONST INTEGER Bits = 7` und `PRINT Bits.One`; der Compiler meldet jedoch
`Member not found`. Mit einer gleichnamigen Variablen statt Konstante
funktioniert die Probe. Auch `Bits.One.Has(Bits.One)` ist betroffen.

[Konstantenersetzung](../crates/icy_board_engine/src/compiler/ast_transform.rs#L720-L728)
ersetzt den Typnamens-Empfänger durch eine Zahl, entgegen der
[semantischen Typnamensauflösung](../crates/icy_board_engine/src/semantic/visitor.rs#L344-L356).
Typ-/Wertnamensräume einheitlich behandeln; falls solche Namen verboten werden
sollen, muss das in Compiler und LSP ausdrücklich und gleich diagnostiziert werden.

### F8: RGB in CONST umgeht Enum-Argumentregeln

`CONST UNSIGNED Packed = RGB(Bits.One, 0, 0)` wird akzeptiert und liefert
`16777471`, während `PRINT RGB(Bits.One, 0, 0)` korrekt abgelehnt wird.
Der [Konstantenevaluator](../crates/icy_board_engine/src/ast/const_eval.rs#L175-L185)
verwendet `as_int()` ohne dieselben Argumentregeln wie der normale Aufruf.
Konstantenprüfung darf explizites `TOINTEGER` nicht umgehen. Separater
Paritätsbefund: `CONST SWORD N = 1; PRINT Bits(N)` läuft im Compilerpfad,
wird aber von direkter Semantik wegen des Nicht-`INTEGER`-Arguments abgelehnt.

### F9: STRING.Split mit Limit ändert die Auswertungsreihenfolge

`Source().Split(Separator(), Limit())` ruft **Limit, Source, Separator** auf.
Ohne Limit werden Empfänger und Separator von links nach rechts ausgewertet.
Das betrifft Seiteneffekte und die Reihenfolge von Fehlern.
[Implementierung](../crates/icy_board_engine/src/vm/expressions/predefined_functions.rs#L761-L768).
Für die neue Member-API eine durchgängige Empfänger-/Argumentreihenfolge sichern.

### F10: Regex.FindAll überspringt Limitprüfung bei ungültigem Start

Mit `REGEX rx = REGEX.Compile("a")`:

- `rx.FindAll("a", 0, -1)` setzt `ErrCode.Invalid`.
- `rx.FindAll("a", 2, -1)` liefert leer und setzt Erfolg.

Die [frühe Rückgabe](../crates/icy_board_engine/src/icy_board/state/ppl_regex.rs#L312-L341)
liegt vor der Limitprüfung. Negative Limits sind laut Referenz ungültig;
Argumentgültigkeit sollte nicht davon abhängen, ob die Suche Treffer haben kann.
Die Quellprüfung zeigt denselben Bypass für zu große Limits; die ausgeführte
Probe prüfte den negativen Fall.

### F11: StripATX verändert gewöhnlichen Text

`"email@".StripATX()` liefert `"email"`; `"a@X1Zb".StripATX()` liefert
`"a@AZb"`. [Scanner](../crates/icy_board_engine/src/vm/expressions/predefined_functions.rs#L920-L970).
Das stammt aus gemeinsamem Altcode, wird aber von der neuen Member-API exponiert.
Ungültige/unvollständige Farbcodes sollten nicht still Text verändern.
Vor Änderung des klassischen Aufrufs dessen Kompatibilitätsvertrag prüfen;
keine Oracle-Verifikation in diesem Review.

### F12: LSP verliert Arrayrang und kennt skalare Methodensignaturen nicht

Mit echten Sprachserver-Anfragen verifiziert:

- `rx.FindAll(` zeigt Rückgabe `RegexMatch` statt `RegexMatch[]`.
- `STRING parts[]; parts.` bietet `Contains` an, ebenso
  `text.Split(",").`, obwohl beide Arrays sind.
- `text.Contains(`, `blob.GetChecksum(` und `STRING.Repeat(` haben keine
  Signaturhilfe.
- Passende qualifizierte Enumargumente wie `Checksum.SHA256` und
  `StringComparison.Ordinal` werden an diesen Aufrufpositionen nicht angeboten.

[Signaturen](../crates/ppl-lsp/src/signature_help.rs#L200-L223),
[Rangauflösung](../crates/ppl-lsp/src/type_lookup.rs#L162-L182),
[Argumentvervollständigung](../crates/ppl-lsp/src/completion.rs#L296-L325).
Elementtyp, Rang und Aufrufparameter sollten aus derselben typisierten
Beschreibung stammen wie die Compilerprüfung, nicht aus weiteren Einzelfällen.

## Vertrags- und Dokumentationspflege

- `Contains("a")` lässt einen vorherigen Fehler stehen;
  `Contains("a", StringComparison.Ordinal)` löscht ihn. Verifiziert mit vorherigem
  `Base64Dec("!")`: gleiche erfolgreiche Suche, aber unterschiedliches
  `Error.Last().OK`. Fehlerlebensdauer bewusst festlegen, nicht pauschal alle
  reinen Funktionen Fehler löschen lassen.
- Englische/deutsche LSP-Hilfen versprechen bei Regex-Split und
  Stringtransformationen noch `BIGSTR`, während Registry/Referenz unbeschränktes
  `STRING` nennen. Hilfetexte gegen die aktuellen Signaturen prüfen.
- Historische Review-Dokumente klar vom aktuellen offenen Backlog trennen.
- `VAR`-Copy-in/Copy-out und erneute Indexauswertung wurden beobachtet, aber
  ohne PCBoard-Oracle ausdrücklich **nicht als neuer Fehler** eingestuft.

## Empfohlene Reihenfolge

1. F1–F4: Typ-/Forminvarianten und einheitliche Konstantenwerte; zu jeder
   Korrektur positive/negative Compiler-, LSP- und serialisierte VM-Regressionen.
2. F5–F10: Rückgabewerte, Indexierung, Namensauflösung und Aufrufkonsistenz.
3. F11 mit Legacy-Kontrolle; F12 und Dokumentationspflege parallel.
4. Danach vollständige PPL-Suite und weitere Kombinationstests, statt zunächst
   zusätzliche Syntax oder weitere Helper einzuführen.