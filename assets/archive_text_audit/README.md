# Textwerbung in der gesamten Archivsammlung

## Implementierungsstand nach dem Audit

Die nachfolgende Bestandsaufnahme beschreibt den ursprünglichen Forschungslauf.
Inzwischen sind separate Ganzdatei-Textregeln implementiert: siehe
[Regelformat und Bedienung](../../docs/upload_processing.md#whole-file-text-templates),
[editierbare Standardregeln](../upload_ad_files.toml) und
[Rohdaten-Gegenprüfung](text-rule-trial/README.md).
Die vier geprüften Vorlagen wurden nach Benutzerfreigabe auf `auto_clean` gesetzt;
vorhandene Hashregeln bleiben unverändert. Die Rohdaten-Gegenprüfung dokumentiert
den vorherigen `report_only`-Stand mit identischen Mustern. Weder Quellarchive
noch aktive Boardkonfiguration wurden angepasst.

## Ergebnis

**Ja: Bytehash und Größe reichen für variable Textwerbung nicht aus.** Der
Korpus enthält unterschiedliche Werbevorlagen, identische Werbung unter vielen
Dateinamen und einen ausdrücklich als automatisch erzeugt gekennzeichneten
Werbetext mit Upload-Metadaten. Für CP437/UTF-8-Äquivalenz existiert ein
synthetischer Test, aber kein entsprechendes reales Anzeigenpaar in diesem Scan.

Dies ist ein **Forschungsbericht, keine Liste freigegebener Löschregeln**.
Produktive Inhaltsmatcher, Kataloge und Upload-Konfiguration wurden in dieser
Auditphase nicht geändert. Originalarchive, Testboard und dessen Datenbank wurden
nicht verändert. Enthaltene Programme wurden nicht ausgeführt.

## Umfang und Grenzen

Quelle war `/home/mkrueger/work/bbs/bbsarchives`, nicht nur die acht zuvor
untersuchten PCBoard-Verzeichnisse. Alle 17.292 Quelldateien wurden inventarisiert,
versucht und vor/nach dem Scan per SHA-256 als unverändert bestätigt.

| Metrik | Ergebnis |
|---|---:|
| Quelldateien | 17.292 |
| Archivöffnungsversuche auf oberster Ebene | 17.244 |
| Besuche erkannter verschachtelter Archive | 4.840 |
| Gesehene Member | 350.653 |
| Als Text inventarisierte Vorkommen | 174.772 |
| Unterschiedliche Text-Rohhashes | 128.911 |
| Unterschiedliche normalisierte Texte | 127.384 |
| Unterschiedliche Texte nach zusätzlicher Ziffernersetzung | 125.849 |
| Gelesene/angerechnete Bytes | 1.412.269.691 |
| Gespeicherte Roh-/Normalisierungsblobs | 1.072.935.426 |

**Status: partial, aber kein globaler Scanabbruch.** Die 313 Fehler enthalten
63 nicht lesbare Quellarchive; diese Zahlen dürfen nicht addiert werden.
Zusätzlich protokolliert: 255 verschlüsselte Einträge, 48 nicht unterstützte
Quelldateien, 630 Member-Größenlimits, sieben Eintragslimits und ein Tiefenlimit.
Globales, Archiv- und Blob-Speicherlimit wurden nicht ausgelöst.

Grenzen: 128 KiB je Textkandidat, 16 MiB je verschachteltem Container,
10.000 Einträge je Archiv, Tiefe drei unterhalb der obersten Ebene, 128 MiB
je Archiv und 2 GiB globales Lesebudget, 1 GiB Roh-/Normalisierungsblobs.
Metadatenberichte sind nicht Bestandteil des Blob-Budgets. Die Textheuristik
verlangt mindestens 32 Bytes, zwölf Buchstaben, 15 Prozent Buchstaben und
höchstens fünf Prozent bestimmte Steuerbytes; NUL und erkannte Binärformate
werden ausgeschlossen. Reine Grafik, kleine Anzeigen und große Texte können
dadurch fehlen. Bibliotheksinterne Dekomprimierungsallokationen sind nicht
vollständig durch diese Grenzen abgesichert; das Werkzeug ist keine Sandbox.

Der Befund ist also keine vollständige Werbezählung aller Archivbytes.
Vorkommen zählen auch Kopien in mehreren Archiven und verschachtelte Dateien.

## Konkrete Varianten

Die Tabelle gruppiert nach Dateinamen, nicht nach einer bewiesenen Werbeidentität.
Die vollständigen normalisierten Vergleichstexte stehen in
[variants.md](variants.md), Kennzahlen in [families.tsv](families.tsv).

| Gruppe | Vorkommen | Rohvarianten | Normalisierte Varianten | Beobachtung |
|---|---:|---:|---:|---|
| WORKSHOP.BBS | 95 | 10 | 10 | Clipper Workshop: Kapazitäten, Node-Zahlen, Datei-/Konferenzzahlen, Telefonnummern und Vorlagen ändern sich. |
| CEOS.AD | 56 | 7 | 6 | Cutting Edge Online: 1,2/13,2 GB, 50/80 Doors, Modembezeichnungen und zusätzliche Angebotszeilen. |
| PCM.805 | 2.122 | 9 | 9 | Pacific Coast Micro: mehrere Layout-/Angebots-/Kontaktfassungen. |
| PCM.NFO | 1.015 | 7 | 6 | Überschneidung mit PCM.805; zusammen 3.137 Vorkommen, 13 Roh- und zwölf normalisierte Varianten. |
| PCMICRO.BBS | 266 | 4 | 4 | Weitere gleichnamige Textvarianten, keine automatische Gleichsetzung mit obiger Gruppe. |
| ARCHIVES.BBS | 710 | 11 | 10 | Mehrere Texte; der Dateiname allein identifiziert keine konkrete Vorlage. |
| OUT.AD | 36 | 15 | 15 | Gegenbeispiel: Eigenwerbung von OutWorld Arts in eigenen Artpacks. |
| TCS_AD.TXT | 26 | 5 | 5 | Weiterer Kandidat für manuelle Prüfung, nicht freigegeben. |

### Änderungen jenseits von Datum und Kodierung

Clipper Workshop nennt je nach Fassung drei, elf oder 16,4 GB und unter anderem
zehn, neun oder zwölf Nodes. Auch „Network Headquarters“ und „FileNet
Headquarters“ sind unterschiedliche Vorlagen. Eine einzige großzügige Regex
mit beliebig ausgelassenen Absätzen wäre kein sicherer Ersatz für die Hashes.

Bei Cutting Edge Online bleiben in betrachteten Fassungen Identität und
Kontaktblock erkennbar, während Angebotsblöcke wechseln. Das belegt variable
Vorlagen, **nicht**, dass sämtliche Änderungen automatisch erzeugt wurden.

### Eindeutiger Generatorbeleg: Buggerer Deluxe

Fundstelle relativ zur Quelle:
`Ansi-Art/ciapak08.zip![65]:CIA-FTR3.ZIP![6]:FFOZOUHM.TXT`.
Die Indizes machen auch doppelte Membernamen eindeutig.

- Zufällig wirkender Dateiname statt eines festen Werbenamens.
- Board-Anzeige für „'da stadium“.
- Separater `description:`-Block mit kopierter Beschreibung des Footer-PPE.
- Uploadzeile mit Archivname `cia-ftr3.zip`, 6.426 Bytes, Uhrzeit `19:17`,
  Datum `01-28-94` und Node `3`.
- Expliziter Erzeugerhinweis „created by buggerer deluxe“.

Roh-SHA-256:
`156e98b129c46e2554ad6210bdff442c3bc4d02ba002c657518f2afc75e0f3a1`.
Der vollständige normalisierte Beleg steht in
[dynamic-generator.md](dynamic-generator.md).

Der gezielte Generator-Suchmarker trifft genau einen inventarisierten Text.
Die Generatorselbstauskunft und die variablen Felder sind belegt; mehrere
Zeitstempelvarianten dieses Generators wurden hier nicht gefunden.
Da sogar eine Programmbeschreibung eingebettet ist, sollte eine spätere Regel
die **gesamte bekannte Generatorstruktur** prüfen, nicht nur den Boardnamen.

### Umbenennen braucht schon heute keinen neuen Hash

Ein konkreter, rein werblicher Durchlaufhinweis „The BBS Archives“ liegt
byteidentisch **2.301-mal unter 38 verschiedenen Basisnamen** vor, beispielsweise
ARCHIVES.BBS, DOORS.BBS, PCMICRO.NFO, CONCORD.RIP und LORD.IGM.

Roh-SHA-256:
`71d056256e41d4ff70832abfe8c37d449ef45f0265bd1d05e0393a339b37cb4a`.
Beispielfundstelle: `Ansi-Art/765no0.zip![11]:ARCHIVES.BBS`.
Der Text enthält das Boardlogo, den Durchlaufhinweis und die Ressourcen-URL.
Das ist nicht der aktuell voreingestellte ARCHIVES.BBS-Memberhash; hier wurde
kein zusätzlicher Hash aktiviert.

Ein exakter Fingerprint ignoriert bereits heute den Dateinamen. Ein Hash pro
Namensvariante wäre unnötig. Globs wären nur eine zusätzliche Eingrenzung von
Textregeln, kein Ersatz für deren Inhaltsprüfung.

## Kodierung und Vergleichsnormalisierung

| Erkennung | Vorkommen | Unterschiedliche Rohtexte |
|---|---:|---:|
| ASCII | 105.687 | 79.746 |
| CP437 angenommen | 69.074 | 49.160 |
| Gültiges UTF-8, nicht ASCII | 11 | 5 |

Es gibt **keine normalisierte Gruppe mit unterschiedlichen erkannten
Kodierungskategorien** und insbesondere kein beobachtetes CP437/UTF-8-Werbepaar.
CP437 ist eine Annahme nach fehlgeschlagener UTF-8-Prüfung; auch zufällig gültige
UTF-8-Bytefolgen sind möglich. Diese Zähler beweisen keine ursprüngliche Kodierung.

Der Forschungsscanner validiert UTF-8 einschließlich BOM-Nutzdaten, verwendet
sonst CP437, entfernt CR, ANSI-CSI-Folgen und PCBoard-`@Xhh`-Farben, vereinheitlicht
Leerraum je Zeile und Kleinschreibung. Zeilengrenzen bleiben erhalten.
Für Vergleiche endet der sichtbare Text am DOS-EOF; die Anzahl nachfolgender
Rohbytes wird separat festgehalten. Ein synthetischer Test bestätigt, dass
entsprechende CP437- und UTF-8-Texte zusammenfallen.

Dies ist nicht identisch mit der vorhandenen zeilenweisen Beschreibungssäuberung
und noch kein produktiver Membermatcher. Insbesondere darf eine spätere
Löschentscheidung unerwartete Bytes hinter DOS-EOF nicht einfach ignorieren.

Die zusätzliche Ersetzung sämtlicher Dezimalziffern durch `<n>` dient
**ausschließlich der Variantensuche**. Sie vereinigt auch Telefonnummern,
Versionsnummern, Registrierformulare, Menüs, Türkonfigurationen und Quelltexte.
Sie darf nicht pauschal zur automatischen Löschregel werden.

## Nachgewiesene Risiken breiter Inhaltsmuster

Der zweite Auswertungslauf suchte die fünf Identitäts-/Generatormarker in allen
gespeicherten normalisierten Texten, ohne Dateinamenfilter. Vollständige
Fundstellen und Rohhashes stehen in [findings.tsv](findings.tsv), Zähler in
[discovery-markers.md](discovery-markers.md).

- „Clipper Workshop BBS“: 116 Vorkommen, darunter 15 als Dokumentation
  eingeordnete Dateinamen. In `dlinfo11.zip` ist der Name Teil des Supportabschnitts
  der Originaldokumentation, nicht die ganze Datei eine Fremdanzeige.
- „Cutting Edge Online“: 313 Vorkommen, darunter 197 dokumentationsartige
  Dateinamen. In der Dokumentation zu Graphics Mode Changer 1.6 stehen derselbe
  Name und die Kontaktangaben in Copyright, Lizenzbedingungen und Support.
  **Auch Boardname plus Telefonnummer reicht dort nicht als Löschkriterium.**
- „Pacific Coast Micro“: 5.076 Vorkommen, darunter 1.566 dokumentationsartige
  Dateinamen und zwei kanonische Beschreibungen. Das sind Suchtreffer, keine
  nachgewiesenen Fremdanzeigen.
- OUT.AD nennt OutWorld Arts und erscheint in eigenen OutWorld-Paketen.
  Unterschiedliche Monatsarchive sind kein Beweis automatischer Datumserzeugung
  und erst recht keine Erlaubnis, originale Künstler-/Gruppenbeigaben zu löschen.

`doc-like` und `canonical-description` sind Dateinamenheuristiken. Die übrigen
Treffer sind nicht automatisch Werbung; die Doc-like-Zahl ist umgekehrt auch
keine Anzahl bewiesener Fehlalarme. Die genannten Dokumentbeispiele wurden
inhaltlich geprüft.

## Empfehlung für die Implementierung

1. **Exakte Fingerprints behalten:** zuverlässiger Schnellpfad für unveränderte
   Inhalte und binäre Intros; keine Dateinamensabhängigkeit ergänzen.
2. **Eigene Text-Memberregeln ergänzen:** sichere CP437/UTF-8-Normalisierung,
   lesbare feste Zeilen plus gezielte Regexfelder. Die bisherige `pattern`-Regex
   prüft den Dateinamen, `keywords` prüfen rohe, groß-/kleinschreibungsabhängige
   Bytes. Diese Semantik nicht stillschweigend umdeuten.
3. **Bekannte komplette Vorlagen prüfen:** feste Identitäts-/Layoutmerkmale,
   eng definierte variable Felder für Datum, Uhrzeit, Node, Größe usw.; alternative
   bekannte Angebotsblöcke statt beliebigem `.*` über unbekannte Absätze.
4. **Größe als Schutzgrenze, nicht Identität:** Textgröße und Zeilenzahl begrenzen,
   aber nicht für jede Kodierung, Uhrzeit oder Kapazitätsangabe neue Hashes brauchen.
5. **Originalinhalte schützen:** zusätzliche Dokumentation, unbekannte Absätze,
   unerwartete EOF-Nutzdaten und mehrdeutige Matches blockieren automatische
   Entfernung. Kopierte Beschreibungen in Generatoranzeigen nur bei explizit
   erkannter Gesamtstruktur erlauben.
6. **Zuerst report_only:** neue Textregeln sollen Fundstelle, Regel, Kodierung und
   Rohhash melden können, ohne zu löschen. Dies fehlt bei heutigen Memberregeln.
   Freigabe erst nach Prüfung positiver Beispiele und des übrigen Korpus,
   einschließlich originaler Autor-/Supportdateien und umbenannter Gegenproben.

Die breiten Suchmarker dieses Berichts sind ausdrücklich **keine** solchen
vollständigen Vorlagenregeln und dürfen nicht als automatische Filter übernommen
werden. Im Audit wurde noch keine löschfähige Inhaltsregex auf Präzision validiert.

## Werkzeuge und Reproduzierbarkeit

- [Scanner](../../crates/dizbase/examples/archive_text_audit.rs):
  Aufrufargumente `SOURCE NEW_OUTPUT DEFAULT_MEMBER_RULES CORPUS_MEMBER_RULES`.
  Ausgabeziel dieses Laufs war `target/archive-text-audit`.
- [Auswerter](../../crates/dizbase/examples/archive_text_review.rs):
  Aufrufargumente `AUDIT NEW_REPORT_DIRECTORY`; liest die SQLite-Datenbank und
  normalisierten Blobs, erzeugt nur Berichte und verlangt ein neues Zielverzeichnis.
- [scan-summary.tsv](scan-summary.tsv) enthält die unveränderte Scannerbilanz.
- Die lokale Audit-Ausgabe enthält zusätzlich SQLite-Inventar, Quellenhashes,
  Fehler-/Skiplisten, Rohblobs und gruppierte Normalisierungssignaturen. Diese
  umfangreichen Rohdaten sind nicht Teil dieses Berichtsverzeichnisses.
- [findings.tsv](findings.tsv) enthält alle Vorkommen der fünf breiten Marker
  sowie aller acht ausgewählten Basisnamen, **nicht** sämtliche 174.772 Texte
  oder sämtliche Werbung im Korpus. Absolute Quellpfade beziehen sich auf den
  dokumentierten Audit-Rechner; Rohhashes erlauben den Abgleich unabhängig davon.

Validierung: sechs Scannertests und ein Auswertertest erfolgreich; keine
Editor-Diagnosen in den beiden Beispielen. Die Scannertests prüfen unter anderem
Kodierungsnormalisierung, ungültige BOM-Nutzdaten, unveränderte Quellen,
verschachtelte Archive, Grenzen und Fortsetzung nach beschädigten ZIP-Einträgen.