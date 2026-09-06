# PPL 4.00: Review der neuen API

Stand des ursprünglichen Reviews: **2026-09-05**. Dies ist ein API-Review,
kein Sprachreview und keine Freigabe des gesamten Runtimes. Die Befunde unten
beschreiben den Stand vor den anschließend beauftragten Korrekturen.

## Nachtrag: bestätigte Implementierungsfehler behoben

Die fünf priorisierten Befunde sind korrigiert, ohne öffentliche Signaturen
oder Sprachsyntax zu ändern:

- Regex-Kompilierungslimits bei Stringvergleichen werden als API-Fehler
  veröffentlicht; der Panic wurde auch durch eine echte PPE-Reproduktion
  abgesichert.
- `FindLast` und `EndsWith` berücksichtigen überlappende Literalvorkommen.
- `Regex.FindAll` beginnt am gewünschten Offset und erhält Anker-/Wortgrenzen-
  Kontext sowie Unicode- und Leer-Treffersemantik.
- USER-Mutationen validieren auf einer Kopie. Schlägt die Speicherung fehl,
  werden Session und In-Memory-Userbase zurückgesetzt; Methoden liefern `FALSE`
  und veröffentlichen `ErrKind.User` / `ErrCode.Io`.
- USER- und MARGINS-Mutatoren veröffentlichen Fehler und löschen ältere Fehler
  bei Erfolg; Fehler aus demselben Statement bleiben erhalten.

Dauerhafte Regressionstests stehen in
[string_members.rs](../crates/icy_board_engine/src/vm/tests/string_members.rs),
[regex.rs](../crates/icy_board_engine/src/vm/tests/regex.rs),
[user_object.rs](../crates/icy_board_engine/src/vm/tests/user_object.rs) und
[margins.rs](../crates/icy_board_engine/src/vm/tests/margins.rs).
Die ursprünglichen Fehlerproben wurden zunächst mit den Soll-Erwartungen rot
ausgeführt und nach den Fixes grün. Zusätzlich werden Dateipersistenz,
Rollback und `ON ERROR` geprüft. Die API-Designvorschläge weiter unten bleiben
separate Entscheidungen, insbesondere Umbenennungen und zusätzliche Enums.

Validierung der Korrekturen: **11 neue Regressionstests**;
**1452 Engine-Unit-Tests erfolgreich, 5 ignoriert** sowie **7 Grammar- und
2 Opcode-Tests erfolgreich** (englische Locale). Zusätzlich sind die
**652 VM-Tests mit deutscher Locale erfolgreich, 3 ignoriert**. Formatprüfung
der geänderten Rust-Dateien und `git diff --check` sind sauber. Strenges
Clippy (`-D warnings`) wird weiterhin durch Meldungen außerhalb dieser Fixes
blockiert, unter anderem in Parser, Werttypen und Door-Code; diese wurden
nicht nebenbei umgebaut. Die Zahlen im ursprünglichen Verifikationsabschnitt
unten gehören zum Review vor den Korrekturen.

## Umfang und Gesamturteil

Geprüft wurden die neuen globalen Funktionen, Objekt- und Wert-APIs,
eingebauten Typen/Enums, vier Record-I/O-Statements sowie ihre Fehler-,
Lebensdauer- und Mutabilitätsverträge. Syntax, Module, Schleifen, Deklarationen,
allgemeine Compilersemantik und das Design von `ON ERROR` bleiben für das
separate Sprachreview außen vor. Dass eine API Fehler für den vorhandenen
Handler veröffentlicht, gehört dagegen zu ihrem API-Vertrag.

**Urteil: Die Struktur beibehalten, aber noch nicht unverändert einfrieren.**
Die Zuständigkeiten und Datentypen sind überwiegend gut gewählt. Vor einem
Freeze sind insbesondere die fünf unten belegten Implementierungsbefunde
und einige kleine, später schwer korrigierbare Vertragsfragen zu behandeln.
Ein pauschales „alles GOOD / freeze“ aus älteren Reviews ist dafür zu stark.

### Tatsächlicher Umfang

Die Objekt-Registry wurde mit dem vorhandenen `dump_api`-Test ausgegeben.
Maßgeblich sind die [Typ-Registry](../crates/icy_board_engine/src/parser/type_registry.rs),
die [Funktionsdefinitionen](../crates/icy_board_engine/src/executable/func_op_codes.rs)
und die [Scalar-Member](../crates/icy_board_engine/src/semantic/members.rs).

- **25 eingebaute Objekt-/Recordtypen**, IDs 30–54; darunter der Wertrecord
  `CONTACT`, nicht 25 Ressourcen oder Klassen.
- **14 eingebaute Enums**: `EventKind`, `MouseAction`, `MouseButton`,
  `MouseMode`, `MouseTracking`, `GfxBackend`, `ErrKind`, `ErrCode`,
  `EditorMode`, `MsgField`, `HttpMethod`, `RegexOptions`, `StringComparison`,
  `Checksum`.
- **18 öffentliche globale 4.00-Signaturen**, einschließlich Overloads und
  Roots: `AreaId`, `Len(array, dimension)`, `Base64Enc`, `Base64Dec`, die beiden
  `Rgb`-Overloads, `Board`, `Session`, `Terminal`, `ToLong`, `ToULong`, `Sin`,
  `Cos`, `Tan`, `Atan`, `Log`, `Sqrt`, `ToBytes`.
- Zusätzlich die Member-APIs von `STRING`, `BYTES` und Arrays; die internen
  String-/Member-Opcodes sind keine zusätzlichen globalen Benutzerfunktionen.
- **Vier neue Dateioperationen**: `FGETREC`, `FPUTREC`, `FREADREC`, `FWRITEREC`.
- Die neuen bzw. erweiterten Wertverträge betreffen insbesondere 64-Bit
  `LONG`/`ULONG`, unbeschränktes 400-`STRING`, `BYTES`, `MSGAREAID` und den
  maskierten, nicht frei deklarierbaren Rückgabetyp `PASSWORD`.

## Priorisierte, belegte Befunde

Prioritäten: **P1** = Robustheit/Datenzuverlässigkeit vor Release beheben;
**P2** = falsches Ergebnis oder inkonsistenter öffentlicher Vertrag.

### 1. P1 — Stringvergleiche können bei gültigen Eingaben paniken

Quelle: [ignore_case_regex](../crates/icy_board_engine/src/vm/expressions/predefined_functions.rs#L582-L588).

Die Implementierung von `OrdinalIgnoreCase` baut aus dem Suchtext einen
escaped Regex und verwendet `expect("an escaped string is always a valid
regex")`. Syntaktische Gültigkeit garantiert aber nicht, dass der Regex unter
dem Kompilierungsgrößenlimit bleibt.

**Verifiziert:** Ein Literal aus 1.000.000 `a`-Zeichen liefert mit genau diesem
`RegexBuilder`-Pfad `regex::Error::CompiledTooBig`. Dieses Ergebnis erreicht in
der API das `expect` und damit einen Rust-Panic statt `Error.Last()`.
Unbeschränktes `STRING` macht solche Eingaben regulär ausdrückbar, z. B. mit
`STRING.Repeat("a", 1000000)` als Suchtext für `Contains` mit
`OrdinalIgnoreCase`. Der Grenzfall wurde am Library-Pfad geprüft, nicht als
vollständiger PPE-Absturztest.

**Empfehlung:** Den Builderfehler behandeln und als `ErrKind.String` /
`ErrCode.Limit` veröffentlichen oder eine begrenzte, nicht-regexbasierte
Literalvergleichsstrategie verwenden. Kein Panic für gültige API-Argumente.

### 2. P1 — USER-Mutationen melden trotz Persistenzfehler Erfolg

Quellen: [USER-Mutationen](../crates/icy_board_engine/src/icy_board/state/ppl_user.rs#L469-L575),
[persist_current_user](../crates/icy_board_engine/src/icy_board/state/mod.rs#L1791-L1807).

`SetPassword`, `AddContact`, `RemoveContact` und `SetNote` ändern den
Benutzerdatensatz, loggen einen fehlgeschlagenen Speichervorgang und liefern
trotzdem `TRUE`. Beschreibbare Properties verschlucken denselben Fehler.
Der Helfer verspricht ausdrücklich sofortige Persistenz.

**VM-Reproduktion:** Das Benutzerdateiziel wurde auf das vorhandene
Scratch-Verzeichnis gesetzt, sodass es nicht als Benutzerdatei geschrieben
werden kann. Nach `Error.Clear()` liefert `SetNote(0, "not persisted")`
`TRUE`; `Error.Last().OK` bleibt ebenfalls `TRUE`.

**Folge:** Ein PPE kann eine nicht dauerhaft gespeicherte Profil- oder
Passwortänderung als erfolgreich bestätigen. Session und In-Memory-Userbase
werden schon vor dem fehlgeschlagenen Speichern verändert.

**Empfehlung:** `ErrKind.User` / `ErrCode.Io` veröffentlichen und bei Methoden
`FALSE` liefern. Festlegen, ob eine Mutation bei Speicherfehler zurückgerollt
wird oder als ausdrücklich ungespeicherte Änderung bestehen bleibt. Eine
Property-Zuweisung darf den Fehler ebenfalls nicht nur loggen.

### 3. P2 — OrdinalIgnoreCase verändert die Suchlogik bei Überlappungen

Quellen: [FindLast](../crates/icy_board_engine/src/vm/expressions/predefined_functions.rs#L612-L633),
[EndsWith](../crates/icy_board_engine/src/vm/expressions/predefined_functions.rs#L665-L683).

Die insensitive Variante nimmt den letzten Treffer von `find_iter`.
Dieser Iterator liefert ausschließlich **nicht überlappende** Treffer.
Für eine Rückwärtssuche oder einen Suffixtest ist das nicht äquivalent zu
`rfind` bzw. `ends_with`.

**VM-Reproduktionen, jeweils ausschließlich Kleinbuchstaben:**

| Aufruf | Ist | Soll |
| :--- | :---: | :---: |
| `"aaa".FindLast("aa")` | 1 | 1 |
| `"aaa".FindLast("aa", 2, StringComparison.OrdinalIgnoreCase)` | 0 | 1 |
| `"aaa".EndsWith("aa")` | TRUE | TRUE |
| `"aaa".EndsWith("aa", StringComparison.OrdinalIgnoreCase)` | FALSE | TRUE |

**Empfehlung:** Suffixvergleich am Ende verankern; bei `FindLast` auch
überlappende mögliche Startpositionen berücksichtigen. Der Vergleichsmodus
darf nur die Zeichenäquivalenz ändern, nicht die Suchrichtung oder Treffermenge.

### 4. P2 — Regex.FindAll beginnt nicht wirklich an start

Quelle: [Find / FindAll](../crates/icy_board_engine/src/icy_board/state/ppl_regex.rs#L316-L349).

`Find` nutzt `captures_at(text, offset)`. `FindAll` beginnt dagegen am
Textanfang und verwirft vollständige frühere Matches mit `skip_while`.
Liegt `start` innerhalb eines solchen Matches, fehlt ein Treffer, den die
Suche ab `start` finden würde.

**VM-Reproduktion:** Für `REGEX.Compile("a+")` liefert `Find("aaa", 1)`
den Wert `"aa"` ab Position 1, aber `FindAll("aaa", 1)` ein leeres Array.
Dies ist kein UTF-8-Byte/Zeichen-Umrechnungsfehler, sondern ein anderer
Suchbeginn.

**Empfehlung:** Ab dem gewünschten Offset iterieren und dabei den Kontext
des vollständigen Textes für Anker und Wortgrenzen erhalten. Einfach den
Text abzuschneiden würde z. B. die Bedeutung von `^` verändern.

### 5. P2 — Einige Mutatoren verletzen den Error.Last-Vertrag

Quellen: [USER](../crates/icy_board_engine/src/icy_board/state/ppl_user.rs#L480-L575),
[MARGINS](../crates/icy_board_engine/src/icy_board/state/ppl_margins.rs#L100-L123),
[Margin-Helfer](../crates/icy_board_engine/src/vm/statements/predefined_procedures.rs#L1001-L1028),
[zentraler Erfolgsvertrag](../crates/icy_board_engine/src/vm/error_handling.rs#L33-L38).

**VM-verifiziert:**

- `SetNote(5, "bad")` liefert `FALSE`, aber nach `Error.Clear()` bleibt
  `Error.Last().OK = TRUE`.
- Nach einem Regex-Kompilierungsfehler und anschließend erfolgreichem
  `SetNote(0, "good")` bleibt der alte Regex-Fehler stehen.
- `Terminal.Margins.SetVertical(0, 10)` liefert `FALSE`, veröffentlicht
  jedoch ebenfalls keinen Fehler.

Weitere USER-Pfade verweigern z. B. leere Kontaktangaben oder einen ungültigen
Kontaktindex ebenso ohne neue Fehlermeldung. Das ist nicht mit dem eigenen
Verhalten bei schreibgeschützten USER-Snapshots oder Kontaktlimits konsistent.
Die vorhandenen Tests prüfen teilweise bereits die boolesche Ablehnung,
aber nicht deren Fehlerregister-Vertrag.

**Empfehlung:** Ungültige Mutationsargumente erhalten `ErrCode.Invalid`;
erfolgreiche fallible Operationen rufen `operation_succeeded()` auf. Erwartetes
Nichtfinden bei einer Suche ist davon zu unterscheiden. Es geht nicht darum,
jede billige Property-Abfrage zu einer fehlerlöschenden Operation zu machen.

## API-Design nach Bereichen

| Bereich | Urteil | Empfehlung vor Freeze |
| :--- | :--- | :--- |
| Globale Funktionen | Beibehalten | Kleine Utility-Schicht statt zweitem großen globalen Namespace. `Rgb`-Overloads und `ToLong`/`ToULong` sind verständlich. Wertebereiche, Clamping, NaN/Domain-Fehler und Encoding gehören in jeden Vertrag. |
| `STRING`, `BYTES`, `MSGAREAID`, `PASSWORD` | Gute Aufteilung | Text, binäre Daten, zusammengesetzte Message-Adresse und maskiertes Passwort nicht zusammenwerfen. `Checksum` mit `CRC32`, `MD5`, `SHA256` ist klar; MD5 nicht als Sicherheitsgarantie beschreiben. Suchbefunde oben beheben. |
| Array-Member | Beibehalten | `Len` ist eine Anzahl, kein höchster Index. Kein zusätzliches gleichbedeutendes `Count`. Das Deklarations-/Iterationsdesign wird separat bewertet. |
| `BOARD`, `CONFERENCE`, `AREA`, `DIRECTORY`, `DOOR`, `MSG` | Schlüssige Navigation | Collections und `Valid` beibehalten. `LowMsg`, `HighMsg`, `Read`, `Find`, `Text` bleiben Methoden, weil sie I/O machen. Zugriffsvertrag ausdrücklich festlegen, siehe unten. |
| `SESSION`, `USER`, `CONTACT` | Gute fachliche Trennung, fehleranfällige Mutation | Aktuelle Sessionwerte und gespeicherte Benutzerwerte nicht verschmelzen. `CONTACT` als offener Wertrecord ist besser als ein festes Netzwerk-Enum. Persistenz- und Fehlervertrag reparieren. |
| `TERMINAL`, `TERMINFO`, `TERMINPUT`, `EVENT` | Gute Bündelung | Fähigkeiten unter `Info`, Steuerung unter `Input`. Keyboard-Parameter klären; Eventfelder pro `Kind` dokumentieren, keine weiteren bedeutungswechselnden Universal-Felder. |
| `GFX`, `SURFACE`, `AUDIO`, `MARGINS`, `PALETTE`, `MACROS` | Grundstruktur beibehalten | Ressourcen-Aliasing und Freigabe ausdrücklich beschreiben; Geometrieeinheiten und optionale Argumente vereinheitlichen, siehe unten. |
| `HTTP`, `HTTPREQUEST`, `HTTPRESPONSE` | Gut komponierbar | Requestzustand, Transporterfolg (`Valid`) und HTTP-Erfolg (`OK`) sind sinnvoll getrennt. Query-/Form-Mutation und Mehrfachheader präzisieren. |
| `REGEX`, `REGEXMATCH` | Sinnvolle wiederverwendbare Pattern-/Ergebnistypen | `Success` für ein Match und `Valid` für ein Pattern sind kein unnötiger Widerspruch. Start- und Ressourcenverträge absichern. |
| `ERROR`, `ErrKind`, `ErrCode` | Beibehalten, konsequent implementieren | Sektor, portable Fehlerkategorie, Meldung und Kanal reichen. Nicht noch einen zweiten Fehlerkanal oder sektorspezifische Zahlencodes einführen. |
| Record-I/O | Beibehalten | Vier Operationen passen zum Channel-Modell. Transaktionale Reads beziehen sich auf den Zielwert, nicht auf Zurücksetzen der Dateiposition. Framelimit und Formatversion klar halten. |

## Noch offene bzw. zu präzisierende Designentscheidungen

### A. Zugriffsprüfung ist derzeit eine Frage an das PPE, keine automatische Schranke

`AREA.Read`/`Find` und `MSG.Text` lesen die Messagebase ohne automatische
Prüfung von `HasAccess`, privaten Empfängern oder Nachrichtenpasswörtern.
`Board.Users` ist ebenfalls kein auf den aktuellen Anrufer eingeschränkter
Adressbuchblick. Siehe [MessageArea](../crates/icy_board_engine/src/icy_board/message_area.rs#L93-L184),
[Msg.Text](../crates/icy_board_engine/src/icy_board/state/ppl_message.rs) und
[Board-Snapshot](../crates/icy_board_engine/src/icy_board/state/ppl_board.rs#L37-L48).

Das ist **nicht allein ein Beleg für einen Sandbox-Ausbruch**: PPEs besitzen
bereits privilegierte Datei-/Benutzeroperationen. Die entscheidende API-Frage
ist, ob diese Objekte privilegierte Werkzeuge oder automatisch anrufergefilterte
Ansichten sein sollen. Empfehlung: den privilegierten Charakter explizit
dokumentieren und sichere Listing-/Lese-Beispiele zeigen; nicht stillschweigend
einen neuen Berechtigungsmechanismus hineininterpretieren. Einzelne
Bereitschafts-/Zugriffsabfragen ersetzen keinen vollständigen Lesefilter.

### B. Ein USER-Typ mit zwei Mutabilitäten ist akzeptabel, aber sichtbar zu machen

`Session.User` ist live und selektiv schreibbar; `Board.Users[i]` verwendet
denselben Typ, verweigert Mutationen aber zur Laufzeit. Das spart doppelte
Typen, ist aus der Signatur allein jedoch nicht erkennbar.

Empfehlung: diese Trennung behalten, in Hover/Referenz deutlich ausweisen und
bei Ablehnung konsistente Fehler liefern. Kein zweiter USER-Typ nur für den
Namen, solange readonly-Qualifikation nicht ohnehin vorgesehen ist.
`Board` erzeugt beim ersten Zugriff auch den vollständigen Benutzersnapshot;
selbst `Board.Name` trägt damit dessen Initialisierungskosten. Spätere
Array-Clones teilen dagegen Arc-Speicher; sie sind keine erneuten vollständigen
Userbase-Kopien. Lazy-Erzeugung ist erwägenswert, würde aber den dokumentierten
Snapshot-Zeitpunkt verändern und benötigt eine bewusste Entscheidung.

### C. Terminalparameter vor dem Freeze bereinigen

- `KeyboardOn([echo])` heißt in der
  [Registry](../crates/icy_board_engine/src/icy_board/state/ppl_terminal_input.rs#L46)
  `echo`; die [Implementierung](../crates/icy_board_engine/src/icy_board/state/mod.rs#L1360-L1369)
  interpretiert das Argument als `suppress`. Positive/negative Bedeutung,
  Default und tatsächlich unterdrückter Eingabestrom müssen identisch
  beschrieben werden. Das Review hat kein physisches Terminalverhalten getestet.
- `Surface.PresentRect` besitzt bis zu neun Positionsargumente und einen
  ganzzahligen `flip`-Bitwert. Empfehlung: zumindest einen typisierten
  Flip-Flags-Enum statt magischer `0/1/2/3`; ein Rect-/Options-Record lohnt sich
  erst, wenn er mehrere Aufrufe tatsächlich vereinfacht.
- Pixelkoordinaten, 1-basierte Terminalpositionen und Millisekunden in
  Parameternamen/Referenz sichtbar halten. Nicht pauschal alle Koordinaten auf
  nullbasiert umstellen.
- Kopieren eines Ressourcenobjekts ist keine Pixel-/Audiodatenkopie.
  `Free()` und das Verhalten weiterer Aliase müssen explizit beschrieben und
  getestet sein. Gemeinsame Handles sind nicht an sich ein Fehler.

### D. HTTP-Mutatoren und Mehrfachwerte

[SetQuery](../crates/icy_board_engine/src/icy_board/state/ppl_http.rs#L109-L120)
ersetzt gleichnamige Parameter, `SetHeader` ersetzt einen Header,
[SetForm](../crates/icy_board_engine/src/icy_board/state/ppl_http.rs#L899-L925)
hängt dagegen ein weiteres gleichnamiges Paar an. Das ist bei HTML-Formularen
fachlich zulässig, aber unter drei `Set*`-Namen überraschend.

Empfehlung: `SetForm` ebenfalls ersetzend definieren und bei Bedarf ein
explizites `AddForm` anbieten; alternativ den aktuellen Namen vor Freeze zu
`AddForm` ändern. Wiederholte Query-Parameter und Response-Header sind eine
bewusste Umfangsentscheidung: `Header(name) -> STRING` kann z. B. mehrere
`Set-Cookie`-Werte nicht verlustfrei modellieren. Keine Cookie-Jar-, Streaming-
oder Async-API ohne tatsächlichen Anwendungsfall hinzufügen.

`Http.New` prüft URLs erst beim Senden und `HTTPREQUEST` hat kein `Valid`.
Das ist als Builder-Modell vertretbar, solange nicht für jeden Konstruktor
pauschal ein sofort nutzbares `Valid` versprochen wird. Shared-Mutation der
Request-Aliase und die Trennung `Valid`/`OK` ausdrücklich beibehalten.

### E. Größen- und Laufzeitverträge statt scheinbarer Garantien

`Regex.FindAll` begrenzt die Trefferanzahl; `Regex.Replace` prüft 16 MiB
erst **nach** Erzeugung des vollständigen Ergebnisses; `Regex.Split` sammelt
ohne vergleichbaren Elementdeckel. Siehe
[Regex-Ausgabeoperationen](../crates/icy_board_engine/src/icy_board/state/ppl_regex.rs#L329-L419).
Das sind unterschiedliche Verträge, kein einheitlicher Speicherschutz.
Bei Regex-Ersetzungen kann ein kleines Input-/Replacement-Paar ein sehr großes
Ergebnis erzeugen, bevor die nachträgliche Ablehnung greift.

Empfehlung: Ergebnisbudgets während des Aufbaus kontrollieren und pro Operation
dokumentieren; keine OOM-Proben auf dem Arbeitsrechner ausführen.
Die pauschale „linear-time matching“-Aussage in der
[Referenz](new_ppl.md#regular-expressions) sollte nicht als Garantie für
`FindAll` inklusive wiederholter Suche, Capture-Kopien und Zeichenpositions-
Berechnung formuliert werden.

## Verifikation und Grenzen

- Registry-Dump: **1 Test erfolgreich**; Inventar daraus abgeglichen.
- VM-Suite: **647 erfolgreich, 3 ignoriert**, einschließlich sechs temporärer
  Review-Proben. Ohne diese Proben sind es **641 bestehende erfolgreiche Tests**.
- Editor-Grammatiken: **7 erfolgreich**; Opcode-Coverage: **2 erfolgreich**.
- Die sechs Charakterisierungsproben prüfen die beobachteten Fehler, nicht
  bereits korrigiertes Sollverhalten: fünf Compiler-/VM-Reproduktionen und ein
  begrenzter RegexBuilder-Test. Nach der Prüfung wurden sie wieder entfernt;
  der produktive Code und die bestehende Testsuite bleiben unverändert.
- Bei Behebung sollten daraus dauerhafte Regressionstests mit den korrigierten
  Erwartungen entstehen, insbesondere für Überlappungen, Start innerhalb eines
  Matches, Schreibfehler und Fehlerstatus nach Erfolg/Ablehnung.
- Kein vollständiger Workspace-Test, kein Penetrationstest, keine realen
  Terminal-/Audio-/Grafiktests und kein allgemeines Sprachreview.
- Nicht bestätigt und deshalb nicht als Fehler aufgenommen: vermeintliche
  partielle Zieländerung von `FGETREC` bei Dekodierfehler. Der Decoder baut einen
  neuen Wert; nur `Ok(value)` wird zugewiesen. Eine weitere Zeile nach einem
  vollständigen Record kann einfach zum nächsten Record gehören.

## Empfohlene Reihenfolge

1. Panic-Pfad und verschluckte Persistenzfehler beheben.
2. Suchsemantik und Mutatoren-Fehlerstatus konsistent machen.
3. Kleine API-Entscheidungen treffen: Keyboard-Parameter, Form-Set/Add,
   Flip-Flags, Grenzen und Zugriffsverantwortung.
4. Referenz/Editorinformationen aus der Registry überprüfen. Insbesondere die
   alte Behauptung, `Regex.Split` ändere ein Zielarray transaktional, passt nicht
   mehr zur tatsächlichen Rückgabe `STRING[]`.
5. Danach diesen API-Umfang einfrieren; Sprachreview separat durchführen.