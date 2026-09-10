# PPL 400: Plan bis zum Ende der Beta

Stand: 2026-09-10. Zieltermin: ungefähr 2026-10-09.

## Ziel und Arbeitsweise

PPL 400 soll eine stabile, erweiterbare Grundlage für PPEs werden. Moderne
Anwendungen sollen BBS-Dienste nutzen können, ohne interne Konfigurationen oder
Datenformate selbst nachzubauen. Terminalanwendungen sollen mit SyncTERM und
icy_term arbeiten und bei einfacheren Terminals kontrolliert zurückfallen.

Dieser Plan ist **keine pauschale Implementierungsfreigabe**.

- **Vor jedem Umsetzungsschritt einzeln besprechen:** Problem, Alternativen,
  Empfehlung, Kompatibilitätsfolgen, Aufwand und konkrete Abnahmekriterien.
- Erst nach ausdrücklicher Freigabe diesen Schritt implementieren. Die Freigabe
  eines Schritts gilt nicht automatisch für den nächsten.
- Nach Umsetzung Ergebnisse und tatsächlich ausgeführte Tests zusammenfassen;
  dann den nächsten Schritt besprechen.
- F1–F6 sind verbindliche Arbeitspakete. Ihre genaue Lösung wird jeweils besprochen.
- Die Sprachnachschärfungen S1–S6 werden einzeln entschieden und die freigegebenen
  Änderungen umgesetzt, **bevor der PPE-400-Container besprochen und geändert wird**.
- Am 2026-09-10 ausdrücklich geänderte Reihenfolge: **C1 → C2 → S7**.
  S7 führt anschließend Sprach- und Formatentscheidungen zusammen. Die separate
  Freigabe jedes Schritts bleibt erforderlich; frühere Reihenfolgen in den
  historischen Abschlussprotokollen sind damit überholt.
- Keine vollständige VM-Neuentwicklung als Vorannahme. Ein neues Containerformat
  ist eine zu begründende Entscheidung, kein bereits beschlossener Selbstzweck.
- Bestehende PCBoard-PPEs behalten ihren Format- und Semantikvertrag. Unveröffentlichte
  Beta-400-PPEs dürfen nach ausdrücklicher Entscheidung neu kompiliert werden müssen.
- Fremde und bereits vorhandene Änderungen erhalten; keine ungefragten Commits.

## Ausgangslage und Evidenz

Das vorausgehende Review fand während einer laufenden Auslagerung des PPL-Kerns
nach `icy_board_ppl` statt. Die API-Validierung scheiterte vor der Testausführung
am Build. Die unten genannten Implementierungsbefunde wurden aus Codepfaden
abgeleitet; sie sind nicht als erfolgreich ausgeführte Reproduktionen zu lesen.

Vor der jeweiligen Korrektur den aktuellen Stand erneut prüfen, den Befund
reproduzieren und möglichst einen zuerst fehlschlagenden Regressionstest ergänzen.
Historische Testberichte ersetzen keine Validierung des aktuellen Arbeitsstands.

**Aktualisierung 2026-09-09:** P0 hat auf `abd2b08e827ba43e234c2beb605a2fe7ef8162c8`
einen erfolgreichen Build und eine neue Testbaseline hergestellt. Die damalige
Buildblockade besteht für den geprüften Umfang nicht mehr. Ergebnisse und weiterhin
offene Reproduktionen stehen im [P0-Protokoll](#p0--2026-09-09).

## Verbindlicher Umfang: F1–F6

| ID | Befund / Risiko | Verbindliches Ergebnis | Einordnung |
| --- | --- | --- | --- |
| F1 | Unicode-Literale wurden über den klassischen CP437-Konstantenpfad gespeichert; nicht darstellbare Zeichen konnten verändert werden. | Nicht-CP437-Text über Quelle, Compiler, PPE-Datei, Loader und Ausführung verlustfrei erhalten; Legacy-Kodierung bewahren. | UTF-8-Literalteil aus C2 ausdrücklich nach S5 vorgezogen; Datei- und Terminal-Roundtrips bestanden. |
| F2 | Aktuell knapp 32 KiB erzeugter Code sowie kleine IDs, Offsets und Routinedeskriptoren. | Alle relevanten Grenzen inventarisieren, Zielgrößen beschließen und das freigegebene Größenkonzept einschließlich Prüfungen umsetzen. | Erst nach den Sprachschritten: C1/C2. |
| F3 | Gespeicherte geschlossene Enum-Domains und kompakte Host-Typ-IDs erschweren API-Erweiterungen. | Vertrag für eigene und Host-Enums sowie dauerhafte Host-Typidentität festlegen; alte PPEs gegen neue API-Werte testen. | Sprachentscheidung S4; eventuelle Formatanteile C1/C2; API-Vertrag A1. |
| F4 | Alte Audio- und Surface-Handles können nach Slot-Wiederverwendung beziehungsweise Grafik-Neustart neue Ressourcen adressieren. | Freigegebene Handles bleiben ungültig; Aliase lebender Ressourcen bleiben kontrolliert nutzbar. | Ressourcenvertrag S1, Korrektur R1. |
| F5 | `Audio.Fade`-Signatur nennt Dauer/Lautstärke, Implementierung und Beispiel verwenden Lautstärke/Dauer. | Genau eine Argumentreihenfolge in API-Katalog, Runtime, LSP, Dokumentation und Ausgabe-Test. | R2. |
| F6 | Ein einfacher Board-Zugriff erstellt auch einen vollständigen Benutzer-Snapshot. | Metadatenzugriff nicht an das Kopieren der gesamten Userbase koppeln; Snapshot-Zeitpunkt und Query-Verhalten ausdrücklich erhalten oder neu festlegen. | A3. |

F3 beschreibt ein Evolutionsrisiko, nicht die Behauptung, dass jede neue
Builtin-Enum schon heute zwangsläufig jedes alte PPE beschädigt. Konkrete
Kompatibilitätsszenarien sind Bestandteil der Abnahme.

## Reihenfolge und Freigabepunkte

### P0 — Belastbaren Ausgangsstand herstellen

Status: abgeschlossen am 2026-09-09; siehe [P0-Protokoll](#p0--2026-09-09).

**Besprechen:** Zustand der laufenden Core-Auslagerung, Zuständigkeit für offene
Buildfehler und ein stabiler Review-/Teststand. Nicht parallel dieselben Dateien
ungeplant umbauen.

**Arbeit:**

- Build und relevante Compiler-/VM-/LSP-Tests auf einem festgehaltenen Stand prüfen.
- Vorhandene F1–F6-Tests und bereits erfolgte Korrekturen inventarisieren.
- Reproduktionsfälle und Baseline festhalten, ohne daraus vorzeitig Lösungen abzuleiten.

**Abnahme:** Reproduzierbarer Build oder ausdrücklich dokumentierte externe
Blockade; klare Trennung von vorhandenen Fehlern und neuen Regressionen.

### S1 — Records, Collections und Ressourcen komponierbar machen

Status: Sprach-/Runtime-Umsetzung am 2026-09-09 abgeschlossen: Alternative 2
einschließlich vorgezogenem R1/F4. Neue Layouts sind noch nicht als PPE-Datei
speicherbar; diese Abnahme bleibt C2. Rekursive Typen sind ausgeschlossen.

**Besprechen:**

- Welche Host-Objekte und Ressourcen dürfen Record-Felder sein?
- Sollen dynamische Arrays in Records zulässig sein?
- Welche Inhalte haben Wertsemantik, welche teilen eine Ressource?
- Wie funktionieren Gleichheit, Default-Werte und verschachtelte Kopien?
- Serialisierbarkeit getrennt von der allgemeinen Nutzbarkeit eines Records behandeln.
- Welche Teilmenge ist für 400 nötig, was darf ausdrücklich später folgen?

**Leitbeispiele:** Sprite mit Surface, Menüeintrag mit Area, Widget mit dynamischer
Kindliste. Nicht zwangsläufig alle drei Modelle sofort implementieren.

**Abnahme:** Freigegebene Modelle lassen sich ohne parallele globale Hilfsarrays
ausdrücken; Kopier-/Aliasverhalten ist getestet. Nicht serialisierbare Inhalte
werden bei Record-I/O eindeutig zurückgewiesen. Ein eigener Record darf keine
bereits freigegebene Ressource wiederbeleben.

**Formatgrenze:** Falls die beschlossene Sprache neue Layoutinformationen braucht,
zunächst Parser, Semantik, interne Darstellung und Tests umsetzen. Die endgültige
On-Disk-Kodierung bleibt bis C1 offen; keine provisorische Kodierung veröffentlichen.

### S2 — Kurzschließende logische Auswertung

Status: Sprach-/Runtime-Umsetzung am 2026-09-09 einzeln freigegeben,
implementiert und in EN/DE abgenommen. Die neue Dateikodierung bleibt C1/C2
vorbehalten; S3 wurde anschließend separat freigegeben.

**Entscheidung:** Keine neuen Schlüsselwörter `ANDALSO`/`ORELSE`. Ab Sprache
400 schließen `&&`/`||` kurz; `&`/`|` werten weiterhin beide Seiten aus. Vor
400 bleiben die doppelten Zeichen Aliase der einfachen, auch bei Zielruntime
400. Neue Kurzschlussausdrücke benötigen mindestens Runtime 400, liefern
BOOLEAN und behalten skalare Wahrheitswertkonvertierungen. Enum-Bitoperationen
bleiben bei `&`/`|`. Übersprungene Operanden werden weiterhin semantisch geprüft.

**Historischer Befund:** 20 Proben wurden mit originalem PPLC 3.40 kompiliert
und auf einer isolierten PCBoard-15.4/M-Kopie ausgeführt. Alle vier Zeichenformen
werteten beide Seiten aus, auch bei konstantem linken Operanden; Funktionsspuren
waren links vor rechts. Originalquellen und Laufzeit bestätigen Vergleich vor
`!`, UND vor ODER. Die Originalquellen bestätigen zudem unäre Vorzeichen vor
Potenz und linksassoziative binäre Operatoren. Temporäre Originalausgaben:
`target/s2-legacy-oracle/run-204z3_m3/logic.out`; Live-Installation unverändert.

**Präzedenzkorrektur:** Für alle Sprachversionen gilt Original-PPLC-Präzedenz:
unäres Vorzeichen, Potenz, Multiplikation, Addition, Vergleich, NOT, AND, OR.
Binäre Operatoren gleicher Stufe sind linksassoziativ. Die bisher gleichrangigen
AND/OR und das zu stark bindende NOT waren Fehler, keine beizubehaltende Variante.
Parser, Decompiler-Klammerung und Tree-sitter verwenden nun diesen Vertrag.

**Formatgrenze:** Kurzschlussprogramme behalten intern den Scriptbaum. Die
PPE-Dateiausgabe wird eindeutig abgelehnt; alte Opcodes und Loadersemantik bleiben
unverändert. Datei- und Containerabnahme dieser Ausdrücke folgt erst nach C1.

**Abnahme:** Seiteneffekte beweisen, wann der rechte Operand ausgeführt wird und
wann nicht. Compileroptimierung, VM, Decompiler, Formatter und LSP stimmen überein.

**Nachweise 2026-09-09:**

- Sechs Engine-S2-Tests bestehen: 96 Kombinationen aus Operator, Wahrheitswerten,
  Sprach-/Runtime-Ziel und Optimierung; Originalpräzedenz, Argumente, Schleifen,
  Indizes, Negation, Fehlerzustand und semantische Prüfung übersprungener Operanden.
- Decompile/Recompile im rohen und rekonstruierten Modus erhält Seiteneffekte;
  binäre Operatorpaare behalten ihre Bäume. Compiler- und LSP-Formatter erhalten
  Schreibweisen und Klammern. Präprozessor berücksichtigt die tatsächliche
  Sprachversion. Neue CONST-Ausdrücke prüfen auch übersprungene Operanden.
- `CARGO_INCREMENTAL=0 cargo test-low -p icy_board_engine -p icy_board_ppl -p pplc -p ppld -p ppl-lsp --no-fail-fast --quiet`:
  EN und DE jeweils **2853 bestanden, 0 fehlgeschlagen, 6 ignoriert**;
  getrennte Locale-Prozesse, gefilterte Kindprozess-Tests nicht doppelt gezählt.
- Tree-sitter: `cargo test-low --test repository_sources` jeweils **3/3** in
  EN und DE, `tree-sitter test` **34/34** auf dem endgültig generierten Parser.
- All-Targets-Check der fünf Rust-Crates sowie Engine ohne Default-Features
  bestehen mit `-j4`; Formatierungsprüfung der berührten Rust-Dateien besteht.
- Neue Kurzschlussprogramme werden an der PPE-Dateigrenze ausdrücklich
  zurückgewiesen. Legacy-Programme werden in den S2-Tests gespeichert, geladen
  und ausgeführt. Die neue Kurzschluss-Dateikodierung ist noch nicht abgenommen.

### S3 — Array- und `VAR`-Verträge schärfen

Status: am 2026-09-09 einzeln freigegeben, einschließlich Alias-Warnungen.
Implementiert und in EN/DE abgenommen. S4 ist nicht freigegeben.

**Vertrag:** Copy-in/copy-out bleibt erhalten, keine Referenzsemantik.
Argumente werden links nach rechts eingelesen, VAR-Ziele samt Indizes dabei
einmal gebunden. Rückschreibung erfolgt in umgekehrter Parameterreihenfolge.
Bei identischen Zielen gewinnt der erste Parameter. Sprache 400 warnt bei
statisch nachweisbaren Überlappungen; dynamische Indizes werden nicht geraten.
Compiler und LSP verwenden dieselbe Prüfung, Editorcode `ppl.var-alias`, EN/DE.

**Originalnachweis:** Zwei eigene Fixtures wurden mit PPLC 3.40 und PCBoard
15.4/M ausgeführt. Fünf skalare/indexierte Fälle bestätigen einmalige Bindung,
Alias-Reihenfolge und klassische Rekursion; eine Arrayrekursion bestätigt
persistente Tails und Copy-out vor Frame-Wiederherstellung. Captures:
`target/s3-legacy-oracle/run-8rr6bh4i`, `run-d_fi7w19`; Live-Dateien unverändert.
Quellen und Messwerte stehen im [DECLARE-Audit](../compat/DECLARE_AUDIT.md).

**Runtime-Grenze:** Klassische PPE-Runtimes schreiben vor der Frame-Restauration
zurück, wie das Original. Runtime 400 behält ihre bisherige rekursionsfeste
Rückgabe nach Wiederherstellung bei, auch für Legacy-Sprachquellen. Die bisherige
doppelte Indexauswertung war eine Abweichung vom Original und ist überall korrigiert.

**Arrays:** Deklarierte Bounds normaler Variablen und Parameter sind Anfangsgrößen;
feste Recordfelder behalten ihre Form, dynamische Felder dürfen Bounds ändern.
Bounds sind nullbasierte Obergrenzen; `Len` zählt Elemente, `REDIM ..., 0` erzeugt
ein Element. REDIM setzt Inhalte auf Defaults zurück. Leere Werte und Rückgaben
behalten Elementtyp und Rank 1–3. Rückschreibziele behalten ihre Formprüfung.

**Schreibweise:** `[]` ist kanonisch. Der Formatter erhält bestehende Klammern;
semantische Migration bleibt eine separate Editoraktion. Ein unerwünschtes
Leerzeichen nach `[` bei Recordzuweisungen wurde in beiden Backends korrigiert.
Decompiler-Roundtrips erhalten Arrayrückgaben und VAR-Zielbindung.

**Abnahme:** Rekursion, Alias-Argumente, Resize, leere Rückgaben und feste
Record-Formen sind durch Compiler-/VM-Tests abgesichert. Dokumentation benutzt
einheitliche Begriffe; keine unbeschlossene Umstellung auf Referenzsemantik.

**Nachweise 2026-09-09:**

- Sechs Engine-S3-Tests bestehen: Originalfixtures mit Sprach-/Runtime-Zielen
  und Optimierung, verschachtelte Recordpfade, überlappende Array-/Elementziele,
  leere Rückgaben und REDIM für Rank 1–3 sowie roher/rekonstruierter Decompiler.
- Zwei Core-Tests prüfen Alias-Warnungen, einschließlich Konstantindizes,
  Recordpfaden, Callback-Aufrufen, Wertparametern und der Sprachversionsgrenze.
- LSP veröffentlicht Warnung, Code und exakten Quellbereich in getrennten
  EN-/DE-Serverprozessen. Beide Formatter erzeugen identische, idempotente
  Arraynotation und erhalten echte Funktionsaufrufe sowie Legacy-Klammern.
- `CARGO_INCREMENTAL=0 cargo test-low -p icy_board_engine -p icy_board_ppl -p pplc -p ppld -p ppl-lsp --no-fail-fast --quiet`:
  EN und DE jeweils **2863 bestanden, 0 fehlgeschlagen, 6 ignoriert**;
  gefilterte Kindprozess-Tests nicht doppelt gezählt.
- All-Targets-Check der fünf Crates und Engine ohne Default-Features mit `-j4`,
  Formatierungsprüfung der berührten Rust-Dateien und `git diff --check` bestehen.
- S1/S2-Dateiformatgrenzen bleiben unverändert C1/C2 vorbehalten. S3 führt keine
  neue PPE-Kodierung ein; gewöhnliche Programme werden gespeichert und geladen.

### S4 — Einheitlich offene nominale Enums (F3)

Status: am 2026-09-09 ausdrücklich freigegeben, umgesetzt und in EN/DE
validiert. Die frühere Variante mit geschlossenen eigenen
Enums wurde zugunsten eines einheitlichen Vertrags verworfen.

**Beschlossener Vertrag:**

- Eigene und Host-Enums sind ab ihrer Einführung in Sprachversion 350 offen:
  jeder vorzeichenbehaftete 32-Bit-Integerwert ist zulässig. Benannte Mitglieder
  sind keine abgeschlossene Wertemenge; das erste Mitglied bleibt der Default.
- Nominale Typprüfung bleibt erhalten: keine impliziten Integerkonvertierungen,
  keine Vermischung verschiedener Enumtypen. `EnumName(integer)` und
  `TOINTEGER(value)` konvertieren ausdrücklich; Arithmetik, unäre numerische
  Operationen und numerische FOR-Zähler bleiben für Enums verboten.
- Alle Enums unterstützen `|`, `&`, `|=`, `&=` und `.Has(mask)`. Ein separates
  Flags-Konzept ist nicht nötig. Unbenannte Kombinationen, Null und unbekannte
  Bits bleiben erhalten; `.Has(Nullmaske)` ergibt TRUE. Operanden werden einmal
  von links nach rechts ausgewertet.
- Unbekannte Werte überstehen Zuweisung, Arrays, Records, Parameter, Ergebnisse,
  Gleichheitsvergleich und Fallunterscheidung. Record-I/O erhält die Zahlen;
  fehlerhafte Integer und beschädigte Frames werden weiterhin atomar abgewiesen.
- Hostoperationen prüfen ihre tatsächlich unterstützten Eingaben vor
  Seiteneffekten. Unbekannte Regexbits, Stringvergleiche, Checksummen,
  Nachrichtenfelder, Maus- und Editormodi melden `Invalid`; HTTP-Methoden und
  Grafik-Backends melden `Unsupported`. Ungültige Grafikinitialisierung und
  Editormodi erhalten bestehende Ressourcen beziehungsweise Benutzereinstellungen.
- Keine neue PPE-Kodierung. Die bisherige geordnete Enum-Metadatenliste bleibt
  erhalten, beschränkt aber nicht mehr den Wertebereich. Alte Beta-Runtimes
  können unbekannte Werte weiterhin ablehnen: die aktualisierte Runtime ist
  erforderlich. Der frühere Domainfehler ist kein verlässlicher Kontrollfluss mehr.

**Fokussierte Abnahme:**

- Mit dem heutigen Katalog kompiliertes und geladenes PPE erhält injizierte
  zukünftige Event-/Fehlerobjekte. Unbekannte positive und negative Werte erreichen
  über Host-Properties, Arrays, Records und Routinen unverändert `CASE ELSE`.
- Eigene Enumkonstanten, Casts, Bitmasken und `.Has(...)` in 350/400; Compiler und
  direkte LSP-Semantik stimmen überein. Nominale Fehlertests bleiben bestehen.
- PPE-/Decompiler-/Recompiler-Roundtrips erhalten unbenannte Werte. Optimierte
  und unoptimierte Ausführung liefern gleiche Werte und Operandenreihenfolge.
- EN-/DE-Hilfetexte mit unabhängigen Locale-Loadern geprüft. Record-I/O prüft
  Werterhalt an allen verschachtelten Blättern sowie atomare Formatfehler.

**Gesamtprüfung:**

- `CARGO_INCREMENTAL=0 cargo test-low -p icy_board_engine -p icy_board_ppl -p pplc -p ppld -p ppl-lsp --no-fail-fast --quiet`:
  EN und DE in getrennten Prozessen jeweils **2871 bestanden, 0 fehlgeschlagen,
  6 ignoriert**; gefilterte Kindprozess-Tests nicht doppelt gezählt.
- All-Targets-Check der fünf Crates und Engine ohne Default-Features mit `-j4`
  bestanden. Formatierungsprüfung, Editor-Diagnosen der produktiven Änderungen
  und `git diff --check` ohne Befund.

**Offene Dateiformatabnahme:** Host-Typidentität ist ein stabiler API-Vertrag,
nicht die aktuelle Werteliste oder eine kompakte dateilokale Typnummer. Die
bisherige Decompiler-Erkennung über ID und Metadatenliste ist noch keine dauerhafte
Zuordnung. Diese Repräsentation und Tests mit tatsächlich unterschiedlichen
Katalog-/Typ-ID-Belegungen bleiben C1/C2; F3 ist damit noch nicht vollständig
geschlossen. S5 wurde anschließend separat besprochen und freigegeben.

### S5 — Text-, Binär- und Positionsverträge (F1)

Status: am 2026-09-09 ausdrücklich freigegeben und implementiert, einschließlich
des vorgezogenen UTF-8-Literalteils aus C2. Fokussierte Abnahmen und vollständige
EN/DE-Gesamtläufe bestanden.

**Beschlossener Vertrag:**

- Textliterale und Strings sind Unicode-Text; `BYTES` enthält Rohbytes.
  `TOBYTES(text)` kodiert UTF-8, `bytes.ToString()` dekodiert strikt und meldet
  ungültige Daten mit `String/Format`. Keine neue Literalart.
- Stringlängen und -positionen zählen Unicode-Codepoints (Skalarwerte), nicht
  Bytes, Grapheme oder Terminalzellen. Keine automatische Normalisierung.
  `Reverse` kehrt Codepoints um; Padding zählt Codepoints, keine Bildschirmbreite.
- Moderne Member bleiben nullbasiert, klassische Funktionen einbasiert.
  Suchfehlschläge bleiben `-1` beziehungsweise `0`; bestehende Rand-, Padding-
  und historische Kapazitätsregeln bleiben erhalten.
- UTF-8-Ausgabe bleibt Unicode. Erst an tatsächlichen CP437-Ausgabegrenzen wird
  jeder nicht darstellbare Codepoint durch `.` ersetzt. Der gespeicherte Text
  bleibt unverändert; virtuelle CP437-Bildschirme spiegeln die Ersatzzeichen.

**Ausdrücklich vorgezogener C2-Teil:** Ab PPE-/Zielruntime 400 werden Literale als
UTF-8 ohne BOM gespeichert und strikt geladen, auch bei Quellsprache 350.
Unter 400 bleibt CP437 unverändert. `u16`-Bytelänge und abschließendes NUL bleiben
erhalten: maximal 65.534 Nutzbytes; eingebettete NULs bleiben erhalten. Ungültiges
UTF-8, abgeschnittene Nutzdaten und fehlende Terminatoren werden zurückgewiesen.
Alte 400-Beta-PPEs mit Nicht-ASCII-Literalen müssen neu kompiliert werden; neue
PPEs benötigen den aktualisierten Loader. Kein heuristischer CP437-Fallback.
Diese Beta-Inkompatibilität wurde ausdrücklich mit freigegeben.

**Verifiziert:**

- Direkte Literaltests: exakte CP437-Bytes für Runtime 100/300/340, UTF-8 ab 400,
  NULs und Leertext, ungültige UTF-8-Sequenzen, abgeschnittene Rahmen sowie
  65.533/65.534/65.535 UTF-8-Nutzbytes.
- Quelle → Compiler → echte PPE-Datei → Loader → VM sowie Roh- und
  rekonstruierter Decompiler → Compiler → Datei → VM erhalten `€`, CJK,
  kombinierende Zeichen und Nicht-BMP-Zeichen. Sprachversion 350/400 gegen
  Runtime 400 sowie ein Legacy-340-Roundtrip sind abgedeckt.
- Codepoint-/Bytezahlen, moderne/klassische Suche, Indizierung, Teilstrings,
  Padding, Reverse, Split, Regex-Capture-Positionen, fehlende Normalisierung,
  striktes UTF-8-Decoding und unveränderte Kapazitätsgrenzen sind getestet.
- Tatsächliche PPE-Ausgabebytes und gerenderte Terminalzeichen samt Cursor und
  Umbruch bei 80×25 und 132×43 bestehen für UTF-8 und CP437. Dies ist eine
  automatisierte Renderer-Abnahme, noch keine interaktive Client-Matrix.
- Englische/deutsche LSP-Hilfe mit unabhängigen Locale-Loadern geprüft.

**Gesamtvalidierung:** `CARGO_INCREMENTAL=0 cargo test-low -p icy_board_engine
-p icy_board_ppl -p pplc -p ppld -p ppl-lsp --no-fail-fast --quiet` in getrennten
EN- und DE-Prozessen: jeweils **2.878 bestanden, 0 fehlgeschlagen, 6 ignoriert**.
All-Target-Checks der fünf Pakete und Engine-All-Targets ohne Default-Features
bestanden. Der neue Integrationstest ist wie die übrigen BBS-Tests an `bbs`
gebunden. Formatierung, Diff-Check und Editor-Diagnostik ohne neue Fehler.

**Abgrenzung:** F1s Unicode-Dateipfad ist damit nachgewiesen. Größere Literale,
Binärkonstanten, S1/S2-Kodierungen und F2/F3 bleiben C1/C2 vorbehalten. Die
übrige Containerentscheidung bleibt in C1 separat freigabepflichtig.

### S6 — Fehlerfluss und strukturiertes Cleanup

Status: am 2026-09-10 einzeln freigegeben, umgesetzt und validiert.

**Beschluss:** `ON ERROR` bleibt erhalten. `TRY … CATCH … FINALLY … ENDTRY`
ist als spätere Sprachentwicklung gewünscht, wird aber ausdrücklich vertagt,
um zuerst den Releaseplan abzuarbeiten. Auch `DEFER` wird nicht eingeführt.
Ohne Handler bleiben operative Fehler manuell über `Error.Last()` prüfbar;
kein neuer automatischer Abbruch.

**Vertrag und Korrekturen:**

- Die erste ausstehende operative Ursache bleibt bis zum Ende der aufrufenden
  VM-Anweisung erhalten, auch über verschachtelte Funktionen hinweg. Spätere
  Operanden und Seiteneffekte laufen weiter; kein Rollback. `Error.Clear()`
  löscht ausdrücklich auch den ausstehenden Handleraufruf.
- Handler sind VM-weit, nicht routine-lokal. GOTO deaktiviert sich vor dem
  Sprung; GOSUB und Prozedurhandler bleiben ohne rekursiven Aufruf aktiv.
  Prozedurargumentfehler werden vor dem Rumpf behandelt; Rücksprung und
  `VAR`-Rückschreiben bleiben erhalten.
- EOF und erfolglose Suche sind normale Ergebnisse; ungültige Ressourcennutzung
  ist ein operativer Fehler. Fatale VM- und Sitzungsfehler umgehen Handler.
- Cleanup erfolgt am äußersten PPE-Ende, nicht bei Routine- oder innerem
  PPE-Rücksprung. Nach Ausgabefehlern werden lokale Ressourcen weiter freigegeben;
  beide Farbwiederherstellungen werden versucht. Ursprüngliche Ausführungs- und
  Ladefehler werden nicht durch spätere Diagnose-/Restore-Fehler verdeckt.
- Tatsächliches Eingabe-EOF beendet zeitbegrenztes und unbegrenztes Event-Warten
  sowie weitere VM-Ausführung. PPE-Parameter werden auch bei Fehlern gelöscht.
- Der AST-Ausgabebesucher erhält jetzt alle ON-ERROR-Formen; die
  Decompiler-Labelbereinigung zählt GOTO-/GOSUB-Handlerziele als Referenzen.

**Gezielte Abnahme:** 29 Fehlervertragstests bestanden, einschließlich echter
PPE-Dateien und Roh-/rekonstruiertem Decompile/Recompile. Rekursive Argumente,
Handlerfehler und leere Aufruf-/VAR-Stapel mit und ohne Optimierung geprüft.
Wiederholte Sendefehler nach realer Ressourcenaktivierung decken EXIT, STOP,
VM-Fehler, ersten Transportfehler, Diagnoseausgabe und Farbwiederherstellung ab.
Eine tatsächlich geschlossene Eingaberichtung prüft EOF, ausbleibenden
Folgecode/Handler und Cleanup. Fehlende und beschädigte PPE-Dateien behalten
ihre Ladeursache trotz fehlgeschlagener Diagnoseausgabe. LSP-Hilfe mit
unabhängigen englischen/deutschen Locale-Loadern geprüft.

**Gesamtvalidierung:** `CARGO_INCREMENTAL=0 cargo test-low -p icy_board_engine
-p icy_board_ppl -p pplc -p ppld -p ppl-lsp --no-fail-fast --quiet` in getrennten
EN- und DE-Prozessen: jeweils **2.889 bestanden, 0 fehlgeschlagen, 6 ignoriert**.
All-Target-Checks der fünf Pakete und Engine-All-Targets ohne Default-Features
bestanden. Die bestehenden Tests für äußeres PPE-Ende und Ressourcen des
aufrufenden PPE bleiben grün.

**Grenzen:** Reset-Sequenzen sind bei ausgefallener Verbindung best-effort;
lokale Freigabe garantiert keinen reparierten entfernten Terminalzustand.
Kein Nachweis für Prozessabsturz, erzwungenen Future-Abbruch oder unerkannte
Verbindungsabbrüche. Keine neue Cleanup-Syntax und keine Containeränderung.
S7 sowie C1/übriges C2 bleiben eigene Freigabeschritte.

## Containerformat: eigene Schritte nach S1–S6

### C1 — PPE-400-Formatentscheidung (F1/F2/F3)

Status: am 2026-09-10 besprochen und ausdrücklich freigegeben. Die Umsetzung
erfolgt in C2.

**Entscheidung: Alternative 3** — ein eigener sektionierter 400-Container mit
tatsächlich breiteren Operanden und Deskriptoren. Ein neuer Header allein hebt
die Grenzen aus F2 nicht auf; die internen Typ-, Routine- und Referenzbreiten
mussten mitwachsen. Bestehende PCBoard-PPEs behalten Container, Verschlüsselung
und Ausführungssemantik unverändert.

**Beschlossener Vertrag:**

- Dateiendung `.ppe` bleibt; der Container wird an neuen Magic-Bytes erkannt,
  nicht an einer Versionsnummer. Beide Container laden in dieselbe interne
  Ausführungsdarstellung.
- Little-Endian; 64-Bit-Dateioffsets und Sektionslängen, 32-Bit-IDs und
  Codeadressen.
- Container-, Bytecode-, Sektionsschema- und Host-ABI-Version sind getrennt
  versioniert und bewegen sich unabhängig voneinander.
- Kein Sprachprofil in der Datei. Sprachsemantik wird in ausdrückliche
  Ausführungsoperationen übersetzt.
- Unbekannte optionale Sektionen werden übersprungen, unbekannte Pflichtdaten
  vor der Ausführung abgelehnt. `META` ist für spätere Metadaten reserviert.
- Laufzeitlayouts und optionale Beschreibungen sind getrennt. Verhaltensrelevante
  Angaben sind kein Debug und überleben das Entfernen der Debugdaten.
- Host-Bindung über stabile qualifizierte Namen und erwartete Signaturen statt
  kompakter IDs und vollständiger Enum-Wertelisten (F3).
- Typisierte, längengerahmte UTF-8- und Binärkonstanten ohne NUL-Terminator (F1).
- Erweiterbare Typarten; nominale Identität getrennt vom Layout; Member- und
  Routinereferenzen unabhängig voneinander.
- Kompression ausdrücklich angegeben, nie aus Längendifferenzen erschlossen.
  Gewählt wurde Zstd, per Sektion, standardmäßig aus, über ein Compilerflag
  steuerbar.
- Deterministische Kodierung und eine Inhaltsidentität. Ausdrücklich **keine**
  Signatur- oder Verschlüsselungsfunktion und kein Archivdateisystem.
- Betriebsbudgets sind von den Wertebereichen des Formats zu unterscheiden;
  16 MiB war eine unbelegte Zahl und ist kein Formatlimit.
- Unveröffentlichte Beta-400-PPEs müssen neu kompiliert werden. Das ist als
  ausdrückliche Ablehnung mit Hinweis implementiert, nicht als stille Fehlfunktion.
- TRY wird bei Einführung in bestehende Operationen übersetzt; keine eigene
  Sektion und kein eigener Opcode auf Vorrat.
- Ausdrücklich nicht Bestandteil: echte Objektorientierung, Record- und
  Methodenattribute sowie externe Serialisierungsnamen. Der Container ist so
  geschnitten, dass diese später additiv ergänzt werden können.

**Nicht übernommen:** ein Format, das sich am Dekompiler ausrichtet. Der
Dekompiler ist Konsument des Formats, nicht sein Maßstab.

**Abnahme:** erfüllt durch diesen Abschnitt, den Formatvertrag in
[ppe_format.md](ppe_format.md) und die Testergebnisse in C2.

### C2 — Freigegebenes Format implementieren und absichern

Status: am 2026-09-10 umgesetzt und validiert. Die vorgezogene
UTF-8-Literalkodierung aus S5 ist darin aufgegangen.

**Implementiert:**

- Neuer Container mit 64-Byte-Header, 48-Byte-Verzeichniseinträgen und den
  Sektionen `TYPE`, `CONS`, `VARS`, `ROUT`, `IMPT`, `CODE` sowie optional
  `IDEN` und `DBUG`. Genaue Wireformate in [ppe_format.md](ppe_format.md).
- Interne Breiten mitgewachsen: Typreferenzen und Routinedeskriptoren sind
  32-bittig, `VAR`-Modi sind eine Liste je Parameter statt der 16-Bit-Maske.
- Eigene Codekodierung: Sprungziele sind Anweisungsindizes; Kurzschlussoperatoren
  und Record-Literale haben endlich eine Dateidarstellung (S1/S2).
- Host-ABI `IMPT`: Bindung über qualifizierte Namen und Signaturen, mit
  Umnummerierung der Typ- und Member-IDs beim Laden.
- Zstd je Sektion über `pplc --compression`, Debugnamen über `pplc --debug`;
  beide Voreinstellungen sind aus.
- `IDEN` als SHA-256 über Laufzeit, Einstiegsroutine und alle Sektionen außer
  `IDEN` und `DBUG`.
- Strikte Ladevalidierung vor der ersten VM-Anweisung: Header, Sektionsgrenzen
  und Überlappung, Budgets, Typgraph, Konstantenrepräsentation, Variablen- und
  Routinetabellen sowie sämtliche Codereferenzen, Argumentzahlen, Ränge,
  Zuweisungsziele und Builtin-Signaturen.
- Beta-400-PPEs im alten Container werden mit Neukompilierungshinweis abgelehnt.

**Ausgeführte Abnahme:**

- F1: Quelle → PPE-Datei → Loader → VM erhält `€`, CJK, kombinierende und
  Nicht-BMP-Zeichen; zusätzlich ein 20.000-fach wiederholtes Literal mit
  eingebettetem NUL über beide Kompressionsmodi.
- F2: ein Programm mit 6.000 Anweisungen, 300 `VAR`-Parametern und einem
  Callback, der diese 300 Parameter weiterreicht, läuft als echte Datei in
  beiden Kompressionsmodi. Die alten Grenzen 255/254/16/32767 sind weiterhin
  getestet, jetzt ausdrücklich gegen Runtime 340.
- F3: Enum- und Host-Rundläufe bestehen, einschließlich Dekompilieren und
  Neuübersetzen. Host-Enums werden über die Importidentität erkannt statt über
  die vollständige Werteliste; ein Unit-Test vertauscht Typ- und Member-IDs
  sowie Signaturtypen und prüft, dass unveränderte Dateibytes weiterhin binden
  und eine echte Signaturänderung abgelehnt wird.
- S1/S2: Record- und Kurzschlussprogramme überstehen jetzt echte Datei-Roundtrips
  statt an der Serialisierungsgrenze abgewiesen zu werden.
- S6: Kontrollbefehle werden beim Erzeugen normalisiert, weil das früher erst der
  alte Decoder tat. Fehler-, Cleanup- und Ressourcentests bleiben unverändert grün.
- Beschädigte, abgeschnittene, übergroße, überlappende und unbekannte
  Pflichtdaten werden kontrolliert abgelehnt; unbekannte optionale Sektionen
  werden übersprungen.
- Legacy-PPEs behalten ihre Ausführung; Legacy-Fixtures laufen unverändert.

**Gesamtvalidierung:** `CARGO_INCREMENTAL=0 cargo test-low -p icy_board_engine
-p icy_board_ppl -p pplc -p ppld -p ppl-lsp --no-fail-fast --quiet` in getrennten
EN- und DE-Prozessen: jeweils **2.900 bestanden, 0 fehlgeschlagen, 6 ignoriert**
über 68 ungefilterte Testziele. All-Target-Check der fünf Pakete und Engine ohne
Default-Features bestanden. Formatierung der berührten Rust-Dateien und
`git diff --check` ohne Befund.

**Offen und bewusst vertagt:**

- `META` ist reserviert, aber ohne Semantik. Unbekannte optionale Sektionen
  werden übersprungen und gehen beim Neuschreiben verloren; eine Erhaltungs- und
  Identitätsregel dafür fehlt noch.
- Debugdaten enthalten nur Variablennamen. Typ- und Feldnamen, Quellpositionen
  und eine getrennte Debugdatei sind nicht umgesetzt.
- Das Codebudget des Compilers rechnet weiterhin in alten logischen Einheiten;
  die tatsächliche Sektionsgrenze greift zusätzlich im Container.
- Recordfeldgrenzen sind intern weiterhin 16-bittig, obwohl das Wireformat
  32 Bit vorsieht.
- `read_file` liest die Datei vor der Größenprüfung vollständig ein.
- Zstd ist nur unter Linux gebaut und getestet; Windows und macOS stehen aus.
- Keine Fuzz-Abnahme des neuen Loaders.

### S7 — Sprachentscheidungen zusammenführen

Status: offen; am 2026-09-10 ausdrücklich hinter C1 und C2 verschoben.

**Besprechen:** Zusammenspiel der Sprach- und Formatänderungen, offen gebliebene
Entscheidungen, verbleibende Abnahmelücken und realistische Restzeit.

**Abnahme:** Die freigegebenen Sprach- und Formatänderungen sind gemeinsam
getestet. Noch offene und bewusst vertagte Anforderungen sind ausdrücklich
aufgelistet. Kein erneuter Format- oder Sprachumbau ohne eigene Freigabe.

## Runtime- und API-Arbeitspakete

Auch die folgenden Schritte werden jeweils einzeln besprochen und freigegeben.
Die am 2026-09-10 freigegebene Reihenfolge lautet C1, C2, anschließend S7.
Die UTF-8-Literalkodierung wurde als begrenzte Ausnahme mit S5 freigegeben.

### R1 — Ressourcenidentität korrigieren (F4)

Status: vorgezogen und mit S1 am 2026-09-09 umgesetzt und getestet. Siehe
[S1-Abschluss](#s1r1--abschluss-2026-09-09).

**Besprechen:** Generationen/Session-Epochen oder nicht wiederverwendete
Ressourcen-IDs; Trennung zwischen Audio-Kanal und Audio-Ressource; Verhalten
geteilter Handles, wiederholtem `Free()` und Grafik-Neustart.

**Abnahme:** Alte Aliase bleiben nach Freigabe und Neuanlage ungültig. Sie können
neue Ressourcen weder verändern noch freigeben. Tests umfassen Audio,
Surface, Grafik-Reinitialisierung sowie relevante verschachtelte PPE-Aufrufe.

### R2 — Audio-Vertrag vereinheitlichen (F5)

**Besprechen:** Kanonische `Fade`-Argumentreihenfolge, Einheiten, Clamp-/Fehlerregeln.

**Abnahme:** API-Katalog, LSP, Dokumentation und Runtime stimmen überein.
Ein Ausgabe-Test prüft sowohl die gewünschte Lautstärke als auch die Dauer der
tatsächlich erzeugten Terminalsequenz.

### A1 — API-ABI und Fehlerverträge einfrieren (F3)

**Besprechen:** Explizite stabile IDs oder gleichwertige maschinelle
ABI-Absicherung; Mindest-API-Versionen; optionale Parameter und Defaults;
einheitliche Regeln für `Valid`, `Success`, `Error.Last()` und unbekannte Werte.

**Abnahme:** Vollständiger Katalog mit Typ-/Member-Identität, Signaturen,
Rückgabetypen und Array-Rängen. Unbeabsichtigtes Umordnen wird durch Tests erkannt.
Neue Runtime-Versionen werden mit unverändert gespeicherten alten PPE-Fixtures
getestet, nicht nur durch Neukompilieren alter Quellen.

### A2 — BBS-Berechtigungen und Objektlebensdauer

**Besprechen:** Privilegierter Sysop-Datenzugriff versus benutzerbezogene sichere
Operationen; Snapshot-/Live-Verhalten; Nachrichtennummer versus Arrayposition;
JAM-Read-Flag versus persönlicher Last-read-Cursor.

**Abnahme:** Ein PPE-Autor kann erkennen, welche Prüfung er selbst durchführen
muss. Die Nachrichtenleser-Abnahme enthält private und nicht zugängliche Inhalte.
Header-Snapshot und später geladenes Message-Body-Verhalten sind ausdrücklich
geregelt. Keine pauschale Behauptung, dass PPEs eine Sandbox darstellen.

### A3 — Board-/Benutzerzugriff skalierbar machen (F6)

**Besprechen:** Lazy-Erzeugung, getrennte Snapshots oder begrenzte Queries;
Snapshot-Zeitpunkt, Suche, Pagination und Verhalten bei parallelen Änderungen.

**Abnahme:** Ein Metadatenzugriff wie `Board.Name` erstellt keinen vollständigen
Benutzer-Clone. Aufwand für große Userbases wird gemessen. Bestehende
Snapshot-Garantien bleiben erhalten oder werden ausdrücklich geändert.

### A4 — Datei- und Nachrichtenworkflows vervollständigen

**Besprechen:** Kleinster notwendiger API-Umfang für die unten beschriebenen
Dateibrowser- und Nachrichtenleser-PPEs. Vorhandene prozedurale Funktionen wie
`JOIN`, `DOWNLOAD` und `MESSAGE` berücksichtigen; eine fehlende Objektmethode
bedeutet nicht automatisch fehlende Gesamtfunktionalität.

**Kandidaten:**

- FileEntry mit Beschreibung, Größe, Datum, Suche und begrenzter Enumeration.
- Download mit Berechtigungen und typisiertem Ergebnis.
- Nachrichtenerzeugung/Antwort, persönliche Lesemarkierung und Attachments.
- Objektbezogene Session-Navigation und Door-Ausführung, soweit konkret benötigt.

**Abnahme:** Die realen PPEs nutzen öffentliche BBS-APIs und keine selbst
geschriebenen Parser für interne TOML-, JAM- oder Filebase-Strukturen.

### A5 — Terminal-Layout und aktuelle Geometrie

**Besprechen:** Connection-Fähigkeiten versus aktuelle Größe; Resize-Ereignisse;
Zell-/Pixelkoordinaten; sichtbare Textbreite; ANSI-/ATX-bewusstes Layout;
sichere Ausgabe fremder Texte ohne Steuersequenzinterpretation.

**Abnahme:** Layout wird anhand gerenderter Terminalausgabe geprüft, nicht nur
an Parserzuständen. Wechselnde Geometrien, Unicode und einfache ANSI-Fallbacks
funktionieren gemäß beschlossenem Umfang. Nicht unterstützte Sixel-/JXL-Funktionen
werden nicht still als gleichwertig behandelt.

### A6 — PPE-eigene Datenhaltung und Utilities

**Besprechen:** Was für die Abnahme wirklich benötigt wird: JSON, PPE-eigener
Datenbereich, atomisches Speichern, Mehrnode-Koordination, Schema-Versionen.
Zusätzliche File-Objekte, UTC-Zeitpunkte oder asynchrones HTTP nur mit konkretem
Bedarf aufnehmen; keine vorsorgliche Vollimplementierung.

**Abnahme:** Mindestens ein realer persistenter Anwendungsfall ist sicher
abgedeckt. Positionaler Record-I/O wird nicht als automatisch migrationssicher
verkauft. Bewusst vertagte Utilities sind einzeln benannt.

## Drei echte PPEs als verbindliche Release-Abnahme

Keine bloßen Sprachproben: Die Programme werden als echte PPE-Dateien gebaut,
geladen und auf einer Test-BBS benutzt. Vor jedem PPE dessen Mindestumfang
einzeln besprechen. API-Lücken zurück in A1–A6 führen; keine privaten Rust-Hilfen
einbauen, die normale PPE-Autoren nicht benutzen können.

### E1 — Dateibrowser

- Verzeichnisse und zugängliche Dateien anzeigen.
- Beschreibungen, Dateigröße und Datum darstellen.
- Suche und Paging; große und leere Bestände berücksichtigen.
- Download über öffentliche BBS-Funktionen einschließlich Ablehnung/Abbruch.
- Keine Kenntnis interner Filebase- oder Konfigurationsformate voraussetzen.

**Abnahme:** Vollständiger Nutzerablauf, korrekte Zugriffsregeln, begrenzter
Ressourcenbedarf und brauchbare Darstellung in Englisch und Deutsch.

### E2 — Nachrichtenleser mit Antwortfunktion

- Konferenz/Area wählen, Header auflisten und Text lesen.
- Sparse Nachrichtennummern, leere Areas und gelöschte Nachrichten behandeln.
- Persönliche ungelesene Nachrichten und Last-read-Verhalten prüfen.
- Private und gesperrte Inhalte korrekt behandeln.
- Antwort schreiben; Attachment-Umfang vor Umsetzung ausdrücklich entscheiden.
- Parallele Änderungen durch andere Nodes berücksichtigen.

**Abnahme:** Lesen → Markieren → Antworten funktioniert ohne JAM-/TOML-Parser
im PPE. Header-/Body-Konsistenz und Benutzerberechtigungen sind nachgewiesen.

### E3 — Interaktive Terminalanwendung

- Maus und Tastatur in einer Eventschleife.
- Resize-fähiges Layout und synchronisierte Ausgabe.
- Mindestens ein Grafikpfad sowie nutzbarer Fallback ohne diese Fähigkeit.
- Audio optional nutzen und fehlende Unterstützung sauber behandeln.
- Ressourcen anlegen, teilen, freigeben und erneut anlegen.
- Cleanup bei normalem Ende, `STOP`, Fehler und Disconnect.

**Abnahme:** Tests mit SyncTERM, icy_term und einem einfacheren ANSI-Terminal;
jeweils Clientversion, Geometrie und tatsächlich getestete Fähigkeiten notieren.
Mindestens 80×25, eine größere Geometrie und Größenwechsel abdecken.
Die BBS bleibt nach Ende/Abbruch bedienbar; keine zurückgelassenen Input-Modi,
Updates, Margins oder unbeabsichtigt weiterlaufenden Medien.

SyncTERM ist hier der Terminalclient. Daraus folgt keine automatische
Ausführbarkeit von PPE 400 auf dem Synchronet-BBS-Server.

## Zeitliche Orientierung

Die Reihenfolge ist verbindlicher als die Wochenzuordnung. Der Monat ist ein
Zielrahmen, keine belastbare Aufwandsschätzung vor den Einzelentscheidungen.

| Zeitraum | Schwerpunkt | Kontrollpunkt |
| --- | --- | --- |
| Woche 1 | P0; S1–S6 einzeln besprechen und freigegebene Sprachänderungen umsetzen | Anforderungen aus den freigegebenen Sprachverträgen für C1 festhalten. |
| Woche 2 | C1 separat entscheiden; C2; anschließend S7; R1/R2 und A1–A3 | F1–F6 bearbeitet beziehungsweise konkret terminiert; Format-/ABI-Roundtrips belastbar. |
| Woche 3 | A4–A6 bedarfsgetrieben; E1–E3 als echte PPEs bauen und abnehmen | BBS- und Terminalworkflows ohne interne Dateiparser; gefundene Lücken priorisieren. |
| Woche 4 | Regressionen, Client-Matrix, Legacy-Kompatibilität, Dokumentationsabgleich | Keine neuen Sprachkonzepte; Go/No-Go anhand nachgewiesener Ergebnisse. |

Falls Woche 1 wegen notwendiger Sprachänderungen mehr Zeit benötigt, C1 nicht
vorziehen. Stattdessen Termin oder späteren additiven API-Umfang besprechen.
F1–F6 nicht kommentarlos aus dem Pflichtumfang streichen.

## Übergreifende Validierung und Release-Gate

- Lokale Rust-Tests bevorzugt mit `cargo test-low`; relevante Crates und
  Integrationstests ausdrücklich auswählen.
- Compiler, VM, Decompiler, Formatter, LSP und Editorgrammatiken gemeinsam prüfen.
- Für neue Konstrukte echte Serializer-/Loader-Roundtrips verwenden.
- Legacy-Fixtures unverändert ausführen; Source-Neukompilierung allein reicht nicht.
- Englische und deutsche Abläufe mit separaten Prozessen oder expliziten
  Locale-Loadern testen, nicht durch globale Locale-Wechsel in parallelen Tests.
- Rendering einschließlich konkreter Geometrie prüfen; Parserausgabe reicht nicht.
- Abbruch, I/O-Fehler, fehlende Fähigkeiten, leere Ergebnisse und Ressourcenlimits
  gehören zur Abnahme, nicht nur der Erfolgsfall.
- Test-Exitcodes erhalten; bei Shell-Pipelines `pipefail` verwenden.
- Nur tatsächlich ausgeführte Tests als bestanden melden; Blockaden und nicht
  getestete Clientpfade ausdrücklich benennen.

### Go/No-Go-Checkliste

- [x] P0 abgeschlossen; stabiler Build und reproduzierbare Baseline.
- [x] S1–S6 jeweils einzeln besprochen; freigegebene Änderungen umgesetzt.
- [x] C1 separat entschieden, C2 umgesetzt; anschließend S7 abgeschlossen. — C1/C2 erledigt, S7 offen.
- [x] F1: Unicode-Datei-Roundtrip nachgewiesen (S5, vorgezogener UTF-8-Literalteil).
- [x] F2: Beschlossenes Größen-/Limitkonzept umgesetzt und an Grenzen getestet.
- [x] F3: Host-Enum- und API-Evolution mit alten PPE-Dateien nachgewiesen.
- [x] F4: Stale Handles bleiben auch nach Wiederverwendung ungültig.
- [ ] F5: `Fade`-Vertrag einschließlich realer Ausgabe konsistent.
- [ ] F6: Metadatenzugriff skaliert unabhängig von vollständigen User-Snapshots.
- [ ] E1 Dateibrowser abgenommen.
- [ ] E2 Nachrichtenleser mit Antwortfunktion abgenommen.
- [ ] E3 Interaktive Terminalanwendung und Client-Matrix abgenommen.
- [ ] Legacy-Kompatibilität und relevante Gesamtregressionen bestanden.
- [ ] Dokumentation entspricht Signaturen, Limits, Fehlern und Lebensdauerregeln.
- [ ] Verbleibende Einschränkungen und bewusst vertagte additive Features benannt.
- [ ] Gemeinsame Releaseentscheidung statt automatischem Freeze zum Stichtag.

## Protokoll je Einzelschritt

Bei der Durchführung diesen Abschnitt für den jeweils besprochenen Schritt
ergänzen, damit Entscheidungen nicht nur im Chat verbleiben:

- Schritt / Datum / geprüfter Arbeitsstand:
- Problem und konkrete Reproduktion:
- Besprochene Alternativen:
- Entscheidung und ausdrückliche Freigabe:
- Umfang / ausdrücklich nicht enthalten:
- Sprach-, API-, Format- und Legacy-Auswirkungen:
- Implementierung:
- Ausgeführte Tests und Ergebnisse:
- Offene Punkte / vertagte Abnahmen:
- Abschluss und nächster separat zu besprechender Schritt:

### P0 — 2026-09-09

**Arbeitsstand und Freigabe:** `abd2b08e827ba43e234c2beb605a2fe7ef8162c8`,
zu Beginn sauberer Git-Arbeitsbaum. Auftrag: den Releaseplan starten; keine
pauschale Freigabe für S1–S6 oder C1. Die Core-Auslagerung ist auf diesem Stand
eingebunden. Es waren keine Compiler-/Runtime-Korrekturen für die Baseline nötig.

**Ausgeführte Prüfungen:** Alle unten genannten Aufrufe erfolgreich. Für die
Cargo-Läufe war `CARGO_INCREMENTAL=0` gesetzt. EN bedeutet separate Prozesse mit
`LANG=en_US.UTF-8 LC_ALL=en_US.UTF-8 LANGUAGE=en`, DE entsprechend
`LANG=de_DE.UTF-8 LC_ALL=de_DE.UTF-8 LANGUAGE=de`.

| Prüfung | Umgebung | Ergebnis |
| --- | --- | --- |
| `cargo check -p icy_board_ppl -p icy_board_engine -p pplc -p ppld -p ppl-lsp --all-targets -j 4 --quiet` | EN | Exit 0; keine Diagnosen ausgegeben. |
| `cargo check -p icy_board_engine --no-default-features --all-targets -j 4 --quiet` | EN | Exit 0; keine Diagnosen ausgegeben. |
| `cargo test-low -p icy_board_ppl -p pplc -p ppld -p ppl-lsp --quiet` | EN und DE, getrennt | Je 608 bestanden, 0 fehlgeschlagen, 1 ignoriert; darin Core-Library 352 bestanden / 1 ignoriert. |
| `cargo test-low -p icy_board_engine --quiet` | EN | 2.154 bestanden, 0 fehlgeschlagen, 5 ignoriert; darin Engine-Library 1.815 bestanden / 5 ignoriert. |
| `cargo test-low -p icy_board_engine --lib vm::tests:: --quiet` | DE | 793 bestanden, 0 fehlgeschlagen, 3 ignoriert, 1.024 herausgefiltert. |
| `cargo test-low -p tree-sitter-ppl --test repository_sources --quiet` | DE | 1 bestanden, 0 fehlgeschlagen. |
| VS-Code-Aufgabe „PPL400 F9-F12 validation“ | Desktop-Umgebung | Follow-up-API 8, Expression-Tests 21, API-Review 11 und LSP 185 bestanden; keine Fehler oder ignorierten Tests. |

Die Gesamtzahlen zählen Accounting-Unterprozess-Worker nicht erneut. Ignorierte
Tests gelten nicht als bestanden. Ein erster DE-Aufruf scheiterte an falsch
übergebenen Shell-Umgebungszuweisungen, bevor Cargo startete; er zählt nicht als
Testlauf. Die oben aufgeführten DE-Ergebnisse stammen aus dem korrigierten Lauf.

**Validierungsaufgabe:** Der Expression-Filter der lokalen
[VS-Code-Aufgabe](../.vscode/tasks.json) zeigte noch auf `icy_board_engine` statt
`icy_board_ppl`. Korrigiert und ausgeführt; der Filter trifft jetzt 21 Tests.
Diese lokale Konfiguration ist Git-ignoriert, daher kein versionierter Fix.

**F1–F6 erneut eingeordnet — kein Befund als behoben markiert:**

- **F1:** Der [Konstanten-Serializer](../crates/icy_board_ppl/src/executable/variable_table.rs)
  benutzt weiterhin CP437 mit `c as u8` als Fallback. Die vorhandene
  [String-Konstanten-Suite](../crates/icy_board_engine/tests/ppl400_followup_constants.rs)
  deckt Längenverträge ab, ersetzt aber keine Nicht-CP437-Roundtrip-Reproduktion.
  Diese mit `€`, CJK, kombinierenden und Nicht-BMP-Zeichen bleibt für S5/C2 offen.
- **F2:** Der Compiler begrenzt erzeugten Code auf **32.767 Bytes**, nicht Wörter;
  Deklarationen auf 32.767. Routinedeskriptoren enthalten weiterhin kleine
  Parameter-/Local-Zähler, Offsets und VAR-Masken. Die vorhandenen
  [Codegrößen-](../crates/icy_board_engine/tests/test_code_size.rs) und
  [Compilerlimit-Tests](../crates/icy_board_engine/tests/compiler_limits.rs)
  sind im EN-Engine-Lauf enthalten; übergroßer Code liefert `ProgramTooLarge`
  statt eines Panics. Das ist Absicherung des alten Limits, nicht seine Aufhebung.
- **F3:** 14 Host-Enums belegen IDs 255 bis 242. Eigene Records/Enums teilen sich
  aktuell 142 weitere IDs; Kollisionen innerhalb einer Kompilierung werden geprüft.
  Enum-Zuweisungen prüfen jedoch die gespeicherte Domain, und Member-IDs hängen
  von der Registrierungsreihenfolge ab. Bestehende
  [Deklarationstests](../crates/icy_board_ppl/src/parser/declaration_tests.rs)
  sichern heutige IDs ab. Alte PPE-Datei gegen erweiterten Host-Wert bleibt eine
  gesonderte Kompatibilitätsabnahme in S4/A1.
- **F4:** Audio identifiziert Ressourcen allein über wiederverwendbare Kanäle.
  Surface-IDs laufen innerhalb eines Grafikzustands abwärts ab −1; erst ein
  neuer Grafikzustand setzt den Zähler zurück. Vorhandene
  [Audio-](../crates/icy_board_engine/src/vm/tests/sound.rs) und
  [Grafiktests](../crates/icy_board_engine/src/vm/tests/graphics.rs) prüfen unter
  anderem Freigabe und Neuanlage, aber nicht den vollständigen alten Alias nach
  Kanal-Wiederverwendung bzw. Grafik-Neustart. Diese Reproduktionen bleiben R1.
- **F5:** Der [API-Katalog](../crates/icy_board_ppl/src/parser/board_catalog.rs)
  nennt `Fade(durationMs, targetVolume)`; Runtime und der bestandene
  [Audio-Ausgabetest](../crates/icy_board_engine/src/vm/tests/sound.rs) verwenden
  Lautstärke/Dauer. Der Test bestätigt die aktuelle Ausgabe, nicht einen
  konsistenten Vertrag. Die Entscheidung bleibt R2.
- **F6:** Der [erste Board-Zugriff](../crates/icy_board_engine/src/vm/expressions/predefined_functions.rs)
  erzeugt einen vollständigen Snapshot einschließlich Users und cached ihn
  **einmal pro VM**, nicht bei jedem Property-Zugriff. Weitere Zugriffe teilen
  die gespeicherten Werte. Die vorhandenen
  [Snapshot-Tests](../crates/icy_board_engine/src/vm/tests/board_session.rs)
  sind bestanden; eine Skalierungsmessung und die Entkopplung bleiben A3.

**Grenzen der Baseline:** Kein vollständiger Workspace-/Feature-Matrix-Lauf,
kein vollständiger Engine-Lauf unter DE, keine neue PCBoard-DOS-Oracle-Ausführung
und keine manuelle SyncTERM-/icy_term-/ANSI-Clientabnahme. Bestehende VM-Tests
ersetzen weder die drei echten Abnahme-PPEs noch deren produktiven Cleanup-Pfad.

**Abschluss:** P0 abgeschlossen. Nächster Freigabepunkt ist S1; Sprache,
Host-ABI, Ressourcenvertrag und Container wurden noch nicht geändert.

### S1 — Freigabe 2026-09-09

**Entscheidung:** Der Benutzer hat Alternative 2 und das Vorziehen von R1/F4
ausdrücklich freigegeben. Rekursive Typen sind wegen der zusätzlichen
Sprachkomplexität ausgeschlossen, nicht als späteres S1-Teilziel vorgesehen.
Der folgende Vorschlag bildet den freigegebenen Umfang; Ergebnisse und
verbleibende Formatabnahmen stehen im anschließenden Abschlussprotokoll.

**Ausgangspunkt:** Verschachtelte Records und feste Array-Felder existieren
bereits. Records/Arrays verwenden Copy-on-write-Wertsemantik; ein gemeinsames
`Arc` bedeutet nicht, dass Feldänderungen durch alle Kopien sichtbar werden.
Host-Objektfelder sind bislang ausgeschlossen. Der bestehende positionale
Record-I/O setzt eine bekannte Form voraus.

**Alternativen:**

1. Nur Host-Objektfelder ergänzen; dynamische Collections weiter außerhalb halten.
2. Host-Objektfelder und dynamische Array-Felder erlauben, ohne rekursive Typen.
3. Zusätzlich rekursive Widget-/Baumtypen erlauben; benötigt einen eigenen
   Vertrag für Referenzen, Zyklen, Gleichheit und Lebensdauer.

**Empfehlung: Alternative 2**, mit folgendem abzugrenzenden Vertrag:

- Bestehende Host-Typen dürfen Felder sein, ohne dadurch neue Schreibrechte zu
  erhalten. Dynamische Arrays dürfen nichtrekursive Elementtypen enthalten;
  leere Arrays sind der Default. Feste Arrays behalten ihre feste Form.
- Record-/Array-Daten werden als Werte kopiert. Kopierte Ressourcenfelder teilen
  dieselbe Ressource, nicht deren Pixel-/Audiodaten. `Free()` betrifft alle Aliase;
  spätere Ressourcen dürfen durch diese Aliase nicht wieder erreichbar werden.
- Default-Hostfelder benötigen typgerechte, sicher abfragbare Leerwerte, keine
  versehentlich gültige Kanalnummer. Details zu Abwesenheit folgen dem Host-Typ.
- Wert-Gleichheit bleibt feldweise, sofern alle Felder vergleichbar sind.
  `AUDIO`/`SURFACE` vergleichen die Ressourcenidentität, nicht Kanalnummern;
  typgleiche leere Ressourcenwerte sind gleich. Für andere Host-Typen zunächst
  keine neue implizite Gleichheit einführen: Vergleiche solcher Felder bzw.
  enthaltender Records/Arrays erhalten eine Compilerdiagnose statt still
  „immer ungleich“ zu ergeben. Bestehende Host-Vergleiche vorher inventarisieren.
- Nutzbarkeit und Speicherbarkeit trennen: keine automatische Persistierung
  von Host-Objekten; dynamische Record-I/O-Formen nicht provisorisch erfinden.
  Nicht unterstützte Formen eindeutig zurückweisen, ohne teilweise zu schreiben.
- Keine selbst-/wechselseitig rekursiven Records, kein GC und keine neue
  Objektorientierung. Ein Containerentscheid wird damit nicht vorweggenommen.

**Abhängigkeit:** R1/F4 wird mit ausdrücklicher Freigabe vorgezogen und zusammen
mit S1 abgenommen. C1 bleibt nach S7; keine provisorische Dateikodierung.

**Auswirkungen/Aufwand:** Mehrtägiges, crateübergreifendes Paket aus Parser,
Semantik, interner Typdarstellung, VM, Formatter/Decompiler und LSP; R1 erhöht
den Umfang. Legacy-Sprache und -Dateiformat bleiben unverändert. Neue
Layoutinformationen zunächst nur intern testen; endgültige Dateikodierung und
Beta-PPE-Neukompilierung werden erst nach S7/C1 entschieden.

**Vorgesehene Abnahme:** Sprite mit Surface, Menüeintrag mit Area und
nichtrekursive Liste von Einträgen; tiefe Wertkopien, Resize, leere Defaults,
unveränderte feste Formen, Gleichheit und explizite I/O-Ablehnung. Für R1 alte
Audio-/Surface-Aliase nach Freigabe, Wiederbelegung und Grafik-Neustart testen.
EN/DE-Diagnosen und Tooling mitprüfen; formatabhängige Roundtrips bleiben C2.

### S1/R1 — Erste Abnahme 2026-09-09

**Arbeitsbasis:** P0-Stand `abd2b08e827ba43e234c2beb605a2fe7ef8162c8` plus
die begonnenen, uncommitteten S1/R1-Änderungen. Zu diesem Zeitpunkt keine Commits
oder Pushes. Das anschließende Review erforderte drei Nachbesserungen; siehe unten.
Freigabe: Alternative 2, R1/F4 vorgezogen, keine rekursiven Typen.

**Implementiert:**

- Host-Objektfelder und dynamische Array-Felder mit Rank 1–3; feste Felder
  behalten ihre Form. Direkte und indirekte Typrekursion bleibt verboten.
- Verschachtelte Wertkopien, Record-Literale, Routine-Defaults und `VAR`-Ziele
  berücksichtigen die deklarierte Feldform. REDIM erfasst verschachtelte
  Indizes einmal, allokiert neue Default-Elemente und schreibt kontrolliert
  zurück; keine neue Bytecode-Anweisung erforderlich.
- Alle 24 Host-Typen haben dispatchfähige typgerechte Leerwerte. Arrays werden
  vor dem Binden initialisiert, damit neue mutable Host-Defaults nicht zwischen
  unabhängigen Elementen geteilt werden. Leere Controller greifen nicht auf die
  aktive Session zu; ungültige Area-/Conference-/Directory-/Door-Werte erteilen
  keinen Zugriff. Frische Records erhalten leere dynamische Felder.
- Audio und Surface tragen neben der technischen Nummer eine eigene, geteilte
  Allokationsidentität. Alte Aliase bleiben nach Free, Kanal-Wiederbelegung,
  Grafik-Neustart und Cleanup ungültig. Auch Blit-Quellen werden geprüft.
  Identitätsgleichheit bleibt nach Free erhalten, bedeutet aber nicht Gültigkeit.
- Wertvergleich ist rekursiv; Audio/Surface vergleichen Allokationsidentitäten,
  typgleiche Leerressourcen sind gleich. Andere Host-Typen und enthaltende Records
  erhalten eine Compilerdiagnose statt impliziter Gleichheit.
- Record-I/O weist dynamische/Host-Felder auch transitiv und bei leeren Arrays
  vor dem ersten Lesen/Schreiben zurück; defensive Bytecode-Tests prüfen das.
- Hover, Completion und In-Memory-Decompiler erhalten dynamische Ränge und
  feste Nullgrenzen. Die neue Nichtvergleichbarkeitsdiagnose wird im LSP unter
  EN/DE mit stabilem Code `ppl.type-not-comparable` ausgegeben.
- Die Tree-sitter-Grammatik erkennt Array-Dimensionen auch in Record-Feldern;
  der generierte Editorparser wurde aktualisiert.

**Gezielt reproduziert und korrigiert:**

- Ressourcen-Leerwerte scheiterten zunächst mit `NoObjectFound`; beide betroffenen
  Tests und alle sechs Ressourcenidentitätstests sind jetzt bestanden.
- Ausgeführte S1-Programme deckten fehlerhafte REDIM-Writeback-Ausdrücke und
  abgelehnte verschachtelte VAR-Ziele auf. Die Regressionen prüfen nicht nur
  Parserausgabe, sondern VM-Ausgabe, Auswertungsreihenfolge und Kopierverhalten.
- Die Gesamtregression fand eine geänderte Fehlerpriorität bei `Surface.Pin()`
  auf Sixel. Der bestehende Fehlervertrag wurde ohne Lockerung der alten
  Assertion wiederhergestellt.
- Feldhover fehlte auf der linken Zuweisungsseite; bestehende und explizite
  verschachtelte Zuweisungsziele werden jetzt durchlaufen.
- Ein zusätzlicher Grammatiktest reproduzierte Parsefehler für feste und
  dynamische Array-Felder. Derselbe Test besteht nach der Grammatikänderung
  mit allen drei Rängen und verschachtelten Zuweisungen.

**Ausgeführte Abschlussprüfungen:** `CARGO_INCREMENTAL=0`; getrennte Prozesse mit
`LANG`/`LC_ALL` auf `en_US.UTF-8` bzw. `de_DE.UTF-8` und `LANGUAGE=en` bzw. `de`.
Die Gesamtzahlen zählen Accounting-Unterprozess-Worker nicht zusätzlich.

| Prüfung | Ergebnis |
| --- | --- |
| `cargo test-low -p icy_board_ppl -p pplc -p ppld -p ppl-lsp --quiet` | EN und DE je **638 bestanden, 0 fehlgeschlagen, 1 ignoriert**; darin Core-Library 372 und LSP 195 bestanden. |
| `cargo test-low -p icy_board_engine --quiet` | EN und DE je **2.194 bestanden, 0 fehlgeschlagen, 5 ignoriert**; darin Library 1.854 bestanden. |
| `cargo test-low -p tree-sitter-ppl --test repository_sources --quiet` | EN und DE je **2 bestanden**; Repository-Quellen und neuer S1-Grammatiktest. |
| `tree-sitter test` nach `tree-sitter generate` (CLI 0.25.10) | **34 von 34 Korpusfällen bestanden**. |
| `cargo test-low -p icy_board_tui --test localization --quiet` | DE: **3 bestanden**. |
| `cargo check -p icy_board_ppl -p icy_board_engine -p pplc -p ppld -p ppl-lsp --all-targets -j 4 --quiet` | Erfolgreich. |
| `cargo check -p icy_board_engine --no-default-features --all-targets -j 4 --quiet` | Erfolgreich. |
| Geänderte Rust-Dateien: `rustfmt --check --edition 2024 --config skip_children=true` | **55 Dateien erfolgreich**; keine automatische Workspace-Umformatierung. |
| `git diff --check` | Erfolgreich. |

**Abdeckung:** 18 neue Sprachkern-Record-Tests, zwei In-Memory-Decompiler-Tests,
30 gezielte Engine-S1-Tests (darunter 18 Source→VM-Tests), sechs Tests für
Ressourcenidentität und drei zusätzliche produktive PPE-Lebensdauertests.
Letztere speichern/lesen echte Ressourcen-PPE-Dateien und rufen ein Kind-PPE
über `CALL` auf. Geprüft sind Grafik-Neustart mit/ohne Shutdown, Audio-Kanalreuse,
Fortbestand lebender Elternressourcen sowie äußeres Cleanup nach normalem Ende,
STOP und VM-Fehler einschließlich Terminalsequenzen und internem Medienzustand.
EN/DE-LSP-Tests verwenden unabhängige Loader bzw. separate Serverprozesse.

**Explizit offen bis C2:** Neue Record-Layouts besitzen nur interne Metadaten.
`Executable::to_buffer()` weist Host-/dynamische Record-Felder mit
`UnsupportedRecordFieldEncoding` zurück, auch wenn der Typ keine Variable hat.
Der bisherige Loader und die bisherige Kodierung bleiben unverändert.
Die neuen Sprachmodelle sind daher **noch nicht als PPE-Dateien deploybar**.
Source→VM-Tests überspringen diese Grenze ausdrücklich; bestehende Tests behalten
ihren Serializer-/Loader-Roundtrip. Auch Ressourcenfelder innerhalb neuer Records
brauchen später die C2-Datei-Roundtrip-Abnahme.

**Weitere Grenzen:** Kein manueller SyncTERM-/icy_term-Test und keine vollständige
E1–E3-Abnahme. Disconnect-/Clientmatrix-Abnahmen bleiben E3. Kein neuer Container,
kein S2–S6-Feature und keine geänderte Legacy-Dateikodierung. Die aktuellen
Sprach- und Ressourcenregeln sind in [new_ppl.md](new_ppl.md) nachgetragen.

**Bewertung nach Review:** Die erste Abschlussbewertung war zu weitgehend:
String-Arraykonvertierungen, geklammerte REDIM-Ziele und mehrdimensionale
Leerwertzugriffe waren nicht ausreichend abgesichert. Die Nachbesserung folgt.

### S1 — Review-Nachbesserung 2026-09-09

**Freigabe:** Die drei bestätigten Reviewbefunde korrigieren, anschließend S1/R1
committen. Kein Push; S2 und die Containerentscheidung bleiben getrennte Schritte.

**Korrigiert und durch Regressionen abgesichert:**

- Record-Arrayfelder akzeptieren wieder die vom Compiler erlaubten Konvertierungen
  zwischen STRING und BIGSTR. Die VM konvertiert jedes Element; String-Längengrenzen,
  feste Formen und die Ablehnung fremder Elementtypen bleiben erhalten. Tests für
  feste Felder und Record-Literale durchlaufen den tatsächlichen PPE-Datei-Roundtrip;
  dynamische Felder werden einschließlich leerer Arrays im Speicher ausgeführt.
- `REDIM (roots)[0].Values, 1` und tiefer geklammerte/indexierte Ziele erzeugen
  beschreibbare Pfade statt ArrayValueAt-Leseausdrücken im Rückschreibziel.
  Compiler- und VM-Tests prüfen alle drei Ränge, unveränderte Lesekopien und
  einmalige Indexauswertung von links nach rechts. Temporäre Werte und
  schreibgeschützte Properties bleiben als Ziele abgelehnt.
- Fehlende Host-/Record-Arrayelemente erhalten bei Runtime 400 in allen drei
  Rängen schema- und hostgerechte Defaults. Direkte Variablenzugriffe, Feldzugriffe,
  Klammernotation und geklammerte Ausdrücke verwenden denselben Default-Pfad.
  Enum-Defaults, die Identität vorhandener Ressourcen und das Legacy-Verhalten
  bleiben durch Tests abgesichert; ein Lesezugriff vergrößert kein Array.

**Frisch ausgeführte Prüfungen nach der Nachbesserung:** Rust-Befehle mit
`CARGO_INCREMENTAL=0`; EN/DE in getrennten Prozessen wie bei der ersten Abnahme.
Accounting-Unterprozess-Worker sind in den Gesamtzahlen nicht doppelt enthalten.

| Prüfung | Ergebnis |
| --- | --- |
| `cargo test-low -p icy_board_engine --lib s1_ --quiet` | EN: **38 bestanden**. |
| `cargo test-low -p icy_board_ppl --lib compiler::record_fields_tests --quiet` | EN: **22 bestanden**. |
| `cargo test-low -p icy_board_engine --quiet` | EN und DE je **2.202 bestanden, 0 fehlgeschlagen, 5 ignoriert**. |
| `cargo test-low -p icy_board_ppl -p pplc -p ppld -p ppl-lsp --quiet` | EN und DE je **642 bestanden, 0 fehlgeschlagen, 1 ignoriert**. |
| `cargo test-low -p tree-sitter-ppl --test repository_sources --quiet` | EN und DE je **2 bestanden**. |
| `tree-sitter test` | **34 von 34 Korpusfällen bestanden**. |
| `cargo test-low -p icy_board_tui --test localization --quiet` | DE: **3 bestanden**. |
| `cargo check -p icy_board_ppl -p icy_board_engine -p pplc -p ppld -p ppl-lsp --all-targets -j 4 --quiet` | Erfolgreich. |
| `cargo check -p icy_board_engine --no-default-features --all-targets -j 4 --quiet` | Erfolgreich. |
| `rustfmt --check --edition 2024 --config skip_children=true` für geänderte Rust-Dateien | **55 Dateien erfolgreich**. |
| `git diff --check` | Erfolgreich. |

**Abschluss:** Die drei Reviewbefunde sind korrigiert. S1 ist im freigegebenen
In-Memory-Umfang einschließlich R1/F4 abgenommen. Die C2-Dateiabnahmen für neue
Record-Layouts und die manuellen E1–E3-/Terminalclient-Abnahmen bleiben offen.
Als Nächstes S2 separat besprechen und freigeben.