# PCBoard-DIZ: Werbefunde und ergänzte Regeln

## Ergebnis

**37 neue Regeln** wurden aus geprüften Release-Beschreibungen ergänzt:
30 entfernen eindeutige angehängte Durchlauf-/Courier-Werbung (`auto_clean`),
7 melden unsichere Werbe-/Herkunftsblöcke nur (`report_only`). Zusammen mit den
bisherigen Regeln für LiQUiD und Shogunat enthält der Standardkatalog jetzt
**39 Regeln**. Alle 39 sind auch im großen Sammlungskatalog enthalten.

- [Standard-Beschreibungsregeln](../upload_ad_descriptions.toml)
- [Sammlungskatalog einschließlich aller PCBoard-Regeln](../ad_corpus_rules/upload_ad_descriptions.toml)
- [Jede tatsächlich geänderte Beschreibung: vorher/nachher](before-after.md)
- [Alle angewendeten Regeln mit Quelle, SHA-256 und Zeilenbereichen](matches.tsv)

Die folgende vollständige Regelliste enthält jeweils den erkannten Text,
die Aktion und **alle Fundstellen im geprüften Korpus**. Gezählt werden
Vorkommen, nicht nur unterschiedliche Dateien: identische Beschreibungen in
mehreren Archiven bleiben getrennte Fundstellen. Bei mehreren Durchläufen
beziehen sich TSV-Zeilennummern auf den jeweiligen Zwischenstand.

**Validierung, ohne Änderungen an den Archiven:** 58 Beschreibungen würden
bereinigt; 10 weitere bleiben mit ausschließlich `report_only`-Treffern
unverändert; 2.660 Vorkommen haben keinen Treffer. Es gab keine mehrdeutigen
Matcher-Ergebnisse. `report_only` bedeutet hier protokollieren und beenden,
nicht automatisch `needs_review`: Ein solcher letzter Block verhindert bewusst
auch die Entfernung davorliegender Durchlaufwerbung.

### Vollständige Übersicht der geprüften Blöcke

| Werbung / Markierung | Regeln | Aktion |
|---|---:|---|
| Critical Strike, mit variablem Upload-Datum und Uhrzeit | 1 | auto_clean |
| The Unknown Realm | 1 | auto_clean |
| eYEs^crE@m [LSD] | 1 | auto_clean |
| L.O.R.D.S. Couriering '94 | 1 | auto_clean |
| White Sands | 1 | auto_clean |
| BLooD PooL | 1 | auto_clean |
| SPÆZM Couriers | 1 | auto_clean |
| SCIMITAR | 1 | auto_clean |
| RTS Couriers und RTS/WHQ | 2 | auto_clean |
| Ambient Hauze | 1 | auto_clean |
| RiSC Couriering '94 | 1 | auto_clean |
| Digital Delusions, zwei Textvarianten | 2 | auto_clean |
| High Tech Couriers, drei Textvarianten | 3 | auto_clean |
| Distinct | 1 | auto_clean |
| UFP Couriering | 1 | auto_clean |
| fATE '95 / '97, drei Textvarianten | 3 | auto_clean |
| Metro Couriers '95 | 1 | auto_clean |
| ROD BBS | 1 | auto_clean |
| The Wild Thing, Kasten und Euro Connection | 2 | auto_clean |
| Holland's BBS No. One | 1 | auto_clean |
| KORT WHQ | 1 | auto_clean |
| BSBBS, normale und mit Buchstabe O geschriebene Telefonnummer | 2 | auto_clean |
| LiQUiD WHQ, bisherige Inline-Regel | 1 | auto_clean |
| Shogunat, bisherige Regel; nur Wörterbuchbeleg in diesem Audit | 1 | auto_clean, unverändert übernommen |
| LiMPY Crack-Markierung | 1 | report_only: Herkunft einer modifizierten Veröffentlichung |
| Nostrum Nine | 1 | report_only: möglicherweise zur ursprünglichen Gruppe gehörend |
| iMMUNE / Nexus Project | 1 | report_only: möglicherweise Autor-/Distributionskontakt |
| Reckless Life | 1 | report_only: Verbindung zum ursprünglichen Autor möglich |
| HACKER [MTNT] | 1 | report_only: gleiche Gruppe wie das Release |
| Disembodied Voices / Lands of Chaos | 1 | report_only: Originalzugehörigkeit unklar |
| Nuclear Insemination | 1 | report_only: Originalzugehörigkeit unklar |

### Abgleich mit dem tatsächlichen Testboard

Geprüft wurden die acht PCBoard-Areas **035–042** des Testboards unter
`target/debug/test/upload-corpus`. Dessen `baseline.toml` verweist auf dieselbe
Originalsammlung: **alle 2.532 geprüften ZIP-Quellhashes stimmen überein**.
Die damals verwendeten `rules.used.toml` enthielten genau die beiden alten
Beschreibungsregeln für Shogunat und LiQUiD; sie fehlen jetzt nicht mehr im
Sammlungskatalog.

Zusätzlich wurden die **2.519 veröffentlichten Area-Archive** einschließlich
**547 verschachtelter Archive** erneut nur gelesen: **2.727 extrahierte
Beschreibungsvorkommen, keine abweichenden Beschreibungen** gegenüber den
Originalbytes bzw. deren exakter Bereinigung mit den damaligen Regeln.
Ein verschachteltes Archiv blieb unlesbar (BLT3.ZIP, siehe Fehlerliste).
Die Hashes aller gelesenen Area-Dateien blieben unverändert. Die Area-Prüfung
umfasst nur veröffentlichte ZIPs; sie ersetzt nicht die Originalprüfung
einschließlich dort vorhandener, nicht veröffentlichter Dateien.

**Weder Testboard-Konfiguration noch Archive oder Dateidatenbank wurden verändert.**

### Abgrenzung: Filterwörterbücher und Testdaten

Nicht jede Datei mit DIZ-Endung ist eine Release-Beschreibung. Die beiden
historischen Filterlisten sind Belege für damalige Suchmuster, aber alleine
kein Nachweis eines vollständigen angehängten Werbeblocks. Folgende Namen
und Marker wurden dort ebenfalls gefunden; Überschneidungen mit der obigen
Regelliste sind möglich und werden nicht als zusätzliche Release-Funde gezählt:

- **P!-STRIP.DIZ** aus `p_2dstrip.zip`: Channel X Press; Nightmare; Kiss of
    Death; False Prophecy; File Propulsion System; The Wild Thing; Velvet
    Underground; Ghostship (zwei Telefonnummern); Mystical Runes; INDIGO;
    Exchange; RiSC '94/'95; Southern Comfort; RAZoR / PWA CHQ / SPeCTRE USHQ;
    Panic Zone; United Couriers; Tramontane / Cyanide WHQ; Cyanide Spreaders;
    The Grille; Darkness Before Dawn; Ice Box; Magpie / T.O.X.I.C; Leecher;
    Undercover / Check Your Next Move; Coffee-Time; PointBreak; Trancental;
    **Shogunat**; Lotus Sound; Universe; Shoebox. Daneben stehen generische
    Fragmente wie `COURIER`, `UPLOADED BY`, `SPREAD BY`, `PASSED THRU`,
    `TRADED BY`, `SENT BY`, `INTERCEPTED BY` und Registrierungs-/Namenszeilen.
- **STRIP.DIZ** aus `pwaecd11.zip::STRIPDIZ.ZIP`: Extreme Trading Crew / ETC;
    Suicide Crew; K-9 Kastle; Bedlam; Velvet Underground; KORT WHQ; RTS/WHQ;
    Brain Blaster; Cocaine Couriers; Diesel; Drift; Lucretia; Cartel;
    Cemetery; MCC.TTAB.POW; Critical Deception; Disembodied Voices; UFP;
    Everlast; FPS; fATE; Back Door; Flesh; Hardwire; High Tech; Legacy;
    Millennium Bytes; Magma; Menace; Mennen; NAFTA; Nuts; Point of No Return;
    Crack House; RiSC; Speed & Ecstasy; Swift; Taking You To New Highs;
    Crazy Courriers; Final Descent; Lexicon of the Cabal; Major BBS;
    Critical Strike; FTS; Dark Water; Underground Rebellion; Turbine Art;
    Vision; Ego Land; The Grille; Dark Forces. Auch hier stehen unvollständige
    Telefon-, Zeit-, ASCII-Rahmen- und allgemeine Courier-Fragmente.
- **P!SAMPLE.ZIP / FILE_ID.DIZ** ist ausdrücklich `STRiP/DiZ TEST FiLE_iD`:
    generische Courier-Marker, Undercover, Coffee-Time und PointBreak stehen
    vor künstlichen `TEST LiNE`-Zeilen. Das ist kein belegter Werbeanhang.

**Korrektur zum alten Shogunat-Beleg:** Der Text steht in P!-STRIP.DIZ,
nicht als passender Endblock einer lesbaren regulären Release-Beschreibung
dieses Audits. Die bestehende Regel wurde erhalten, aber hier mit **0**
realen End-to-End-Treffern ausgewiesen. Aus den übrigen Wörterbuchfragmenten
wurden keine pauschalen Löschregeln erzeugt.

### Bewusst erhaltene Originalangaben

Autorenlogos von PWA, ACID, AEGIS, Elements, FOOD, KASA oder JM_ sind nicht
automatisch fremde Werbung. Ebenso bleiben belegte Originalkontakte und
Registrierungsangaben erhalten, etwa POPIT SOFTWARES / POPIT BBS, Cyberspace
Entertainment / Altered Ego, Wizard's BBS / WizWare, POB, ANTi-X / No-Name,
Laser BBS, Software Kitchen, MoonScape, Barnabo, Table BBS, Global Software
und Happy Pager. Die CRC-/F-Prot-Prüfstatuszeilen sind keine Werbung.
Es gibt absichtlich keine allgemeine Telefonnummern-, Autorenlogo- oder
`UPLOADED BY`-Löschregel.

### Reproduzierbarkeit und Grenzen

Der [Offline-Auditor](../../crates/dizbase/examples/pcboard_diz_audit.rs) hat
die Modi `SOURCE NEW_OUTPUT`, `curate AUDIT DEFAULT_RULES CORPUS_RULES NEW_REPORT`
und `compare-live AUDIT BOARD_CORPUS NEW_OUTPUT`. Quellen und gelesene Rohdaten
werden mit SHA-256 geprüft; Archivpfade werden nicht auf das Dateisystem
übernommen, sondern Bytes unter ihrem Hash gespeichert. Programme werden
nicht ausgeführt. Neue Ausgabeverzeichnisse sind Pflicht.

Die Prüfung beschränkt sich auf ZIPs und verschachtelte ZIPs, maximal vier
Verschachtelungsebenen, 10.000 Einträge pro Archiv, 256 KiB pro Beschreibung
und 16 MiB pro verschachteltem ZIP. Ein Lesefehler beendet die Prüfung des
betroffenen Archivs; nachfolgende Einträge können deshalb fehlen. Die
**13 Fehler** der Originalprüfung sind unten vollständig aufgeführt. Eine
vollständige Aussage über beschädigte/unlesbare Teile ist damit nicht möglich.

Beide Kataloge wurden gegen alle extrahierten Vorkommen mit acht Durchläufen
geprüft: gleiche bereinigte Bytes, keine neuen Mehrdeutigkeiten, nichtleeres
Ergebnis und idempotente Bereinigung. Zusätzlich wurden die 37 ausgewählten
Rohblöcke isoliert positiv, unter falschem Membernamen und in der Mitte einer
Beschreibung negativ geprüft. Die [portablen Regressionstests](../../crates/dizbase/tests/pcboard_description_rules.rs)
decken Literalblöcke, Critical-Strike-Zeitvarianten, unvollständige Banner,
gestapelte Werbung, Report-only-Stopps, Inline-Rahmen und Autorenangaben ab.

## Vollständige Fundstellen und Regeltexte

Source: /home/mkrueger/work/bbs/bbsarchives (all eight PCBoard categories, original archives read-only).

2532 archives, 547 nested archives; 2728 extracted description/dictionary occurrences.
13 archive/read errors are listed below, so completeness is limited to readable members.

58 descriptions cleaned, 2660 unchanged without a match; 0 review cases.

Only complete suffixes are matched. Original author artwork, technical details, registration terms and support contacts are not generalized into removal rules. Report-only promotional blocks remain unchanged.

## shogunat-footer

Action: AutoClean; matched occurrences: 0.

```toml
id = "shogunat-footer"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
lines = [
    "^.*fucking fast shareware.*$",
    "^.*shogunat 030 746 67 93.*$",
]
action = "auto_clean"

```
No end-to-end match in readable descriptions; may be behind another report-only marker or only historically known.
## liquid-whq-footer

Action: AutoClean; matched occurrences: 1.

```toml
id = "liquid-whq-footer"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
inline_start = "I WAS FIRST ON"
literal_lines = [
    "I WAS FIRST ON LiQUiD'S WHQ, @USER@",
    "WHY DON'T YA ASK FOR JAPANESE BOARDS",
]
action = "auto_clean"

```
- PCboard-PPE-s-A-C/amiu060.zip::FILE_ID.DIZ
## critical-strike-transit

Action: AutoClean; matched occurrences: 12.

```toml
id = "critical-strike-transit"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
lines = [
    "^φ hσy lämεγ! φ$",
    "^┌────────────────────────── · · · · · ·$",
    '^│ this file passσd thru \(γiti\[al stγikε$',
    '^│ üplφadσd φ∩ [0-9]{2}\.[0-9]{2}\.[0-9]{2} at [0-9]{2}:[0-9]{2}$',
    '^│ call nθw \- \(2o1\)535\-3902$',
    "^└────────────────────────── · · · · · ·$",
]
action = "auto_clean"

```
- PCBoard-PPE-s-by-PWA/p2pcbpwa.zip::FILE_ID.DIZ
- PCboard-PPE-s-A-C/911lv1b.zip::FILE_ID.DIZ
- PCboard-PPE-s-A-C/acd_2dag20.zip::FILE_ID.DIZ
- PCboard-PPE-s-A-C/acd_2dmcs1.zip::FILE_ID.DIZ
- PCboard-PPE-s-A-C/afl_2dup.zip::FILE_ID.DIZ
- PCboard-PPE-s-A-C/afl_run1.zip::FILE_ID.DIZ
- PCboard-PPE-s-A-C/ag_2deul10.zip::FILE_ID.DIZ
- PCboard-PPE-s-A-C/agscnf10.zip::FILE_ID.DIZ
- PCboard-PPE-s-A-C/agsentr1.zip::FILE_ID.DIZ
- PCboard-PPE-s-A-C/agsjoin1.zip::FILE_ID.DIZ
- PCboard-PPE-s-A-C/agslog23.zip::FILE_ID.DIZ
- PCboard-PPE-s-A-C/agsnmai0.zip::FILE_ID.DIZ

Reviewed source block: 911lv1b.zip::FILE_ID.DIZ.
## unknown-realm-transit

Action: AutoClean; matched occurrences: 1.

```toml
id = "unknown-realm-transit"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = [
    "This File Passed Thru",
    "(9o5) -=≡ The Unknown Realm ≡=- (9o5)",
]
action = "auto_clean"

```
- PCboard-PPE-s-A-C/2_2ddownx.zip::FILE_ID.DIZ

Reviewed source block: 2_2ddownx.zip::FILE_ID.DIZ.
## eyes-cream-transit

Action: AutoClean; matched occurrences: 4.

```toml
id = "eyes-cream-transit"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = [
    '.,-²"~~~~~~~~~~"²-,.',
    "$ fiLe PassEd THru $",
    "$ eYEs^crE@m [LSD] $",
    "$ 972-6-6725664 $",
    "`'-,............,-`'",
]
action = "auto_clean"

```
- PCboard-PPE-s-A-C/caz_2dentr.zip::FILE_ID.DIZ
- PCboard-PPE-s-A-C/caz_2dentz.zip::FILE_ID.DIZ
- PCboard-PPE-s-A-C/caz_2dgte.zip::FILE_ID.DIZ
- PCboard-PPE-s-A-C/caz_2dszd.zip::FILE_ID.DIZ

Reviewed source block: caz_2dentr.zip::FILE_ID.DIZ.
## lords-couriering-94

Action: AutoClean; matched occurrences: 1.

```toml
id = "lords-couriering-94"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = ["-=[ The L.O.R.D.S. Couriering '94 ]=-"]
action = "auto_clean"

```
- PCboard-PPE-s-A-C/blot_ul.zip::FILE_ID.DIZ

Reviewed source block: blot_ul.zip::FILE_ID.DIZ.
## white-sands-transit

Action: AutoClean; matched occurrences: 2.

```toml
id = "white-sands-transit"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = [
    ".",
    "Leeched from White Sands",
    "One of the Fastest on the East Coast!",
]
action = "auto_clean"

```
- PCBoard-PPE-s-by-PWA/art1bpwa.zip::FILE_ID.DIZ
- PCBoard-PPE-s-by-PWA/lur98pwa.zip::FILE_ID.DIZ

Reviewed source block: art1bpwa.zip::FILE_ID.DIZ.
## blood-pool-transit

Action: AutoClean; matched occurrences: 1.

```toml
id = "blood-pool-transit"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = ["───■[ BLooD PooL ≡ 187 ]■───"]
action = "auto_clean"

```
- PCBoard-PPE-s-by-PWA/lur98pwa.zip::FILE_ID.DIZ

Reviewed source block: lur98pwa.zip::FILE_ID.DIZ.
## spaezm-couriers

Action: AutoClean; matched occurrences: 1.

```toml
id = "spaezm-couriers"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = ["<]─────≡] SPÆZM COURiERS [≡─────[>"]
action = "auto_clean"

```
- PCBoard-PPE-s-by-PWA/ciafonlr.zip::FILE_ID.DIZ

Reviewed source block: ciafonlr.zip::FILE_ID.DIZ.
## scimitar-spreader

Action: AutoClean; matched occurrences: 5.

```toml
id = "scimitar-spreader"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = ["-* SCIMITAR * -<WWC/PWA>- Spreader *-"]
action = "auto_clean"

```
- PCBoard-PPE-s-by-PWA/infopwa.zip::FILE_ID.DIZ
- PCBoard-PPE-s-by-PWA/pwa_2daa1.zip::FILE_ID.DIZ
- PCBoard-PPE-s-by-PWA/pwa_2dlog1.zip::FILE_ID.DIZ
- PCBoard-PPE-s-by-PWA/pwa_aa1.zip::FILE_ID.DIZ
- PCBoard-PPE-s-by-PWA/pwa_log1.zip::FILE_ID.DIZ

Reviewed source block: infopwa.zip::FILE_ID.DIZ.
## rts-couriers

Action: AutoClean; matched occurrences: 1.

```toml
id = "rts-couriers"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = ["────── Brought to you by RTS Couriers ──────"]
action = "auto_clean"

```
- PCBoard-PPE-s-D-F/dodnew11.zip::FILE_ID.DIZ

Reviewed source block: dod_2ddl1c.zip::FILE_ID.DIZ.
## limpy-crack-mark

Action: ReportOnly; matched occurrences: 1.

```toml
id = "limpy-crack-mark"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = ["[[[[[[[[[ CRACKED BY -=LiMPY=- ]]]]]]]]]]]]]"]
action = "report_only"

```
- PCBoard-PPE-s-D-F/dod_2ddl1c.zip::FILE_ID.DIZ

Reviewed source block: dod_2ddl1c.zip::FILE_ID.DIZ.
## ambient-hauze-transit

Action: AutoClean; matched occurrences: 1.

```toml
id = "ambient-hauze-transit"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = ['-/PASSED THRU AMBIENT HAUZE\-']
action = "auto_clean"

```
- PCBoard-PPE-s-D-F/dod_2due12.zip::FILE_ID.DIZ

Reviewed source block: dod_2due12.zip::FILE_ID.DIZ.
## risc-couriering-94

Action: AutoClean; matched occurrences: 1.

```toml
id = "risc-couriering-94"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = ["──═══ RiSC COURiERiNG '94! ═══──"]
action = "auto_clean"

```
- PCBoard-PPE-s-D-F/dod_2dup32.zip::FILE_ID.DIZ

Reviewed source block: dod_2dup32.zip::FILE_ID.DIZ.
## digital-delusions-transit

Action: AutoClean; matched occurrences: 5.

```toml
id = "digital-delusions-transit"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = ["Passed Through Digital Delusions -[315]-"]
action = "auto_clean"

```
- PCBoard-PPE-s-J-O/lspd_2dnup.zip::FILE_ID.DIZ
- PCBoard-PPE-s-J-O/lspd_2dnup.zip::file_id.diz
- PCBoard-PPE-s-J-O/lspderv1.zip::FILE_ID.DIZ
- PCBoard-PPE-s-J-O/lspdex1.zip::FILE_ID.DIZ
- PCBoard-PPE-s-J-O/lspdol12.zip::FILE_ID.DIZ

Reviewed source block: lspd_2dnup.zip::FILE_ID.DIZ.
## digital-delusions-transit-315

Action: AutoClean; matched occurrences: 1.

```toml
id = "digital-delusions-transit-315"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = ["-[315]- Passed Thru Digital Delusions -[315]-"]
action = "auto_clean"

```
- PCBoard-PPE-s-D-F/flx_2deblt.zip::FILE_ID.DIZ

Reviewed source block: flx_2deblt.zip::FILE_ID.DIZ.
## high-tech-couriers-95

Action: AutoClean; matched occurrences: 1.

```toml
id = "high-tech-couriers-95"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = ["<< .xXx. HiGH TeCh COURiERS '95 .xXx. >>"]
action = "auto_clean"

```
- PCBoard-PPE-s-J-O/lspd_2dnup.zip::file_id.diz

Reviewed source block: lspd_2dnup.zip::file_id.diz.
## high-tech-couriers-slash

Action: AutoClean; matched occurrences: 1.

```toml
id = "high-tech-couriers-slash"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = ["-/- High Tech Couriers -/-"]
action = "auto_clean"

```
- PCBoard-PPE-s-J-O/lspdex1.zip::FILE_ID.DIZ

Reviewed source block: lspdex1.zip::FILE_ID.DIZ.
## high-tech-couriers-cross

Action: AutoClean; matched occurrences: 1.

```toml
id = "high-tech-couriers-cross"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = ["┼─∙■ HiGH TeCH CouRieRS ■∙─┼"]
action = "auto_clean"

```
- PCBoard-PPE-s-D-F/ftatul16.zip::FILE_ID.DIZ

Reviewed source block: ftatul16.zip::FILE_ID.DIZ.
## distinct-courier

Action: AutoClean; matched occurrences: 1.

```toml
id = "distinct-courier"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = ["[DtC] -[ Courier'd By Distinct ] - [DtC]"]
action = "auto_clean"

```
- PCBoard-PPE-s-J-O/lspderv1.zip::FILE_ID.DIZ

Reviewed source block: lspderv1.zip::FILE_ID.DIZ.
## ufp-couriering

Action: AutoClean; matched occurrences: 2.

```toml
id = "ufp-couriering"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = ["∙{TAB}·¡Dístributéd By UFP Cóurieríng!·{TAB}∙"]
action = "auto_clean"

```
- PCBoard-PPE-s-J-O/lspd_2dvfb.zip::FILE_ID.DIZ
- PCBoard-PPE-s-J-O/lspd_2dvts.zip::FILE_ID.DIZ

Reviewed source block: lspd_2dvfb.zip::FILE_ID.DIZ.
## fate97-courier

Action: AutoClean; matched occurrences: 1.

```toml
id = "fate97-courier"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = ["[fATE97] COURiERED BY fATE [fATE97]"]
action = "auto_clean"

```
- PCBoard-PPE-s-D-F/food_21bll.zip::FILE_ID.DIZ

Reviewed source block: food_21bll.zip::FILE_ID.DIZ.
## fate95-courier

Action: AutoClean; matched occurrences: 1.

```toml
id = "fate95-courier"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = ["[*] fATE95 COURiERiNG! [*]"]
action = "auto_clean"

```
- PCBoard-PPE-s-P-R/ror_2dmail.zip::FILE_ID.DIZ

Reviewed source block: ror_2dmail.zip::FILE_ID.DIZ.
## fate95-courier-slash

Action: AutoClean; matched occurrences: 1.

```toml
id = "fate95-courier-slash"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = ['-/[*]\- fATE95 COURiERiNG! -/[*]\-']
action = "auto_clean"

```
- PCBoard-PPE-s-P-R/ror_2dslam.zip::FILE_ID.DIZ

Reviewed source block: ror_2dslam.zip::FILE_ID.DIZ.
## rts-whq-transit

Action: AutoClean; matched occurrences: 1.

```toml
id = "rts-whq-transit"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = [".:. Scortched from the RTS/WHQ .:."]
action = "auto_clean"

```
- PCBoard-PPE-s-D-F/fsn_2dnuu.zip::FILE_ID.DIZ

Reviewed source block: fsn_2dnuu.zip::FILE_ID.DIZ.
## metro-couriers-95

Action: AutoClean; matched occurrences: 1.

```toml
id = "metro-couriers-95"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = ["<<--------[metro couriers 95]"]
action = "auto_clean"

```
- PCBoard-PPE-s-G-I/gnxzmrf1.zip::FILE_ID.DIZ

Reviewed source block: gnxzmrf1.zip::FILE_ID.DIZ.
## rod-bbs-transit

Action: AutoClean; matched occurrences: 2.

```toml
id = "rod-bbs-transit"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = ["■ Leeched From: R·O·D BBS ■"]
action = "auto_clean"

```
- PCBoard-PPE-s-J-O/nwread2.zip::FILE_ID.DIZ
- PCBoard-PPE-s-J-O/nwreader.zip::FILE_ID.DIZ

Reviewed source block: nwread2.zip::FILE_ID.DIZ.
## wild-thing-transit-box

Action: AutoClean; matched occurrences: 2.

```toml
id = "wild-thing-transit-box"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = [
    "┌───────────────────────────────────┐",
    "│ LεεCHεD FRoM The Wild Thing BBs │",
    "└───────────────────────────────────┘",
]
action = "auto_clean"

```
- PCBoard-PPE-s-J-O/jm_mf_10.zip::FILE_ID.DIZ
- PCBoard-PPE-s-S-Z/spell111.zip::FILE_ID.DIZ

Reviewed source block: jm_mf_10.zip::FILE_ID.DIZ.
## wild-thing-euro-connection

Action: AutoClean; matched occurrences: 2.

```toml
id = "wild-thing-euro-connection"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = [
    "┌»»LεεCHεD FRoM The Wild Thing BBs««┐",
    "└───¥■Φ ThΣ ΣÜro ÇÖnnεctïon Φ■¥─────┘",
]
action = "auto_clean"

```
- PCBoard-Utils/pfed_104.zip::FILE_ID.DIZ
- PCboard-PPE-s-A-C/bcwm051.zip::FILE_ID.DIZ

Reviewed source block: bcwm051.zip::FILE_ID.DIZ.
## holland-number-one-transit

Action: AutoClean; matched occurrences: 1.

```toml
id = "holland-number-one-transit"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = ["(---- went trough holland's bbs no.one ----)"]
action = "auto_clean"

```
- PCBoard-PPE-s-D-F/dma_2dwall.zip::FILE_ID.DIZ

Reviewed source block: dma_2dwall.zip::FILE_ID.DIZ.
## kort-whq-upload

Action: AutoClean; matched occurrences: 1.

```toml
id = "kort-whq-upload"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = [
    "4 Ñodes KORT WHQ*313.699*0530",
    "Files: 15, Nfo: NONE, Diz: 09.17.94",
    "Uploaded [16:22] by: Tosh10",
]
action = "auto_clean"

```
- PCboard-PPE-s-A-C/bcpg10.zip::FILE_ID.DIZ

Reviewed source block: bcpg10.zip::FILE_ID.DIZ.
## bsbbs-phone

Action: AutoClean; matched occurrences: 3.

```toml
id = "bsbbs-phone"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = ["bsbbs.1.603.889.8903.pcb15"]
action = "auto_clean"

```
- PCBoard-PPE-s-D-F/elt_2dnfse.zip::FILE_ID.DIZ
- PCboard-PPE-s-A-C/bpc_2dub10.zip::FILE_ID.DIZ
- PCboard-PPE-s-A-C/bpc_2duj11.zip::FILE_ID.DIZ

Reviewed source block: elt_2dnfse.zip::FILE_ID.DIZ.
## bsbbs-phone-obfuscated

Action: AutoClean; matched occurrences: 5.

```toml
id = "bsbbs-phone-obfuscated"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = ["bsbbs.1.6O3.889.89O3.pcb15"]
action = "auto_clean"

```
- PCboard-PPE-s-A-C/bpc_2dqb.zip::FILE_ID.DIZ
- PCboard-PPE-s-A-C/bpc_2dreg3.zip::FILE_ID.DIZ
- PCboard-PPE-s-A-C/bpc_2dub10.zip::FILE_ID.DIZ
- PCboard-PPE-s-A-C/bpc_2duj11.zip::FILE_ID.DIZ
- PCboard-PPE-s-A-C/bpc_2dzp4.zip::FILE_ID.DIZ

Reviewed source block: bpc_2dub10.zip::FILE_ID.DIZ.
## nostrum-nine-promotion

Action: ReportOnly; matched occurrences: 2.

```toml
id = "nostrum-nine-promotion"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = [
    "\u000F +31(o)317-314125 \u000F",
    "-=[NoS 9] -=[ NOSTRUM -9- BBS ]=- [NoS 9]=-",
]
action = "report_only"

```
- PCboard-PPE-s-A-C/cr_2ddisc.zip::FILE_ID.DIZ
- PCboard-PPE-s-A-C/cr_2drands.zip::FILE_ID.DIZ

Reviewed source block: cr_2ddisc.zip::FILE_ID.DIZ.
## nexus-project-promotion

Action: ReportOnly; matched occurrences: 3.

```toml
id = "nexus-project-promotion"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = [
    '(---/\//[ iMMUNE! tRADERS ]\\/\---)',
    "[= NP =] -= NEXUS PROJECT BBS =- [= NP =]",
]
action = "report_only"

```
- PCBoard-PPE-s-G-I/imn_2daloh.zip::FILE_ID.DIZ
- PCBoard-PPE-s-G-I/imn_2dbdkk.zip::FILE_ID.DIZ
- PCBoard-PPE-s-G-I/imn_2ddisc.zip::FILE_ID.DIZ

Reviewed source block: imn_2daloh.zip::FILE_ID.DIZ.
## reckless-life-promotion

Action: ReportOnly; matched occurrences: 2.

```toml
id = "reckless-life-promotion"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = ["∙[ RΣ¢KLΣS$ LiFΣ BBS:P0RTÜGA¿'$ CΘ0¿Σ2T¡! ]∙"]
action = "report_only"

```
- PCBoard-PPE-s-J-O/nk_2drmr20.zip::FILE_ID.DIZ
- PCBoard-PPE-s-J-O/nkpage12.zip::FILE_ID.DIZ

Reviewed source block: nkpage12.zip::FILE_ID.DIZ.
## mtnt-hacker-courier

Action: ReportOnly; matched occurrences: 1.

```toml
id = "mtnt-hacker-courier"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = [
    "│ Couriered by : HACKER [MTNT] │",
    "-─-─--──────────────────────────────-",
]
action = "report_only"

```
- PCBoard-PPE-s-J-O/mtntmes2.zip::FILE_ID.DIZ

Reviewed source block: mtntmes2.zip::FILE_ID.DIZ.
## disembodied-lands-promotion

Action: ReportOnly; matched occurrences: 1.

```toml
id = "disembodied-lands-promotion"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = [
    "Disembodied Voices :: 718-279-2766",
    '<=/\=>·LANDS·OF·CHAOS∙SPHERE!·APP·HQ·<=/\=>',
]
action = "report_only"

```
- PCBoard-PPE-s-S-Z/wkd_2dlam1.zip::FILE_ID.DIZ

Reviewed source block: wkd_2dlam1.zip::FILE_ID.DIZ.
## nuclear-insemination-promotion

Action: ReportOnly; matched occurrences: 1.

```toml
id = "nuclear-insemination-promotion"
member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'
position = "suffix"
literal_lines = ["·∙:|Nuclear Insemination|:∙·"]
action = "report_only"

```
- PCBoard-PPE-s-D-F/fsn_2dnuu.zip::FILE_ID.DIZ

Reviewed source block: fsn_2dnuu.zip::FILE_ID.DIZ.

## Unreadable / damaged archives

- PCBoard-PPE-s-D-F/delay30a.zip: Invalid checksum
- PCBoard-PPE-s-D-F/dod_2dmm31.zip: invalid Zip archive: Could not find EOCD
- PCBoard-PPE-s-J-O/mushfb10.zip: invalid Zip archive: Could not find EOCD
- PCBoard-PPE-s-P-R/pob396ls.zip: invalid Zip archive: Extra field content truncated
- PCBoard-PPE-s-P-R/pr_2dfp20.zip: invalid Zip archive: Could not find EOCD
- PCBoard-PPE-s-P-R/pr_2dus11.zip: invalid Zip archive: Could not find EOCD
- PCBoard-PPE-s-S-Z/slfdstrc.zip: Invalid checksum
- PCBoard-PPE-s-by-PWA/pwapcp03.zip::EZ_BULL/EZ_BULL.ZIP::BLT3.ZIP: invalid Zip archive: Could not find EOCD
- PCboard-PPE-s-A-C/ags_ic12.zip: invalid Zip archive: Could not find EOCD
- PCboard-PPE-s-A-C/agsuol20.zip: Invalid checksum
- PCboard-PPE-s-A-C/agswal10.zip: Invalid checksum
- PCboard-PPE-s-A-C/agswho25.zip: Invalid checksum
- PCboard-PPE-s-A-C/bpc_2dprt.zip: invalid Zip archive: Could not find EOCD

## Matcher review cases

