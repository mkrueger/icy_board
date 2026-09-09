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

Status: offen; zuerst besprechen.

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

Status: offen; einzeln besprechen und freigeben.

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

- [ ] P0 abgeschlossen; stabiler Build und reproduzierbare Baseline.
- [ ] S1–S6 jeweils einzeln besprochen; freigegebene Änderungen umgesetzt.
- [ ] S7 abgeschlossen; Container erst danach separat entschieden.
- [ ] F1: Unicode-Datei-Roundtrip nachgewiesen.
- [ ] F2: Beschlossenes Größen-/Limitkonzept umgesetzt und an Grenzen getestet.
- [ ] F3: Host-Enum- und API-Evolution mit alten PPE-Dateien nachgewiesen.
- [ ] F4: Stale Handles bleiben auch nach Wiederverwendung ungültig.
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