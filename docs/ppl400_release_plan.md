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

**Vergleich auf einen Blick:**

| Bereich | Alter PCBoard-Container (bis Runtime 3.40) | Neuer PPE-400-Container | Warum der neue Vertrag besser passt |
| :--- | :--- | :--- | :--- |
| Erkennung und Aufbau | Textpräambel mit einer gekoppelten Runtime-Version; 48-Byte-Header, danach Tabellen und Code weitgehend nacheinander | Eigene Magic-Bytes `ICYPPE\0\0`; 64-Byte-Header, Sektionsverzeichnis und explizit adressierte Sektionen | Eindeutige Formaterkennung und unabhängig prüfbare, erweiterbare Bestandteile |
| Größen und Referenzen | 16-Bit-Codegröße, 16-Bit-IDs und -Grenzen sowie bytebreite Typ- und Parameterzahlen begrenzen wachsende Programme | 64-Bit-Dateioffsets und Sektionslängen; 32-Bit-Zähler, IDs, Typreferenzen und Codeadressen | Formatbreiten bilden große Programme direkt ab; engere Betriebsbudgets bleiben davon getrennt |
| `VAR`-Parameter | Eine 16-Bit-Bitmaske im Prozedurdeskriptor | Ein eigener 32-Bit-Moduswert je Parameter | Mehr als 16 Referenzparameter sind darstellbar und weitere Parametermodi bleiben ergänzbar |
| Text und Konstanten | CP437-Text, NUL-Terminierung und typabhängige kompakte Nutzdaten | Typisierte, längengerahmte UTF-8- und Binärkonstanten; eingebettete NUL-Bytes erlaubt | Unicode und Binärdaten überstehen Quelle, Datei und VM verlustfrei |
| Codekodierung | Historische Wortkodierung; Sprünge und Routinen verwenden schmale Codeoffsets | Längengerahmte Anweisungen mit 32-Bit-Operanden; Sprünge adressieren Anweisungsindizes | Neue Ausdrücke wie Kurzschlussoperatoren und Record-Literale haben eine eindeutige Dateidarstellung |
| Typen und Records | Bytebreite Typ-IDs und kompakte Typ-/Enumtabellen; neue Layoutmerkmale passen nicht zuverlässig hinein | Eigene `TYPE`-Sektion mit 32-Bit-Typen, Feldangaben und erweiterbaren Typarten | Nominale Typidentität und Laufzeitlayout können unabhängig wachsen |
| Host-API-Bindung | Kompakte gespeicherte IDs und vollständige Enum-Wertelisten koppeln PPEs an den damaligen Katalog | `IMPT` bindet qualifizierte Namen und erwartete Signaturen; IDs werden beim Laden umgesetzt | Katalog-IDs dürfen sich ändern und APIs oder Enums dürfen wachsen, ohne alte PPEs ungültig zu machen |
| Versionierung | Eine Runtime-Version steuert zugleich Layout, Kodierung und Entschlüsselung | Container, Bytecode, Sektionsschema und Host-ABI sind getrennt versioniert | Änderungen bleiben auf die tatsächlich betroffene Schicht begrenzt |
| Erweiterbarkeit | Kein Sektionsmodell; neue Daten verändern das sequentielle Layout | Bekannte Pflichtsektionen werden verlangt; unbekannte optionale Sektionen werden übersprungen | Additive Erweiterungen sind möglich, während fehlende notwendige Semantik vor der Ausführung abgelehnt wird |
| Kompression und Verschlüsselung | Historische Codekomprimierung und -verschlüsselung sind an die Runtime gekoppelt | Explizites Zstd je Sektion, standardmäßig aus; bewusst keine Verschlüsselung | Kompression ist lokal, prüfbar und austauschbar; Verschlüsselung wird nicht mit Integrität verwechselt |
| Identität und Reproduzierbarkeit | Keine formatdefinierte Inhaltsidentität | Deterministische Kodierung und optionale `IDEN`-SHA-256 über den laufzeitrelevanten Inhalt | Gleicher Programminhalt bleibt trotz Kompression oder entfernter Debugnamen identifizierbar |
| Debugdaten | Keine getrennte, abstreifbare Debugsektion | Optionale `DBUG`-Sektion, derzeit mit Variablennamen | Laufzeitvertrag und Diagnoseinformationen sind sauber getrennt |
| Laden beschädigter Dateien | Grenzen ergeben sich teilweise erst beim sequentiellen Dekodieren | Größen, Überlappungen, Typgraph, Tabellen, Referenzen und Signaturen werden vor der ersten VM-Anweisung validiert | Fehlerhafte oder nicht unterstützte Dateien laufen nicht teilweise an |
| Rückwärtskompatibilität | Vertrag für bestehende PCBoard-PPEs | Eigener Container nur für Runtime 400; beide Formate werden in dieselbe interne Darstellung geladen | Alte PPEs behalten Format, Verschlüsselung und Semantik; Runtime 400 muss keine alten Grenzen mitschleppen |

**Größengrenzen im Vergleich:**

Die alten Werte sind harte Grenzen der Kodierung: Ein Zähler in einem Byte oder
eine 16-Bit-Bitmaske lässt sich nicht vergrößern, ohne das Format zu brechen.
Die 400-Werte sind überwiegend Betriebsbudgets gegen beschädigte oder bösartige
Dateien; das Wireformat selbst rechnet mit 32-Bit-Zählern und 64-Bit-Offsets.

| Größe | Alter Container (bis Runtime 3.40) | Neuer PPE-400-Container |
| :--- | ---: | ---: |
| Code je Programm | 32.767 Bytes, 16-Bit-Codegrößenfeld | 32 MiB je `CODE`-Sektion |
| Deklarationen der Variablentabelle | 32.767, 16-Bit-Tabellenzähler | 1.000.000 |
| Parameter je Routine | 255 | 4.096 |
| davon `VAR`-Parameter | 16, Position der Bitmaske | alle Parameter, eigener Modus je Parameter |
| Lokale Variablen je Routine | 254 | 65.536 |
| Records und Enums je Programm | 156 gemeinsame IDs 100–255, davon 14 Builtin-Enums | 65.536 Records, 32-Bit-Typreferenzen |
| Felder je Record | 255, Zähler in einem Byte | 4.096 |
| Stringliteral | 65.534 Bytes einschließlich Terminator | 32-Bit-Länge, praktisch durch Sektions- und Dateibudget begrenzt |
| Ausdrucksverschachtelung beim Laden | 64 | 96 |
| Dateigröße | keine formatdefinierte Grenze | 64 MiB |
| Sektionen je Datei | kein Sektionsmodell | 64, alle dekodierten Sektionen zusammen 64 MiB |

Der neue Container ist damit kein Ersatz für ein Archiv-, Signatur- oder
Rechtesystem. Er löst gezielt die Skalierungs-, Unicode-, Erweiterungs- und
ABI-Stabilitätsprobleme des alten ausführbaren Formats. Die vollständige
Wirebeschreibung steht in [ppe_format.md](ppe_format.md).

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

- `read_file` liest die Datei vor der Größenprüfung vollständig ein.
- Zstd ist nur unter Linux gebaut und getestet; Windows und macOS stehen aus.
- Keine Fuzz-Abnahme des neuen Loaders.
- Debugdaten enthalten keine Quellpositionen und keine getrennte Debugdatei.

**Am 2026-09-10 nachträglich geschlossen** (ursprünglich vertagt):

- Unbekannte optionale Sektionen werden erhalten statt verworfen. Die
  Inhaltsidentität deckt nur noch die sechs Programmsektionen ab, damit eine
  zukünftige Zusatzsektion die Prüfung nicht ungültig macht — das war ein echter
  Vorwärtskompatibilitätsfehler, nicht nur eine fehlende Bequemlichkeit.
  Unbekannte Kompressionscodes gelten jetzt als fehlerhaft statt als überspringbar.
- Debugdaten tragen zusätzlich Record-, Feld- und Enum-Namen; der Dekompiler gibt
  sie aus, statt `TYPE001`/`FIELD001` zu erfinden.
- Das Codebudget misst die tatsächlich erzeugte `CODE`-Sektion statt alter
  logischer Worteinheiten.
- Recordfeldgrenzen sind intern 32-bittig wie im Wireformat.

### S7 — Sprachentscheidungen zusammenführen

Status: am 2026-09-10 umgesetzt, nachdem C1 und C2 abgeschlossen waren.

**Gemeinsame Abnahme:** Ein Programm führt die Verträge aus S1–S6 zusammen —
Record mit Host-, dynamischem und Enum-Feld (S1), Kurzschlussoperator (S2), zwei
`VAR`-Parameter mit Rückschreibung (S3), nominaler Enum-Cast (S4), Unicode-Text
mit Codepoint-Länge (S5) und ein `ON ERROR`-Handler nach fehlgeschlagenem
Dateizugriff (S6). Es wird als echte Datei in beiden Kompressionsmodi
geschrieben, geladen, ausgeführt, dekompiliert und neu übersetzt; die Ausgabe ist
jedes Mal identisch und die deklarierten Namen bleiben erhalten.

Zusätzlich wurde von Hand geprüft, dass ein Programm mit 18 `VAR`-Parametern
— jenseits der alten 16-Bit-Maske — samt Records und Enums über `pplc` und `ppld`
den vollständigen Weg Quelle → PPE → Quelle → PPE fehlerfrei durchläuft.

**Noch offene und bewusst vertagte Anforderungen:** F5 (`Fade`-Vertrag, R2),
F6 (Skalierung des Board-/Benutzerzugriffs, A3), A1–A6, die drei Abnahme-PPEs
E1–E3 samt Client-Matrix sowie die unter C2 genannten Restpunkte.

**Kein erneuter Format- oder Sprachumbau ohne eigene Freigabe.**

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

Status: am 2026-09-10 besprochen, freigegeben und umgesetzt.

**Befund:** Runtime, Dokumentation und Tests lasen `Fade(targetVolume, durationMs)`;
nur der API-Katalog und damit die LSP-Signaturhilfe nannten die Dauer zuerst. Wer
der Signaturhilfe folgte und `Fade(250, 0)` schrieb, bekam Lautstärke 250 —
geklemmt auf 100 — und Dauer 0, also einen sofortigen Sprung auf volle Lautstärke
statt eines Ausblendens.

**Entscheidung:** Der Katalog wird an Runtime und Dokumentation angeglichen, nicht
umgekehrt. Die Reihenfolge lautet verbindlich `Fade(targetVolume, durationMs)`.
Die Alternative, die Runtime umzustellen, hätte das Verhalten jedes bestehenden
Programms geändert und wurde ausdrücklich verworfen.

**Kompatibilität:** keine. Parameternamen werden nur von der LSP-Signaturhilfe
gelesen — beide Parameter sind `INTEGER`, benannte Argumente gibt es nicht. Kein
Programm ändert seine Bedeutung, nichts muss neu übersetzt werden.

**Einheiten:** Lautstärke ist ein Prozentwert und wird auf 0–100 geklemmt; eine
Dauer von null oder weniger ändert die Lautstärke sofort statt gleitend.

**Abnahme:** Katalog, LSP, Dokumentation und Runtime nennen dieselbe Reihenfolge.
Zwei Ausgabe-Tests prüfen die tatsächlich gesendete Terminalsequenz mit
Lautstärke *und* Dauer (`Volume;C=2;V=-60.00dB;T=250`) sowie das Klemmen von
Lautstärke und negativer Dauer. Ein LSP-Test sichert die Reihenfolge in der
Signaturhilfe ab.

### A1 — API-ABI und Fehlerverträge einfrieren (F3)

Status: am 2026-09-10 besprochen, freigegeben und umgesetzt.

**Befund.** Die Bindung war bereits stärker als angenommen: ein gespeichertes
Programm bindet über den qualifizierten Namen und die Signatur, nicht über die
gespeicherte ID. Umordnen von Typen und Membern ist deshalb unschädlich, ebenso
das Hinzufügen neuer Typen und Member — nachgemessen an allen vier üblichen
Änderungen. Eine Signatur- oder Namensänderung endet in einem harten Ladefehler
mit benanntem Member, nicht in stiller Fehlinterpretation.

Genau eine übliche Evolutionsform war unnötig blockiert: das Anhängen eines
**optionalen** Parameters wurde als Bruch behandelt, weil `parameters` und
`required` auf exakte Gleichheit verglichen wurden. Wer `Regex.Find(text, start)`
um ein `limit` ergänzt hätte, hätte jede bestehende PPE unladbar gemacht.

**Entscheidung.** `bind()` akzeptiert zusätzlich angehängte optionale Parameter
und ein gelockertes `required`, sofern die gespeicherte Parameterliste ein
Präfix der aktuellen ist. Die Lockerung ist einseitig: ein Programm, das gegen
die längere Signatur gebaut wurde, wird auf der älteren Laufzeit weiterhin
abgelehnt. Alles andere bleibt streng. Laufzeitseitig ist das gedeckt, weil die
VM die Argumentzahl gegen die aktuelle Registry prüft und fehlende optionale
Argumente ohnehin mit Vorgabewerten liest.

**Verworfen:** explizite stabile Member-IDs samt Versionsnummern. Sie hätten
Format und Pflege belastet, ohne etwas zu lösen, das die Namensbindung nicht
schon leistet. Ebenfalls gestrichen: der ursprünglich geplante Punkt
„Mindest-API-Versionen". Die Bindung meldet bereits `host member
icy_board.Conference.Name`, was einer Versionsnummer überlegen ist.

**Kompatibilität:** rein erweiternd. Was heute lädt, lädt weiter; es kommen nur
Fälle hinzu, die bisher abgelehnt wurden. Keine Formatänderung.

**Fehlerverträge.** Die Prüfung ergab kein Durcheinander, sondern drei
verschiedene Fragen: `Valid` beantwortet, ob ein per Nummer, Index oder Suche
geholtes Handle auf etwas Vorhandenes zeigt; `OK`, ob die Antwort selbst gut
war; `Success`, ob eine Suche getroffen hat. `HTTPRESPONSE` führt bewusst beide
ersten, weil ein Netzwerkfehler und ein 404 unterscheidbar bleiben müssen.
Zugangsobjekte ohne Lookup führen keins davon. Aktionen liefern `BOOLEAN` und
legen die Einzelheiten in `Error.Last()`. Die Regel wird festgeschrieben, nicht
geändert — ein Vereinheitlichen wäre ein Verhaltensbruch ohne Gewinn. Die beiden
subtilen Regeln zur Fehlerlebensdauer (ein Erfolg löscht einen älteren Fehler;
der erste Fehler einer Anweisung gewinnt) waren bereits dokumentiert.

**Nicht in A1 gelöst:** `Regex.Find()` auf einem ungültigen Regex liefert ein
`REGEXMATCH` mit `Success = FALSE`, also dasselbe wie „kein Treffer";
unterscheiden lässt sich das nur über `Error.Last()`. Das ist eine
Verhaltensfrage und gehört nicht in einen Freeze.

**Abnahme.** Der Katalog liegt als eingecheckte Textdatei
[api_catalog.txt](../crates/icy_board_ppl/tests/api_catalog.txt) mit allen Typen,
Membern, Signaturen, optionalen Parametern, Rängen, Rückgabetypen und
Enum-Varianten; ein Test vergleicht sie und zeigt bei Abweichung die geänderten
Zeilen. Ein zweiter Test hält fest, welcher Typ `Valid`, `OK` oder `Success`
führt. Drei mit 4.00 gebaute PPE-Dateien liegen binär im Repo und werden
geladen und ausgeführt, nicht neu übersetzt; sie decken Host-Objekte samt
Read-only-Vertrag, Aufrufe mit ausgelassenen optionalen Argumenten sowie
Records, Enums, `VAR`-Parameter, Kurzschluss und `ON ERROR` ab. Beide Fixture-
Arten werden nur auf ausdrückliche Anweisung neu erzeugt.

### A2 — BBS-Berechtigungen und Objektlebensdauer

Status: am 2026-09-10 besprochen, freigegeben und umgesetzt.

**Befund (verifiziert).** `Area.Read()` und `Area.Find()` in
[message_area.rs](../crates/icy_board_engine/src/icy_board/message_area.rs)
wenden keine der Prüfungen an, die der interaktive Leser über
`may_read_header` und `requires_read_password` anwendet: privat, gelöscht,
`MSG_NODISP` und Passwortschutz. Ein PPE liest damit auch private Post.

Die Benutzerseite ist dagegen bereits abgesichert, an einem laufenden PPE
gemessen: `USER` hat keinen `Password`-Getter, Konferenz-, Verzeichnis- und
Tür-Passwörter werden über `protected()` herausgegeben, und Schreibversuche auf
fremde Benutzer scheitern mit gesetztem `Error.Last()`.

**Entscheidung: dokumentieren statt erzwingen.** PPL ist eine API für
Programmierer und Sysops, nicht für Endanwender; PPEs installiert der Sysop und
sie laufen mit der Reichweite des Boards. Eine erzwungene Filterung nähme
Auswertungen — etwa statistische Erhebungen über alle Nachrichten — die
Grundlage. Der ungefilterte Blick ist gewollt, das Risiko wird benannt statt
weggeregelt. Verworfen wurden deshalb die Varianten „Runtime filtert wie der
interaktive Leser" und „neuer `CanRead()`-Member".

**Was der PPE-Autor selbst prüfen muss**, steht jetzt in
[new_ppl.md](new_ppl.md) unter „Who may see a message is the program's
decision", mit einem übersetzbaren Muster für `IsDeleted`, `NeedsPassword`,
`IsPrivate` samt Name und Alias sowie Sysop-Ausnahme.

**Ehrlich benannte Lücke:** Ein als nicht anzeigbar markierter Header
(`MSG_NODISP`) hat keinen eigenen Member. Ein PPE kann den interaktiven Leser
deshalb derzeit nicht exakt nachbilden.

**Zugesichert bleibt:** Lesen verändert nichts. `Read()`, `Find()` und `Text()`
setzen kein Read-Flag und bewegen keinen Last-read-Zeiger, ein zweiter Durchlauf
sieht dasselbe. `HighMsg()`/`LowMsg()` liefern unverändert die rohen Grenzen der
Message-Base; Lücken beantwortet `Valid = FALSE`. Nachrichtennummer bleibt
Nummer, nicht Position.

**Nebenbefund behoben:** `"[" + conf.Password + "]"` brachte die VM zum Absturz
(`promote_to` kannte `Password` nicht und fiel auf `Integer` zurück, worauf
`as_int()` panickte), und `conf.Password.Len()` war ein Übersetzungsfehler.
Passwörter verhalten sich jetzt bei Textoperationen wie ihre Maske, und `Len()`
antwortet wie das bereits vorhandene `LEN()` mit der Maskenlänge. Ein Test hält
fest, dass die Maske nicht mit dem gespeicherten Passwort variiert und ein
Vergleich weiterhin funktioniert.

### A3 — Board-/Benutzerzugriff skalierbar machen (F6)

Status: am 2026-09-10 gemessen, besprochen, freigegeben und umgesetzt.

**Messung zuerst.** Der ursprüngliche Verdacht „jeder Zugriff klont“ traf nicht zu:
`Board` wird bereits einmal pro VM zwischengespeichert. Das eigentliche Problem lag
tiefer — der Snapshot materialisierte die gesamte Userbase sofort. Fünf Läufe je
Zelle, jeweils das Minimum, Grundlinie ist derselbe Aufbau ohne `Board`-Zugriff:

| Users | Grundlinie | `Board.Name` vorher | `Board.Name` nachher |
| ---: | ---: | ---: | ---: |
| 1.000 | 1,1 ms | +0,49 ms | +0 |
| 10.000 | 3,6 ms | +5,01 ms | +0 |
| 50.000 | 19,0 ms | +35,60 ms | +0 |

Vorher kostete `Board.Name` genauso viel wie `Board.Users` — der direkte Nachweis,
dass ein reiner Metadatenzugriff die ganze Userbase bezahlte, linear mit rund
0,5–0,7 µs je Benutzer. Bei 50.000 Benutzern verdreifachte das die Laufzeit eines
kleinen PPEs.

**Umsetzung:** `PplBoard` erfasst beim ersten Board-Zugriff nur noch die geteilte
Benutzerliste — `UserBase` speichert bereits `Snapshot<Vec<User>>`, also ein `Arc`
mit Copy-on-Write, was die Erfassung O(1) macht — und baut das PPL-Array erst,
wenn `Users` gelesen wird. Dasselbe für `Conferences`.

**Snapshot-Zeitpunkt unverändert.** Weil das `Arc` den Stand beim ersten
Board-Zugriff festhält, bleibt die eingefrorene Sicht exakt dieselbe wie zuvor.
Eine spätere Materialisierung liest nicht neu. Es gab also nichts neu
festzulegen; die Alternative mit verschobenem Zeitpunkt war dafür nicht nötig.

**Abnahme:** Zwei Unit-Tests belegen deterministisch, dass Metadaten die
Benutzerliste nicht materialisieren und dass die einmal gebaute Liste geteilt
wird — statt einer zeitbasierten Prüfung, die in CI schwanken würde. Die 17
Board-/Session- und 11 Benutzer-Snapshot-Tests bleiben unverändert gültig:
`Board.Users` ist weiterhin ein typisiertes Array mit `.Len()`, jedes Element ein
unabhängiger Snapshot, Schreibversuche werden abgelehnt und ein Index außerhalb
liefert `Valid = FALSE`.

**Offen:** Suche und Pagination über die Userbase sind weiterhin nicht Teil der
API; wer alle Benutzer durchgeht, materialisiert sie weiterhin vollständig. Das
ist der Preis des zugesicherten Array-Vertrags und bleibt A2/A4 vorbehalten.

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

**Freigegebener Datei-API-Schritt (2026-09-11).** Nach der E1-Grundlage wurde
die schreibgeschützte Filebase-API ausdrücklich freigegeben. Implementiert sind
`DIRECTORY.Find(text [, after [, limit]])`, `FILEPAGE` und `FILEENTRY` mit
Name, gespeicherter Beschreibung, 64-Bit-Größe und UTC-Datum. Neue Typ-IDs 56/57
und der angehängte Directory-Member 9 lassen bestehende IDs unverändert.

Die Suche liest ausschließlich den vorhandenen SQLite-Index: höchstens 100
Treffer und 1.024 geprüfte Datensätze pro Aufruf, höchstens 16 KiB Beschreibung
pro Eintrag, expliziter Cursor für die Fortsetzung auch nach leeren Seiten.
Sie prüft aktuelle Konferenz-/Listenrechte und überspringt gelöschte, fehlende,
verlinkte und unsichere Dateinamen. Es gibt keinen versteckten Verzeichnisscan,
keine Archivextraktion und keinen PPE-eigenen Filebase-Parser. Ein noch nicht
initialisierter Index meldet `Unavailable`. Beschreibungssuche arbeitet nur auf
dem begrenzten Präfix; Seiten sind keine dauerhafte Transaktionsansicht und
folgen der Datensatz-ID statt der konfigurierten Anzeigesortierung.

Vier `dizbase`-Lesertests und zwei serialisierte PPE-/Compiler-Vertragstests
prüfen Metadaten, Unicode-Suche, 2-GiB-Größe, Fortsetzung, Scanbudget,
Beschreibungslimit, Rechte, Standardwerte, Fehlerbehandlung, Runtime-Grenze und
Schreibschutz. Die Runtime-Registry-Brücke besteht mit vier Tests, der
Workspace-Check mit allen Targets ist erfolgreich. Vollständiger Vertrag und
Beispiel stehen unter [Filebase Search](new_ppl.md#filebase-search-400).
Die Abschlussregression umfasst 28 Filebase-Tests, 14 Board-Objekt-Tests,
beide vorhandenen E1-Tests und 398 PPL-Bibliothekstests (einer weiterhin
ignoriert), jeweils ohne Fehler. Die LSP-Suite besteht in getrennten EN/DE-
Prozessen; alle drei Lokalisierungstests bestehen ebenfalls.

**Dateiansicht ergänzt (2026-09-11):** Das E1-PPE nutzt `Directory.Find` für
Dateiseiten, Beschreibungssuche und Dateidetails mit Größe und Datum. Der Benutzer
hat die befüllte Demo als passend bestätigt. Die gerenderten EN/DE-Tests prüfen
auch leere Such-Zwischenseiten, Rückwärtsblättern und lange Beschreibungen.

**Markierung freigegeben und umgesetzt (2026-09-11):** Der Benutzer hat den
bestehenden Ablauf ausdrücklich bestätigt: Dateien markieren, Download separat.
`Directory.Flag(fileName) -> BOOLEAN` markiert genau die indexierte Datei im
gewählten Bereich. Der reproduzierte Legacy-Abgleich von `FLAG` sucht dagegen
in allen Bereichen der Konferenz; ein vollständiger Pfad ist dort keine exakte
Bereichsauswahl. Diese Legacy-Semantik wurde nicht geändert. Gleichnamige Dateien
innerhalb einer Area bleiben unzulässig; gleiche Namen in verschiedenen Areas
werden durch das Directory-Objekt eindeutig zugeordnet.

`Flag` ist Member 10, additiv nach `Find`; keine neue Typ-ID und keine neue
Download-API. Die Methode prüft aktuelle Konferenz-/Listen-/Downloadrechte,
Batch-Limit und Guthaben, erhält vorhandene Markierungen und Aufrufer-Tokens
und startet keinen Transfer. Bereits markierte Dateien liefern TRUE ohne
Duplikat. Fehlende/gelöschte/unindexierte/verlinkte Dateien und veraltete
Directory-Snapshots werden abgewiesen; Fehler stehen in `Error.Last()`.
Tests prüfen exakte Auswahl bei gleichen Namen in zwei Areas, stille Ausgabe,
Idempotenz, Rechteentzug, ausgetauschte Bereiche, Indexfehler und Guthaben ohne
Abbuchung. Der Browser markiert mit `M` in Liste und Details; gerenderte
80x25-EN/DE-Tests decken Erfolg, wiederholtes Markieren und Ablehnung ab.

**Validierung dieses Schritts:** sechs E1-, fünf A4-, vier Registry- und
32 Download-Kompatibilitätstests bestanden; die Gruppen überlappen teilweise.
29 Filebase-Tests, 398 PPL-Bibliothekstests (einer ignoriert) und 37
Accounting-Tests (einer ignoriert) bestanden ebenfalls. Die LSP-Suite besteht
in getrennten EN/DE-Prozessen. Compiler, Runtime und LSP wurden neu gebaut;
das Browser-Paket wurde kompiliert und in der lokalen Demo aktualisiert.

**Separater Download-Befehl im Demo-PPE (2026-09-11):** Auf Benutzerwunsch öffnet
`D` aus Dateiliste und Dateidetails den normalen BBS-Befehl über
`COMMAND FALSE, "D"`. Markieren bleibt eine eigene Aktion; die hervorgehobene
Datei wird nicht automatisch hinzugefügt. Das PPE gibt die Terminal-Eingabe frei
und zeichnet nach Ende oder Abbruch dieselbe Ansicht erneut. Der bestehende
Befehl prüft die Download-Berechtigung und übernimmt Batch, Dialoge und Transfer.
Bei restlichen Aufrufer-Tokens wird `D` mit einem Hinweis abgelehnt, damit die
gemeinsame Legacy-Tokenliste nicht versehentlich als BBS-Befehl ausgeführt wird.
Der Demo-Aufrufer übergibt nur die Sprache und ist davon nicht betroffen.

Ein neuer Test führt das serialisierte Browser-PPE in explizitem EN/DE aus:
lokale Dateikopie mit Bytevergleich für zwei markierte Dateien, keine Übertragung
der nur ausgewählten Datei, Abbruch mit erhaltener Batch, verweigerter BBS-Befehl,
leere Batch und Aufrufer-Token-Schutz. Gerenderte 80x25-Ausgabe und Navigation nach
der Rückkehr werden geprüft. Legacy-`COMMAND`, `FLAG` und `DOWNLOAD` bleiben
unverändert; keine neue API.

**Noch offen:** Vollständige manuelle E1-Abnahme und ein Remote-Protokolltransfer
aus dem Browser. Die übrigen A4-Kandidaten bleiben separat zu entscheiden.

**Befunde aus E2 (2026-09-10).** Der Nachrichtenteil wurde nicht entworfen,
sondern beim Portieren von [LiQUiD Read](https://github.com/mkrueger/liquid_read)
gemessen. Lesen, Filtern und Antworten sind mit der heutigen API vollständig
möglich. Zwei dabei gefundene Fehler wurden behoben; der Editoranschluss ist
implementiert, die echte DOS-Editor-Abnahme bleibt teilweise offen:

- **Behoben: der Last-read-Zeiger war schreibbar, aber nicht lesbar.** `SETLMR`
  schrieb korrekt in die JAM-Base — die zuvor gemeldete fehlende Lesemarkierung
  war ein Suchfehler. `U_LMR(conf)` ignorierte jedoch sein Argument und lieferte
  `session.last_msg_read`, einen Wert, den nur der interaktive Leser füllte:
  gemessen `setlmr|8|err=0`, unmittelbar danach `lmr-after-set|0`. `U_LMR` liest
  den Zeiger jetzt dort, wo `SETLMR` ihn hinschreibt, und `SETLMR` hält den
  Sitzungswert mit. Auf der Test-BBS liest ein Programmlauf jetzt den Stand des
  vorherigen (`lmr-at-start|8`).
- **Behoben: `MESSAGE` stellte dem Nachrichtentext ein BOM voran.** `FPUTLN`
  beginnt eine neue Datei mit einem UTF-8-BOM, und `message()` las sie mit
  `read_to_string` samt Markierung in den Text. Der Body wird jetzt wie bei
  `FGET` über `read_with_encoding_detection` gelesen, womit auch die
  CP437-Erkennung übereinstimmt.
- **Implementiert: Nachrichten-API mit lokalem Header-Wert.**
  `MSGHEADER` und `MSG.Header` liefern veränderbare Kopien ohne Write-through.
  `Session.PostMessage`, `ReplyMessage`, `EditMessage` und `ReplyHeader` verwenden
  die Board-Abläufe samt Zugriffsprüfung, Editor und Speicherung. Die Rückgabe
  entsteht direkt beim Speichern; bestehende Nachrichten werden mit Konfliktprüfung
  ersetzt. MSGINF/MSGTMP sind ausschließlich intern verwaltete Editor-Dateien.
  `icbsetup` bietet Internal, Program,
  Script, DOS und PPE samt Dropfile, Argumenten und Laufzeitgrenze. Ein Editor-PPE
  erhält das Austauschverzeichnis per GETTOKEN; EXIT speichert, STOP bricht ab.
  Script-/PPE-Tests prüfen Speichern, Abbruch, Fehler, Timeout, Token-Rückgabe
  und die Erhaltung privater Nachrichten- und Thread-/Attachment-Metadaten.
  Die DOS-Rückgabecodes 0 bis 3 und fehlende RESULT.ED wurden mit FreeDOS geprüft.
  Kompilierte PPE-Tests decken neue Nachrichten, editierbare Textvorgaben,
  Antworten in der Herkunfts-Area, Bearbeitung, Abbruch und Fehlerfälle ab.
  Siehe [Nachrichten-API und Konfiguration](new_ppl.md#composing-and-editing-messages).
- **DOS-Editoren geprüft (2026-09-11), ICE nur teilweise.** Die fehlende
  Eingabe mit X00 1.24 wurde auf einen nicht quittierbaren THRE-Interrupt im
  nativen x86-UART zurückgeführt. Eine unabhängige FreeDOS-Registerprobe liest
  ohne Fix zweimal Interrupt-ID 2 statt 2 und anschließend 1. Der freigegebene
  lokale Fix im separaten x86-Repository besteht zwei UART-Unit-Tests und
  diese DOS-Probe. ICE Edit 2.35 und GEdit 2.10 bestehen damit jeweils sichtbare
  Texteingabe bei 80x25, Speichern samt zurückgegebenem MSGTMP und bestätigten
  Benutzerabbruch ohne Logoff. ICE zeigt auch die MSGINF-Metadaten korrekt an;
  GEdits DORINFO-Startansicht zeigt diese nicht. Die versionierte x86-Abhängigkeit
  wurde nicht geändert; X00-Ergebnisse gelten bislang nur mit lokalem Override.
- **GEdit vollständig im Antwortablauf geprüft.** GEdit funktioniert auch
  ohne X00 direkt über COM1 mit der versionierten Emulator-Abhängigkeit.
  Der echte Reader-Test übernimmt eine Zitatzeile mit Ctrl-Q, Enter, Ctrl-K,
  schreibt Antworttext und speichert mit Ctrl-Z. Die anschließende
  Rechtschreibprüfungsfrage wird mit N und Enter beantwortet. JAM enthält
  Zitat, Antwort, Privatstatus und Thread-Verknüpfung; nach Benutzerabbruch
  entsteht keine Nachricht. Der Reader kehrt zur aktualisierten Liste zurück.
- **Offen: ICE-Quote-Funktion.** Der vollständige Reader-Test mit
  `ICB_LIQUID_READ_EDITOR=iceedit` bleibt rot: Ctrl-Q zeigt die korrekte
  Zitatquelle, danach entstehen etwa 2,6 MB Ausgabe überwiegend aus Leerzeichen
  bis zum Timeout. Begrenztes Warten zwischen Tasten und ein versuchsweiser
  CRLF-Abschluss von MSGTMP beheben dies nicht. Die CRLF-Probe wurde entfernt.
  Ursache und vollständige ICE-Kompatibilität bleiben offen; erfolgreiche
  Eingabe-/Speicher-Einzeltests ersetzen diesen fehlenden Zitatnachweis nicht.

**Historischer E2-Zwischenstand (2026-09-11, vor der unten dokumentierten Abnahme).** Der bestehende
LiQUiD-Read-Port wurde als `956bbc4` gepusht. Die anschließende, lokal als
`b661403` committete Umstellung verwendet `Session.ReplyMessage` statt eines eigenen
Editors und erfasst `Error.Last()` unmittelbar nach dem Aufruf. Listenaufbau,
Lesemarkierung und erneute Zugriffsprüfung wurden angepasst. Das Paket
kompiliert für Runtime 400 mit null Fehlern und zehn Warnungen.
Der opt-in Test `message_api_liquid_read_real_package` lädt die echte PPE über
`ICB_LIQUID_READ_PPE` auf einer isolierten In-Memory-Test-BBS mit temporärer
JAM-Base. Zwei Ursachen des zunächst fehlgeschlagenen Laufs sind nachgewiesen:

- `TOKENIZE` hängt an die vorhandenen Session-Tokens an. Ein Aufruftoken vor
  den Layoutwerten verschob deshalb die Koordinaten in beiden Reader-Ansichten.
  Die Konfigurationsloader verwenden jetzt lokale String-Arrays. Der unveränderte
  Test zeigte danach korrekte Kopfzeilen und Vorschau, aber noch den Timeout.
- `InKey()` wartet nach Escape bis zu 100 ms auf eine ANSI-Fortsetzung. Der
  Test sendete das zweite Escape schon nach 30 ms; es wurde beim ersten Aufruf
  mitverbraucht. Mit 150 ms Eingabeabstand kehrt die PPE zurück. Eine Änderung
  an Compiler, Runtime oder Nachrichten-API war hierfür nicht nötig.

Der verstärkte Test besteht in getrennten EN-/DE-Prozessen mit kontrolliertem
PPE-Editor jeweils für Speichern und Abbruch. Er prüft die persistierte Antwort
samt Thread-Verknüpfung, Privatstatus und zitiertem Originaltext, die aktualisierte
Liste, exakte Betreff-/Vorschaupositionen bei 80x25, unveränderte Aufruftokens,
den Sitzungs-Lesestand und die Rückkehr zur ursprünglichen Konferenz/Area.
Beide Prozesse verwenden die englischen Standard-Board-Texte; eine deutsche
Dialog-/Reader-Lokalisierung ist damit nicht abgenommen.

Die erweiterte Abnahme besteht ebenfalls mit internem Zeileneditor und echtem
GEdit, jeweils für Speichern und Abbruch. `ICB_LIQUID_READ_EDITOR` wählt
`ppe` (Standard), `internal`, `gedit` oder den noch fehlschlagenden ICE-Fall.
`message_api_liquid_read_empty_filtered_and_reopened` prüft die leere Area,
sichtbare Nummern 1 und 5 bei ausgeblendeten privaten/gelöschten/geschützten
Einträgen 2 bis 4, private Aliaszustellung und exakte Vorschaupositionen.
Verbotene Inhalte dürfen auch während der Bildschirmaktualisierungen nicht
erscheinen. Eine frische Sitzung liest den persistierten Lesestand 5; erneutes
Lesen von Nachricht 1 setzt ihn nicht zurück. Dies ist kein BBS-Prozessneustart.

**Ausgeführte Abschlussprüfungen:** `cargo test-low -p icy_board_engine --lib`
mit Filter `external_editor_ -- --include-ignored --test-threads=1`: 13 Tests
je EN/DE mit lokalem x86-Pfadoverride. Filter `message_api_liquid_read`
mit `--ignored`: beide Tests je EN/DE für PPE-, internen und GEdit-Editor.
Filter `message_api_ -- --include-ignored --test-threads=1`: 21 Tests je EN/DE
ohne Override und mit GEdit ohne X00. Die Sprachprozesse sind getrennt;
Board- und DOS-Dialoge bleiben englisch. Die Opt-in-Tests verwenden ausschließlich
temporäre Installationskopien und Diskimages, keine Änderungen an DOS-Originalen.

Zu diesem Zwischenstand waren Konferenz-/Area-Auswahl, echte Löcher im JAM-Index,
parallele Änderungen mit Header-/Body-Konsistenz, langer Text/Scrollgrenzen,
interner Vollbildeditor und deutsche Dialoge noch offen. Für ICE fehlt der
Zitatablauf. Diese Liste dokumentiert den damaligen Prüfstand; die spätere
E2-Abnahme und die Entscheidung zur BBS-Bedienung stehen unten.

**PPE-Editor-Proof-of-Concept (2026-09-11).** Separat freigegeben ist ein
LiQUiD-Read-Editor-PPE: Unterpaket `editor`, Artefakt `lredit.ppe`, Modus `Ppe`,
Argument `en` oder `de`. Der Reader bleibt `lread.ppe`; nur der Editor wird
eingesteckt, damit kein rekursiver Antwortaufruf entsteht. Der Zeileneditor
liest das Board-Verzeichnis per GETTOKEN, übernimmt MSGINF/MSGTMP und gibt
MSGTMP/RESULT.ED mit EXIT zurück; STOP bricht ab. Er besitzt keinen eigenen
Nachrichtenspeicherpfad. Der echte Reader-Ablauf und Session.EditMessage prüfen
Speichern, Abbruch, Betreff/PID, vorhandene Quotes, private Thread-Metadaten,
unveränderte Aufruftokens, UTF-8 sowie abgewiesene übergroße Entwürfe und
unveränderte lange Zeilen. Der Umfang ist ein Zeileneditor mit 100 Zeilen,
68-Byte-Eingabefeldern und explizit gewählter EN-/DE-Oberfläche.

Abschlussprüfung: 22 `message_api_`-Tests je EN/DE in getrennten Prozessen mit
`--include-ignored --test-threads=1`, `ICB_LIQUID_READ_EDITOR=lredit`, beiden
echten PPE-Artefakten und jeweils passendem `ICB_LIQUID_EDIT_LANGUAGE` bestanden.
Der Editor-Vertragstest führt sieben Bearbeitungs-/Abbruchszenarien pro Sprache
aus. Nicht-ASCII-Eingabe verlangt weiterhin die Board-Freigabe
`disable_high_ascii_filter`; bestehende UTF-8-Dateitexte werden erhalten.
Reader und Editor wurden direkt mit `pplc --runtime 400` gebaut: Reader mit
zehn bestehenden Warnungen, Editor ohne Warnungen. Die angepasste just-Rezeptur
konnte mangels installiertem `just` nicht direkt ausgeführt werden; ihre
Compiler- und Kopierschritte wurden einzeln erfolgreich ausgeführt.

**Eigenständiger Vollbildeditor (2026-09-11).** Auf ausdrücklichen Wunsch wurde
statt des neuen Zeileneditors der vorhandene `ppe/ledit` als Grundlage verwendet.
`ledit.ppe` ist ein eigenes Runtime-400-Paket ohne Abhängigkeit von LiQUiD Read.
Es übernimmt MSGINF/MSGTMP, gibt den Entwurf mit EXIT zurück und bricht mit
STOP ab; der frühere eigene MESSAGE-Speicherpfad entfällt. Die vorhandene
Vollbildbedienung bleibt erhalten. Konfiguration und Grenzen stehen in
`docs/new_ppl.md`: Modus Ppe, Argument en/de, bis zu 200 Zeilen mit jeweils
76 Zeichen; größere Eingangsentwürfe werden ohne Kürzung abgelehnt.

Der reguläre Test `message_api_ledit_standalone_editor_contract` kompiliert den
Quelltext unabhängig vom Reader. Er prüft 13 Szenarien je Editor-Sprache über
Post/Edit/Reply, tatsächliche 80x25-Ausgabe, Navigation und Textänderungen,
UTF-8-Netzwerkeingabe, Zitate, Makrotext ohne Auswertung, Größenlimits und
leeren Entwurf sowie Speichern/Abbruch und persistierte JAM-Metadaten.
Die zugehörigen regulären `message_api_`-Läufe bestanden mit je 20 Tests in
getrennten EN-/DE-Prozessen; drei bestehende Opt-in-Tests waren ausgelassen.
Der Benutzer bestätigte anschließend den lokalen Editorstart als funktionierend.

Die Reader-Abnahme unterstützt nun zusätzlich `ICB_LIQUID_READ_EDITOR=ledit`
mit `ICB_LIQUID_EDIT_PPE` auf das eigenständige Artefakt. Beide opt-in
`message_api_liquid_read`-Tests bestanden je EN/DE: echter Reader, ledit,
Speichern und Abbruch, sichtbare Eingabe und Zitate, passende EN-/DE-Kopfzeile,
Rückkehr zur aktualisierten Liste, Thread-/Privatdaten, Editor-PID und
unveränderte Aufruftokens; außerdem leere/gefilterte Areas und persistierter
Lesestand in einer frischen Sitzung. Dies koppelt ausschließlich den Test,
nicht die beiden PPE-Pakete. Reader- und Board-Texte bleiben englisch.
**E2-Abnahme durch den Benutzer (2026-09-11): erfüllt.** Nach erfolgreicher
lokaler Probe hat der Benutzer E2 ausdrücklich als erfüllt bewertet und die
vorgeschlagene Konferenz-/Area-Auswahl im Reader abgelehnt: Die Auswahl gehört
zum bestehenden BBS-Bedienablauf, nicht in LiQUiD Read. Die dokumentierten
Testlücken und die ICE-Quote-Einschränkung bleiben sichtbar, blockieren diese
Abnahme aber nicht und gelten dadurch nicht als nachgewiesen. Dies ist keine
Freigabe für den nächsten Umsetzungsschritt oder die vorgeschlagene
Program/Script-Kodierungsoption.

**Kodierungsgrenze:** DOS bleibt CP437; PPE-Austausch verwendet UTF-8 mit BOM.
Program/Script-Dateien und -Pipes sind derzeit ebenfalls CP437. Vorgeschlagen,
aber noch nicht umgesetzt/freigegeben, ist eine Kodierungswahl nur für
Program/Script, mit CP437 als kompatiblem Standard. Dateien und Terminal-I/O
müssen dabei dieselbe Einstellung verwenden; automatische Erkennung allein
ist bei mehrdeutigen CP437-Bytefolgen keine verlässliche DOS-Dekodierung.

Ungeprüft geblieben ist, warum `Session.IsSysop` bei einem über `--runppe`
angemeldeten SYSOP `FALSE` meldet.

### A5 — Terminal-Layout und aktuelle Geometrie

**Bestandsprüfung und freigegebene Renderkorrektur (2026-09-11):** Ein neuer
`Terminal.Info`-Abruf liefert die aktuelle logische Größe; gespeicherte
`TERMINFO`-Werte bleiben Snapshots. Ein serialisiertes PPE bestätigt dies in
EN/DE bei 80x25 und 132x43 in beiden Größenrichtungen. Resize-Ereignisse und
neue öffentliche Layout-/Text-APIs waren zu diesem Zeitpunkt noch nicht freigegeben.

Der Randtest reproduzierte zwei gekoppelte Fehler: Ein ANSI-Resize änderte nur
die Puffergröße, während die Zeichenebene bei 80x25 blieb und Randzeichen
verwarf. Der Viewport wurde erst nach dem gesamten Parser-Aufruf aktualisiert;
ein unmittelbar folgendes `CLS` konnte daher die alte Größe wiederherstellen.
Der lokale Parser-Adapter synchronisiert Puffer, Viewport und Zeichenebenen
jetzt vor dem nächsten Befehl. Explizites `set_terminal_size` nutzt dieselbe
Korrektur. `icy_tools` und die öffentlichen PPE-Signaturen bleiben unverändert.

Drei A5-Engine-Tests prüfen tatsächliche Randzeichen, Snapshot-Verhalten,
Benutzer-/Sysop-Bildschirme und jede Zweiteilung des Ausgabestroms einschließlich
UTF-8- und Escape-Grenzen sowie byteweisen Empfang. Ein zusätzlicher Test führt
Resize, Löschen und Randzeichen im lokalen Terminal-Thread aus. Alle bestanden;
auch 71 Terminal-, 13 Bildschirm- und sieben E1-Tests bestanden (überlappende
Testgruppen). Remote-Client-Resize, Zellbreiten-/Pixelverträge und sichere
Textausgabe bleiben separat zu klären; keine vollständige A5-/E3-Abnahme.

**Anschließend freigegeben und implementiert: `EventKind.Resize = 6`.** Die
bisherigen Enumwerte und EVENT-Felder bleiben unverändert. Vor der Auslieferung
sind logischer Bildschirm und ein neuer `Terminal.Info`-Snapshot aktuell;
gespeicherte Snapshots bleiben stabil. Nur `Kind` und `Time` sind relevant.
`Wait(-1)` wird ohne Tastendruck geweckt, identische Größen erzeugen nichts,
schnelle Änderungen dürfen auf den neuesten Stand zusammengefasst werden.
Resize erzeugt keine Eingabebytes und setzt die Idle-Zeit nicht zurück.
`Release()` verwirft anstehende Resize-Meldungen.

Quellen dieses Schritts sind Telnet NAWS und vorhandene logische
ANSI-/Board-Größenänderungen. Die logische NAWS-Größe ist entsprechend dem
ANSI-Renderer auf 132x60 begrenzt; rohe gemeldete Maße bleiben intern erhalten.
Meldungen mit einer Null-Dimension werden ignoriert. Der Transport behält seinen
Read-Vertrag: protokollreine Pakete werden nicht als Eingabe oder EOF ausgegeben.

Ein echter TCP-/Telnet-Socket speist ein serialisiertes PPE mit EN-/DE-Ausgabe.
Der bestandene Test prüft 80x25, 132x43 und die Begrenzung von 65535x65535 auf
132x60 anhand gerenderter Randzeichen, außerdem unbegrenztes/endliches Warten,
unveränderte Snapshots, Duplikate, Null-Maße, gemischte Resize-/Tastendaten,
Idle-Zeit, Aufrufer-Tokens und Verbindungsabbruch während `Wait(-1)`.
Logische Coalescing-/Release-Tests, der Telnet-Parser-Test, zwölf Enum-Tests und
der aktualisierte API-Katalog wurden ebenfalls erfolgreich geprüft.

Abschließend bestanden fünf A5-, 16 Telnet-, 38 Event-, 73 Terminal-,
13 Bildschirm-, sieben E1- und sechs lokale Terminal-Thread-Tests
(überlappende Gruppen). Die gesamte LSP-Suite, 45 Completion-Tests in einem
separaten deutschen Prozess und die Hilfeprüfung mit unabhängigen EN-/DE-Loadern
bestanden. `pplc`, `icboard` und `ppl-lsp` wurden neu gebaut; der unveränderte
Dateibrowser kompiliert mit diesem Compiler.

Weitere Transportquellen, Pixelgeometrie, sichere Text-/Layout-APIs und die
manuelle Remote-Client-Matrix bleiben separat; weiterhin keine vollständige
A5-/E3-Abnahme.

**STRIPATX-Originalabgleich und freigegebene Korrektur (2026-09-11):**
`PPL/VAR.CPP::cVARVAL::stripatx` und der ältere Pfad über
`MAIN/SCRIPT.C::removecodes` entfernen nur vollständige `@Xhh`-Farbcodes.
Der Benutzer verlangt ausdrücklich Originalkompatibilität. Der klassische
IcyBoard-Scanner beschädigte dagegen unvollständige/ungültige Tokens; die bisher
absichtlich konservierten Fehlererwartungen wurden deshalb ersetzt. Beide
Opcodes verwenden jetzt denselben Scanner, ohne ihre Signaturen oder
String-Speicherregeln zu verändern. Allgemeine Makros und ANSI-Sequenzen bleiben
wie im Original erhalten; dieser Schritt führt keine neue Terminal-API ein.

Der neue Regressionstest scheiterte vor der Korrektur und besteht danach über
serialisierte PPEs mit Sprache/Runtime 340/340, 350/350, 350/400 und 400/400.
Er prüft unvollständige, ungültige und überlappende Tokens, alle 484 Kombinationen
gültiger Hexziffern, unveränderte Makro-/ANSI-Texte und bestehende String-Limits.
Der moderne Unicode-Test besteht ebenfalls. Referenz ist der Originalquelltext;
ein neuer DOS-Referenzlauf wurde für diese Korrektur nicht durchgeführt.
Abschließend bestanden alle acht `ppl400_followup_api`-Tests und 15
String-Regressionen; Diagnosen und `git diff --check` waren ohne Befund.

**Vertagung durch den Benutzer (2026-09-11):** `Terminal.WriteText` wird als
sinnvolle spätere Erweiterung betrachtet, ist derzeit aber nicht erforderlich.
Keine Implementierung und kein Beta-Abnahmeblocker; vorhandene PPE-Schutzroutinen
bleiben bestehen. Textbreite, zellgenaues Kürzen und Pixelverträge sind damit
noch nicht entschieden oder zur Umsetzung freigegeben.

**Freigegebene Textbreiten-Bestandsprüfung (2026-09-11):** Nur Tests und
Dokumentation, keine neue API und keine Änderung an Renderer, `LEN`, `LEFT`
oder `Substring`. Die fünf Proben sind ASCII, Umlaute, ein vorgefertigtes `ä`,
`a` mit U+0308 und zwei breite CJK-Zeichen. Explizite EN-/DE-Beschriftungen und
80x25/132x43 werden jeweils im Bildschirminneren und an der rechten Kante geprüft.

Der neue Engine-Test besteht mit 40 Fällen über serialisierte PPEs,
UTF-8-/CP437-Ausgabebytes, Caller-Bildschirm und Replay der Ausgabe. `LEN` und
`.Len()` zählen Codepoints; `LEFT` und `Substring` können Basiszeichen und
kombinierende Zeichen trennen. Nicht vorhandene Substring-Positionen bleiben
wie bisher mit Leerzeichen aufgefüllt. UTF-8 erhält die Proben; CP437 bildet
Umlaute ab und ersetzt nicht darstellbare Codepoints durch `.`.

Weitere 20 Fälle bestehen über den tatsächlichen lokalen UTF-8-Terminal-Thread:
Auch dort belegt jeder Codepoint eine Zelle, einschließlich kombinierender
Zeichen und CJK. Der finale TUI-Frame wurde separat mit weiteren 20 Fällen über
`Tui::ui` und Ratatuis Test-Backend geprüft, mit zwei zusätzlichen Host-Zeilen
für die Statusleiste. Dabei zeigt sich eine konkrete Einschränkung: ASCII und
vorgefertigte Umlaute bleiben erhalten; der isolierte kombinierende Codepoint
und die breiten CJK-Zeichen fehlen im Frame. Die Einzelzeichen-Ausgabe mit
einer Spalte Platz lässt deren Zellen leer, während nachfolgende Marker ihre
Codepoint-Positionen behalten. Die Tests charakterisierten diesen damaligen Ist-Zustand,
keine gewünschte universelle Unicode-Semantik.

Die Ergebnismatrix steht im String-Abschnitt von `new_ppl.md`. Sichtbare Glyphen
im Host-Terminal, Fonts und manuelle Remote-Client-Vergleiche sind nicht geprüft.
Eine Renderkorrektur, ein allgemeiner Zellbreiten-/Kürzungsvertrag und
Pixelverträge benötigten zu diesem Zeitpunkt gesonderte Besprechung und Freigabe;
`Terminal.WriteText` bleibt vertagt. Keine vollständige A5-/E3-Abnahme.

**Anschließende Zielentscheidung des Benutzers (2026-09-11): echte
Unicode-Zellbreiten für UTF-8.** Der begrenzte Ein-Zellen-Vertrag mit sichtbarem
Ersatz wurde nicht gewählt. Dies legte zunächst den Zielvertrag fest, noch ohne
Freigabe für die Renderer-Implementierung. Die obige Bestandsmatrix beschreibt
den Zustand vor der unten dokumentierten Integration.

- CP437 bleibt unverändert: eine Zelle pro ausgegebenem Zeichen und `.` als
  Ersatz für nicht darstellbare Eingabe.
- Unter UTF-8 belegen ASCII und vorgefertigte Umlaute je eine Zelle;
  Basiszeichen plus kombinierende Zeichen werden gemeinsam dargestellt.
  `a` mit U+0308 belegt insgesamt eine Zelle, ein breites CJK-Zeichen zwei.
- Cursor und ANSI-Koordinaten beziehen sich auf Zellen. Bildschirmmodell,
  lokaler Terminal-Thread und TUI müssen dieselben Regeln für Cursorfortschritt,
  Umbruch und Überschreiben verwenden; eine reine Ratatui-Korrektur genügt nicht.
- `LEN`, `.Len()`, `LEFT` und `Substring` behalten ihre Codepoint-Semantik.
  Keine implizite Normalisierung und keine neue Breiten-/Kürzungs-API.

**Anschließend bestätigte UTF-8-Randfallregeln (2026-09-11):**

- Kombinierende Zeichen ohne Basis erhalten U+25CC (gepunkteter Kreis) als
  rein visuellen Träger in einer Zelle. Die Originalzeichenfolge bleibt erhalten.
- Mehrdeutige Zeichenbreiten gelten als schmal, also eine Zelle, unabhängig
  von EN/DE.
- Grapheme einschließlich Emoji-/ZWJ-/Variantenfolgen werden gemeinsam
  behandelt. Die Breite folgt `unicode-width`, nicht addierten
  Codepoint-Breiten; vollständig qualifizierte Emoji-ZWJ-Folgen belegen zwei Zellen.
- Paket- und `PRINT`-Grenzen trennen ein Graphem nicht. Explizite
  Cursorbewegungen oder Löschen beenden die Zuordnung zum vorherigen
  ausgegebenen Graphem.
- Bei Platzmangel wird ein breites Graphem mit automatischem Umbruch als
  Ganzes in die nächste Zeile verschoben. Ohne automatischen Umbruch wird
  es stattdessen durch einen einspaltigen Punkt ersetzt.
- Überschreiben oder Löschen einer belegten Teilzelle entfernt das gesamte
  breite Graphem, ohne Nachbartext zu verschieben.

**Anschließend freigegeben und integriert (2026-09-11):** Der Benutzer hat
Änderung und Push im regulären `icy_tools`-Repository und danach die
IcyBoard-Integration freigegeben. Alle vier Engine-/Parser-Abhängigkeiten sind
auf `588230b0` gebunden. Die neue Revision benötigt auch den dort geprüften
`unarc-rs`-Stand `e3c98114`; der ViewData-Adapter wurde an die neue Callback-
Signatur angepasst. Cargo-Checkouts wurden nicht verändert.

UTF-8-Caller und lokales Terminal aktivieren den Graphem-/Zellmodus mit
`unicode-segmentation` 1.13.3 und `unicode-width` 0.2.2. CP437, Sysop-Spiegel
und DOS-Ausgabe behalten den Ein-Zellen-Modus. Das TUI liest vollständige
Grapheme, überspringt Fortsetzungszellen und zeichnet keine am Host-Viewport
abgeschnittenen breiten Grapheme. SGR darf eine Graphemfolge fortsetzen und
behält deren Basisattribute. DCH verschiebt weiterhin Nachbarzellen;
ECH/EL/ED löschen ohne Verschieben.

Die aktualisierte PPE-Matrix besteht mit 40 Fällen einschließlich
`SAVESCRN`/`RESTSCRN`, UTF-8-/CP437-Bytes, Bildschirm-Replay und unveränderter
Codepoint-Semantik der Stringoperationen. 24 Fälle prüfen den tatsächlichen
Terminal-Thread mit einzeln verarbeiteten Bytes einschließlich Emoji-ZWJ.
32 Fälle führen ein serialisiertes PPE über getrennte `PRINT`-Aufrufe bis zum
Ratatui-Frame aus und vergleichen den Caller- mit dem lokalen Bildschirm.
Alle Matrizen verwenden explizite EN-/DE-Beschriftungen und 80x25/132x43.
Der Frame-Test prüft außerdem verwaiste kombinierende Zeichen, schmale
mehrdeutige Zeichen und einen abgeschnittenen Viewport; reguläre Frames
enthalten die beiden Statuszeilen. Beim Drei-Spalten-Test war deren Inhalt
zunächst ausgeschaltet, weil die normale Statusleiste dort einen unabhängigen
arithmetischen Unterlauf hatte (siehe Folgekorrektur unten). Direkte Byteausgabe und DOS-/UTF-8-Moduswechsel
sowie exakte S5-Ausgabe und Sysop-Replay bestehen ebenfalls.

Die zuvor im regulären Engine-Repository ausgeführten 297 Tests (16 bestehende
ignoriert) decken zusätzlich Variantenfolgen, Umbruch ohne Autowrap, breite
Überschreib-/Löschbereiche, Einfügen, Scrollen und Resize ab. Die Anzeige begrenzt
ein Graphem auf 4096 UTF-8-Bytes; Originalstring und Ausgabebytes bleiben davon
unberührt. `SCRTEXT` bleibt bewusst unverändert bei einem gespeicherten Zeichen
pro Zelle: keine kombinierenden Enden, Leerzeichen für breite Fortsetzungszellen.
Eine neue Graphem-Projektion dieser klassischen API ist nicht festgelegt.

Abschließend bestanden sieben A5-, 14 Bildschirm- und 74 Terminal-Tests der
IcyBoard-Engine sowie sieben lokale Terminal-Thread-Tests. Drei lokale A5-
und 17 TUI-/Sixel-Tests bestanden jeweils in getrennten EN-/DE-Prozessen
(überlappende Testgruppen). `cargo check --workspace` und der Neubau von
`icboard` bestanden ebenfalls; die geänderten Rust-Dateien sind formatiert.

**Statusleisten-Folgekorrektur (2026-09-11):** Der Drei-Spalten-PPE-/Frame-Test
reproduzierte den Unterlauf mit wieder aktivierter Statusleiste und besteht
nach der Korrektur ohne Ausnahme. Alle vier Statusansichten begrenzen ihre
Ausgabe auf das verfügbare Rechteck, einschließlich leerer und einzeiliger
Flächen. Zeiten erscheinen nur bei ausreichend Platz; Text, Hilfe und Zeiten
bekommen getrennte Spaltenbereiche. Ein eigener Renderingtest besteht mit
360 Kombinationen aus EN-/DE-Beschriftungen, 15 Breiten von 0 bis 132 Spalten,
drei Höhen und vier Statusansichten. Er prüft versetzte Rechtecke, unveränderte
Zellen außerhalb der Statusleiste sowie Hilfe-/Zeitpositionen bei Normalbreite.
Alle 18 lokalen TUI-/Sixel-Tests bestanden anschließend in getrennten EN-/DE-
Prozessen; `icboard` wurde neu gebaut und `git diff --check` blieb ohne Befund.

**Manuelle Rückmeldung des Benutzers (2026-09-11):** Die Darstellung scheint
remote zu funktionieren; der Benutzer hält den aktuellen Unicode-Layoutstand
für ausreichend. Dies gilt als positive Remote-Stichprobe, nicht als Nachweis
der vollständigen Client-/Font-/Geometriematrix. Konkreter Client, Font und
Prüffälle wurden nicht angegeben; ein separater visueller lokaler Hosttest
ist nicht dokumentiert. Ein zusätzliches Abnahme-PPE wird vorerst nicht erstellt.

Die automatisierten Tests und diese Rückmeldung ergeben keine universelle
Host-/Remote-Client-Garantie. CP437-Sysop- und UTF-8-
Caller-Koordinaten müssen nicht identisch sein. Pixelverträge, Breiten-/Kürzungs-
APIs und `Terminal.WriteText` werden nicht erweitert. Keine vollständige
A5-/E3-Abnahme und kein Commit/Push der IcyBoard-Integration in diesem Schritt.

**Besprechen:** Connection-Fähigkeiten versus aktuelle Größe; Resize-Ereignisse;
Zell-/Pixelkoordinaten; sichtbare Textbreite; ANSI-/ATX-bewusstes Layout;
sichere Ausgabe fremder Texte ohne Steuersequenzinterpretation.

**Abnahme:** Layout wird anhand gerenderter Terminalausgabe geprüft, nicht nur
an Parserzuständen. Wechselnde Geometrien, Unicode und einfache ANSI-Fallbacks
funktionieren gemäß beschlossenem Umfang. Nicht unterstützte Sixel-/JXL-Funktionen
werden nicht still als gleichwertig behandelt.

### A6 — PPE-eigene Datenhaltung und Utilities

**Freigegebener Abnahmefall und Bestandsprüfung (2026-09-11):** PPE-Einstellungen
pro Benutzer, zunächst ausschließlich mit vorhandenen APIs. Keine neue Runtime-
oder API-Implementierung in diesem Schritt. Die Probe verwendet einen expliziten
Versionsheader vor einem Record mit Zeilenzahl, Suchtext und Sortierrichtung;
Dateipfade und Versionskonvention werden vom PPE gewählt, nicht durch einen
neuen verwalteten Datenbereich bereitgestellt.

Drei neue Tests laufen über serialisierte PPEs und echte Dateioperationen:

- Zehn Ladefälle mit expliziten EN-/DE-Bezeichnungen: gültige, fehlende,
  fremdversionierte, beschädigte und abgeschnittene Dateien. Fehlende/ungültige
  Daten behalten die Standardwerte; unbekannte Versionen werden vor dem
  positionsabhängigen Record-Lesen abgewiesen. Keine automatische Migration.
- Neue PPE-Läufe lesen den zuvor gespeicherten Stand; getrennte Benutzerdateien
  bleiben unabhängig. Das Muster Temp-Datei im Zielverzeichnis, Fehlerprüfung,
  Schließen und `RENAME` erhält unter Linux die vorherige Datei bytegenau bei
  fehlgeschlagener Temp-Datei-Erstellung, Schreiben über einen Read-only-Kanal
  (Header und Record separat), fehlender Rename-Quelle und `STOP` vor dem Rename.
  Eine solche Unterbrechung lässt eine Temp-Datei zurück; ein nachfolgender
  erfolgreicher Lauf mit demselben Temp-Pfad funktioniert. Keine Prüfung von
  Prozessabsturz, Stromausfall, vollem Dateisystem oder Windows-Ersetzungssemantik.
- Zwei gleichzeitig laufende PPE-Sitzungen desselben Benutzers auf zwei Nodes
  wurden über Terminal-Eingabeereignisse deterministisch verschachtelt. Beide
  öffneten dieselbe Datei trotz `S_DB`, lasen den Wert 20 und speicherten je 21.
  Beide meldeten Erfolg; der Endstand war 21 statt 22. Diese ursprüngliche
  Charakterisierung wurde durch die folgende Sperrregression ersetzt.

Alle zehn Tests der betroffenen Record-I/O-Gruppe bestanden, einschließlich
der drei neuen A6-Tests. Der damalige `DiskIO`-Pfad ignorierte den Share-Modus;
`FFLUSH` ruft lediglich `File::flush` auf. Das getestete Temp-/Rename-Muster
ersetzt daher weder eine Sperre über Lesen, Ändern und Veröffentlichen noch
einen Vertrag für dauerhafte Speicherung bei Stromausfall.

**Freigegebener Sperrvertrag und Umsetzung (2026-09-12):** Der Benutzer hat
die Share-Korrektur und eine stabile, separate Sperrdatei für die gesamte
Transaktion freigegeben. `FOPEN`, `FCREATE` und `FAPPEND` prüfen nun die
Zugriffs-/Deny-Kombination in beiden Richtungen für alle PPE-Kanäle im selben
BBS-Prozess. Konflikte warten nicht, sondern melden `FERR` und
`Error.Last()` mit `ErrKind.File`/`ErrCode.IO`; eine verweigerte Schreiböffnung
kürzt die Datei nicht. Die portable Dateiidentität liefert `same-file`.
PPE-Signaturen, Share-Konstanten und Record-Formate bleiben unverändert.

Die Share-Reservierung gehört zum Kanal, nicht zum temporären Dateileser:
`FGET`-Pufferung, EOF und `FERR` geben sie nicht frei. `FCLOSE`, Verwerfen von
`DiskIO` und der Cleanup des äußeren PPE-Aufrufs geben sie frei. Der Cleanup
läuft auch bei `STOP`, echtem VM-Fehler, Disconnect und Future-Abbruch.

Der Zwei-Node-Test erwirbt jetzt eine separate Sperrdatei vor dem Lesen und
behält sie bis nach dem Temp-Datei-/Rename-Schritt. Der zweite Node erhält
zunächst einen kontrollierten Konflikt, liest nach Freigabe neu und speichert
22 statt 21. Die Sperrdatei bleibt bestehen; alle beteiligten PPEs müssen
dieselbe Sperrdatei verwenden, ohne sie zu ersetzen oder
zu löschen. Das Einstellungs-Speichermuster verwendet dieselbe Sperrkonvention.

Die 15 bestandenen `DiskIO`-Tests enthalten eine 144-Fälle-Matrix, Schutz vor
Kürzung, kompatible Mehrfachleser, gepufferte Textleser, EOF, Lesefehler,
Append, Close/Drop, Symlink-/Hardlink-/Pfadaliase und gleichzeitige Vergabe aus
acht Threads. Zwölf Record-I/O-Tests bestehen einschließlich Zwei-Node-
Transaktion, aller fünf PPE-Cleanup-Ausgänge mit weiterlebendem `DiskIO`,
echtem `FAPPEND` und bisherigen Einstellungs-/Fehlerfällen.

Zusätzlich ist der native, nicht UTF-8-kodierte Antwortdateipfad unter Unix
geprüft, einschließlich Share-Konflikt über einen Alias. Abschließend bestehen
alle elf A6-Tests in getrennten EN-/DE-Prozessen. Der Engine-Check ohne
Default-Features und der Neubau von `icboard` bestehen ebenfalls.

Der umfassendere Engine-Library-Lauf ergab 1957 bestandene, 14 ignorierte und
drei fehlgeschlagene Tests. Die Befunde liegen bei
`s1_all_catalog_placeholders_bind_without_replacing_live_objects` (24 erwartet,
26 Hosttypen vorhanden),
`s1_invalid_native_objects_do_not_grant_access_or_open_message_files`
(leere Argumentliste in `ppl_files::find`) und
`a_url_followed_immediately_by_a_color_macro_closes_both_cleanly`
(`ESC[1;37m` statt `ESC[37m`). Diese Stellen wurden im A6-Schritt nicht geändert;
der Gesamtlauf ist ausdrücklich nicht vollständig grün. Er erfolgte vor dem
abschließenden zusätzlichen Test des nativen Antwortdateipfads.

Die drei damaligen Befunde sind inzwischen reproduziert und korrigiert;
der aktuelle Nachweis und die separat geprüften Grammatikbefunde stehen unter
[Regressionen 2026-09-12](#regressionen-2026-09-12).

**Grenzen:** Kooperative PPE-Kanäle innerhalb eines BBS-Prozesses, keine
Betriebssystem-Sperre gegen externe Programme, DOS-Doors, weitere BBS-Prozesse
oder Rechner. Direkte Dateioperationen (`DELETE`/`RENAME`/`COPY`) nehmen nicht
am Share-Protokoll teil. Eine Sperre nur auf der ersetzten Datendatei genügt
nicht. Stromausfallfestigkeit und Windows-Ausführung sind nicht geprüft;
`FFLUSH` bleibt ohne fsync-Vertrag. Der freigegebene Mehrnode-Einstellungsfall
ist damit im geprüften Prozessumfang umgesetzt, nicht automatisch ganz A6
abgenommen. JSON, zusätzliche File-Objekte, UTC-Zeitpunkte und asynchrones HTTP
werden weiterhin nicht erweitert.

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

**Stand 2026-09-11: Dateiansicht bestätigt, Markierung implementiert; Download separat.**
Ohne vorhandenen Dateibrowser wurde ein eigenständiges PPE unter
[ppe/files](../ppe/files/src/main.pps) begonnen. Der freigegebene Umfang war
zunächst die Prüfung der bestehenden APIs und ein Verzeichnisbrowser; neue
öffentliche API-Erweiterungen benötigen eine eigene Freigabe.

Das PPE zeigt die zugänglichen Verzeichnisse der aktuellen Konferenz, mit
BBS-konformen Nummern ab 1, Downloadberechtigung, Neu-Markierung und Details.
Die Liste hat 15 sichtbare Zeilen, Tastaturnavigation, Suche nach Verzeichnisnamen
und Rücksetzen. Die Suche verwendet Terminal-Events statt `INPUTSTR`, damit
verbleibende Aufrufer-Tokens nicht als Eingabe verbraucht werden. Escape verwirft
die Suchänderung oder kehrt aus den Details zurück; aus der Liste beendet es das
PPE. Die Oberfläche ist für 80x25 ausgelegt, die Suche auf 48 ASCII-Zeichen begrenzt.

**Nachweis der ursprünglichen Grundlage:** Der damalige Lauf von
`cargo test-low -p icy_board_engine --lib e1_` bestand mit zwei Tests.
Der Browser-Test kompiliert und serialisiert das tatsächliche PPE,
führt es über die VM mit ChannelConnection aus und prüft gerenderte 80x25-Frames.
Er wählt Englisch und Deutsch explizit und prüft je eine leere, eine vollständig
gesperrte und eine große Liste mit 1.001 sichtbaren Verzeichnissen. Paging,
Home/End, Details, verweigerte Downloads als Statusanzeige, Suche, Backspace,
Abbruch, Eingabelimit, Rücksetzen, leere Treffer und Aufrufer-Token-Erhalt sind
abgedeckt. Lange Namen und literale `@CLS@`-/`@HANGUP@`-Texte bleiben im Rahmen
und lösen keine Makros aus. Der separate Paketbau mit
`target/debug/pplc ppe/files/ppl.toml --mono` besteht ebenfalls.

**Historische API-Lücke der Grundlage:** Das damalige `DIRECTORY` bot noch keine
Filebase-Einträge mit Beschreibung, Größe und Datum. Die Legacy-Probe bestätigt,
dass `FINDFIRST`/`FINDNEXT` auch Metadaten und Unterverzeichnisse enumerieren;
`FILEINF(..., 4)` liefert für eine 2-GiB-Datei `-2147483648`. Diese Legacy-Semantik
wurde nicht geändert. Das PPE liest keine privaten Filebase-Formate. Es zeigt
inzwischen Dateien und Beschreibungen über die unter A4 dokumentierte
FileEntry-/FilePage-API; der separate Download-Befehl ist oben beschrieben. Das Paging
der Verzeichnisse begrenzt nur die Anzeige, nicht den vollständig materialisierten
`Conference.Directories`-Snapshot; Dateiabrufe sind dagegen API-seitig begrenzt.

**Erweiterung und Rückmeldung (2026-09-11):** Enter öffnet aus den
Verzeichnisdetails die Dateiliste und von dort die Dateidetails. Dateiseiten,
Vorwärts-/Rückwärtsblättern, Suche nach Name/Beschreibung und lange Beschreibungen
sind angeschlossen. Escape führt jeweils eine Ebene zurück. Die befüllte lokale
Demo wurde vom Benutzer als passend bestätigt; diese Rückmeldung gilt für den
vorhandenen Anzeigeumfang, nicht als Transfernachweis.

Die bisherigen vier E1-Tests prüfen zusätzlich zur Grundlage
den verschachtelten `TOKENIZE`-/`CALL`-Start in EN/DE sowie gerenderte
Dateiansichten mit 2-GiB-Größe, wörtlichen `@`-Makros, langen Beschreibungen,
Seitenhistorie, Trefferfortsetzung nach einer leeren Such-Zwischenseite,
leerem Index und fehlendem Index. Hinzu kommen der Legacy-Auswahltest und der
gerenderte Markierungstest. `M` markiert die ausgewählte Datei über die unter A4
beschriebene `Directory.Flag(fileName)`-API und zeigt Erfolg oder Ablehnung.
Die Markierungen bleiben beim Verlassen des Browsers in derselben BBS-Sitzung
erhalten. Download bleibt gemäß Benutzerentscheidung eine separate Aktion:
`D` öffnet nun den bestehenden BBS-Downloadablauf und kehrt danach zum Browser
zurück. Der zusätzliche Downloadtest prüft diesen Übergang einschließlich
tatsächlicher lokaler Übertragung und Abbruch.

- Verzeichnisse und zugängliche Dateien anzeigen.
- Beschreibungen, Dateigröße und Datum darstellen.
- Suche und Paging; große und leere Bestände berücksichtigen.
- Exakt markieren; Download anschließend separat über öffentliche BBS-Funktionen,
  einschließlich Ablehnung/Abbruch.
- Keine Kenntnis interner Filebase- oder Konfigurationsformate voraussetzen.

**Abnahme:** Vollständiger Nutzerablauf, korrekte Zugriffsregeln, begrenzter
Ressourcenbedarf und brauchbare Darstellung in Englisch und Deutsch.

### E2 — Nachrichtenleser mit Antwortfunktion

**Status: erfüllt, vom Benutzer am 2026-09-11 ausdrücklich abgenommen.**
Die Konferenz-/Area-Auswahl bleibt im BBS-Bedienablauf; eine zusätzliche Auswahl
im Reader ist nicht gewünscht. Grundlage sind der bestehende Reader-Ablauf,
der unabhängige Editor-PPE und die oben protokollierten Tests und lokale Probe.
Verbleibende Testlücken sind dokumentiert, aber keine E2-Abnahmeblocker.

- Im BBS-Ablauf Konferenz/Area wählen; im Reader Header auflisten und Text lesen.
- Sparse Nachrichtennummern, leere Areas und gelöschte Nachrichten behandeln.
- Persönliche ungelesene Nachrichten und Last-read-Verhalten prüfen.
- Private und gesperrte Inhalte korrekt behandeln.
- Antwort schreiben; Attachment-Umfang vor Umsetzung ausdrücklich entscheiden.
- Parallele Änderungen durch andere Nodes berücksichtigen.

**Abnahmeziel:** Lesen → Markieren → Antworten ohne JAM-/TOML-Parser im PPE;
Header-/Body-Konsistenz und Benutzerberechtigungen prüfen. Der tatsächlich
nachgewiesene Umfang und seine Grenzen stehen im Abnahmeprotokoll oben.

### E3 — Interaktive Terminalanwendung

**Freigegeben und umgesetzt (2026-09-12): Paint als Abnahme-PPE.** Der Benutzer
hat Maus-/Tastaturzeichnen, Resize, Grafik plus ANSI-Fallback, EN/DE und
Ressourcen-Cleanup freigegeben. Audio und zusätzliche Speicherfunktionen bleiben
ausgeklammert. Die Umsetzung erweitert `ppe/paint`, nicht die öffentliche API
oder Runtime. Die folgende Client-Abnahme ist damit noch nicht abgeschlossen.

Paint verwendet eine gemeinsame, begrenzte Text-Werkzeugleiste mit Farbfeldern,
Löschen, Beenden und im Grafikmodus Pinselgröße. Das erste Aufrufargument `de`
wählt Deutsch, sonst Englisch. Pfeile bewegen den Zeichenpunkt, Leertaste
zeichnet, Delete/Backspace löscht dort, `1` bis `8` wählen Farben, `C` löscht
das Bild und `Q`/Escape beendet. Grafik unterstützt zusätzlich `+`/`-`, Mausrad
und freie Striche mit linker bzw. rechter Maustaste zum Zeichnen bzw. Radieren.
Werkzeuge sind auch anklickbar. Pixelmaus wird angefragt; ohne Bestätigung
werden Textmauskoordinaten verwendet, fehlende Mausfähigkeit verhindert die
Tastaturbedienung nicht.

Ohne Grafikfähigkeit startet eine benutzbare ANSI-Fläche mit acht DOS-Farben,
nicht bloß eine Fehlermeldung. Das ist ein zellbasierter Fallback, kein
verlustfreier Pixelrenderer. Der Backend-Modus wird beim Start gewählt; ein
Wechsel zwischen ANSI und Grafik während der Bearbeitung ist nicht enthalten.
Der Bildspeicher ist auf 128×64 ANSI-Zellen bzw. 1280×1024 RGBA-Pixel begrenzt.
Die sichtbare Fläche folgt der logischen Terminalgröße und den gemeldeten
Zellmaßen (bei fehlenden Zellmaßen 8×16); Resize schneidet die Ansicht zu,
löscht aber das Bild nicht. Die Grafik-Ansichtsfläche wird über ein geteiltes
Surface-Handle freigegeben und neu angelegt. Der Pinselcursor liegt nur auf
dieser Ansicht, nicht im Bild. Grafik verarbeitet höchstens 64 Ereignisse je
Ausgabebündel. JXL bleibt über die bestehende Auto-Auswahl erreichbar, wurde
in dieser Paint-Abnahme aber nicht geprüft.

**Automatisierter Nachweis:** Sieben Paint-Tests bestehen in getrennten EN-/DE-
Prozessen mit `CARGO_INCREMENTAL=0 cargo test-low -p icy_board_engine --lib paint_
--quiet`. Vier neue Tests ergänzen zwei vorhandene Pixelmaus-/Bündelungsfälle.
Hinzu kommt ein Grafik-Unit-Test für außerhalb liegende Paint-Cursorrechtecke.
Die interaktiven Tests kompilieren und serialisieren das tatsächliche PPE,
laden es über `run_ppe` von einer temporären Datei und bedienen eine echte
Telnet-Verbindung. Sie prüfen gerenderte ANSI-Farbzellen und aus der Ausgabe
dekodierte Sixel-Pixel, nicht nur PPE-Variablen:

- EN/DE, Zeichnen/Radieren, Pfeiltasten, Farbwahl, Werkzeugklicks und Pinselgröße;
  Sixel jeweils mit bestätigter Pixelmaus und mit Textmaus-Fallback.
- 80×25 → 132×43 → sehr kleine Geometrien bis 1×1 → 80×25, Erhalt des Bilds,
  korrekte Sixel-Positionierung und Wiederanlage der Ansichtsressourcen.
- Normales Ende, injiziertes `STOP`, echter Seek-VM-Fehler und Disconnect in
  ANSI/Sixel. Grafik, Maus-/Tastaturmodi, Updates und Margins sind danach frei;
  bei weiterbestehender Verbindung liest ein nachfolgendes PPE klassische
  Eingabe. Ein nicht von Paint benötigter Aufruftoken bleibt bis zum Ende im
  PPE erhalten; der äußere Datei-Start bereinigt seine Parameter wie bisher.
- Die beiden bestehenden Tests bestätigen weiterhin Pixelmaus-Probing und
  genau zwei Grafikframes für den gebündelten Mausablauf.

`target/debug/pplc ppe/paint/ppl.toml --mono` baut das Paket ohne Warnungen.
Die feste Toolbar-Bitmap wird nicht mehr benötigt oder mitkopiert; ihre
Quelldatei bleibt erhalten. Zwei aus Testausgabe gespeicherte Sixel-Bilder
wurden zusätzlich visuell geprüft. Das ist keine Prüfung echter Clientfonts,
der vollständigen Hostoberfläche oder realer Remote-Clients.

**Korrektur nach lokaler Probe (2026-09-12):** Der gemeldete Panic in
`clipped_rect` wurde mit einem Cursorrahmen am unteren rechten Rand einer
624×320-Fläche reproduziert. `then_some` wertete die unsigned-Subtraktionen
auch für vollständig außerhalb liegende Rechteckkanten aus; die verzögerte
Auswertung mit `then` verhindert den Überlauf. Der Unit-Test deckt außerdem
vollständig außerhalb liegende Füllungen, Rahmen und Bildausschnitte ab.
Das tatsächliche Paint-PPE prüft nun alle vier Ecken bei 80×25 und 132×43,
jeweils EN/DE mit Pixel- und Textmaus sowie Pfeiltasten, anhand dekodierter
Sixel-Pixel einschließlich abgeschnittener Cursorrahmen. Alle sechs
Grafik-Unit-Tests und die sieben Tests der Paint-Gruppe in separaten EN-/DE-
Prozessen bestehen; `icboard` wurde neu gebaut. Der Benutzer bestätigte
anschließend die Wiederholungsprobe als funktionierend. Clientversion und
Geometrie wurden dabei nicht separat protokolliert.

**Noch offen:** Manuelle Clientversionen/-geometrien für SyncTERM, icy_term und
ein einfaches ANSI-Terminal; JXL-Paint, abweichende Fonts/Zellmaße und
Pixelgrößenänderungen ohne logisches Resize sind nicht nachgewiesen. Der
umfassende Engine-Library-Lauf wurde im Paint-Schritt nicht wiederholt;
die drei zuvor dokumentierten Fehler wurden erst im separaten
[Regressionsschritt](#regressionen-2026-09-12) bearbeitet. Die erfolgreiche lokale
Wiederholungsprobe ersetzt keine vollständige E3-Client-Matrix.

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

### Regressionen 2026-09-12

**Freigabe und Umfang:** Ausgehend von `cb4a662b` hat der Benutzer die drei
bekannten Engine-Testfehler zur Reproduktion und gezielten Korrektur sowie
anschließende Engine-Läufe in EN/DE freigegeben. Alle drei Fehler wurden vor
der jeweiligen Korrektur erneut mit dem einzelnen Test reproduziert.

- `Directory.Find` prüft die erforderliche Suchanfrage vor dem Zugriff auf
  das erste Argument. Ein direkter Hostaufruf ohne Argumente liefert nun
  einen ungültigen `FilePage` und `File/Invalid` statt eines Panics.
  Der Hostdefault-Test prüft für Dateioperationen die File-Fehlerdomäne und
  löscht den Fehlerstatus zwischen direkten Aufrufen. Andernfalls bewahrt
  `set_error` absichtlich den ersten Fehler des vermeintlich selben Statements.
  Rückgabetyp, Dimension und neu gesetzter Fehlerstatus werden mitgeprüft.
- Der Katalogtest erwartet 26 statt 24 Hosttypen: `FileEntry` und `FilePage`
  waren bereits registriert. Die Bindungs- und Identitätsprüfungen für alle
  Typen in ein- bis dreidimensionalen Arrays bleiben erhalten und bestehen.
- Der Hyperlink-Test erwartet für `@X0F` korrekt `ESC[1;37m`: Nach dem
  vorangegangenen Reset wäre `ESC[37m` nicht hellweiß. Die Farbausgabe selbst
  wurde nicht geändert. Neben der exakten Ausgabe einschließlich beider
  OSC-8-Abschlüsse prüft der Test jetzt gerenderte Zeichen und DOS-Farben auf
  einem 80×25-Terminal. Dabei wird die CRLF-Normalisierung des Testhelpers
  vor dem Replay rückgängig gemacht.

**Validierung:** Die drei gezielten Tests bestehen. Anschließend wurde
`CARGO_INCREMENTAL=0 cargo test-low -p icy_board_engine --no-fail-fast --quiet`
in getrennten Prozessen mit `LANG`/`LC_ALL` auf `en_US.UTF-8` bzw. `de_DE.UTF-8`
und passendem `LANGUAGE` ausgeführt. Die Library besteht in beiden Prozessen
mit jeweils **1966 bestandenen, 0 fehlgeschlagenen und 14 ignorierten Tests**.
Die ignorierten Tests gelten nicht als bestanden. Formatprüfung der drei
geänderten Rust-Dateien und `git diff --check` bestehen ebenfalls.

**Zwischenstand vor dem Grammatik-Abgleich:** Beide vollständigen Engine-Läufe scheiterten
ausschließlich im Integrationstest `grammars_know_every_type_name` aus
`editor_grammars`: `FILEENTRY` und `FILEPAGE` fehlen in `grammar.js`.
Das Ziel hat jeweils sechs bestandene und einen fehlgeschlagenen Test;
alle übrigen ausgeführten Engine-Testziele bestehen. Der nachfolgende
TextMate-Typvergleich dieses Tests wird wegen der ersten Assertion nicht
erreicht und ist damit noch nicht nachgewiesen. Die Grammatikdateien wurden
in diesem auf drei Regressionen begrenzten Schritt nicht geändert.
Der anschließend separat freigegebene Abgleich ist unten dokumentiert.

**Grammatik-Abgleich, separat freigegeben am 2026-09-12:** `FILEENTRY` und
`FILEPAGE` wurden in Tree-sitter und TextMate ergänzt. Der gezielte
Katalogtest reproduzierte zunächst die Tree-sitter-Lücke und nach deren
Korrektur dieselbe Lücke in TextMate; mit beiden Ergänzungen besteht er.
Die eingecheckten Dateien `parser.c`, `grammar.json` und `node-types.json`
wurden mit Tree-sitter 0.25.10 regeneriert. Eine zweite Generierung ergibt
identische SHA-256-Prüfsummen für alle drei Artefakte.

Der neue Parser-/Highlight-Test prüft beide Typen in Groß-, Klein- und
Mischschreibung als `builtin_type` und `type.builtin`. Mit dem alten
generierten Parser schlug er fehl (`type_identifier` statt `builtin_type`),
mit dem neuen besteht er. Alle 34 Tree-sitter-Korpusfälle und beide
Rust-Library-Tests einschließlich Query-Kompilierung bestehen. TextMate-JSON,
Rust-Testformatierung und `git diff --check` sind ebenfalls geprüft.

Die vollständigen Engine-Läufe mit obigem Befehl wurden danach in EN/DE
wiederholt: **Alle Engine-Testziele bestehen**, einschließlich der sieben
Tests aus `editor_grammars`; die Library weiterhin mit 1966 bestandenen,
0 fehlgeschlagenen und 14 ignorierten Tests je Sprachprozess.

**Tree-sitter-Befund vor der Folgekorrektur:** Der separate Lauf
`cargo test-low -p tree-sitter-ppl --quiet` hatte drei bestandene und einen
fehlgeschlagenen Repository-Test. `every_source_in_the_repository_parses`
findet Fehler in zwei von 333 Quellen: `language_core.pps:42:9` unter
`crates/icy_board_engine/tests/stored_ppe` und `ppe/files/src/main.pps:135:13`.
Beide Fehler sind mit den unveränderten Parserartefakten aus `cb4a662b` in
einem temporären Verzeichnis an exakt denselben Stellen reproduziert.
Sie wurden nicht durch die Typ-Ergänzung verursacht. Nach dem gescheiterten
Repository-Test wurden die Rust-Doctests dieses Crates damals nicht ausgeführt.

**FOR-Abschlusskorrektur, separat freigegeben am 2026-09-12:** Beide
Originalfehler haben dieselbe Ursache: Die rechtsassoziative FOR-Regel band
den Namen der nächsten Zuweisung vorzeitig als optionalen Abschlussnamen an
`NEXT`. Bei `item.flag = TRUE` blieb dadurch der Punkt übrig, bei
`selected = 0` wurde eine falsche Deklaration begonnen. Ein neuer Strukturtest
reproduzierte den Fehler unabhängig von den beiden vollständigen Quellen.

Die FOR-Regel verwendet jetzt einen gezielten GLR-Konflikt statt statischer
Rechtsassoziativität. Beide Parsewege bleiben verfügbar; eine dynamische
Gewichtung bevorzugt einen gültigen expliziten Abschlussnamen, etwa
`NEXT index`, ohne Folgezuweisungen zu verschlucken. Der Regressionstest
prüft acht Kombinationen aus `NEXT`, `NEXT index`, `ENDFOR`, `END FOR`
und einfacher bzw. Member-Zuweisung. Er verlangt zwei getrennte Anweisungen,
den richtigen Abschlussnamen und das unveränderte Zuweisungsziel.
Die PPE-Quellen und der Compiler wurden nicht geändert.

Die Parserartefakte wurden erneut mit Tree-sitter 0.25.10 generiert; eine
weitere Generierung liefert identische SHA-256-Prüfsummen. Der vollständige
Tree-sitter-Crate-Lauf besteht nun mit **2 Library-Tests, 5 Repository-Tests
und 1 Doctest**, ohne Fehler oder ignorierte Tests. Der Repositorylauf mit
333 gefundenen Quellen besteht einschließlich der beiden Originalfälle.
Alle **34 Korpusfälle** und die jeweils **7 Editorgrammatik-Tests in EN/DE**
bestehen ebenfalls. Rust-Testformatierung, Diagnosen und `git diff --check`
sind sauber. Die vollständigen Engine-Läufe wurden in diesem reinen
Parser-Schritt nicht wiederholt; ihr vorheriger Nachweis steht oben.
Das gesamte Release-Gate bleibt offen, ebenso E1-Abnahme und E3-Client-Matrix.

### Go/No-Go-Checkliste

- [x] P0 abgeschlossen; stabiler Build und reproduzierbare Baseline.
- [x] S1–S6 jeweils einzeln besprochen; freigegebene Änderungen umgesetzt.
- [x] C1 separat entschieden, C2 umgesetzt; anschließend S7 abgeschlossen.
- [x] F1: Unicode-Datei-Roundtrip nachgewiesen (S5, vorgezogener UTF-8-Literalteil).
- [x] F2: Beschlossenes Größen-/Limitkonzept umgesetzt und an Grenzen getestet.
- [x] F3: Host-Enum- und API-Evolution mit alten PPE-Dateien nachgewiesen.
- [x] F4: Stale Handles bleiben auch nach Wiederverwendung ungültig.
- [x] F5: `Fade`-Vertrag einschließlich realer Ausgabe konsistent.
- [x] F6: Metadatenzugriff skaliert unabhängig von vollständigen User-Snapshots.
- [ ] E1 Dateibrowser abgenommen.
- [x] E2 Nachrichtenleser mit Antwortfunktion abgenommen.
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