# PPL 400: Plan bis zum Ende der Beta

Stand: 2026-09-09. Zieltermin: ungefähr 2026-10-09.

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
- Die Sprachnachschärfungen werden einzeln entschieden und die freigegebenen
  Änderungen umgesetzt, **bevor der PPE-400-Container besprochen und geändert wird**.
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
| F1 | Unicode-Literale werden über den klassischen CP437-Konstantenpfad gespeichert; nicht darstellbare Zeichen können verändert werden. | Nicht-CP437-Text über Quelle, Compiler, PPE-Datei, Loader und Ausführung verlustfrei erhalten; Legacy-Kodierung bewahren. | Sprachvertrag in S5, Formatlösung nach C1, Umsetzung C2. |
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

Status: offen; erst nach S1 einzeln besprechen.

**Besprechen:** Explizite kurzschließende Operatoren, etwa `ANDALSO`/`ORELSE`,
gegen eine versionsgebundene Änderung bestehender Operatoren abwägen. Namen,
Präzedenz und Typregeln festlegen. Legacy-Auswertung nicht still ändern.

**Abnahme:** Seiteneffekte beweisen, wann der rechte Operand ausgeführt wird und
wann nicht. Compileroptimierung, VM, Decompiler, Formatter und LSP stimmen überein.

### S3 — Array- und `VAR`-Verträge schärfen

Status: offen; erst nach S2 einzeln besprechen.

**Besprechen:**

- Copy-in/copy-out und Rückschreibreihenfolge bei Alias-Argumenten.
- Warnung oder Fehler beim mehrfachen Übergeben desselben beschreibbaren Ziels.
- Anfangsgröße einer Variablen versus feste Form eines Record-Felds.
- Bounds versus Elementanzahl, leere Arrays, Rank und Verhalten bei Rückgaben.
- Kanonische Schreibweise mit `[]`, ohne unnötige Legacy-Brüche.

**Abnahme:** Rekursion, Alias-Argumente, Resize, leere Rückgaben und feste
Record-Formen sind durch Compiler-/VM-Tests abgesichert. Dokumentation benutzt
einheitliche Begriffe; keine unbeschlossene Umstellung auf Referenzsemantik.

### S4 — Eigene Enums und erweiterbare Host-Enums unterscheiden (F3)

Status: offen; erst nach S3 einzeln besprechen.

**Besprechen:**

- Geschlossene eigene Enums beibehalten oder begründet ändern.
- Für Host-Enums offene nominale Werte, `Unknown`-Behandlung oder versionierte
  API-Profile vergleichen; die bloße Existenz eines `Unknown`-Members löst nicht
  automatisch die Behandlung aller zukünftigen Zahlenwerte.
- Verhalten unbekannter Event-/Fehlerwerte bei Zuweisung, Vergleich und Fallunterscheidung.
- Host-Typidentität und dateilokale Typnummern konzeptionell trennen.

**Abnahme:** Ein altes Programm kann mit einer neueren Runtime entsprechend dem
beschlossenen Vertrag umgehen. Neue Werte führen nicht überraschend vor der
eigenen Fallback-Behandlung zum Abbruch. Eigene nominale Typen bleiben getrennt.

Die endgültige Binärrepräsentation dieser Entscheidung wird erst in C1 festgelegt.

### S5 — Text-, Binär- und Positionsverträge (F1)

Status: offen; erst nach S4 einzeln besprechen.

**Besprechen:**

- Unicode-Text und Binärdaten, Literaltypen und Konvertierungsgrenzen.
- Bedeutung von Zeichenposition, Byteposition, Graphem und Terminalzelle.
- Nullbasierte moderne Member versus klassische einbasierte Funktionen.
- Umgang mit nicht darstellbaren Zeichen bei tatsächlicher CP437-Ausgabe.

**Abnahme:** Ein eindeutiger Sprachvertrag und Regressionen mit `€`, CJK,
kombinierenden Zeichen und Nicht-BMP-Zeichen. `é` allein ist kein geeigneter
Nicht-CP437-Test. Die noch offene Datei-Roundtrip-Korrektur wird als F1 nach C2
übernommen, nicht als bereits erledigt markiert.

### S6 — Fehlerfluss und strukturiertes Cleanup

Status: offen; erst nach S5 einzeln besprechen.

**Besprechen:**

- `ON ERROR` am Anweisungsende, erste Fehlerursache und spätere Seiteneffekte.
- Abwesenheit, EOF, ungültige Ressource, operative Fehler und fatale VM-Fehler.
- Ob ein kleiner Cleanup-Mechanismus wie `DEFER` jetzt sinnvoll und zeitlich
  tragbar ist; keine große Exception-Hierarchie ohne konkreten Bedarf.
- Reichweite bei Routineende, `EXIT`, `STOP`, Fehler und Disconnect.

**Abnahme:** Beschlossene Regeln sind eindeutig getestet. Ein bewusst vertagtes
Sprachfeature wird als vertagt dokumentiert, nicht als implementiert dargestellt.

### S7 — Sprachentscheidungen zusammenführen

Status: offen; eigener Besprechungspunkt nach S1–S6.

**Besprechen:** Zusammenspiel der Änderungen, offen gebliebene Entscheidungen,
notwendige Layoutinformationen und realistische Restzeit.

**Abnahme:** Die freigegebenen Sprachänderungen sind umgesetzt und soweit ohne
neues Dateiformat möglich getestet. Formatabhängige Abnahmen sind ausdrücklich
aufgelistet. Erst danach C1 beginnen.

## Containerformat: eigener Schritt nach den Sprachänderungen

### C1 — PPE-400-Formatentscheidung (F1/F2/F3)

Status: offen; **keine Umsetzung ohne eigene Besprechung und Freigabe**.

**Zentrale Frage:** Welche Änderungen sind für die beschlossene Sprache und
langfristige PPE-Nutzung wirklich erforderlich, und welche wären unnötiger Umbau?

**Zu vergleichen:**

1. Bestehendes Format mit klar versionierten Erweiterungen behalten.
2. Sektionierten 400-Container einführen, aber VM und wesentliche Bytecodes behalten.
3. Container plus ausgewählte Operanden/Routinedeskriptoren modernisieren, wenn
   ein neuer Header allein die erforderlichen Grenzen nicht aufhebt.

**Entscheidungspunkte:**

- UTF-8-Konstanten mit expliziter Länge, Binärkonstanten und eingebettete NULs.
- Zielgrößen für Code, Deklarationen, Sprungziele, Typen, Felder und Routinen.
- Bestehende API-IDs versus Import-/Bindungstabelle mit stabiler Host-Identität.
- Darstellung der neuen Record-/Array-/Enum-Verträge.
- Sektionen, Längen, Pflicht-/optionale Daten und unbekannte Erweiterungen.
- Explizite Kompressionsangabe und strikte Loader-Validierung.
- Formatversion, Sprachversion und Mindest-API-Anforderungen getrennt behandeln.
- Optionale Debug-Daten; keine Pflicht zur Veröffentlichung von Source-Namen.
- Grenzen für Datei-, Speicher- und Laufzeitressourcen.
- Legacy-Loader beibehalten und beide Formate nach Möglichkeit in dieselbe
  interne Ausführungsdarstellung überführen.

**Abnahme:** Schriftliche Entscheidung mit Alternativen, genauem Umfang,
Kompatibilitätsmatrix, Aufwand und Testfällen. Kein Komplettumbau allein wegen
des Wunsches nach einem „modernen Format“.

**Terminregel:** Ist die freigegebene Lösung im Zeitfenster nicht belastbar
umsetzbar, Releaseumfang oder Termin ausdrücklich neu besprechen. Nicht still
eine provisorische Binärschnittstelle als langfristig stabil veröffentlichen.

### C2 — Freigegebenes Format implementieren und absichern

Status: offen; eigener Freigabepunkt nach C1.

**Arbeit:** Ausschließlich die in C1 beschlossene Lösung implementieren;
Compiler, Serializer, Loader und Decompiler gemeinsam aktualisieren.

**Abnahme:**

- F1: Quelle → PPE-Datei → Loader → VM → passende Terminalausgabe erhält Unicode.
- F2: Programme jenseits der alten Codegrenze funktionieren; neue Grenzen haben
  eindeutige Diagnosen und Tests unmittelbar unter, auf und über der Grenze.
- F3: Alte/neue Host-Enum- und API-Profile verhalten sich wie beschlossen.
- Die durch S1–S6 benötigten Konstrukte überleben echte Datei-Roundtrips.
- Beschädigte, abgeschnittene, übergroße oder unbekannte Pflichtdaten werden
  kontrolliert zurückgewiesen; optionale Erweiterungen gemäß Vertrag behandelt.
- Bestehende Legacy-PPEs behalten ihre Ausführung.

## Runtime- und API-Arbeitspakete

Auch die folgenden Schritte werden jeweils einzeln besprochen und freigegeben.
Eine Reihenfolgeänderung ist möglich, aber nur ausdrücklich; C1 bleibt nach S7.

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
| Woche 1 | P0; S1–S6 einzeln besprechen und freigegebene Sprachänderungen umsetzen; S7 | Keine offenen, unbeabsichtigten Sprachverträge in die Formatentscheidung mitnehmen. |
| Woche 2 | C1 separat entscheiden; C2 sowie R1/R2; A1–A3 | F1–F6 bearbeitet beziehungsweise konkret terminiert; Format-/ABI-Roundtrips belastbar. |
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
- [ ] S1–S6 jeweils einzeln besprochen; freigegebene Änderungen umgesetzt.
- [ ] S7 abgeschlossen; Container erst danach separat entschieden.
- [ ] F1: Unicode-Datei-Roundtrip nachgewiesen.
- [ ] F2: Beschlossenes Größen-/Limitkonzept umgesetzt und an Grenzen getestet.
- [ ] F3: Host-Enum- und API-Evolution mit alten PPE-Dateien nachgewiesen.
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