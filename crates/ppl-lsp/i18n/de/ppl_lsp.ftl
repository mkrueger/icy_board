hint-preprocessor-langversion=Legt die von dieser Quelldatei verwendete PPL-Sprachversion fest. Die Direktive muss vor Programmcode stehen und hat Vorrang vor Workspace-, Kommandozeilen- und Umgebungseinstellungen.
hint-preprocessor-define=Definiert eine von Groß-/Kleinschreibung unabhängige Präprozessorvariable. Der optionale Wert kann in bedingten Ausdrücken verwendet oder mit `;#name` in den Quelltext eingefügt werden; eine Definition ohne Wert ist wahr.
hint-preprocessor-if=Beginnt einen Zweig der bedingten Übersetzung. Der Ausdruck kann `VERSION`, `RUNTIME`, `LANGVERSION` und mit `;$DEFINE` eingeführte Variablen verwenden. Quelltext in einem inaktiven Zweig wird nicht übersetzt.
hint-preprocessor-elseif=Beginnt einen weiteren Zweig der bedingten Übersetzung, wenn im selben Block noch kein vorheriger Zweig gewählt wurde.
hint-preprocessor-elif=Kurzform von `;$ELSEIF`; beginnt einen weiteren Zweig, wenn noch kein vorheriger Zweig gewählt wurde.
hint-preprocessor-else=Beginnt den Ersatzzweig der bedingten Übersetzung, wenn im selben Block kein vorheriger Zweig gewählt wurde.
hint-preprocessor-endif=Beendet den mit `;$IF` begonnenen Block der bedingten Übersetzung.
hint-preprocessor-usefuncs=Historische Kompatibilitätsmarkierung für Quelltexte mit benutzerdefinierten Funktionen. Aktuelle Compiler akzeptieren sie ohne Wirkung.
hint-preprocessor-substitution=Fügt den Wert einer vordefinierten, im Workspace gesetzten oder mit `;$DEFINE` angelegten Präprozessorvariable in den Tokenstrom ein. Ein unbekannter Name ist ein Fehler.
hint-keyword-if=Beginnt einen bedingten Block. Sein Inhalt wird ausgeführt, wenn die Bedingung wahr ist; optionale Zweige mit `ELSEIF` und `ELSE` können folgen.
hint-keyword-let=Leitet eine Zuweisung ein. `LET` ist bei gewöhnlichen Zuweisungen optional und bleibt zur Kompatibilität mit klassischem PPL-Quelltext erhalten.
hint-keyword-while=Beginnt eine kopfgesteuerte Schleife, die wiederholt wird, solange ihre Bedingung wahr bleibt.
hint-keyword-endwhile=Beendet eine `WHILE`-Schleife.
hint-keyword-else=Beginnt den Auffangzweig eines `IF`-Blocks, wenn keine vorherige Bedingung zutraf.
hint-keyword-elseif=Fügt einem `IF`-Block eine weitere Bedingung hinzu. Sie wird nur geprüft, wenn alle vorherigen Bedingungen falsch waren.
hint-keyword-endif=Beendet einen `IF`-Block.
hint-keyword-for=Beginnt eine Zählschleife mit Startwert, Grenzwert und optionaler Schrittweite.
hint-keyword-next=Beendet eine `FOR`-Schleife und erhöht ihre Steuervariable. `ENDFOR` ist eine gleichwertige Schreibweise.
hint-keyword-endfor=Beendet eine `FOR`-Schleife und erhöht ihre Steuervariable. `NEXT` ist eine gleichwertige Schreibweise.
hint-keyword-break=Verlässt sofort die innerste aktive Schleife oder Auswahlanweisung.
hint-keyword-continue=Überspringt den Rest des aktuellen Durchlaufs und beginnt den nächsten Durchlauf der innersten Schleife.
hint-keyword-return=Kehrt aus der aktuellen Funktion oder Prozedur zurück. Bei einer Funktion wird dabei ihr Ergebniswert geliefert.
hint-keyword-gosub=Ruft eine Marke als Unterprogramm auf. Nach dessen Rückkehr wird die Ausführung hinter `GOSUB` fortgesetzt.
hint-keyword-goto=Setzt die Ausführung an der angegebenen Marke fort.
hint-keyword-select=Beginnt einen mehrarmigen Auswahlblock, dessen Ausdruck mit den `CASE`-Zweigen verglichen wird.
hint-keyword-case=Leitet innerhalb eines `SELECT`-Blocks einen Zweig für einen Wert ein.
hint-keyword-default=Leitet den Auffangzweig eines `SELECT`-Blocks ein, wenn kein `CASE` zutraf.
hint-keyword-endselect=Beendet einen `SELECT`-Block.
hint-keyword-declare=Deklariert die Signatur einer Funktion oder Prozedur vor ihrer Implementierung und ermöglicht dadurch frühere Aufrufe und Typprüfungen.
hint-keyword-function=Beginnt eine benannte Routine, die einen Wert berechnet und zurückgibt.
hint-keyword-procedure=Beginnt eine benannte Routine, die Aktionen ausführt, ohne einen Wert zurückzugeben.
hint-keyword-endproc=Beendet die Implementierung einer `PROCEDURE`.
hint-keyword-endfunc=Beendet die Implementierung einer `FUNCTION`.
hint-keyword-repeat=Beginnt eine fußgesteuerte Schleife. Ihr Inhalt wird mindestens einmal ausgeführt; danach folgt `UNTIL`.
hint-keyword-until=Beendet eine `REPEAT`-Schleife und stoppt die Wiederholung, sobald ihre Bedingung wahr wird.
hint-keyword-loop=Beginnt eine Endlosschleife, die gewöhnlich mit `BREAK` oder `RETURN` verlassen wird.
hint-keyword-endloop=Beendet einen `LOOP`-Block und beginnt den nächsten Durchlauf.
hint-keyword-const=Deklariert eine benannte Konstante, die zur Übersetzungszeit ausgewertet wird.
hint-keyword-enum=Beginnt die Deklaration benannter Integerkonstanten, die zu einem gemeinsamen Enumtyp gehören.
hint-keyword-endenum=Beendet eine `ENUM`-Deklaration.
hint-keyword-type=Beginnt die Deklaration eines benutzerdefinierten Datensatztyps mit benannten Feldern.
hint-keyword-endtype=Beendet eine `TYPE`-Deklaration.
hint-keyword-begin=Beginnt nach den Deklarationen den ausführbaren Teil eines strukturierten PPL-400-Programms.
hint-keyword-onerror=Legt die Routine oder Marke fest, die Laufzeitfehler des aktuellen Programms behandelt.
hint-keyword-foreach=Beginnt eine PPL-400-Schleife über ein echtes Array. Jeder Durchlauf weist der Schleifenvariable das nächste Element zu; bei einem leeren Array gibt es keinen Durchlauf. Die Elementvariable muss zum Arraytyp kompatibel sein.
hint-keyword-endforeach=Beendet eine `FOREACH`-Schleife und wechselt zum nächsten Element der Sammlung.
hint-keyword-exit=Beendet das aktuelle PPE. In PPL 400 ersetzt diese kontextabhängige Anweisung den klassischen Anweisungsnamen `END`.
hint-keyword-usage=Verwendung
hint-const-builtin=Eine vordefinierte PPL-Konstante.

hint-type-boolean=Unsigned Char (1 Byte) 0 = `FALSE`, sonst `TRUE`
hint-type-date=Unsigned Integer (2 Bytes) PCBoard julianisches Datum (Anzahl der Tage seit 1/1/1900)
hint-type-ddate=
    Long Int mit Vorzeichen für julianisches Datum. DDATE ist für die Verwendung mit DBase-Datumsfeldern.
    Es hält einen langen Integer für julianische Daten. Wenn es in den Zeichenfolgentyp gezwungen wird, ist es im Format CCYYMMDD oder 19940527
hint-type-integer=Signed long Integer (4 Bytes) Bereich: -2,147,483,648 → +2,147,483,647
hint-type-money=Signed long Integer (4 Bytes) Bereich: -$21,474,836.48 → +$21,474,836.47
hint-type-string=Zeichenfolge mit maximaler Länge von 256 Zeichen
hint-type-string-unbounded=Unbegrenzte Zeichenfolge (unbounded string). Ab PPL 400 ist `STRING` nicht mehr auf 256 Zeichen beschränkt.
hint-type-time=Signed long Integer (4 Bytes) Anzahl der Sekunden seit Mitternacht
hint-type-bigstr=Zeichenfolge mit maximaler Länge von 2048 Zeichen. Kann auch CHR(0) Zeichen enthalten.
hint-type-edate=Julianisches Datum im Earth Datum Format YYMM.DD. Gleicher Bereich wie DATE.
hint-type-float=4-Byte Fließkommazahl Bereich: +/-3.4E-38 - +/-3.4E+38 (7-Stellen Präzision)
hint-type-double=8-Byte Fließkommazahl Bereich: +/-1.7E-308 - +/-1.7E+308 (15-Stellen Präzision)
hint-type-byte=1-Byte-Integer ohne Vorzeichen, Bereich: 0 bis 255
hint-type-sbyte=1-Byte-Integer mit Vorzeichen, Bereich: -128 bis 127
hint-type-unsigned=4-Byte unsigned Integer Bereich: 0 - 4,294,967,295
hint-type-long=8-Byte signed Integer Bereich: -9,223,372,036,854,775,808 - 9,223,372,036,854,775,807
hint-type-ulong=8-Byte unsigned Integer Bereich: 0 - 18,446,744,073,709,551,615
hint-type-bytes=Kompakte, zusammenhängende Binärdaten für Kodierungen, Prüfsummen und binäre Ein-/Ausgabe ohne die Kosten einzelner Elemente eines `BYTE[]`-Arrays.
hint-bytes-len=Liefert die Anzahl der Bytes in diesem Wert.
hint-bytes-to-string=Dekodiert diese Bytes als UTF-8-Text. Ungültiges UTF-8 meldet `ErrCode.Format`.
hint-bytes-to-base64=Kodiert diese Bytes als Base64-Text.
hint-bytes-to-hex=Liefert großgeschriebenen Hexadezimaltext mit zwei Stellen je Byte; führende Nullbytes bleiben erhalten.
hint-bytes-get-checksum=Berechnet den gewählten `Checksum`-Algorithmus und liefert Rohbytes: CRC32 erzeugt 4 Bytes in Netzwerkreihenfolge, MD5 16 Bytes und SHA256 32 Bytes. Ein ungültiger Algorithmus liefert leere `BYTES` und meldet `ErrCode.Invalid`.
hint-bytes-from-base64=Dekodiert Base64-Text mit oder ohne Padding in Rohbytes. ASCII-Leerraum wird ignoriert, daher wird auch MIME-umgebrochene Eingabe akzeptiert. Ungültige Eingabe liefert leere `BYTES` und meldet `ErrCode.Format`.
hint-type-regex=Ein kompilierter regulärer Ausdruck. Die Suche verwendet standardmäßig Unicode und garantiert lineare Laufzeit ohne Look-around oder Rückverweise.
hint-type-regex-match=Eine unveränderliche Momentaufnahme eines Treffers und seiner Capture-Gruppen.
hint-type-board=Schreibgeschützte Momentaufnahme der Boardkonfiguration und der indizierten Konferenz- und Benutzersammlungen.
hint-type-session=Aktiver Zustand des aktuellen Anrufers einschließlich ausgewählter Konferenz, Area, Verzeichnis und Benutzer.
hint-type-user=Ein Benutzerdatensatz. Beschreibbare Profilfelder aktualisieren den aktuellen Benutzer sofort; Momentaufnahmen bleiben schreibgeschützt.
hint-type-http=Statischer Einstiegspunkt für richtliniengesteuerte HTTP-Anfragen und Downloads.
hint-type-http-request=Veränderliche HTTP-Anfrage. `SetHeader`, `SetText` und `SetForm` ändern diese Anfrage und geben zurück, ob dies erfolgreich war.
hint-type-http-response=HTTP-Ergebnis mit Status, Headern und begrenzt gespeichertem Body oder ein ungültiger Wert mit Einzelheiten in `Error.Last()`.
hint-type-checksum=Von `BYTES.GetChecksum` verwendeter Algorithmus: `CRC32`, `MD5` oder `SHA256`.
hint-type-gfx=Die Grafiksitzung des Anrufers. Über `Terminal.Gfx` werden Backend und Frame-Pacing gesteuert.
hint-type-gfx-backend=Grafiktransport einer Sitzung: `None`, `Auto`, `Sixel` oder `Jxl`.
hint-type-surface=Eine nicht sichtbare RGBA-Zeichenfläche. Erstellung mit `Surface.New()`, Laden eines Bildes mit `Surface.Load()`.
hint-member-board-users=Die beim ersten Lesen von `Board` registrierten Benutzer als schreibgeschützte `USER[]`-Momentaufnahme.
hint-member-user-valid=Gibt an, ob dieses `USER`-Objekt einen vorhandenen Datensatz darstellt. Ein ungültiger `Board.Users`-Index liefert einen leeren Benutzer mit `Valid` gleich false.
hint-member-terminal-gfx=Die Grafiksitzung des Anrufers. Vor dem Erstellen oder Anzeigen von Flächen initialisieren und nach Abschluss der Ausgabe beenden.
hint-member-terminal-input=Tastatur- und Mauseingabe des Anrufers. Ereignismeldungen vor `Poll` oder `Wait` einschalten und die Eingabe mit `Release` an das Board zurückgeben.
hint-member-terminal-margins=Die aktiven Scroll- und Textausgaberänder des Terminals.
hint-member-margins-set-vertical=
    Setzt 1-basierte obere und untere Zeilen; `top` muss mindestens 1 und kleiner als `bottom` sein.
    <br><br>**Terminalprotokoll:** sendet DECSTBM `CSI top ; bottom r`, Bytes `ESC [ top ; bottom r`. Benötigt ein VT-/ANSI-Terminal mit DECSTBM-Unterstützung; nicht unterstützende Terminals können die Sequenz ignorieren.
hint-member-margins-set-horizontal=
    Setzt 1-basierte linke und rechte Spalten; `left` muss mindestens 1 und kleiner als `right` sein.
    <br><br>**Terminalprotokoll:** sendet zuerst DECLRMM `CSI ? 69 h` (DECSET Private Mode 69), danach DECSLRM `CSI left ; right s`. Benötigt ein DEC-kompatibles Terminal mit Unterstützung für linke/rechte Ränder; viele einfache ANSI-Terminals unterstützen dies nicht.
hint-member-margins-reset-vertical=
    Stellt die volle Terminalhöhe wieder her. <br><br>**Terminalprotokoll:** sendet DECSTBM-Reset `CSI r`, Bytes `ESC [ r`.
hint-member-margins-reset-horizontal=
    Stellt die volle Terminalbreite wieder her. <br><br>**Terminalprotokoll:** sendet `CSI ? 69 l` (DECRST Private Mode 69), Bytes `ESC [ ? 6 9 l`.
hint-member-margins-reset-all=
    Stellt beide Randachsen wieder her. <br><br>**Terminalprotokoll:** sendet DECSTBM-Reset `CSI r`, gefolgt von DECRST 69 `CSI ? 69 l`.
hint-member-margins-edge=Die aktuelle 1-basierte Randposition oder null, wenn dieser Rand nicht aktiv ist.
hint-member-margins-active=Gibt an, ob der entsprechende Terminalrand aktiv ist.
hint-member-terminput-poll=Liefert das nächste anstehende Eingabeereignis, ohne zu warten.
hint-member-terminput-wait=Wartet bis zur angegebenen Anzahl Millisekunden auf ein Eingabeereignis.
hint-member-terminput-mouse-on=Aktiviert Mausereignisse in Textzellen- oder Pixelkoordinaten und meldet, ob der Modus angenommen wurde. Tracking kann `Buttons`, `Drag` oder `All` sein; ohne Angabe gilt `MouseTracking.All`. <br><br>**Terminalprotokoll:** deaktiviert zunächst DECSET-Modi 1000, 1002 und 1003, aktiviert dann 1000 (Tasten), 1002 (Ziehen) oder 1003 (alle Bewegungen) und anschließend SGR-Mausmodus 1006. Der Pixelmodus aktiviert zusätzlich Modus 1016 und fragt ihn mit `CSI ? 1016 $ p` ab; der Textmodus deaktiviert 1016. Benötigt xterm-/DEC-Mausmeldungen, für Pixelkoordinaten zusätzlich Modus 1016.
hint-member-terminput-mouse-off=Deaktiviert Mausereignisse. <br><br>**Terminalprotokoll:** sendet DECRST für die privaten Modi 1000, 1002, 1003, 1006 und 1016: `CSI ? 1000 l` … `CSI ? 1016 l`.
hint-member-terminput-keyboard-on=Aktiviert physische Tastaturereignisse. Das optionale `echo`-Flag steuert, ob übersetzte Tasteneingaben unterdrückt werden. <br><br>**Terminalprotokoll:** aktiviert immer den physischen CTerm-Tastenmodus mit `CSI = 1 h`. Bei `echo = FALSE` wird zuvor `CSI = 2 l` gesendet, bei `echo = TRUE` dagegen `CSI = 2 h`. Dies sind CTerm-/SyncTERM-Erweiterungen, kein Standard-ANSI.
hint-member-terminput-keyboard-off=Deaktiviert physische Tastaturereignisse. <br><br>**Terminalprotokoll:** sendet `CSI = 2 l`, gefolgt von `CSI = 1 l`. Dies sind CTerm-/SyncTERM-Erweiterungen, kein Standard-ANSI.
hint-member-terminput-release=Deaktiviert Eingabeereignisse und gibt Tastatur und Maus an das Board zurück.
hint-type-terminal=Das aktive Terminal des Anrufers und Einstiegspunkt für Fähigkeiten, Eingabe, Grafik, Ränder, Palette, Makros, synchronisierte Ausgabe und ladbare Schriftarten.
hint-type-terminfo=Schreibgeschützte Momentaufnahme der beim Sitzungsstart ausgehandelten Terminalfähigkeiten.
hint-type-terminput=Steuerung der Tastatur- und Mausereignisse des aktuellen Anrufers.
hint-type-margins=Der aktive vertikale Scrollbereich und die horizontalen Textausgaberänder.
hint-type-palette=Steuert die 16 von `COLOR` verwendeten DOS-Farben mittels OSC-Palettenbefehlen.
hint-type-macros=Zeichnet rohe Terminalausgabe in 64 sitzungslokalen Slots auf und spielt Text und Escape-Sequenzen unverändert ab.
hint-type-audio=Ein Terminal-Audiokanal. Fehlgeschlagene Ladevorgänge liefern ein ungültiges Objekt; Einzelheiten stehen in `Error.Last()`.
hint-type-error=Momentaufnahme des letzten PPL-400-Operationsergebnisses mit Subsystem, Fehlercode, Meldung und optionalem Kanal.
hint-type-event=Ein Tastatur-, Maus-, Warteschlangenüberlauf- oder Audiokanalereignis aus `Terminal.Input`.
hint-type-msg=Schreibgeschützter Nachrichtenkopf mit verzögertem Zugriff auf den Nachrichtentext.
hint-type-conference=Eine Board-Konferenz mit Nachrichtenbereichen, Dateiverzeichnissen, Doors und Zugriffsprüfungen.
hint-type-area=Ein Konferenz-Nachrichtenbereich mit Zugriffs-, Lese- und Suchfunktionen.
hint-type-directory=Ein Konferenz-Dateiverzeichnis mit Download-Zugriffsinformationen.
hint-type-door=Ein konfiguriertes externes Programm oder Spiel mit Zugriffsanforderung.
hint-type-contact=Ein Dienst-/Kontopaar aus der schreibgeschützten Kontaktliste eines Benutzers.
hint-type-enum-400=Ein benannter PPL-400-Enumwert. Enumwerte werden mit ihrer dokumentierten Integerdarstellung gespeichert.
hint-member-terminal-info=Schreibgeschützte Fähigkeiten und Abmessungen, die mit dem Terminal des Anrufers ausgehandelt wurden.
hint-member-terminal-palette=Steuert die 16 DOS-Paletteneinträge über xterm-kompatible Befehle OSC 4 und OSC 104.
hint-member-terminal-macros=Zeichnet den an diesen Anrufer gesendeten Rohdatenstrom auf und spielt ihn wieder ab. Slots gelten nur für diese Sitzung.
hint-member-terminal-begin-update=
    Startet oder verschachtelt eine synchronisierte Terminalaktualisierung. Unterstützende Terminals puffern die Zeichenausgabe bis zum passenden äußersten `EndUpdate`; das vermeidet sichtbares Flackern. Bei fehlender Unterstützung wird `FALSE` mit `ErrKind.Term`/`ErrCode.Unavailable` geliefert.
    <br><br>**Terminalprotokoll:** der äußerste Aufruf sendet `CSI ? 2026 h`, Bytes `ESC [ ? 2 0 2 6 h`. Dies ist DEC Private Mode 2026, **Synchronized Output**; die Unterstützung steht in `Terminal.Info.SynchronizedOutput`.
hint-member-terminal-end-update=
    Beendet eine Verschachtelungsebene. Beim äußersten Ende zeigt das Terminal die gesammelte Ausgabe an. Ohne aktive Aktualisierung wird `FALSE` mit `ErrCode.Invalid` geliefert.
    <br><br>**Terminalprotokoll:** das äußerste Ende sendet `CSI ? 2026 l`, Bytes `ESC [ ? 2 0 2 6 l` (DECRST Private Mode 2026).
hint-member-terminal-set-font=
    Wählt Schriftart 0–255 für Attributslot 0–3. Ohne `slot` wird sie allen vier Slots zugewiesen; `LoadFont` kann die beschreibbaren Schriftnummern 43–255 laden.
    <br><br>**Terminalprotokoll:** sendet die SyncTERM-/CTerm-Erweiterung `CSI slot ; font SP D`, Bytes `ESC [ slot ; font SPACE D`, einmal je ausgewähltem Attributslot. Dies ist kein Standard-ANSI.
hint-member-terminal-load-font=
    Liest eine board-relative Schriftdatei, dekodiert sie mit `BitFont` und lädt sie in eine beschreibbare Terminal-Schriftnummer von 43–255. Datei-, Format- und Sitzungslimits stehen in `Error.Last()`.
    <br><br>**Terminalprotokoll:** sendet die CTerm-DCS `DCS CTerm:Font:font:base64 ST`, Bytes `ESC P CTerm:Font:… ESC \`. Die Nutzdaten sind base64-kodierte Bitmapdaten für 256 Glyphen. Dies ist eine CTerm-Erweiterung, kein Standard-ANSI.
hint-member-terminfo-program=Erkanntes Terminalprogramm: `IcyTerm`, `SyncTerm` oder `Unknown`.
hint-member-terminfo-device-attrs=Rohe primäre/sekundäre Device-Attributes-Antwort aus der Terminalaushandlung.
hint-member-terminfo-cells=Aktuelle Terminalgröße in Textspalten beziehungsweise Textzeilen.
hint-member-terminfo-utf8=Gibt an, ob UTF-8 statt einer alten Codepage ausgehandelt wurde.
hint-member-terminfo-rip=Ausgehandelte RIP-Grafikversion oder eine leere Zeichenfolge ohne RIP.
hint-member-terminfo-cterm=Erkannte CTerm-Protokollrevision oder null ohne CTerm-Erweiterungen.
hint-member-terminfo-graphics=Gibt an, ob das Terminal den gewählten Inline-Grafiktransport unterstützt: Sixel, JPEG XL oder allgemeine Inline-Blobs.
hint-member-terminfo-audio=Gibt an, ob terminalseitige Audiowiedergabe verfügbar ist.
hint-member-terminfo-physical-keys=Gibt an, ob physische Tastenübergänge unabhängig von übersetzter Texteingabe gemeldet werden.
hint-member-terminfo-pixel-mouse=Gibt an, ob Mauskoordinaten in Pixeln gemeldet werden können (CTerm-Revision mindestens 1330).
hint-member-terminfo-client-blit=Gibt an, ob clientseitiges Bild-Blitting verfügbar ist (CTerm-Revision mindestens 1318).
hint-member-terminfo-synchronized-output=Gibt an, ob der private DEC-Modus 2026 für `BeginUpdate` und `EndUpdate` angeboten wird.
hint-member-terminfo-terminal-macros=Gibt an, ob Terminalmakrofunktionen ausgehandelt wurden.
hint-member-terminfo-cell-pixels=Erkannte Breite beziehungsweise Höhe einer Textzelle in Pixeln; null, wenn unbekannt.
hint-member-terminfo-screen-pixels=Erkannte physische Bildschirmbreite beziehungsweise -höhe in Pixeln; null, wenn unbekannt.
hint-member-palette-set=
    Ersetzt DOS-Farbe 0–15 durch gepacktes `0xRRGGBBAA`; Alpha wird ignoriert. Ungültige Farben setzen `ErrCode.Invalid`.
    <br><br>**Terminalprotokoll:** sendet xterm `OSC 4 ; index ; rgb:RR/GG/BB ST`, Bytes `ESC ] 4 ; index ; rgb:RR/GG/BB ESC \`. Benötigt eine xterm-kompatible beschreibbare Palette. DOS-Indizes werden nach ANSI als `0,4,2,6,1,5,3,7,8,12,10,14,9,13,11,15` abgebildet.
hint-member-palette-reset=
    Setzt eine DOS-Farbe 0–15 auf den Terminalstandard zurück.
    <br><br>**Terminalprotokoll:** sendet xterm `OSC 104 ; index ST`, Bytes `ESC ] 104 ; index ESC \`, nach DOS-zu-ANSI-Übersetzung.
hint-member-palette-reset-all=
    Stellt alle 16 Farben auf die DOS-Standardpalette zurück.
    <br><br>**Terminalprotokoll:** sendet einen kombinierten OSC-4-Befehl mit allen 16 Paaren `index ; rgb:RR/GG/BB`, abgeschlossen durch ST (`ESC \`). Benötigt xterm-kompatible OSC-4-Unterstützung.
hint-member-macros-recording=Gibt an, ob gerade ein Terminalmakro aufgezeichnet wird.
hint-member-macros-begin-record=Startet die Aufzeichnung aller Rohdaten für Slot 0–63. Text, Steuerzeichen und ANSI/VT-Sequenzen bleiben bytegenau erhalten; jeder Datenstrom ist auf 512 KiB begrenzt. Bis zur Wiedergabe wird keine Terminalsequenz erzeugt.
hint-member-macros-end-record=Beendet die aktive Aufzeichnung und speichert die Anrufer-/Sysop-Datenströme. Es wird keine Terminalsequenz gesendet.
hint-member-macros-play=Schreibt die aufgezeichneten Bytes direkt in den Anrufer-/Sysop-Terminalstrom. Sie können beliebige ANSI-/VT- oder proprietäre Sequenzen enthalten; es erfolgt weder Umwandlung noch Fähigkeitsprüfung.
hint-member-macros-delete=Löscht einen sitzungslokalen Makroslot. Es wird keine Terminalsequenz gesendet.
hint-member-macros-delete-all=Löscht alle sitzungslokalen Makroslots. Es wird keine Terminalsequenz gesendet.
hint-member-audio-valid=Gibt an, ob dieser Wert einen erfolgreich geladenen Terminal-Audiokanal bezeichnet.
hint-member-audio-playing=Gibt an, ob dieser geladene Kanal momentan als aktiv verfolgt wird.
hint-member-audio-set-volume=Setzt die logische Lautstärke von 0 bis 100 und gibt zurück, ob dies erfolgreich war. Bei einem Fehler liefert `Error.Last()` `ErrKind.Audio`, Fehlercode, Meldung und Kanal.
hint-member-audio-channel=Logischer PPL-Kanal 0–13; er entspricht SyncTERM-/CTerm-Kanal 2–15.
hint-member-audio-play=Startet die Wiedergabe; optionales `looping` ist standardmäßig `FALSE`. <br><br>**Terminalprotokoll:** sendet `APC SyncTERM:A;Load;S=slot;cache ST`, `…;Volume;C=channel;V=dB ST` und `…;Queue;C=channel;S=slot[;L] ST`. Ohne Schleife folgt `…;Update;C=channel ST`. Benötigt SyncTERM-/CTerm-Audio-APC.
hint-member-audio-stop=Stoppt diesen Kanal, ohne seine Daten freizugeben. <br><br>**Terminalprotokoll:** sendet `APC SyncTERM:A;Flush;C=channel;O=0 ST` (`APC` = `ESC _`, `ST` = `ESC \`).
hint-member-audio-fade=Blendet den Kanal in `durationMs` auf `targetVolume`. <br><br>**Terminalprotokoll:** sendet `APC SyncTERM:A;Volume;C=channel;V=dB;T=durationMs ST`; Lautstärke 0–100 wird in Dezibel umgerechnet.
hint-member-audio-free=Stoppt und gibt den Kanal frei. <br><br>**Terminalprotokoll:** sendet `APC SyncTERM:A;Flush;C=channel;O=0 ST`; der Clientcache kann zur Wiederverwendung erhalten bleiben.
hint-member-audio-load=Lädt eine board-relative WAV-, AIFF-, FLAC-, Ogg/Vorbis- oder Opus-Datei bis 16 MiB. <br><br>**Terminalprotokoll:** prüft mit `APC SyncTERM:Q;libsndfileFormat;major;subtype ST`, erwartet `CSI = 7 ; 101 ; major ; subtype ; supported n` und lädt mit `APC SyncTERM:C;S;cacheName;base64 ST`. Benötigt SyncTERM-/CTerm-Medien- und Audioerweiterungen.
hint-member-audio-stop-all=Stoppt alle aktiven PPL-Audiokanäle mit `APC SyncTERM:A;Flush;C=channel;O=0 ST` pro Kanal.
hint-member-error-ok=Gibt an, ob `Code` gleich `ErrCode.Ok` ist.
hint-member-error-kind=Subsystem des Ergebnisses, etwa Datei, Grafik, Schrift, Audio, Terminal, Nachricht, Netzwerk, Benutzer, String oder Regex.
hint-member-error-code=Maschinenlesbare Ergebniskategorie wie ungültige Eingabe, fehlende Fähigkeit, E/A-Fehler, Limit oder Zeitüberschreitung.
hint-member-error-message=Menschenlesbares Diagnosedetail. Programme sollten `Kind` und `Code` statt übersetzter Texte auswerten.
hint-member-error-channel=Zugehöriger Audio-/Medienkanal oder `-1`, wenn das Ergebnis keinen Kanal betrifft.
hint-member-error-last=Liefert eine stabile Kopie des zuletzt veröffentlichten PPL-400-Operationsergebnisses.
hint-member-error-clear=Löscht den gespeicherten Fehler und liefert `TRUE`.
hint-member-event-kind=Unterscheidungsmerkmal dieses Ereignisses. Nur die zum gewählten `EventKind` gehörenden Eigenschaften sind relevant.
hint-member-event-key=Übersetzter Tastencode oder Text für `EventKind.Key`; andere Arten liefern null oder eine leere Zeichenfolge.
hint-member-event-scan-code=Physischer Tastencode für `EventKind.KeyEdge`; andere Arten liefern null.
hint-member-event-pressed=Druckstatus für `EventKind.Key` oder `EventKind.KeyEdge`; andere Arten liefern `FALSE`.
hint-member-event-repeated=Wiederholungsstatus für `EventKind.KeyEdge`; andere Arten liefern `FALSE`.
hint-member-event-position=Mausposition und Koordinatenmodus für `EventKind.Mouse`; andere Arten liefern neutrale Werte.
hint-member-event-mouse=Mausaktion, geänderte Taste oder Mausradwert für `EventKind.Mouse`; andere Arten liefern neutrale Werte.
hint-member-event-time=Terminal-Ereigniszeitstempel in Millisekunden.
hint-member-event-channel=Audiokanal, der durch ein Ereignis `EventKind.Audio` als geleert gemeldet wird.
hint-member-event-dropped=Anzahl verlorener Warteschlangeneinträge vor einem Ereignis `EventKind.Overflow`.
hint-member-event-buttons=Gibt an, ob die entsprechende Maustaste bei `EventKind.Mouse` gehalten wurde; andere Arten liefern `FALSE`.
hint-member-event-modifiers=Gibt an, ob der entsprechende Modifikator bei `EventKind.Key` oder `EventKind.Mouse` aktiv war; andere Arten liefern `FALSE`.
hint-member-msg-number=Nachrichtennummer, Antwortziel oder gespeicherte Textgröße aus dem schreibgeschützten JAM-Kopf.
hint-member-msg-valid=Gibt an, ob dieser Wert eine vorhandene Nachricht bezeichnet. Fehlende Nummern liefern ein ungültiges `MSG`, ohne die Memberkette abzubrechen.
hint-member-msg-header=Schreibgeschützter Absender, Empfänger, Betreff oder Statustext aus dem Nachrichtenkopf.
hint-member-msg-written=Erstellungsdatum beziehungsweise -zeit; eine ungültige Nachricht liefert null.
hint-member-msg-flags=Schreibgeschütztes Nachrichtenattribut aus dem JAM-Kopf.
hint-member-msg-text=Lädt den Nachrichtentext bei Bedarf. E/A-Fehler liefern eine leere Zeichenfolge und aktualisieren `Error.Last()`.
hint-member-contact-service=Name des Kontaktdienstes, etwa E-Mail, Web, IRC oder ein anderer konfigurierter Dienst.
hint-member-contact-account=Kontoname oder Adresse bei diesem Kontaktdienst.
hint-member-conference-identity=Schreibgeschützter Konferenzname, konfigurierte Nummer oder Gültigkeitsstatus.
hint-member-conference-options=Schreibgeschütztes Konferenzverhalten und Nachrichtenrichtlinien.
hint-member-conference-password=Geschütztes Konferenzkennwort; Zugriffsprüfungen können es verwenden, der Klartext wird nicht offengelegt.
hint-member-conference-collections=Schreibgeschützte eindimensionale Momentaufnahme der Dateiverzeichnisse, Nachrichtenbereiche beziehungsweise Doors dieser Konferenz.
hint-member-conference-access=Prüft die Sicherheit des aktuellen Anrufers und die Konferenzkonfiguration für allgemeinen Zugriff, Schreiben oder Anhänge.
hint-member-area-identity=Schreibgeschützter Bereichsname, konfigurierte Nummer oder Gültigkeitsstatus.
hint-member-area-options=Schreibgeschützte Bereichsrichtlinien sowie QWK-/Echomail-Metadaten.
hint-member-area-access=Prüft, ob der aktuelle Anrufer auf diesen Bereich zugreifen, ihn betreten oder Dateien anhängen darf.
hint-member-area-range=Niedrigste beziehungsweise höchste vorhandene Nachrichtennummer in der Nachrichtenbasis dieses Bereichs.
hint-member-area-read=Liest eine genaue Nachrichtennummer. Eine fehlende Nachricht liefert ein ungültiges `MSG`; dazu `Valid` prüfen.
hint-member-area-find=Sucht die nächste Nachricht, deren Feld `To`, `From` oder `Subject` den Text enthält. Das optionale `startAfter` setzt nach einer Nachrichtennummer fort.
hint-member-directory-identity=Schreibgeschützter Dateiverzeichnisname, konfigurierte Nummer oder Gültigkeitsstatus.
hint-member-directory-options=Schreibgeschützter Speicherpfad, Gratisdownload-/Neue-Dateien-Status oder geschütztes Kennwort.
hint-member-directory-access=Prüft die Sicherheit des aktuellen Anrufers für Verzeichniszugriff beziehungsweise Download.
hint-member-door-identity=Schreibgeschützter Doorname, konfigurierte Nummer oder Gültigkeitsstatus.
hint-member-door-options=Schreibgeschützte Beschreibung, Programmpfad oder geschütztes Kennwort dieses externen Programms.
hint-member-door-access=Prüft, ob der aktuelle Anrufer die Sicherheitsanforderung dieser Door erfüllt.
hint-member-board-property=Schreibgeschützte Boardidentität, Standort, Betreiber-/Sysopname oder konfigurierte Nodeanzahl.
hint-member-board-conferences=Schreibgeschützte `CONFERENCE[]`-Momentaufnahme der konfigurierten Konferenzen.
hint-member-user-editor-mode=Bevorzugte Vollbildeditorrichtlinie: immer verwenden, nie verwenden oder jedes Mal fragen.
hint-member-user-profile=Benutzerprofil oder Nutzungsstatistik. `Session.User` ist live und soweit unterstützt beschreibbar; Einträge aus `Board.Users` sind schreibgeschützte Momentaufnahmen.
hint-enum-event-kind-none=Es war kein Ereignis verfügbar, etwa nach einem nicht blockierenden `Poll` oder einem abgelaufenen `Wait`.
hint-enum-event-kind-value=Wählt die relevante `EVENT`-Eigenschaftsgruppe: übersetzte Taste, physische Tastenkante, Maus, Warteschlangenüberlauf oder Audioende.
hint-enum-mouse-action=Von `EVENT.Action` gemeldeter Mausübergang: keiner, Drücken, Loslassen, Bewegung oder Rad.
hint-enum-mouse-button=Maustaste oder Radrichtung, die das Ereignis ausgelöst hat; gehaltene Tasten stehen separat als Booleans bereit.
hint-enum-mouse-mode-text=Meldet Mauskoordinaten in 1-basierten Terminal-Textzellen.
hint-enum-mouse-mode-pixels=Meldet Mauskoordinaten in Pixeln; erfordert `Terminal.Info.PixelMouse`.
hint-enum-mouse-tracking=Wählt reine Tasten-, Ziehbewegungs- oder vollständige Bewegungsmeldung.
hint-enum-error-kind=Subsystemkategorie aus `Error.Kind`; `None` bedeutet keinen Subsystemfehler.
hint-enum-error-code=Portables Operationsergebnis aus `Error.Code`: Erfolg, nicht verfügbar, ungültig, E/A, Format, Limit, nicht unterstützt, Stack, verweigert oder Zeitüberschreitung.
hint-enum-editor-mode=Benutzerwunsch für den Vollbildeditor: `Yes`, `No` oder `Ask`.
hint-enum-msg-field=Von `AREA.Find` durchsuchtes Nachrichtenkopffeld: Empfänger, Absender oder Betreff.
hint-enum-http-method=Von der richtliniengesteuerten Anfrage unterstützte HTTP-Methode: GET, HEAD oder POST.
hint-enum-regex-options=Bitflags für die Regex-Kompilierung: `None`, `IgnoreCase`, `MultiLine`, `DotMatchesNewLine`, `IgnoreWhitespace`, `SwapGreed` und `Ascii`. Mehrere Flags werden mit `|` kombiniert.
hint-enum-string-comparison=Ordinale Unicode-Zeichenfolgenprüfung mit oder ohne Beachtung der Groß-/Kleinschreibung.
hint-enum-checksum=Algorithmus für `Bytes.GetChecksum`: `CRC32` liefert 4 Rohbytes in Netzwerkreihenfolge, `MD5` 16 Bytes und `SHA256` 32 Bytes. Für Textdarstellung kann `ToHex()` aufgerufen werden.
hint-member-gfx-init=
    Startet eine Grafiksitzung mit dem angeforderten `GfxBackend`. `Auto` wählt die beste Terminalfähigkeit; Vollbild ist standardmäßig `TRUE`.
    <br><br>**Terminalprotokoll:** `Auto` verwendet ausgehandelte Sixel-/JPEG-XL-Fähigkeiten; eine explizite Sixel-Anforderung führt keine zusätzliche Abfrage aus. Vollbild sendet `CSI 2 J`, `CSI H`, danach `CSI ? 25 l`, `CSI ? 7 l`, `CSI ? 80 l` und `CSI ? 1070 l`. Sixel benötigt Sixel-DCS, JPEG XL SyncTERM-/CTerm-Medien-APC.
hint-member-gfx-shutdown=
    Beendet die Grafiksitzung und stellt die normale Textausgabe wieder her.
    <br><br>**Terminalprotokoll:** Vollbild sendet `CSI ? 80 h`, `CSI ? 7 h` und `CSI ? 25 h`. Nach Sixel-Ausgabe werden alle 16 DOS-Farben mit OSC 4 restauriert; danach folgen SGR-Reset `CSI 0 m` und die Boardfarbe.
hint-member-gfx-backend=Das ausgewählte schreibgeschützte `GfxBackend`, oder `None`, wenn keine Grafiksitzung aktiv ist.
hint-member-gfx-set-pacing=Aktiviert oder deaktiviert das Frame-Pacing und gibt zurück, ob dies erfolgreich war. Bei Aktivierung wartet die Ausgabe auf die Terminalbestätigung, bevor der nächste Frame gesendet wird. Bei einem Fehler liefert `Error.Last()` Details.
hint-param-backend=Angefordertes Grafik-Backend; `Auto` wählt das beste verfügbare Backend.
hint-param-enabled=Gibt an, ob das Frame-Pacing aktiviert werden soll.
hint-parameters-title=Parameter
hint-param-optional=optional
hint-param-fullscreen=Gibt an, ob der Bildschirm gelöscht und das Terminal in den Vollbild-Grafikmodus geschaltet wird. Standardwert ist `TRUE`.
hint-param-top=1-basierte erste Zeile des vertikalen Scrollbereichs.
hint-param-bottom=1-basierte letzte Zeile des vertikalen Scrollbereichs.
hint-param-left=1-basierte erste Spalte des horizontalen Randbereichs.
hint-param-right=1-basierte letzte Spalte des horizontalen Randbereichs.
hint-param-timeout-ms=Maximale Wartezeit in Millisekunden.
hint-param-mode=Zu verwendender Eingabe-, Darstellungs- oder Betriebsmodus.
hint-param-tracking=Maus-Tracking-Richtlinie; ohne Angabe gilt `MouseTracking.All`.
hint-param-echo=Gibt an, ob angenommene Tastatureingaben zusätzlich ausgegeben werden. Standardwert ist `FALSE`.
hint-param-color=DOS-Palettenfarbe von 0 bis 15.
hint-param-rgba=Gepackte Farbe im Format `0xRRGGBBAA`.
hint-param-slot=Nummer des Zielslots.
hint-param-looping=Gibt an, ob die Wiedergabe nach dem Ende erneut beginnt. Standardwert ist `FALSE`.
hint-param-duration-ms=Dauer der Überblendung in Millisekunden.
hint-param-target-volume=Ziellautstärke von 0 bis 100.
hint-param-volume=Lautstärke von 0 bis 100.
hint-param-font=Nummer der Terminalschriftart.
hint-param-file=Board-relativer Name der Quell- oder Zieldatei.
hint-param-password=Neues Klartextpasswort, das geprüft und sicher gespeichert wird.
hint-param-service=Name des Kontaktdienstes, beispielsweise E-Mail- oder Chat-Anbieter.
hint-param-account=Kontoname oder Adresse beim gewählten Dienst.
hint-param-index=Nullbasierter Element- oder Capture-Gruppenindex.
hint-param-text=Von der Operation verwendeter Eingabe- oder Ersetzungstext.
hint-param-url=Absolute HTTP-URL, die von der Netzwerk-Richtlinie des Boards zugelassen sein muss.
hint-param-method=Zu verwendende HTTP-Methode: `Get`, `Head` oder `Post`.
hint-param-name=Name des auszuwählenden Headers, der Gruppe oder des Feldes.
hint-param-value=Wert, der dem ausgewählten Namen zugewiesen wird.
hint-param-content-type=Optionaler MIME-Inhaltstyp des Text-Bodys.
hint-param-form=Optionaler Dialektschalter; `TRUE` (Vorgabe) nutzt die Regeln von `application/x-www-form-urlencoded`, bei denen ein Leerzeichen `+` ist, `FALSE` nutzt die Regeln von RFC 3986, bei denen ein Leerzeichen `%20` ist.
hint-param-pattern=Zu kompilierendes oder zu prüfendes reguläres Ausdrucksmuster.
hint-param-options=Optionale `RegexOptions`-Flags; Standardwert ist `RegexOptions.None`.
hint-param-start=Optionale nullbasierte Unicode-Zeichenposition, an der die Suche beginnt.
hint-param-limit=Optionale Höchstzahl der Ergebnisse oder Ersetzungen; null verwendet die dokumentierte unbegrenzte Obergrenze.
hint-param-replacement=Ersetzungsvorlage; `$1` und `$name` setzen Capture-Gruppen ein.
hint-param-message-number=Nummer der zu lesenden Nachricht.
hint-param-field=Für die Operation ausgewähltes Feld.
hint-param-start-message=Optionale Nachrichtennummer, bei der die Suche beginnt.
hint-param-x=Nullbasierte horizontale Pixelkoordinate.
hint-param-y=Nullbasierte vertikale Pixelkoordinate.
hint-param-width=Breite in Pixeln.
hint-param-height=Höhe in Pixeln.
hint-param-source=Quellfläche, von der kopiert wird.
hint-param-source-x=Nullbasierte linke Kante des Quellrechtecks.
hint-param-source-y=Nullbasierte obere Kante des Quellrechtecks.
hint-param-source-width=Breite des Quellrechtecks in Pixeln.
hint-param-source-height=Höhe des Quellrechtecks in Pixeln.
hint-param-destination-x=Nullbasierte horizontale Ziel-Pixelkoordinate.
hint-param-destination-y=Nullbasierte vertikale Ziel-Pixelkoordinate.
hint-param-column=1-basierte Terminal-Textspalte.
hint-param-row=1-basierte Terminal-Textzeile.
hint-param-destination-width=Optionale Ausgabebreite in Pixeln; ohne Angabe bleibt die Quellbreite erhalten.
hint-param-destination-height=Optionale Ausgabehöhe in Pixeln; ohne Angabe bleibt die Quellhöhe erhalten.
hint-param-flip=Optionale Spiegelungsflags des Grafik-Backends.
hint-member-gfx-backend-none=Es ist kein Grafik-Backend aktiv.
hint-member-gfx-backend-auto=Wählt das beste von `Terminal.Info` gemeldete Grafik-Backend.
hint-member-gfx-backend-sixel=Verwendet Sixel-Grafik.
hint-member-gfx-backend-jxl=Verwendet das JPEG-XL-Grafikprotokoll.
hint-member-surface-dimension=Die schreibgeschützte Abmessung der Fläche in Pixeln.
hint-member-surface-valid=Gibt an, ob diese Fläche ein aktives Bild referenziert. Ressourcenfehler liefern eine ungültige Fläche und setzen `Error.Last()`.
hint-member-surface-clear=Füllt die gesamte Fläche mit einer gepackten `0xRRGGBBAA`-Farbe.
hint-member-surface-set-pixel=Schreibt eine gepackte `0xRRGGBBAA`-Farbe an die nullbasierten Pixelkoordinaten.
hint-member-surface-get-pixel=Liefert die gepackte `0xRRGGBBAA`-Farbe an den nullbasierten Pixelkoordinaten.
hint-member-surface-fill-rect=Füllt das Pixelrechteck mit einer gepackten `0xRRGGBBAA`-Farbe.
hint-member-surface-draw-rect=Umrandet das Pixelrechteck mit einer gepackten `0xRRGGBBAA`-Farbe.
hint-member-surface-blit=Überblendet die Quellfläche per Alpha-Compositing an den Ziel-Pixelkoordinaten.
hint-member-surface-blit-rect=Überblendet ein Quell-Pixelrechteck per Alpha-Compositing an den Ziel-Pixelkoordinaten.
hint-member-surface-present=Zeigt die gesamte Fläche an der aktuellen Terminalposition an. <br><br>**Terminalprotokoll:** Sixel sendet ein Sixel-DCS-Bild. JPEG XL verwendet `APC SyncTERM:C;DrawJXLBlob;DX=x;DY=y;base64 ST` oder lädt mit `…;S;cacheName;base64 ST` und zeichnet mit `…;DrawJXL;DX=x;DY=y;cacheName ST`.
hint-member-surface-present-at=Zeigt die gesamte Fläche an der angegebenen 1-basierten Textspalte und -zeile an. <br><br>**Terminalprotokoll:** Sixel sendet `CSI ? 1070 h`, speichert mit `ESC 7`, positioniert mit `CSI row ; column H`, sendet Sixel-DCS, stellt mit `ESC 8` wieder her und sendet `CSI ? 1070 l`. JPEG XL rechnet Zellen in Pixel um und verwendet SyncTERM-APC `DrawJXLBlob`/`DrawJXL`.
hint-member-surface-present-rect=Zeigt ein Quell-Pixelrechteck mit optionalem Ziel, Skalierung und Spiegelung an. <br><br>**Terminalprotokoll:** unveränderte Ausgabe kann Sixel-DCS verwenden. Skalierung oder Spiegelung benötigt JPEG XL und SyncTERM-/CTerm-APC `DrawJXLBlob` oder `DrawJXL` mit `DX`, `DY` und Transformationsoptionen.
hint-member-surface-pin=Lädt einen unveränderlichen JPEG-XL-Clientpuffer. <br><br>**Terminalprotokoll:** sendet `APC SyncTERM:C;LoadJXLBlob;B=buffer;base64 ST`. Benötigt JPEG XL und SyncTERM-/CTerm-Clientpuffer-Unterstützung.
hint-member-surface-unpin=Löst die serverseitige Zuordnung zum JPEG-XL-Clientpuffer. Es wird keine Terminalsequenz gesendet; der Client darf den Cache behalten.
hint-member-surface-free=Gibt die Fläche und ihren residenten Pixelspeicher frei.
hint-member-surface-new=Erstellt eine transparente Fläche mit den angeforderten Pixelabmessungen. Flächen sind auf 2048 mal 2048 Pixel begrenzt.
hint-member-surface-load=Dekodiert eine Bilddatei in eine Fläche. Die Quelldatei ist auf 32 MiB begrenzt.
hint-member-session-context=Das aktuell von dieser aktiven Sitzung ausgewählte Objekt. Nach einem Wechsel von Konferenz, Area, Verzeichnis oder Benutzer muss es erneut gelesen werden.
hint-member-session-value=Ein aktiver, schreibgeschützter Wert aus der Sitzung des aktuellen Anrufers.
hint-member-user-record-number=Die nullbasierte permanente Nummer des Benutzerdatensatzes oder `-1`, wenn der Benutzer nicht gespeichert ist.
hint-member-user-contacts=Schreibgeschützte `CONTACT[]`-Momentaufnahme mit höchstens 100 Einträgen. Der aktuelle Benutzer wird mit `AddContact` und `RemoveContact` geändert.
hint-member-user-notes=Schreibgeschützte `STRING[]`-Momentaufnahme der fünf Notizfelder. Der aktuelle Benutzer wird mit `SetNote` aktualisiert.
hint-member-user-set-password=Prüft und hasht ein neues Passwort für den aktuellen Benutzer. Momentaufnahmen können keine Passwörter ändern.
hint-member-user-contact-method=Fügt beim aktuellen Benutzer einen Kontakt hinzu oder entfernt ihn und meldet Fehler über `Error.Last()`.
hint-member-user-set-note=Aktualisiert eines der fünf Notizfelder des aktuellen Benutzers.
hint-http-get=Führt eine richtliniengesteuerte GET-Anfrage aus und liefert eine `HTTPRESPONSE`. Transport- oder Richtlinienfehler liefern eine ungültige Antwort und Einzelheiten über `Error.Last()`; HTTP-Fehlerstatus bleiben gültige Antworten.
hint-http-new=Erzeugt eine veränderliche `HTTPREQUEST` für `HttpMethod.Get`, `Head` oder `Post` und die angegebene URL. Netzwerkzugriff erfolgt erst beim Aufruf von `Send()`.
hint-http-download=Streamt eine erfolgreiche GET-Antwort zunächst in eine temporäre board-relative Datei und übernimmt sie erst nach vollständigem Abschluss atomar. Das konfigurierte Größenlimit gilt; bei Fehlern bleibt das Ziel unverändert und die Antwort ist ungültig.
hint-http-url-encode=Prozentkodiert ein einzelnes Formularfeld oder eine URL-Komponente. Kodiere nur einzelne Werte, niemals eine ganze `name=value&...`-Zeichenkette, da die Trennzeichen unkodiert bleiben müssen.
hint-http-url-decode=Kehrt `UrlEncode()` um. Bytefolgen, die kein gültiges UTF-8 sind, werden ersetzt und nicht gemeldet.
hint-http-request-property=Schreibgeschützte Metadaten der Anfrage.
hint-http-request-set-header=Setzt den Header dieser Anfrage und gibt zurück, ob dies erfolgreich war. Gesperrte oder ungültige Header geben `FALSE` zurück; `Error.Last()` meldet `ErrKind.Net` und `ErrCode.Invalid`.
hint-http-request-set-text=Setzt den UTF-8-Body und den optionalen Inhaltstyp dieser Anfrage und gibt zurück, ob dies erfolgreich war. GET- und HEAD-Anfragen geben `FALSE` zurück; `Error.Last()` meldet den Fehler. Der Body wird unverändert gesendet, Formularfelder müssen daher zuvor mit `Http.UrlEncode()` kodiert werden.
hint-http-request-set-form=Hängt ein prozentkodiertes `application/x-www-form-urlencoded`-Feld an den Body dieser Anfrage an und setzt diesen Inhaltstyp. Wiederholte Aufrufe sammeln sich an und behalten doppelte Namen. GET- und HEAD-Anfragen sowie ein Body, der keine Formulardaten enthält, geben `FALSE` zurück.
hint-http-request-send=Sendet diese Anfrage gemäß der Board-HTTP-Richtlinie und liefert ihre Antwort.
hint-http-response-property=Schreibgeschützte Antwortmetadaten. Vor der Nutzung von Netzwerkdaten `Valid` und für einen 2xx-Status `OK` prüfen.
hint-http-response-text=Dekodiert den gespeicherten Antwortbody strikt als UTF-8. Ungültiger Text meldet `ErrCode.Format`.
hint-http-response-header=Liefert den angegebenen Antwortheader oder einen leeren String, wenn er fehlt.
hint-http-response-save=Schreibt den gespeicherten Antwortbody atomar in eine board-relative Datei.
hint-regex-valid=Gibt an, ob der reguläre Ausdruck erfolgreich kompiliert wurde.
hint-regex-pattern=Das Quellmuster dieses regulären Ausdrucks.
hint-regex-compile=Kompiliert ein Muster mit optionalen `RegexOptions`. Ein ungültiges Muster liefert ein ungültiges `REGEX` und setzt `Error.Last()`.
hint-regex-escape=Maskiert alle Regex-Metazeichen in einem Literaltext.
hint-regex-is-valid=Prüft, ob Muster und optionale Optionen kompiliert werden können, ohne einen Fehler zu melden.
hint-regex-is-match=Prüft ab einer optionalen nullbasierten Unicode-Zeichenposition auf einen Treffer.
hint-regex-find=Liefert den ersten Treffer ab einer optionalen nullbasierten Unicode-Zeichenposition.
hint-regex-find-all=Liefert Treffer ab einer optionalen nullbasierten Startposition. Ein positives Limit begrenzt die Anzahl; null erlaubt bis zu 100.000 Treffer.
hint-regex-replace=Ersetzt alle Treffer oder höchstens ein positives Limit und unterstützt `$1`- und `$name`-Gruppen.
hint-regex-split=Liefert durch Aufteilen des Texts ein dynamisches BIGSTR-Array. Leere Felder bleiben erhalten; ein positives Limit belässt den Rest im letzten Element.
hint-regex-match-success=Gibt an, ob ein Treffer gefunden wurde.
hint-regex-match-value=Der vollständige Treffertext.
hint-regex-match-start=Die nullbasierte Unicode-Zeichenposition des Treffers oder -1 ohne Treffer.
hint-regex-match-length=Die Länge des Treffers in Unicode-Zeichen.
hint-regex-match-group-count=Die Anzahl der Capture-Gruppen ohne Gruppe null für den vollständigen Treffer.
hint-regex-match-group=Liefert eine nummerierte Capture-Gruppe. Gruppe null ist der vollständige Treffer.
hint-regex-match-named-group=Liefert eine benannte Capture-Gruppe.
hint-regex-match-group-matched=Gibt an, ob die gewählte optionale Capture-Gruppe am Treffer beteiligt war.
hint-regex-match-group-start=Liefert die nullbasierte Unicode-Zeichenposition der Gruppe oder -1, wenn sie nicht beteiligt war.
hint-regex-match-group-length=Liefert die Länge der Gruppe in Unicode-Zeichen.
hint-string-len=Gibt die Anzahl der Unicode-Zeichen im String zurück.
hint-string-find=Gibt die nullbasierte Position des ersten Treffers zurück, optional ab einer Startposition und mit StringComparison; -1 bedeutet nicht gefunden.
hint-string-find-last=Gibt die nullbasierte Position des letzten Treffers zurück, optional bis zu einer Startposition und mit StringComparison; -1 bedeutet nicht gefunden.
hint-string-contains=Gibt an, ob der String einen nicht leeren Suchtext enthält, optional mit StringComparison.
hint-string-starts-with=Gibt an, ob der String mit dem Präfix beginnt, optional mit StringComparison.
hint-string-ends-with=Gibt an, ob der String mit dem Suffix endet, optional mit StringComparison.
hint-string-count=Zählt nicht überlappende Vorkommen eines nicht leeren Suchtexts, optional mit StringComparison.
hint-string-equals=Prüft Stringgleichheit, optional mit StringComparison.
hint-string-replace=Ersetzt alle Vorkommen eines Teilstrings und liefert einen `BIGSTR`.
hint-string-trim=Entfernt Leerraum oder angegebene Zeichen an beiden Enden und liefert einen `BIGSTR`.
hint-string-trim-start=Entfernt Leerraum oder angegebene Zeichen am Anfang und liefert einen `BIGSTR`.
hint-string-trim-end=Entfernt Leerraum oder angegebene Zeichen am Ende und liefert einen `BIGSTR`.
hint-string-to-upper=Konvertiert den String in Großbuchstaben und liefert einen `BIGSTR`.
hint-string-to-lower=Konvertiert den String in Kleinbuchstaben und liefert einen `BIGSTR`.
hint-string-split=Liefert durch Aufteilen des Strings ein dynamisches BIGSTR-Array. Leere Elemente bleiben erhalten; bei einem Limit landet der unzerlegte Rest im letzten Element.
hint-string-join=Verbindet ein eindimensionales Stringarray mit einem Trenntext und liefert einen `BIGSTR`.
hint-string-repeat=Wiederholt einen String wie angegeben und liefert einen `BIGSTR`.
hint-type-Byte=1-Byte unsigned Integer Bereich: 0 - 255
hint-type-word=2-Byte unsigned Integer Bereich: 0 - 65,535
hint-type-sByte=1-Byte signed Integer Bereich: -128 - 127
hint-type-sword=2-Byte signed Integer Bereich: -32,768 - 32,767
hint-function-rgb=Packt Rot, Grün, Blau und optional Alpha in einen RGBA-Farbwert.
hint-function-tolong=Konvertiert einen Ausdruck in den vorzeichenbehafteten 64-Bit-Typ `LONG`.
hint-function-toulong=Konvertiert einen Ausdruck in den vorzeichenlosen 64-Bit-Typ `ULONG`.
hint-function-terminal=Das Terminal des Anrufers und die Wurzel für Grafik, Eingabe, Ränder, Palette, Schriften, Makros, Audio und zwischengespeicherte Fähigkeiten.
hint-function-board=Eine Momentaufnahme des konfigurierten Boards: Name, Ort, Betreiber, Sysop-Name, Knotenzahl, Konferenzen und registrierte Benutzer.
hint-function-session=Der laufende Anruf, live gelesen: Konferenz, Bereiche, Anrufer, Sicherheitsstufe, Knoten, verbleibende Minuten und Sprache.
hint-statement-fgetrec=Liest je ein maskiertes Textfeld pro skalarem Feld aus Kanal @1 in Datensatz @2. Das Ziel wird erst geändert, wenn der vollständige Datensatz gültig ist; nachfolgende Zeilen bleiben ungelesen.
hint-statement-fputrec=Schreibt Datensatz @2 als eine maskierte Textzeile pro skalarem Feld in Kanal @1. Zusätzliche Dokumentation kann danach mit `FPUTLN` geschrieben werden.
hint-statement-freadrec=Liest einen längengerahmten binären Datensatz aus Kanal @1 in @2. Das Ziel wird erst geändert, wenn der vollständige Rahmen zum Datensatzlayout passt.
hint-statement-fwriterec=Schreibt Datensatz @2 als kompakten längengerahmten Binärwert in Kanal @1.

hint-statement-end=Beendet die Programmausführung
hint-statement-cls=Löscht den Bildschirm
hint-statement-clreol=Löscht bis zum Zeilenende
hint-statement-more=Pausiert und wartet auf einen Tastendruck (Zeigt eine MEHR?-Eingabeaufforderung an)
hint-statement-wait=Pausiert und wartet auf einen Tastendruck
hint-statement-color=Setzt die Textfarbe auf @1
hint-statement-goto=Springt zum angegebenen Label
hint-statement-let=Weist `var1` den Wert von `exp` zu
hint-statement-print=Gibt eine Zeile auf dem Bildschirm aus

    ### Hinweise
    Diese Anweisung verarbeitet alle @-Codes und zeigt sie wie erwartet an.
hint-statement-println=Geben Sie eine Zeile auf dem Bildschirm aus und hängen Sie eine neue Zeile an das Ende des Ausdrucks bzw. der Ausdrücke an.

    ### Hinweise
    Diese Anweisung verarbeitet alle @-Codes und zeigt sie wie erwartet an.
hint-statement-confflag=Aktivieren Sie die durch @2 angegebenen Konferenz-@1-Flags
hint-statement-confunflag=Deaktivieren Sie die durch @2 angegebenen Konferenz-@1-Flags
hint-statement-dispfile=Datei @1 mit alternativen Dateiflags @2 anzeigen
    ### Gültige Flags
    - `GRAPH`
    - `SEC`
    - `LANG`
hint-statement-input=Zeigen Sie @1 an, erhalten Sie Eingaben vom Benutzer und weisen Sie diese @2 zu (maximal 60 Zeichen).
hint-statement-fcreate=Verwenden Sie den Kanal @1, um die Datei @2 im Zugriffsmodus @3 und Freigabemodus @4 zu erstellen und zu öffnen
    | Gültig | Werte |
    | :--- | :--- |
    | Kanäle | `0` - `7` (`0` wird für Umfragen verwendet) |
    | Zugriffsmodi | `O_RD`, `O_WR`, `O_RW` (sollte `O_WR` verwenden) |
    | Freigabemodi | `S_DN`, `S_DR`, `S_DW`, `S_DB` |
hint-statement-fopen=Verwenden Sie den Kanal @1, um die Datei @2 im Zugriffsmodus @3 und im Freigabemodus @4 zu öffnen
    | Gültig | Werte |
    | :--- | :--- |
    | Kanäle | `0` - `7` (`0` wird für Umfragen verwendet) |
    | Zugriffsmodi | `O_RD`, `O_WR`, `O_RW` (sollte `O_WR` verwenden) |
    | Freigabemodi | `S_DN`, `S_DR`, `S_DW`, `S_DB` |
hint-statement-fappend=Verwenden Sie den Kanal @1, um im Zugriffsmodus @3 und im Freigabemodus @4 an die Datei @2 anzuhängen
    | Gültig | Werte |
    | :--- | :--- |
    | Kanäle | `0` - `7` (`0` wird für Umfragen verwendet) |
    | Zugriffsmodi | `O_RD`, `O_WR`, `O_RW` (sollte `O_WR` verwenden) |
    | Freigabemodi | `S_DN`, `S_DR`, `S_DW`, `S_DB` |
hint-statement-fclose=Kanal @1 schließen

    Akzeptieren Sie Kanal -1 als `ReadLine()`-Funktion „Kanal“ und schließen Sie ihn
hint-statement-fget=Lesen Sie eine Zeile vom Kanal @1 und weisen Sie sie @2 zu
hint-statement-fput=Schreiben Sie einen oder mehrere @2 in den Kanal @1
hint-statement-fputln=Schreiben Sie ein oder mehrere @2 in den Kanal @1 und schließen Sie mit einem Wagenrücklauf/Zeilenvorschubpaar ab
hint-statement-resetdisp=Setzen Sie die Anzeige nach einem Benutzerabbruch zurück
hint-statement-startdisp=Starten Sie die Anzeigeüberwachung im Modus @1
    ### Gültige Modi
    - `NC`
    - `FNS`
    - `FCL`
hint-statement-fputpad=Schreiben Sie @2 aus, füllen Sie es nach Bedarf auf oder kürzen Sie es auf die Länge @3, um @1 zu kanalisieren
hint-statement-hangup=Der Benutzer legt ohne Benachrichtigung auf
hint-statement-getuser=Füllen Sie die vordefinierten Variablen (U_…) mit aktuellen Informationen aus dem Benutzerdatensatz
hint-statement-putuser=Schreiben Sie die Informationen aus den vordefinierten Variablen (U_…) in den Benutzerdatensatz
    Diese Anweisung dient nur dazu, Benutzerinformationen zu aktualisieren, wenn zuvor ein erfolgreicher GetUser- oder GetAltUser-Befehl ausgegeben wurde.
    Dies wurde durchgeführt, um sicherzustellen, dass Informationen für den aktuellen Benutzer nicht an einen anderen Benutzer geschrieben wurden oder umgekehrt.
hint-statement-defcolor=Setzt die aktuelle Farbe auf den Systemstandard zurück
hint-statement-delete=Löscht den durch @1 angegebenen Dateinamen (`ERASE` ist ein Synonym)
hint-statement-deluser=Markiert den aktuellen Benutzerdatensatz zum Löschen
hint-statement-adjtime=Addieren oder subtrahieren Sie @1 Minuten zur verfügbaren Zeit des Benutzers für diese Sitzung
hint-statement-log=Schreiben Sie die Zeichenfolge @1 linksbündig in das Anruferprotokoll, wenn @2 gleich `TRUE` ist
hint-statement-inputstr=Zeigen Sie @1 in der Farbe @3 an und holen Sie sich vom Benutzer eine Zeichenfolge (maximale Länge @4, gültige Zeichen @5, Flags @6) und weisen Sie sie @2 zu

    ### Gültige Flags
    `ECHODOTS`, `FIELDLEN`, `GUIDE`, `UPCASE`, `STACKED`, `ERASELINE`, `NEWLINE`, `LFBEFORE`, `LFAFTER`, `WORDWRAP`, `NOCLEAR`, `HIGHASCII`, `AUTO`, `YESNO`
hint-statement-inputyn=Zeigen Sie @1 in der Farbe @3 an und erhalten Sie eine Ja/Nein-Antwort vom Benutzer, indem Sie es @1 zuweisen (maximal 1 Zeichen, gültige Zeichen werden durch die Sprache bestimmt).
hint-statement-inputmoney=Zeigen Sie @1 in der Farbe @3 an und erhalten Sie vom Benutzer eine geldformatierte Zeichenfolge, die Sie @1 zuweisen (maximal 13 Zeichen, gültige Zeichen `0-9 $ .`).
hint-statement-inputint=Zeigen Sie @1 in der Farbe @3 an und erhalten Sie vom Benutzer eine Zeichenfolge im Ganzzahlformat, die Sie @1 zuweisen (maximal 11 Zeichen, gültige Zeichen `0-9`).
hint-statement-inputcc=Zeigen Sie @1 in der Farbe @3 an und erhalten Sie vom Benutzer eine Zeichenfolge im Kreditkartenformat, die Sie @1 zuweisen (maximal 16 Zeichen, gültige Zeichen `0-9`).
hint-statement-inputdate=Zeigen Sie @1 in der Farbe @3 an und erhalten Sie vom Benutzer eine datumsformatierte Zeichenfolge, die Sie @1 zuweisen (maximal 8 Zeichen, gültige Zeichen `0-9 - /`).
hint-statement-inputtime=Zeigen Sie @1 in der Farbe @3 an und erhalten Sie vom Benutzer eine zeitformatierte Zeichenfolge, die Sie @1 zuweisen (maximal 8 Zeichen, gültige Zeichen `0-9 :`).
hint-statement-gosub=Übertragen Sie die Kontrolle an `LABEL` und markieren Sie den aktuellen PPE-Standort für eine zukünftige Rückgabeerklärung (`GO SUB` ist ein Synonym).
hint-statement-return=Kehren Sie zur Anweisung nach dem letzten `GoSub` zurück oder beenden Sie die PPE, wenn kein `GoSub` auf einen `RETURN` wartet
hint-statement-promptstr=Zeigen Sie den PCBTEXT-Eintrag @1 an und holen Sie sich vom Benutzer eine Zeichenfolge (maximale Länge @3, gültige Zeichen @4, Flags @5) und weisen Sie sie @1 zu
    ### Gültige Flags
    `ECHODOTS`, `FIELDLEN`, `GUIDE`, `UPCASE`, `STACKED`, `ERASELINE`, `NEWLINE`, `LFBEFORE`, `LFAFTER`, `WORDWRAP`, `NOCLEAR`, `HIGHASCII`, `AUTO`, `YESNO`
hint-statement-dtron=Schalten Sie das DTR-Signal ein
hint-statement-dtroff=Schalten Sie das DTR-Signal aus,

    Hinweis: Bei den meisten Modems führt die Verringerung von DTR dazu, dass das Modem aufhängt. Dies ist eine gute Möglichkeit, wenn Sie eine schlechte Verbindung simulieren möchten.
    und dann auflegen ohne Abschiedsbildschirme... Das ist der beste Weg für Sie, der nette Sysop, Ihre Leitung schnell freizugeben... :)
hint-statement-cdchkon=Aktivieren Sie die Überprüfung der Trägererkennung
hint-statement-cdchkoff=Schalten Sie die Überprüfung der Trägererkennung aus
hint-statement-delay=Pause für @1 Uhrenticks (1 Uhrentick = 1/18,2 Sekunde)
hint-statement-sendmodem=Senden Sie den Text in @1 an das Modem
hint-statement-inc=Erhöhen Sie den Wert von @1
hint-statement-dec=Dekrementieren Sie den Wert von @1
hint-statement-newline=Schreiben Sie eine neue Zeile in die Anzeige
hint-statement-newlines=Schreiben Sie @1 Zeilenumbrüche in die Anzeige
hint-statement-tokenize=Zerlegen Sie die Zeichenfolge @1 in einzelne Elemente, getrennt durch Semikolons oder Leerzeichen
hint-statement-gettoken=### Rückgabewert
    Das nächste String-Token aus einem vorherigen Aufruf von `Tokenize` (identisch mit der `GETTOKEN`-Anweisung, kann aber in einem Ausdruck ohne vorherige Zuweisung an eine Variable verwendet werden)

    ### Beispiel
    `GETTOKEN VAR`

    Holen Sie sich ein Token von einem früheren Aufruf von Tokenize und weisen Sie es `VAR` zu
hint-statement-shell=Shell (über COMMAND.COM, wenn @1 `TRUE` ist) zum Programmieren/Befehlen von @2 mit den Argumenten @3, wobei der Rückgabewert in @1 gespeichert wird
    HINWEIS: Wenn @1 `TRUE` ist, ist der @1 zugewiesene Wert der Rückkehrcode von COMMAND.COM, nicht @3.
hint-statement-disptext=Zeigen Sie die PCBTEXT-Eingabeaufforderung @1 mit den Flags @2 an

    ### Gültige Flags
    `NEWLINE`, `LFBEFORE`, `LFAFTER`, `BELL`, `LOGIT`, `LOGITLEFT`
hint-statement-stop=Brechen Sie die PPE-Ausführung ab, ohne Antworten (Kanal 0) an die Antwortdatei anzuhängen
hint-statement-inputtext=Zeigen Sie @1 in der Farbe @3 an und erhalten Sie vom Benutzer eine Zeichenfolge (maximale Länge @4), die Sie @1 zuweisen
hint-statement-beep=Der Lautsprecher piept
hint-statement-push=Schieben Sie eine Liste ausgewerteter Ausdrücke auf den Stapel
hint-statement-pop=Fügen Sie Werte (zuvor auf den Stapel verschoben) in eine Liste von Variablen ein
hint-statement-kbdstuff=Füllen Sie den Tastaturpuffer mit dem Inhalt von @1
hint-statement-call=Laden Sie den durch @1 angegebenen PPE-Dateinamen und führen Sie ihn aus
hint-statement-join=Führt einen Befehl zum Beitreten zur Konferenz aus und übergibt ihn als Argumente @1
hint-statement-quest=Erstellen Sie einen Skript-Fragebogen @1
hint-statement-blt=Bulletin-Nummer @1 anzeigen
hint-statement-dir=Führt einen Dateiverzeichnisbefehl aus und übergibt ihn als Argumente @1
hint-statement-kbdfile=Füllen Sie den Tastaturpuffer mit dem Inhalt der Datei @1
hint-statement-bye=Das Gleiche gilt, wenn der Benutzer an der Eingabeaufforderung „BYE“ eingibt
hint-statement-goodbye=Das Gleiche gilt, wenn der Benutzer an der Eingabeaufforderung „G“ eingibt
hint-statement-broadcast=Broadcast-Nachricht @3 an Knoten von @1 bis einschließlich @2
hint-statement-waitfor=Warten Sie bis zu @3 Sekunden auf die Zeichenfolge @1. Weisen Sie `TRUE` @1 zu, wenn die Zeichenfolge in der angegebenen Zeit gefunden wird, oder `FALSE`, wenn die Zeichenfolge nicht gefunden wird (`WAIT FOR` ist ein Synonym).
hint-statement-kbdchkon=Aktivieren Sie die Zeitüberschreitungsprüfung für die Tastatur
hint-statement-kbdchkoff=Deaktivieren Sie die Tastatur-Timeout-Überprüfung
hint-statement-optext=Schreibt die Zeichenfolge @1 in das Makro `@OPTEXT@`
hint-statement-dispstr=Datei anzeigen, wenn @1 `“%filename”` ist, PPE ausführen, wenn @1 `“!filename”` ist, oder Zeichenfolge @1 anzeigen
hint-statement-rdunet=Lesen Sie Informationen aus USERNET.XXX für den Knoten @1
hint-statement-wrunet=Schreiben Sie Informationen für den Knoten @1 in USERNET.XXX, wobei @2 der neue Knotenstatus ist.
     @3 ist der neue Knotenbenutzername.
     @4 ist die neue Knotenstadt,
     @5 ist der neue Knotenoperationstext.
     und @6 ist Broadcast-Text
hint-statement-dointr=Generieren Sie die Interrupt-Nummer „intr“ (0-255) mit den als Parameter übergebenen Registerwerten
hint-statement-varseg=Weisen Sie @2 die Segmentadresse von @1 zu
hint-statement-varoff=Weisen Sie @2 die Offset-Adresse von @1 zu
hint-statement-pokeb=Weisen Sie der Speicheradresse @1 den Wert @2 (0-255) zu (POKE ist ein Synonym)
hint-statement-pokew=Weisen Sie der Speicheradresse @1 den Wert @2 (0-65535) zu
hint-statement-varaddr=Weisen Sie @2 die Adresse (Segment und Offset) von @1 zu
hint-statement-ansipos=Bewegen Sie den Cursor auf die Spalte @1 und die Zeile @2

    ```
    1 <= @1 <= 80
    1 <= @2 <= 23 (Because of the status lines)
    ```
    (1,1) ist die obere linke Ecke des Bildschirms
hint-statement-backup=Sichern Sie (bewegen Sie den Cursor nach links) @1-Spalten, ohne über Spalte 1 hinauszugehen
hint-statement-forward=Bewegen Sie den Cursor in den nächsten @1-Spalten, ohne über Spalte 80 hinauszugehen
hint-statement-freshline=Wenn sich der Cursor nicht in Spalte 1 befindet, führen Sie einen Zeilenumbruch aus
hint-statement-wrusys=Schreibt (erstellt) eine USERS.SYS-Datei, die von einer SHELL-Anwendung verwendet werden kann
hint-statement-rdusys=Liest eine USERS.SYS-Datei, falls vorhanden, und aktualisiert den Benutzerdatensatz
hint-statement-newpwd=
    Ändert das Passwort des aktuellen Benutzers mit PSA-Prüfung.

    `@1` ist das neue Passwort. `@2` erhält `TRUE`, wenn es angenommen wurde, oder `FALSE` bei fehlgeschlagener Prüfung. Bei Erfolg werden Passwortverlauf, Ablaufdatum und Änderungszähler aktualisiert.
hint-statement-opencap=Öffnen Sie @1 und erfassen Sie alle Bildschirmausgaben darin.
    Wenn beim Erstellen oder Öffnen von @1 ein Fehler auftritt, wird @2 auf `TRUE` gesetzt, andernfalls wird @2 auf `FALSE` gesetzt.
hint-statement-closecap=Schließen Sie die zuvor mit OpenCap geöffnete Capture-Datei
hint-statement-message=Schreiben Sie eine Nachricht in der Konferenz @1 an den Benutzer @2 (eine leere Zeichenfolge ist standardmäßig der aktuelle Anrufer).
    vom Benutzer @3 (leere Zeichenfolge ist standardmäßig der aktuelle Anrufer), Betreff @4,
    Sicherheit in @5 („N“ oder „R“; „N“ ist die Standardeinstellung),
    Auspackdatum in @6 (0 für kein Auspackdatum),
    @7 True, wenn eine Empfangsbestätigung gewünscht wird,
    @8 TRUE, wenn die Nachricht wiedergegeben werden soll, und
    @9 ist der Dateiname, der für den Nachrichtentext verwendet werden soll
hint-statement-savescrn=Speichern Sie den aktuellen Bildschirm in einem Puffer zur späteren Wiederherstellung mit RestScrn
hint-statement-restscrn=Stellen Sie den Bildschirm aus dem zuvor mit SaveScrn gespeicherten Puffer wieder her
hint-statement-sound=Schalten Sie den BBS PC-Lautsprecher mit der durch @1 angegebenen Frequenz (1-65535) ein (oder schalten Sie ihn aus, wenn die Frequenz 0 ist).
hint-statement-chat=Starten Sie den SysOp-Chat-Modus
hint-statement-sprint=Einen oder mehrere String-Ausdrücke nur auf dem BBS-Bildschirm anzeigen (diese Anweisung sendet nichts an das Modem)
hint-statement-sprintln=Zeigt null oder mehr Zeichenfolgenausdrücke nur auf dem BBS-Bildschirm an und folgt mit einer neuen Zeile (diese Anweisung sendet nichts an das Modem).
hint-statement-mprint=Zeigt einen oder mehrere Zeichenfolgenausdrücke nur auf dem Anruferbildschirm an (diese Anweisung sendet nichts an den BBS-Bildschirm).
hint-statement-mprintln=Zeigt null oder mehr Zeichenfolgenausdrücke nur auf dem Anruferbildschirm an und folgt mit einer neuen Zeile (diese Anweisung sendet nichts an den BBS-Bildschirm).
hint-statement-rename=Benennen Sie die Datei @1 in @2 um
hint-statement-frewind=Spulen Sie den Kanal @1 zurück, nachdem Sie die Puffer geleert und die Datei auf die Festplatte übertragen haben.
hint-statement-pokedw=Weisen Sie der Speicheradresse @1 den Wert @2 (-2147483648 - +2147483647) zu
hint-statement-dbglevel=Weisen Sie @1 die Debug-Ebene zu
hint-statement-showon=Aktiviert die Anzeige von Informationen auf dem Bildschirm
hint-statement-showoff=Schaltet die Anzeige von Informationen auf dem Bildschirm aus
hint-statement-pageon=Schalten Sie die SysOp-Page-Anzeige ein (blinkendes p in der Statuszeile).
hint-statement-pageoff=Schalten Sie die SysOp-Page-Anzeige aus (blinkendes p in der Statuszeile).
hint-statement-fseek=Positionieren Sie es an einer beliebigen Stelle innerhalb einer Datei
    @2 ist die Anzahl der Bytes, die relativ zur Position verschoben werden sollen (+/-).
    @3 ist der Basisstandort, von dem aus die Suche gestartet werden soll:

    `SEEK_SET (0)` für den Anfang der Datei

    `SEEK_CUR (1)` für den aktuellen Speicherort des Dateizeigers

    `SEEK_END (2)` für das Ende der Datei
hint-statement-fflush=Leeren Sie die Änderungen eines bestimmten Kanals auf der Festplatte
hint-statement-fread=Binärdaten aus einer Datei lesen.

    @1 ist die Kanalnummer

    @2 ist die Variable zum Speichern der Daten

    @3 ist die Anzahl der zu lesenden Bytes
hint-statement-fwrite=Binärdaten in eine Datei schreiben

    @1 ist die Kanalnummer

    @2 ist der Ausdruck, dessen Ergebnis geschrieben werden soll

    @3 ist die Größe der Daten, die in die Variable geschrieben werden sollen
hint-statement-fdefin=Geben Sie einen Standard-Eingabedateikanal an (zur Beschleunigung der Dateieingabe).
hint-statement-fdefout=Geben Sie einen Standard-Ausgabedateikanal an (zur Beschleunigung der Dateiausgabe).
hint-statement-fdget=Standard-Kanaleingabeanweisung: Verwenden Sie genau die gleichen Argumente wie FGet, außer einem Kanalparameter (es wird der durch FDefIn angegebene Kanal angenommen).
hint-statement-fdput=Standard-Kanalausgabeanweisung: Verwenden Sie genau die gleichen Argumente wie FPut, außer einem Kanalparameter (es wird der durch FDefOut angegebene Kanal angenommen).
hint-statement-fdputln=Standard-Kanalausgabeanweisung: Verwenden Sie genau die gleichen Argumente wie FPutLn, mit Ausnahme eines Kanalparameters (es wird der durch FDefOut angegebene Kanal angenommen).
hint-statement-fdputpad=Standard-Kanalausgabeanweisung: Verwenden Sie genau die gleichen Argumente wie FPutPad, mit Ausnahme eines Kanalparameters (es wird der durch FDefOut angegebene Kanal angenommen).
hint-statement-fdread=Standard-Kanaleingabeanweisung: Verwenden Sie genau die gleichen Argumente wie FRead, mit Ausnahme eines Kanalparameters (es wird der durch FDefIn angegebene Kanal angenommen).
hint-statement-fdwrite=Standard-Kanalausgabeanweisung: Verwenden Sie genau die gleichen Argumente wie FWrite, außer einem Kanalparameter (es wird der durch FDefOut angegebene Kanal angenommen).
hint-statement-adjbytes=Passen Sie den gesamten und täglichen Download des Benutzers an.

    Um Bytes zu subtrahieren, verwenden Sie eine negative Zahl für Bytes.

    Um Bytes hinzuzufügen, verwenden Sie eine positive Zahl.
hint-statement-kbdstring=Stuff-Strings an die Tastatur (genau wie KbdStuff, außer dass „Tastenanschläge“ auf dem Display wiedergegeben werden)
hint-statement-alias=Aktiviert (`TRUE`) oder deaktiviert (`FALSE`) den Alias des aktuellen Benutzers. Sind Aliase für Benutzer oder Konferenz nicht erlaubt, bleibt der Aufruf wirkungslos. `ALIAS()` liefert den aktuellen Zustand.
hint-statement-redim=
    Ändert die Größe eines zuvor deklarierten Arrays zur Laufzeit: `REDIM array, dim1 [, dim2 [, dim3]]`.

    Die Anzahl der Dimensionen muss der Deklaration entsprechen; nur ihre Grenzen dürfen sich ändern. Werte außerhalb der neuen Grenzen gehen verloren. Arrayfelder in Datensätzen besitzen feste Grenzen und können nicht geändert werden.
hint-statement-append=Hängen Sie den Inhalt einer Datei an eine andere Datei an.
hint-statement-copy=Kopieren Sie den Inhalt einer Datei in eine andere Datei.
hint-statement-kbdflush=Leeren Sie den lokalen Tastaturpuffer und alle überfüllten Tastaturpuffer. Es braucht keine Argumente.
hint-statement-mdmflush=Leeren Sie den eingehenden Modempuffer. Es braucht keine Argumente.
hint-statement-keyflush=Leeren Sie sowohl die lokalen Puffer als auch den eingehenden Modempuffer. Es braucht keine Argumente.
hint-statement-lastin=Legen Sie den Wert für die letzte Konferenz des Benutzers fest. Es kann während des Anmeldevorgangs verwendet werden, um den Benutzer beim Start zu einer bestimmten Konferenz zu zwingen (z. B. über ein Anmeldeskript).
hint-statement-flag=Ermöglichen Sie den direkten Download von Markierungsdateien von einer PPE.
hint-statement-download=Herunterladen von Dateien von PPL.

    Die an DOWNLOAD übergebene Zeichenfolge ist eine Liste von Befehlen im gleichen Format wie das, was ein Benutzer nach einem D- oder DB-Befehl eingeben würde.

    Wenn hier ein Dateiname zum Herunterladen angegeben wird, muss dieser gemäß den in den Dateien FSEC und DLPATH.LST festgelegten Kriterien herunterladbar sein.

    Wenn es notwendig ist, eine Datei herunterzuladen, die normalerweise nicht über die FSEC- und/oder DLPATH.LST-Dateien verfügbar ist, kann die FLAG-Anweisung verwendet werden, um sie in die Liste der herunterzuladenden Dateien zu zwingen.
hint-statement-wrusysdoor=Schreiben Sie eine USERS.SYS-Datei mit einem TPA-Datensatz für eine DOOR-Anwendung.
hint-statement-getaltuser=Rufen Sie die Informationen für einen alternativen Benutzer ab.

    Es füllt die Benutzervariablen mit Informationen aus dem angegebenen Benutzerdatensatz und leitet Benutzeranweisungen und -funktionen um.

    Wenn versucht wird, eine Datensatznummer abzurufen, die nicht existiert,
    Die Benutzerfunktionen werden auf den aktuellen Benutzer zurückgesetzt und die Benutzervariablen werden ungültig gemacht, als ob kein GetUser/GetAltUser vorhanden wäre
    Die Erklärung wurde ausgestellt (allerdings behalten sie weiterhin den gehaltenen Wert bei).

    `PutUser`/`PutAltUser` sollte ausgegeben werden, um alle Variablenänderungen am Benutzerdatensatz festzuschreiben.
    Darüber hinaus gibt es mindestens eine Anweisung, die keine Auswirkungen auf alternative Benutzer hat: `AdjTime`.

    Es ist auf den aktuellen Benutzer online beschränkt.

    Wenn der alternative Benutzer außerdem online ist, werden Änderungen am Datensatz erst wirksam, nachdem sich der Benutzer abgemeldet hat.
    Auch wenn nicht genügend Speicher verfügbar ist (hauptsächlich für die Lesezeiger der letzten Nachricht), schlägt diese Anweisung fehl.
hint-statement-adjdbytes=Passen Sie die täglichen Download-Bytes des Benutzers an.

    Um Bytes zu subtrahieren, verwenden Sie eine negative Zahl für Bytes.

    Um Bytes hinzuzufügen, verwenden Sie eine positive Zahl.
hint-statement-adjtbytes=Passen Sie die Gesamt-Downloadbytes des Benutzers an.

    Um Bytes zu subtrahieren, verwenden Sie eine negative Zahl für Bytes.

    Um Bytes hinzuzufügen, verwenden Sie eine positive Zahl.
hint-statement-adjtfiles=Passen Sie die Gesamtzahl der Download-Dateien des Benutzers an.

    Um Dateien zu subtrahieren, verwenden Sie eine negative Zahl für Dateien.

    Um Dateien hinzuzufügen, verwenden Sie eine positive Zahl.
hint-statement-lang=Ändern Sie die vom aktuellen Benutzer verwendete Sprache.
hint-statement-sort=Sortieren Sie den Inhalt eines Arrays in ein Zeiger-Array.

    Beachten Sie, dass sortArray und pointerArray auf eindimensionale Arrays beschränkt sind
hint-statement-mousereg=Richten Sie eine RIP-Mausregion auf dem Remote-Terminal ein.

    | | |
    | --- | --- |
    | @1 | Ist die RIP-Regionsnummer|
    | @2, @3 | Die (X,Y)-Koordinaten oben links in der Region |
    | @4, @5 | Die (X,Y)-Koordinaten unten rechts in der Region |
    | @6 | Die Breite jedes Zeichens in Pixel |
    | @7 | Die Höhe jedes Zeichens in Pixel |
    | @8 | Ein boolesches Flag (TRUE, um den Bereich beim Klicken umzukehren) |
    | @9 | Ein boolesches Flag (TRUE, um das Textfenster zu löschen und im Vollbildmodus anzuzeigen) |
    | @10 | Text, den die Gegenstelle senden soll, wenn die Region angeklickt wird |
hint-statement-scrfile=Suchen Sie nach einem Dateinamen und einer Zeilennummer, die derzeit auf dem Bildschirm angezeigt werden.
hint-statement-searchinit=Initialisieren Sie Suchparameter für einen schnelleren BOYER-MOORE-Suchalgorithmus.
hint-statement-searchfind=Führen Sie eine BOYER-MOORE-Suche in einem Textpuffer aus, indem Sie Kriterien verwenden, die zuvor mit einer SearchInit-Anweisung definiert wurden.
hint-statement-searchstop=Löscht zuvor eingegebene Suchkriterien. Es werden keine Parameter benötigt.
hint-statement-prfound=Diese funktionieren genauso wie Print und PrintLn, aber wenn die letzte SearchFind-Anweisung zu einer Übereinstimmung geführt hat, werden die gefundenen Wörter automatisch hervorgehoben.
hint-statement-prfoundln=Diese funktionieren genauso wie Print und PrintLn, aber wenn die letzte SearchFind-Anweisung zu einer Übereinstimmung geführt hat, werden die gefundenen Wörter automatisch hervorgehoben.
hint-statement-tpaget=Erhalten Sie statische Informationen von einem benannten TPA im String-Format.
hint-statement-tpaput=Geben Sie statische Informationen im String-Format an einen benannten TPA an.
hint-statement-tpacget=Erhalten Sie Informationen von einem benannten TPA für eine bestimmte Konferenz im Zeichenfolgenformat.

    @1 Das Schlüsselwort des zu verwendenden TPA

    @2 Die Variable, in der die Informationen gespeichert werden sollen

    @3 Die Konferenznummer, für die Informationen abgerufen werden sollen
hint-statement-tpacput=Geben Sie Informationen im String-Format an einen benannten TPA für eine bestimmte Konferenz weiter.

    @1 Das Schlüsselwort des zu verwendenden TPA

    @2 Der Ausdruck, der zum Speichern des TPA geschrieben werden soll

    @3 Die Konferenznummer, für die Informationen abgerufen werden sollen
hint-statement-tparead=Erhalten Sie statische Informationen von einem benannten TPA.

    @1 Das Schlüsselwort des zu verwendenden TPA

    @2 Die Variable, in der die Informationen gespeichert werden sollen
hint-statement-tpawrite=Geben Sie statische Informationen an einen benannten TPA weiter.

    @1 Das Schlüsselwort des zu verwendenden TPA

    @2 Der Ausdruck, der zum Speichern des TPA geschrieben werden soll
hint-statement-tpacread=Erhalten Sie Informationen von einem benannten TPA für eine bestimmte Konferenz.

    @1 Das Schlüsselwort des zu verwendenden TPA

    @2 Die Variable, in der die Informationen gespeichert werden sollen

    @3 Die Konferenznummer, für die Informationen abgerufen werden sollen
hint-statement-tpacwrite=Geben Sie Informationen für eine bestimmte Konferenz an einen benannten TPA weiter.

    @1 Das Schlüsselwort des zu verwendenden TPA

    @2 Der Ausdruck, der zum Speichern des TPA geschrieben werden soll

    @3 Die Konferenznummer, für die Informationen abgerufen werden sollen
hint-statement-bitset=Setzt ein bestimmtes Bit aus einer Variablen.

    Diese Anweisung ist in erster Linie für die Verwendung mit BIGSTR-Variablen gedacht, die bis zu 2048 Bytes lang sein können.
    Bei Bedarf funktioniert es jedoch auch mit anderen Datentypen.

    Seien Sie sich nur der potenziellen Probleme bewusst, die beim „Bit-Twidling“ von Nicht-String-Puffer entstehen und dann versuchen, später wie „beabsichtigt“ auf sie zuzugreifen.
    Geben Sie ein, ohne die Variable neu zu initialisieren.

    Wenn der Bitparameter (eine Ganzzahl von 0 bis zur Anzahl der Bits im Objekt) ungültig ist, findet keine Verarbeitung statt.
hint-statement-bitclear=Löscht ein angegebenes Bit aus einer Variablen.

    Diese Anweisung ist in erster Linie für die Verwendung mit BIGSTR-Variablen gedacht, die bis zu 2048 Bytes lang sein können.

    Bei Bedarf funktioniert es jedoch auch mit anderen Datentypen. Seien Sie sich nur der potenziellen Probleme beim „Bit Twidling“ bewusst.
    Nicht-String-Puffer und dann versuchen, später als ihr „beabsichtigter“ Typ auf sie zuzugreifen, ohne die Variable neu zu initialisieren.

    Wenn der Bitparameter (eine Ganzzahl von 0 bis zur Anzahl der Bits im Objekt) ungültig ist, findet keine Verarbeitung statt.
hint-statement-brag=Veralteter PCBoard-Befehl für die frühere BRAG-Anzeige. PCBoard 15.3 und IcyBoard akzeptieren ihn aus Kompatibilitätsgründen, führen aber keine Aktion aus.
hint-statement-frealtuser=Da jeweils nur ein `GETALTUSER` aktiv sein kann, kann `FREALTUSER` anderen Prozessen, die `GETALTUSER` verwenden müssen (z. B. dem Befehl `MESSAGE`), dies ermöglichen.
hint-statement-setlmr=Legen Sie die letzten Lesezeiger für die angegebene Konferenz fest.

    Wenn @1 größer als die Anzahl der tatsächlichen Konferenzen ist, verwendet @1 standardmäßig die höchste Konferenznummer.

    Wenn @2 größer als die höchste Nachrichtennummer in dieser Konferenz ist, wird standardmäßig die höchste Nachrichtennummer in dieser Konferenz verwendet.
    Dies könnte verwendet werden, um einem neuen Benutzer Nachrichtenzeiger auf aktuelle Nachrichten zu setzen, damit dieser nicht auf drei Jahre alte Nachrichten antwortet.
    Eine nützliche Funktion wäre es, die hohe Konferenznummer zu erhalten.
hint-statement-setenv=Legen Sie eine Umgebungsvariable fest.

    Das Zeichenfolgenformat lautet:`"VAR=VALUE"`
hint-statement-fcloseall=Schließt alle Dateikanäle
hint-statement-stackabort=Dadurch kann der Programmierer das Laufzeitmodul anweisen, sein Bestes zu geben, um die Ausführung fortzusetzen, nachdem ein Stapelfehler aufgetreten ist.

    Wenn `FALSE` übergeben wird, wird die Ausführung nach einem Stapelfehler abgebrochen. Wenn `TRUE` bestanden wird, läuft die PPE weiter.

    > [!ACHTUNG]
    > Wenn Sie die Ausführung nach einem Stapelfehler fortsetzen, ist die Programmausführung unvorhersehbar.
    > PPL lässt nicht zu, dass der Systemspeicher aufgrund eines Stapelfehlers beschädigt wird.
hint-statement-dcreate=Erstellen Sie eine DBF-Datei
hint-statement-dopen=Öffnen Sie die DBF-Datei
hint-statement-dclose=DBF-Datei schließen
hint-statement-dsetalias=DBF-Alias ​​festlegen
hint-statement-dpack=Packen Sie die DBF-Datei
hint-statement-dcloseall=Schließen Sie alle NDX-Dateien
hint-statement-dlock=DBF-Datei sperren
hint-statement-dlockr=einen Datensatz sperren
hint-statement-dlockg=Sperren Sie eine Gruppe von Datensätzen
hint-statement-dunlock=Entsperren Sie alle aktuellen Schlösser
hint-statement-dncreate=NDX-Datei erstellen
hint-statement-dnopen=NDX-Datei öffnen
hint-statement-dnclose=NDX-Datei schließen
hint-statement-dncloseall=Schließen Sie alle NDX-Dateien
hint-statement-dnew=einen neuen Rekord starten
hint-statement-dadd=Fügen Sie den neuen Datensatz hinzu
hint-statement-dappend=einen leeren Datensatz anhängen
hint-statement-dtop=Gehe zum obersten Datensatz
hint-statement-dgo=Gehen Sie zu einem bestimmten Datensatz
hint-statement-dbottom=Gehe zum unteren Datensatz
hint-statement-dskip=+/- eine Anzahl von Datensätzen überspringen
hint-statement-dblank=Löschen Sie den Datensatz
hint-statement-ddelete=den Datensatz löschen
hint-statement-drecall=Erinnern Sie sich an die Aufzeichnung
hint-statement-dtag=Wählen Sie ein Tag aus
hint-statement-dseek=gibt den Fehlerstatus zurück ( 0|1 )
    ; oder Erfolg suchen (0 = Fehler
    ; 1 = Erfolg, 2 = folgender Datensatz
    ; 3 = Ende der Datei)
hint-statement-dfblank=Leeren Sie ein benanntes Feld
hint-statement-dget=Holen Sie sich einen Wert aus einem benannten Feld
hint-statement-dput=Geben Sie einen Wert in ein benanntes Feld ein
hint-statement-dfcopy=Kopieren Sie ein Feld in ein Feld
hint-statement-account=@1 ist ein Wert zwischen 0 und 14. Es wird empfohlen, Systemkonstanten zu verwenden.

    @2 ist die Menge an Credits, die zum Feld hinzugefügt oder abgezogen werden müssen
hint-statement-recordusage=@1 ist die Feldnummer, auf die zugegriffen werden soll (mit DEB…-Konstanten). Descr1 ist die Beschreibung der Gebühr. Descr2 ist eine Unterbeschreibung der Kosten pro Einheit
    ist der Kosten-pro-Einheit-Wert die Anzahl der Einheiten. Recordusage aktualisiert Soll-Werte in PCBoard sowie Datensatzbeschreibungen und anderes
    Informationen in einer Buchhaltungsdatei.

    Gültige Werte für den Feldparameter sind 2–16. Die diesen Werten entsprechenden Konstanten (DEB???) könnten und sollten hier verwendet werden.

    (Eine Liste der Konstanten finden Sie im Abschnitt „Buchhaltung“).
hint-statement-msgtofile=Schreibt eine Nachricht in eine Datei.

    Diese Anweisung nimmt die gegebene Nachricht und schreibt sie in eine Textdatei.
    Die ersten 15 Zeilen der Datei enthalten Standard-Header-Informationen. (Ein Feld pro Zeile) Die Kopfzeilen sind formatiert, um das Parsen zu erleichtern.
    In der 16. Zeile wird angegeben, wie viele erweiterte Header vorhanden sind. Die folgenden Zeilen enthalten erweiterte Header.
    (eine pro Zeile) Schließlich folgt nach den erweiterten Headern eine Zeile mit „Nachrichtentext:“. Alles danach ist der Hauptteil der Nachricht.
hint-statement-qwklimits=Mit dieser Anweisung kann der PPL-Programmierer die QWK-Grenzwerte eines Benutzers ändern. Vier Felder können mit ihrer Anweisung geändert werden.
hint-statement-command=Verarbeiten Sie einen Befehl so, als ob er an der Eingabeaufforderung eingegeben würde.

    @1: Ein boolescher Wert, der angibt, ob versucht werden soll, den Befehl in CMD.LST zu finden.
    Wenn TRUE und der Befehl nicht in CMD.LST enthalten ist, werden die Standardbefehle automatisch ausprobiert. Wenn der Befehl nicht vorhanden ist, schlägt der Befehl fehl.

    @2: Ein Zeichenfolgenwert mit dem auszuführenden Befehl und den Parametern. (wie „R A Y O S“)

    HINWEISE!!! Nicht alle Teile von PCBoard sind wiedereintrittsfähig. Sie sollten beispielsweise nicht versuchen, zwei Nachrichteneditorprozesse gleichzeitig aktiv zu haben (mit anderen Worten, Sie sollten den Nachrichteneditor nicht aus einem MNU heraus starten und dann einen PPE über eine Umschalttaste starten, der versucht, eine andere Nachricht einzugeben). Daher müssen Sie bei verschachtelten COMMAND-Aufrufen (oder gleichwertigen Aufrufen) vorsichtig sein. Aber die sequentielle Verarbeitung sollte überhaupt kein Problem darstellen.
    Wenn sich irgendwann in der Zukunft herausstellt, dass das Zulassen dieser Flexibilität mehr Probleme verursacht als löst, wird die COMMAND-Anweisung zurückgefahren, um sicherzustellen, dass keine Versuche unternommen werden, Code erneut einzugeben. Nutzen Sie es also gut und mit Bedacht!
hint-statement-uselmrs=Steuert, ob nachfolgende `GETALTUSER`-Aufrufe die Last-Message-Read-Zeiger des alternativen Benutzers laden. `FALSE` spart Speicher, wenn keine LMR-Daten benötigt werden; `TRUE` aktiviert das Laden wieder. `USELMRS()` liefert den aktuellen Zustand.
hint-statement-confinfo=Mit dieser Anweisung kann ein Feld in der Konferenz geändert werden
    Konfiguration.

    @1 = Die Konferenznummer, über die Informationen abgerufen werden sollen

    @2 = Zu änderndes Konferenzfeld. (Siehe Hinweis)

    @3 = Neuer Wert, der im Feld gespeichert werden soll

    { Conference_access_constants }
hint-statement-adjtubytes=Diese Anweisung kann verwendet werden, um die gesamten Upload-Bytes eines Benutzers anzupassen
    nach oben oder unten.

    @1 = Anzahl der Bytes, mit denen die Upload-Bytes aktueller Benutzer angepasst werden sollen.
    Dies kann ein positiver oder negativer Wert sein
hint-statement-grafmode=Diese Anweisung kann verwendet werden, um den Grafikmodus eines Benutzers zu ändern, während er online ist.

    @1 = Der Grafikmodus, in den gewechselt werden soll.
    1 = Wenn der Benutzer über ANSI-Fähigkeiten verfügt, werden Grafiken in farbiges ANSI geändert
    2 = Versucht, den Benutzer ungeachtet seiner Ansi-Fähigkeit in Farb-Ansi zu versetzen
    3 = Versetzt den Benutzer in den Ansi-Schwarzweißmodus
    4 = Versetzt den Benutzer in den Nicht-ANSI-Schwarzweißmodus
    5 = Wenn der Benutzer RIP-fähig ist, wird der Benutzer in den RIP-Modus versetzt.
    (IcyBoard: 6 = Avatar-Modus
    )
hint-statement-adduser=@1 = Name des neuen Benutzers, der hinzugefügt werden soll
    @2 = TRUE weist PCBoard an, die Variablen des neuen Benutzers aktiv zu lassen, als ob ein GETALTUSER ausgeführt würde
    unter Verwendung der neuen Benutzerdatensatznummer. FALSE stellt die aktuellen Benutzervariablen wieder her.

    ### Hinweise
    Mit dieser Anweisung kann PPL einen neuen Benutzerdatensatz erstellen und ausfüllen
    in allen Feldern außer dem Namen mit Platinen-Standardwerten.
hint-statement-killmsg=@1 = Konferenznummer, in der sich die zum Scheitern verurteilte Nachricht befindet.
    @2 = zu tötende Nachrichtennummer
hint-statement-chdir=Änderungen am Verzeichnis
hint-statement-mkdir=Erstellt ein neues Verzeichnis

    @1 = zu erstellendes Verzeichnis
hint-statement-rmdir=Entfernt ein Verzeichnis

    @1 = zu entfernendes Verzeichnis

    ### Hinweis
    Das Verzeichnis muss leer sein, bevor es entfernt werden kann.
hint-statement-fdowraka=Kompatibilitätsplatzhalter zum Schreiben eines PCBoard-FidoNet-AKA-Eintrags. Die ursprüngliche PCBoard-Implementierung wurde nie fertiggestellt; IcyBoard protokolliert eine Warnung und ändert nichts.
hint-statement-fdoaddaka=Kompatibilitätsplatzhalter zum Hinzufügen eines PCBoard-FidoNet-AKA-Eintrags. Die ursprüngliche PCBoard-Implementierung wurde nie fertiggestellt; IcyBoard protokolliert eine Warnung und ändert nichts.
hint-statement-fdowrorg=Kompatibilitätsplatzhalter zum Schreiben einer PCBoard-FidoNet-Origin-Zeile. Die ursprüngliche PCBoard-Implementierung wurde nie fertiggestellt; IcyBoard protokolliert eine Warnung und ändert nichts.
hint-statement-fdoaddorg=Kompatibilitätsplatzhalter zum Hinzufügen einer PCBoard-FidoNet-Origin-Zeile. Die ursprüngliche PCBoard-Implementierung wurde nie fertiggestellt; IcyBoard protokolliert eine Warnung und ändert nichts.
hint-statement-fdoqmod=Ersetzt einen Eintrag der Ausgangswarteschlange

    @1 = Datensatznummer, gezählt von eins
    @2 = Adresse des Links, für den die Datei bestimmt ist
    @3 = zu sendende Datei
    @4 = NORMAL oder ABSTURZ, gelesen und ignoriert
hint-statement-fdoqadd=Stellt eine Datei in die Ausgangswarteschlange eines Links

    @1 = Adresse des Links, für den die Datei bestimmt ist
    @2 = zu sendende Datei
    @3 = NORMAL oder ABSTURZ, gelesen und ignoriert
hint-statement-fdoqdel=Entnimmt einen Eintrag aus der Ausgangswarteschlange

    @1 = Datensatznummer, gezählt von eins
hint-statement-sounddelay=@1 = Frequenz, mit der der PC-Lautsprecher ertönen soll
    @2 = Länge in Taktschritten (18 = 1 Sekunde), um den Lautsprecher eingeschaltet zu lassen

    ### Hinweise
    Diese Funktion wurde hinzugefügt, um die zu ersetzen
    ```
    SOUND 500
    SOUND 0
    ```
    Diese Kombination ist für DOS erforderlich, da diese Funktionalität unter OS/2 nicht verfügbar ist.
hint-statement-shortdesc=Legt den Status des aktuellen Benutzers für die Anzeige kurzer (eine Zeile) oder vollständiger Dateibeschreibungen fest.

    @1 = Ein boolescher Ausdruck, der angibt, ob die Kurzbeschreibung aktiviert ist.
hint-statement-movemsg=Verschiebt die Nachricht von ihrem aktuellen Speicherort an das Ende der Nachrichtenbasis.

    @1 = Konferenznummer, in der sich die Nachricht befindet
    @2 = zu verschiebende Nachrichtennummer
    @3 = Ein boolescher Ausdruck, der angibt, wo die Nachricht sein soll
    bewegen oder nicht.  TRUE, wenn die Nachricht verschoben werden soll, FALSE, wenn die Nachricht kopiert werden soll.
hint-statement-setbankbal=Legt den Wert eines angegebenen Felds fest.

    @1 Ein ganzzahliger Ausdruck, der das abzurufende Feld angibt.
    @2 Ein ganzzahliger Ausdruck, der den Wert angibt, auf den das angegebene Feld gesetzt werden soll.

    ### Felder
    Zeitfelder (in Minuten)
    ------------------------
        0 = Datum der letzten Einzahlung
        1 = Letztes Auszahlungsdatum
        2 = Letzter Transaktionsbetrag (in Minuten)
        3 = Gesparter Betrag (Zeitguthaben auf dem Konto)
        4 = Max. Auszahlung (das Maximum, das ein Benutzer an einem Tag abheben kann)
        5 = Max Stored Amount (maximal zulässige Speicherzeit)

    Bytefelder (in K Bytes)
    ------------------------
        6 = Letztes Einzahlungsdatum
        7 = Letztes Auszahlungsdatum
        8 = Letzter Transaktionsbetrag (in K Bytes)
        9 = Gesparter Betrag (ihr K-Byte-Saldo auf ihrem Konto)
        10 = Max. Auszahlung (das Maximum, das ein Benutzer an einem Tag abheben kann)
        11 = Max Stored Amount (maximal zulässige K-Bytes, die gespeichert werden dürfen)
hint-function-len=### Rückgabewert
    Gibt die Länge der Zeichenfolge @1 zurück
hint-function-lower=### Rückgabewert
    Gibt die in Kleinbuchstaben konvertierte Zeichenfolge @1 zurück
hint-function-upper=### Rückgabewert
    Gibt die in Großbuchstaben konvertierte Zeichenfolge @1 zurück
hint-function-mid=### Rückgabewert
    Gibt eine Teilzeichenfolge von @1 zurück, beginnend an der Position @2 und @3 Zeichen lang
hint-function-left=### Rückgabewert
    Gibt die ganz linken @2-Zeichen von @1 zurück
hint-function-right=### Rückgabewert
    Gibt die ganz rechten @2-Zeichen von @1 zurück
hint-function-space=### Rückgabewert
    Gibt eine Zeichenfolge aus @1-Leerzeichen zurück
hint-function-ferr=Gibt an, ob auf Dateikanal `@1` seit der letzten Prüfung ein Fehler aufgetreten ist. Das Lesen von `FERR()` löscht das Fehlerflag dieses Kanals. Auch das Dateiende nach `FGET` oder `FREAD` setzt das Flag.
hint-function-chr=### Rückgabewert
    Gibt eine aus einem Zeichen lange Zeichenfolge des durch die ASCII-Codevariable (0-255) dargestellten Zeichens zurück.
hint-function-asc=### Rückgabewert
    Gibt den ASCII-Wert des ersten Zeichens in @1 zurück
hint-function-instr=Gibt die Position von @2 in @1 `(1-LEN(@1))` oder `0` zurück, wenn @2 nicht in @1 ist
hint-function-abort=Gibt ein Flag zurück, das angibt, ob der Benutzer die Anzeige der Daten über ^K / ^X abgebrochen hat oder mit „Nein“ auf ein MEHR? geantwortet hat. prompt
hint-function-ltrim=Gibt eine Zeichenfolge von @1 zurück, wobei das erste Zeichen von @2 von links abgeschnitten ist
hint-function-rtrim=Gibt eine Zeichenfolge von @1 zurück, wobei das erste Zeichen von @2 von rechts abgeschnitten ist
hint-function-trim=Gibt eine Zeichenfolge von @1 zurück, wobei das erste Zeichen von @2 an beiden Enden abgeschnitten ist
hint-function-random=Gibt eine Zufallszahl zwischen 0 und @2 einschließlich zurück
hint-function-date=Gibt das heutige Datum zurück
hint-function-time=Gibt die aktuelle Uhrzeit zurück
hint-function-u_name=Gibt den aktuellen Benutzernamen zurück
hint-function-u_ldate=Gibt das letzte Datum des aktuellen Benutzers im System zurück
hint-function-u_ltime=Gibt die aktuellen Benutzer zurück, die zuletzt auf dem System waren
hint-function-u_ldir=Gibt das Datum des letzten Verzeichnisscans des aktuellen Benutzers zurück
hint-function-u_logons=Gibt die Anzahl der Anmeldungen des aktuellen Benutzers zurück
hint-function-u_ful=Gibt die Anzahl der hochgeladenen Dateien des aktuellen Benutzers zurück
hint-function-u_fdl=Gibt die Anzahl der heruntergeladenen Dateien des aktuellen Benutzers zurück
hint-function-u_bdlday=Gibt die Anzahl der heute heruntergeladenen Bytes des aktuellen Benutzers zurück
hint-function-u_timeon=Gibt die heutige Online-Zeit des aktuellen Benutzers in Minuten zurück
hint-function-u_bdl=Gibt die Anzahl der heruntergeladenen Bytes des aktuellen Benutzers zurück
hint-function-u_bul=Gibt die Anzahl der heruntergeladenen Bytes des aktuellen Benutzers zurück
hint-function-year=Gibt das Jahr (1900-2079) von @1 zurück
hint-function-month=Gibt den Monat des Jahres (1-12) von @1 zurück
hint-function-day=Gibt den Tag des Monats (1-31) von @1 zurück
hint-function-dow=Gibt den Wochentag (0 = Sonntag, 6 = Samstag) zurück, auf den @1 gefallen ist
hint-function-hour=Gibt die Stunde des Tages (0-23) von @1 zurück
hint-function-min=Gibt die Minute der Stunde (0-59) von @1 zurück
hint-function-sec=Gibt die Sekunde der Minute (0-59) von @1 zurück
hint-function-timeap=Gibt eine Zeichenfolge zurück, die die Zeit @1 im zivilen Format darstellt (XX:XX:XX AM).
hint-function-ver=Gibt die Versionsnummer von PCBoard zurück, das ausgeführt wird
hint-function-nochar=Gibt die aktuelle Sprache ohne Zeichen zurück
hint-function-yeschar=Gibt das Ja-Zeichen der aktuellen Sprache zurück
hint-function-stripatx=Gibt eine Zeichenfolge @1 zurück, wobei alle @X-Codes entfernt wurden
hint-function-replace=Gibt eine Zeichenfolge von @1 zurück, wobei alle Vorkommen des ersten Zeichens von @2 durch das erste Zeichen von @3 ersetzt werden
hint-function-strip=Gibt eine Zeichenfolge von @1 zurück, wobei alle Vorkommen des ersten Zeichens von @2 entfernt wurden
hint-function-inkey=Gibt den nächsten Tastendruck als aus einem Zeichen lange Zeichenfolge oder als Zeichenfolge mit dem Namen der Funktions- oder Cursorsteuertaste zurück
hint-function-tostring=Konvertiert einen Ausdruck in einen `STRING`-Typ
hint-function-mask_pwd=Gibt eine gültige Zeichenmaske für Eingabeanweisungen von Passwörtern zurück
hint-function-mask_alpha=Gibt eine gültige Zeichenmaske für Eingabeanweisungen von A bis Z und a bis z zurück
hint-function-mask_num=Gibt eine gültige Zeichenmaske für Eingabeanweisungen von 0 bis 9 zurück
hint-function-mask_alnum=Gibt eine gültige Zeichenmaske für Eingabeanweisungen von A bis Z, a bis z und 0 bis 9 zurück
hint-function-mask_file=Gibt eine gültige Zeichenmaske für Eingabeanweisungen von Dateinamen zurück
hint-function-mask_path=Gibt eine gültige Zeichenmaske für Eingabeanweisungen von Pfadnamen zurück
hint-function-mask_ascii=Gibt eine gültige Zeichenmaske für Eingabeanweisungen mit Leerzeichen („“) bis Tilde („~“) zurück.
hint-function-curconf=Gibt die aktuelle Konferenznummer zurück
hint-function-pcbdat=Gibt eine Zeichenfolge mit dem Pfad und Dateinamen von PCBOARD.DAT zurück
hint-function-ppepath=Gibt eine Zeichenfolge mit dem Pfad (kein Dateiname) der aktuell ausgeführten PPE-Datei zurück
hint-function-valdate=Gibt `TRUE` zurück, wenn @1 ein gültiges Datumsformat hat
hint-function-valtime=Gibt `TRUE` zurück, wenn @1 in einem gültigen Zeitformat vorliegt
hint-function-u_msgrd=Gibt die Anzahl der Nachrichten zurück, die der Benutzer gelesen hat
hint-function-u_msgwr=Gibt die Anzahl der Nachrichten zurück, die der Benutzer geschrieben hat
hint-function-pcbnode=Gibt die Knotennummer zurück
hint-function-readline=Zeilennummer @2 aus der Datei @1 lesen und zurückgeben
hint-function-sysopsec=Gibt die in PCBOARD.DAT definierte SysOp-Sicherheit zurück
hint-function-onlocal=Gibt `TRUE` zurück, wenn der Benutzer lokal angemeldet ist
hint-function-un_stat=Gibt einen Knotenstatus von USERNET.XXX nach einer RdUnet-Anweisung zurück
hint-function-un_name=Gibt einen Knotenbenutzernamen aus USERNET.XXX nach einer RdUnet-Anweisung zurück
hint-function-un_city=Gibt eine Knotenstadt aus USERNET.XXX nach einer RdUnet-Anweisung zurück
hint-function-un_oper=Gibt einen Knotenoperationstext aus USERNET.XXX nach einer RdUnet-Anweisung zurück
hint-function-cursec=Gibt die aktuelle Sicherheitsstufe des Benutzers zurück
hint-function-gettoken=Gibt das nächste String-Token von einem vorherigen Aufruf von `Tokenize` zurück (identisch mit der `GETTOKEN`-Anweisung, kann aber in einem Ausdruck ohne vorherige Zuweisung an eine Variable verwendet werden)
hint-function-minleft=Gibt die aktuellen Minuten zurück, die dem Anrufer noch online zur Verfügung stehen
hint-function-minon=Gibt die aktuellen Online-Minuten des Anrufers in dieser Sitzung zurück
hint-function-getenv=Gibt den Wert der durch @1 benannten Umgebungsvariablen zurück
hint-function-callid=Gibt die Anrufer-ID-Zeichenfolge zurück
hint-function-regal=Gibt den Wert des AL-Registers nach einer DoIntr-Anweisung zurück
hint-function-regah=Gibt den Wert des AH-Registers nach einer DoIntr-Anweisung zurück
hint-function-regbl=Gibt den Wert des BL-Registers nach einer DoIntr-Anweisung zurück
hint-function-regbh=Gibt den Wert des BH-Registers nach einer DoIntr-Anweisung zurück
hint-function-regcl=Gibt den Wert des CL-Registers nach einer DoIntr-Anweisung zurück
hint-function-regch=Gibt den Wert des CH-Registers nach einer DoIntr-Anweisung zurück
hint-function-regdl=Gibt den Wert des DL-Registers nach einer DoIntr-Anweisung zurück
hint-function-regdh=Gibt den Wert des DH-Registers nach einer DoIntr-Anweisung zurück
hint-function-regax=Gibt den Wert des AX-Registers nach einer DoIntr-Anweisung zurück
hint-function-regbx=Gibt den Wert des BX-Registers nach einer DoIntr-Anweisung zurück
hint-function-regcx=Gibt den Wert des CX-Registers nach einer DoIntr-Anweisung zurück
hint-function-regdx=Gibt den Wert des DX-Registers nach einer DoIntr-Anweisung zurück
hint-function-regsi=Gibt den Wert des SI-Registers nach einer DoIntr-Anweisung zurück
hint-function-regdi=Gibt den Wert des DI-Registers nach einer DoIntr-Anweisung zurück
hint-function-regf=Gibt den Wert des Flag-Registers nach einer DoIntr-Anweisung zurück
hint-function-regcf=Gibt den Wert des Carry-Flag-Registers nach einer DoIntr-Anweisung zurück
hint-function-regds=Gibt den Wert des DS-Registers nach einer DoIntr-Anweisung zurück
hint-function-reges=Gibt den Wert des ES-Registers nach einer DoIntr-Anweisung zurück
hint-function-b2w=Gibt ein Wort zurück, das aus zwei bytegroßen Werten nach folgender Formel besteht:
    `(@1*0100h+@2)`
hint-function-peekb=Gibt einen Bytewert (0-255) zurück, der sich an der Speicheradresse @1 befindet (PEEK ist ein Synonym)
hint-function-peekw=Gibt einen Wortwert (0-65535) zurück, der sich an der Speicheradresse @1 befindet
hint-function-mkaddr=Gibt eine segment:offset-Adresse als lange Ganzzahl zurück, die aus zwei wortgroßen Werten nach der Formel besteht:
    `@1*00010000h+@2`
hint-function-exist=Gibt einen booleschen `TRUE`-Wert zurück, wenn die Datei @1 vorhanden ist
hint-function-i2s=Gibt eine Zeichenfolge zurück, die den ganzzahligen Wert @1 darstellt, der in den Basiswert @2 konvertiert wurde
hint-function-s2i=Gibt eine Ganzzahl zurück, die die aus der Basis @2 konvertierte Zeichenfolge @1 darstellt
hint-function-carrier=Gibt die vom Modem gemeldete Trägergeschwindigkeit an PCBoard zurück
hint-function-tokenstr=Gibt eine zuvor tokenisierte Zeichenfolge zurück, die mit Semikolons rekonstruiert wurde, die die Komponenten-Tokens trennen
hint-function-cdon=Gibt `TRUE` zurück, wenn das Trägererkennungssignal eingeschaltet ist, `FALSE`
hint-function-langext=Gibt die Dateierweiterung für die Sprachauswahl des Benutzers zurück
hint-function-ansion=Gibt `TRUE` zurück, wenn der Benutzer lokal angemeldet ist
hint-function-valcc=Gibt `TRUE` zurück, wenn @1 eine gültige Kreditkartennummer ist
hint-function-fmtcc=Gibt eine formatierte Kreditkartennummer basierend auf @1 zurück
hint-function-cctype=Gibt den Aussteller der Kreditkartennummer @1 zurück
hint-function-getx=Gibt die aktuelle Spalte (X-Position) des Cursors auf dem Display zurück
hint-function-gety=Gibt die aktuelle Zeile (Y-Position) des Cursors auf dem Display zurück
hint-function-band=Gibt das bitweise Und von zwei ganzzahligen Ausdrücken zurück
hint-function-bor=Gibt das bitweise Oder zweier ganzzahliger Ausdrücke zurück
hint-function-bxor=Gibt das bitweise Exklusiv-Oder zweier ganzzahliger Ausdrücke zurück
hint-function-bnot=Gibt das bitweise Komplement (alle Bits invertiert) eines ganzzahligen Ausdrucks zurück
hint-function-u_pwdhist=Gibt das angegebene Passwort aus dem Passwortverlauf zurück. Gültige Werte für @1 sind 1 bis 3
hint-function-u_pwdlc=Gibt das Datum der letzten Passwortänderung zurück
hint-function-u_pwdtc=Gibt die Häufigkeit zurück, mit der das Passwort geändert wurde
hint-function-u_stat=Gibt eine Statistik über den Benutzer zurück, die von PCBoard verfolgt wird
    Gültige Werte für @1 sind 1 bis 15
    |||
    | --- | --- |
    | 1 | Erster Termin, an dem der Benutzer das System aufgerufen hat |
    | 2 | Anzahl der SysOp-Seiten, die der Benutzer angefordert hat |
    | 3 | Anzahl der Gruppenchats, an denen der Benutzer teilgenommen hat |
    | 4 | Anzahl der Kommentare, die der Benutzer hinterlassen hat |
    | 5 | Anzahl der 300-Bit/s-Verbindungen |
    | 6 | Anzahl der 1200-Bit/s-Verbindungen |
    | 7 | Bumber von 2400 bps verbindet |
    | 8 | Anzahl der 9600-Bit/s-Verbindungen |
    | 9 | Anzahl der 14400-Bit/s-Verbindungen |
    | 10 | Anzahl der Sicherheitsverstöße |
    | 11 | Anzahl der „Nicht in der Konferenz registriert“-Warnungen |
    | 12 | wie oft das Download-Limit des Benutzers erreicht wurde |
    | 13 | Anzahl der Warnungen „Datei nicht gefunden“ |
    | 14 | Anzahl der Passwortfehler, die der Benutzer hatte |
    | 15 | Anzahl der Überprüfungsfehler, die der Benutzer hatte |
hint-function-defcolor=Gibt die Standardfarbe des Systems zurück.
hint-function-abs=Gibt den absoluten Wert von @1 zurück
hint-function-sin=Gibt den Sinus von @1 zurück (angegeben im Bogenmaß).
hint-function-cos=Gibt den Kosinus von @1 zurück (angegeben im Bogenmaß).
hint-function-tan=Gibt den Tangens von @1 zurück (angegeben im Bogenmaß).
hint-function-atan=Gibt den Arkustangens von @1 im Bogenmaß zurück.
hint-function-log=Gibt den natürlichen Logarithmus von @1 zurück.
hint-function-sqrt=Gibt die Quadratwurzel von @1 zurück.
hint-function-grafmode=Gibt ein Zeichen zurück, das den Grafikstatus des Benutzers angibt

    | Wert | Bedeutung |
    | :--- | :--- |
    | R | RIPscrip unterstützt |
    | G | ANSI-Grafiken (Farbe und Positionierung) werden unterstützt |
    | A | ANSI-Positionierung (keine Farbe) unterstützt |
    | N | Keine Grafik (RIP oder ANSI) unterstützt |
hint-function-psa=Gibt den Wert der angegebenen PSA-Variablen zurück

    @1 = Die abzurufende PSA-Variable

    ### PSA
    | | |
    | :--- | :--- |
    | 1 | Alias-Unterstützung aktiviert |
    | 2 | Überprüfen Sie, ob der Support aktiviert ist |
    | 3 | Adressunterstützung aktiviert |
    | 4 | Passwortunterstützung aktiviert |
    | 5 | Statistikunterstützung aktiviert |
    | 6 | Notes-Unterstützung aktiviert |
hint-function-fileinf=Gibt Informationen über die durch @1 angegebene Datei zurück

    @1 = Die Datei, über die Informationen abgerufen werden sollen

    @2 = Die zurückzugebenden Informationen

    ### Gültige Optionen
    | | |
    | :--- | :--- |
    | 1 | Gibt TRUE zurück, wenn die Datei vorhanden ist |
    | 2 | Datumsstempel der Rücksendedatei |
    | 3 | Zeitstempel der Rückgabedatei |
    | 4 | Dateigröße zurückgeben |
    | 5 | Dateiattribute zurückgeben 1) |
    | 6 | Dateilaufwerk zurückgeben |
    | 7 | Dateipfad zurückgeben |
    | 8 | Dateibasisnamen zurückgeben |
    | 9 | Dateierweiterung zurückgeben |

    | 1) Dateiattribut | |
    | :--- | :--- |
    | 01h | Schreibgeschützt |
    | 02h | Versteckt |
    | 04h | System |
    | 20h | Archiv |
hint-function-ppename=Gibt den Namen der aktuell ausgeführten PPE-Datei abzüglich Pfad und Erweiterung zurück
hint-function-mkdate=Gibt ein Datum zurück, wobei das Jahr durch Jahr (1900-2079), den Monat durch Monat (1-12) und den Tag durch Tag (1-31) angegeben wird.
hint-function-curcolor=Gibt die aktuelle Farbe (0-255) zurück, die vom ANSI-Treiber verwendet wird
hint-function-kinkey=Gibt den nächsten Tastendruck von der BBS-Tastatur als eine aus einem Zeichen lange Zeichenfolge oder eine Zeichenfolge mit dem Namen der Funktions- oder Cursorsteuertaste zurück
hint-function-minkey=Gibt den nächsten Tastendruck des Remote-Aufrufers als aus einem Zeichen lange Zeichenfolge oder als Zeichenfolge mit dem Namen der Funktions- oder Cursorsteuertaste zurück
hint-function-maxnode=Gibt den mit der aktuellen Software maximal möglichen Knoten zurück (d. h. /2 würde 2 zurückgeben, /10 würde 10 zurückgeben usw.)
hint-function-slpath=Gibt den in PCBSetup angegebenen Pfad zu den Anmeldesicherheitsdateien zurück
hint-function-helppath=Gibt den in PCBSetup angegebenen Pfad zu den Hilfedateien zurück
hint-function-temppath=Gibt den in PCBSetup angegebenen Pfad zum temporären Arbeitsverzeichnis zurück
hint-function-modem=Gibt die Modem-Verbindungszeichenfolge zurück, wie sie vom Modem an PCBoard gemeldet wurde
hint-function-loggedon=Gibt `TRUE` zurück, wenn sich der Benutzer bereits beim BBS angemeldet hat, andernfalls `FALSE`
hint-function-callnum=Gibt die Anrufernummer des aktuellen Benutzers zurück.
hint-function-mgetbyte=Gibt den Wert des nächsten Bytes vom Modem (0-255) oder -1 zurück, wenn keine Bytes auf die Eingabe warten
hint-function-tokcount=Gibt die Anzahl der Token zurück, die über die GetToken-Anweisung und/oder -Funktion verfügbar sind
hint-function-u_recnum=Gibt die Benutzerdatensatznummer (0-65535) für den Benutzernamen user oder -1 zurück, wenn der Benutzer nicht auf diesem System registriert ist.
hint-function-u_inconf=Gibt `TRUE` zurück, wenn die Benutzerdatensatznummer @1 in der Konferenz @2 registriert ist
hint-function-peekdw=Gibt einen vorzeichenbehafteten Ganzzahlwert (-2147483648 - +2147483647) zurück, der sich an der Speicheradresse „var“ befindet.
hint-function-dbglevel=Gibt die aktuelle Debugstufe zurück
hint-function-scrtext=### Rückgabewert
    Gibt eine Zeichenfolge von @3-Zeichen vom Bildschirm bei @1, @2 zurück.
    Wenn @3 `TRUE` ist, wird die Zeichenfolge mit allen intakten @-Codes zurückgegeben.
hint-function-showstat=Gibt `TRUE` zurück, wenn das Schreiben auf die Anzeige aktiv ist, `FALSE`, wenn das Schreiben auf die Anzeige deaktiviert ist
hint-function-pagestat=Gibt `TRUE` zurück, wenn der Benutzer den SysOp ausgelagert hat (oder PageOn ausgegeben wurde), andernfalls `FALSE` (oder PageOff ausgegeben wurde).
hint-function-replacestr=Sie funktioniert genau wie die Funktion „Ersetzen“, mit der Ausnahme, dass sowohl für die Suche als auch für das Ersetzen eine vollständige Teilzeichenfolge angegeben werden kann
hint-function-stripstr=Funktioniert genauso wie die Strip-Funktion, außer dass eine vollständige Teilzeichenfolge für die Suche angegeben werden kann
hint-function-tobigstr=Konvertiert einen Ausdruck in einen `BIGSTR`-Typ
hint-function-toboolean=Konvertiert einen Ausdruck in einen `BOOLEAN`-Typ
hint-function-tobyte=Konvertiert einen Ausdruck in einen `BYTE`-Typ
hint-function-todate=Konvertiert einen Ausdruck in einen `DATE`-Typ
hint-function-todreal=Konvertiert einen Ausdruck in einen `DREAL`-Typ
hint-function-toedate=Konvertiert einen Ausdruck in einen `EDATE`-Typ
hint-function-tointeger=Konvertiert einen Ausdruck in einen `INTEGER`-Typ
hint-function-tomoney=Konvertiert einen Ausdruck in einen `MONEY`-Typ
hint-function-toreal=Konvertiert einen Ausdruck in einen `REAL`-Typ
hint-function-tosbyte=Konvertiert einen Ausdruck in einen `SBYTE`-Typ
hint-function-tosword=Konvertiert einen Ausdruck in einen `SWORD`-Typ
hint-function-totime=Konvertiert einen Ausdruck in einen `TIME`-Typ
hint-function-tounsigned=Konvertiert einen Ausdruck in einen `UNSIGNED`-Typ
hint-function-toword=Konvertiert einen Ausdruck in einen `WORD`-Typ
hint-function-mixed=Konvertiert eine Zeichenfolge in die gemischte Groß-/Kleinschreibung (oder den Eigennamen).
hint-function-alias=Gibt die aktuelle ALIAS-Einstellung des Benutzers zurück (TRUE = Alias-Verwendung aktiviert, FALSE = Alias-Verwendung deaktiviert)
hint-function-confreg=Gibt TRUE zurück, wenn das Flag „Benutzer registriert“ gesetzt ist, andernfalls FALSE
hint-function-confexp=Gibt TRUE zurück, wenn das Flag „Benutzer abgelaufen“ gesetzt ist, andernfalls FALSE
hint-function-confsel=Gibt TRUE zurück, wenn der Benutzer die Konferenz ausgewählt hat, andernfalls FALSE
hint-function-confsys=Gibt TRUE zurück, wenn der Benutzer Konferenz-SysOp-Zugriff hat, andernfalls FALSE
hint-function-confmw=Gibt TRUE zurück, wenn auf den Benutzer E-Mails in der Konferenzkonferenz warten, andernfalls FALSE
hint-function-lprinted=Gibt die Anzahl der auf dem Display gedruckten Zeilen zurück
hint-function-isnonstop=Gibt zurück, ob sich die Anzeige derzeit im Non-Stop-Modus befindet (d. h. ob der Benutzer „NS“ als Teil seiner Befehlszeile eingegeben hat).
hint-function-errcorrect=Gibt TRUE zurück, wenn festgestellt wird, dass eine Sitzung fehlerkorrigiert ist (oder FALSE für nicht fehlerkorrigierte Sitzungen).
hint-function-confalias=Gibt TRUE zurück, wenn die aktuelle Konferenz so konfiguriert ist, dass Aliase zulässig sind
hint-function-useralias=Gibt TRUE zurück, wenn der aktuelle Benutzer einen Alias ​​verwenden darf
hint-function-curuser=Bestimmen Sie, welche Benutzerinformationen gegebenenfalls über die Benutzervariablen verfügbar sind. Es akzeptiert keine Argumente und gibt einen der folgenden Werte zurück:
    NO_USER (-1) – Benutzervariablen sind derzeit nicht definiert
    CUR_USER (0) – Benutzervariablen gelten für den aktuellen Benutzer
    Andere – Die Datensatznummer eines alternativen Benutzers, für den Benutzervariablen definiert sind
hint-function-u_lmr=Funktion, um die Nummer der zuletzt gelesenen Nachricht für die angegebene Konferenz zurückzugeben.
hint-function-chatstat=Gibt den Chat-Verfügbarkeitsstatus des aktuellen Benutzers zurück (TRUE bedeutet verfügbar, FALSE bedeutet nicht verfügbar).
hint-function-defans=Gibt die letzte Standardantwort zurück, die an eine Input-Anweisung übergeben wurde. Dies ermöglicht es einem PPE beispielsweise zu bestimmen, wie die Standardantwort ausgefallen wäre, wenn eine PCBTEXT-Eingabeaufforderung nicht durch ein PPE ersetzt worden wäre.
hint-function-lastans=Funktion, um die letzte von einer Eingabeanweisung akzeptierte Antwort zurückzugeben.
hint-function-meganum=Konvertiert eine Dezimalzahl (von 0 bis 1295) in eine Hexa-Tridezimalzahl oder Meganum.
hint-function-evttimeadj=Erkennt, ob die Zeit des Benutzers für ein bevorstehendes Ereignis angepasst wurde. Dies ist nützlich, um festzustellen, ob die verbleibende Zeit eines Benutzers mit der AdjTime-Anweisung erhöht werden kann.
hint-function-isbitset=Überprüfen Sie den Status eines angegebenen Bits in einer Variablen.
    Diese Funktion ist in erster Linie für die Verwendung mit BIGSTR-Variablen gedacht, die bis zu 2048 Bytes lang sein können.
    Bei Bedarf funktioniert es jedoch auch mit anderen Datentypen (und Ausdrücken).
hint-function-fmtreal=Formatiert REAL/DREAL-Werte für Anzeigezwecke.
    ### Parameter
    realExp Ein REAL/DREAL-Gleitkommaausdruck
    fieldWidth Die Mindestanzahl der anzuzeigenden Zeichen
    decimalPlaces Die Anzahl der Zeichen, die rechts vom Dezimalpunkt angezeigt werden sollen
hint-function-flagcnt=Gibt die Anzahl der zum Download markierten Dateien zurück.
hint-function-kbdbufsize=Gibt die Anzahl der im KbdString-Puffer ausstehenden Tastendrücke zurück
hint-function-pplbufsize=Gibt die Anzahl der im KbdStuff-Puffer ausstehenden Tastendrücke zurück.
hint-function-kbdfilused=Liefert `TRUE`, solange die Tastatureingabe aus einem `KBDFILE`-Skript stammt, andernfalls `FALSE`. Dadurch lässt sich dateigesteuerte Eingabe von `KBDSTUFF` und `KBDSTRING` unterscheiden.
hint-function-lomsgnum=Gibt die niedrige Nachrichtennummer für die aktuelle Konferenz zurück.
hint-function-himsgnum=Gibt die höchste Nachrichtennummer für die aktuelle Konferenz zurück.
hint-function-drivespace=Rückgabewert: Von der Laufwerksspezifikation verbleibender Divespace.
hint-function-outbytes=Gibt die Anzahl der Bytes zurück, die im Ausgabepuffer des Modems warten. Im lokalen Modus nicht verfügbar.
hint-function-hiconfnum=Gibt die höchste auf der Tafel verfügbare Konferenznummer zurück
hint-function-inbytes=Gibt die Anzahl der im Modem-Eingabepuffer wartenden Bytes zurück. Im lokalen Modus nicht verfügbar.
hint-function-crc32=Gibt einen UNSIGNED-Wert des CRC einer Datei oder Zeichenfolge zurück.
hint-function-pcbmac=Gibt einen BIGSTR zurück, der den erweiterten Text eines PCB-MAKROs enthält

    ### PCB-MAKROS werden nicht unterstützt
    @automore@ @beep@ @clreol@ @cls@ @delay@ @more@ @pause@ @poff@ @pon@ @pos@ @qoff@ @qon@ @wait@ @who@ @x@
hint-function-actmsgnum=### Rückgabewert
    Gibt die Anzahl der aktiven Nachrichten in der aktuellen Konferenz zurück

    ### Beispiel
    ```
    integer i
    println "There are ",ACTMSGNUM()," messages in conference ",CURCONF()
    ```
hint-function-stackleft=Gibt die Anzahl der auf dem Systemstapel verbleibenden Bytes zurück.
hint-function-stackerr=Gibt einen booleschen Wert zurück, der angibt, dass ein Stapelfehler aufgetreten ist, wenn TRUE.
hint-function-dgetalias=Gibt den aktuellen Alias ​​zurück
hint-function-dbof=Liefert `TRUE`, wenn der Datensatzzeiger des ausgewählten DBase-Kanals vor dem ersten Datensatz steht, andernfalls `FALSE`.
hint-function-dchanged=Gibt das geänderte Flag zurück
hint-function-ddecimals=Gibt Dezimalzahlen des benannten Feldes zurück
hint-function-ddeleted=Gibt das gelöschte Flag zurück
hint-function-deof=Gibt den End-of-File-Status zurück
hint-function-derr=Fehlerflag für den Kanal zurückgeben
hint-function-dfields=Gibt die Anzahl der Felder zurück
hint-function-dlength=Gibt die Länge des benannten Felds zurück
hint-function-dname=Gibt den Namen des nummerierten Feldes zurück
hint-function-dreccount=Gibt die Anzahl der Datensätze zurück
hint-function-drecno=Gibt die aktuelle Datensatznummer zurück
hint-function-dtype=Rückgabetyp des benannten Feldes
hint-function-fnext=Gibt einen verfügbaren Dateikanal zurück. -1, wenn keine verfügbar sind.
hint-function-dnext=
    Liefert die nächste unbenutzte DBase-Kanalnummer oder `-1`, wenn kein Kanal verfügbar ist.

    Der Kanal wird erst beim Öffnen einer Datei reserviert. Wiederholte `DNEXT()`-Aufrufe liefern daher dieselbe Nummer; sie muss gespeichert und die Datei geöffnet werden, bevor erneut gefragt wird.
hint-function-toddate=Konvertiert ein Datum in eine Zeichenfolge im Format MM/TT/JJJJ
hint-function-dcloseall=Schließen Sie alle DBF-Dateien
hint-function-dopen=Öffnen Sie die DBF-Datei
hint-function-dclose=DBF-Datei schließen
hint-function-dsetalias=DBF-Alias ​​festlegen
hint-function-dpack=Packen Sie die DBF-Datei
hint-function-dlockf=DBF-Datei sperren
hint-function-dlock=DBF-Datei sperren
hint-function-dlockr=Versucht, einen Datensatz auf dem ausgewählten DBase-Kanal zu sperren, und meldet, ob die Sperre erfolgreich war. Nach der Aktualisierung muss die passende Entsperrfunktion verwendet werden.
hint-function-dunlock=Entsperren Sie alle aktuellen Schlösser
hint-function-dnopen=NDX-Datei öffnen
hint-function-dnclose=NDX-Datei schließen
hint-function-dncloseall=Schließen Sie alle NDX-Dateien
hint-function-dnew=einen neuen Rekord starten
hint-function-dadd=Fügen Sie den neuen Datensatz hinzu
hint-function-dappend=einen leeren Datensatz anhängen
hint-function-dtop=Gehe zum obersten Datensatz
hint-function-dgo=Gehen Sie zu einem bestimmten Datensatz
hint-function-dbottom=Gehe zum unteren Datensatz
hint-function-dskip=+/- eine Anzahl von Datensätzen überspringen
hint-function-dblank=Löschen Sie den Datensatz
hint-function-ddelete=den Datensatz löschen
hint-function-drecall=Erinnern Sie sich an die Aufzeichnung
hint-function-dtag=Wählen Sie ein Tag aus
hint-function-dseek=gibt den Fehlerstatus zurück ( 0|1 )
    oder Erfolg suchen (0 = Fehler
    1 = Erfolg, 2 = folgender Datensatz
    3 = Ende der Datei)
hint-function-dfblank=Leeren Sie ein benanntes Feld
hint-function-dget=Holen Sie sich einen Wert aus einem benannten Feld
hint-function-dput=Geben Sie einen Wert in ein benanntes Feld ein
hint-function-dfcopy=Kopieren Sie ein Feld in ein Feld
hint-function-dselect=Gibt den mit dem Alias ​​verknüpften Kanal zurück
hint-function-dchkstat=Liefert `0`, wenn DBase-Kanal `@1` geöffnet ist, und `1`, wenn er geschlossen oder nicht verfügbar ist.
hint-function-pcbaccount=Gibt zurück, was PCBoard einem Benutzer für eine bestimmte Aktivität berechnet. Dies sind Werte, die der SysOp in PCBsetup zuweist, wenn die Buchhaltung konfiguriert und aktiviert wird.
    Gültige Werte für den Feldparameter sind 0–14. Die Verwendung der entsprechenden Konstanten wird empfohlen. (siehe Abschnitt Buchhaltung)

    { Buchhaltungskonstanten }
hint-function-pcbaccstat=Gibt den Wert im Statusfeld zurück
    Diese Funktion kann und sollte in Verbindung mit dem ACC_??? verwendet werden. Konstanten als Feldparameter. Gültige Werte für das Feld sind 0-3.

 | Feld | Dez. | Feldbeschreibung |
 | :--- |  :--- | :--- |
 | `ACC_STAT` | `0` | Gibt den Status des Schalters „Enable Accounting“ in der PWRD-Datei zurück.  |
 | `ACC_TIME` | `1` | Die Menge der ZUSÄTZLICH zu berechnenden Einheiten |
 | `ACC_MSGR` | `2` | Der Betrag, der ZUSÄTZLICH für jede gelesene Nachricht in der aktuellen Konferenz berechnet wird. |
 | `ACC_MSGW` | `3` | Der Betrag, der ZUSÄTZLICH für jede in der aktuellen Konferenz eingegebene Nachricht berechnet wird. |
hint-function-derrmsg=gibt den letzten DBase-Fehlertext zurück
hint-function-account=Gibt den Betrag der Credits zurück, die für Dienste entsprechend dem Feldparameter berechnet wurden.
hint-function-scanmsghdr=Gibt die erste Nachrichtennummer in der Nachrichtenbasis zurück, die den Suchkriterien entspricht.

    { message_header_constants }
hint-function-checkrip=Gibt `TRUE` zurück, wenn das Terminal über RIP verfügt.
hint-function-ripver=Gibt eine Zeichenfolge zurück, die die RIP-Version enthält. Wenn kein RIP verfügbar ist, wird „0“ zurückgegeben.
hint-function-qwklimits=
    Liefert eine QWK-Grenze des aktuellen Benutzers. `@1` ist `MAXMSGS`, `CMAXMSGS`, `ATTACH_LIM_U` oder `ATTACH_LIM_P`.

    Zuvor muss `GETUSER` aufgerufen werden. Systemweite Grenzen aus PCBSetup begrenzen weiterhin die Werte einzelner Benutzer.
hint-function-findfirst=Suchen Sie das erste Vorkommen von Dateispezifikation in einem Verzeichnis. Wird in Verbindung mit FindNext verwendet, um eine Verzeichnisliste abzurufen.

    ### Parameter
    @1 = Ein Zeichenfolgenausdruck mit dem Pfad und Dateinamen, über den auf Informationen zugegriffen werden soll.
    Sehr oft handelt es sich bei diesem Ausdruck um einen DOS-Platzhalter (z. B. *.*, *.BAT usw.).

    ### Rückgabewert
    Der erste Dateiname, der den Dateinamenkriterien entspricht.

    ### Hinweise
    Diese Funktion soll dabei helfen, Dateien zu finden, die einer bestimmten Datei entsprechen
    Kriterien.  Beispielsweise möchten Sie möglicherweise alle Dateien löschen, die mit *.BAK übereinstimmen
    im aktuellen Verzeichnis.  Das geht ganz einfach, denn
    FINDFIRST() findet die erste Übereinstimmung, während FINDNEXT() sucht
    zusätzliche Übereinstimmungen.

    Es ist zu beachten, dass nur die Dateinamen zurückgegeben werden.  Wenn Sie brauchen
    Zusätzliche Informationen wie Datum, Uhrzeit oder Größe der Datei verwenden
    die Funktion FILEINF().
hint-function-findnext=Diese Funktion ermittelt, ob weitere Dateien vorhanden sind, die einem angegebenen Muster entsprechen.

    ### Rückgabewert
    Der nächste Dateiname, der den Dateinamenkriterien entspricht, oder ein
    leere Zeichenfolge, wenn keine passenden Dateien mehr vorhanden sind.

    ### Hinweise
    Diese Funktion dient dazu, die Funktion FINDFIRST() fortzusetzen
    bricht ab, da alle weiteren Dateien gefunden werden, die dem Muster entsprechen
    zuletzt gesucht.  Bei der Rückgabe sind keine passenden Dateien mehr vorhanden
    Der Wert ist null oder eine leere Zeichenfolge. Weil Sie nicht wissen, wie viele
    Wenn es passende Dateien gibt, ist das Sammeln normalerweise mit einer WHILE-Schleife verbunden
    alle Dateinamen.

    Es ist zu beachten, dass nur die Dateinamen zurückgegeben werden.  Wenn Sie brauchen
    Zusätzliche Informationen wie Datum, Uhrzeit oder Größe der Datei verwenden Sie
    Funktion FILEINF().
hint-function-uselmrs=### Parameter
    @1 = Weist PCBoard an, das LMRS eines alternativen Benutzers NICHT zu laden
    wenn ein GETALTUSER ausgeführt wird.

    ### Hinweise
    Diese Anweisung kann bei einem GETALTUSER eine erhebliche Menge an Speicher einsparen
    wird zu einem späteren Zeitpunkt ausgeführt. Wenn GETALTUSER ausgeführt wird, wird es geladen
    standardmäßig die LMRs des Benutzers. Wenn Sie eine erhebliche Anzahl von Konferenzen haben
    Auf Ihrem System kann dies sehr viel Speicher beanspruchen. Seit PCBoard
    ist so reich an Funktionen, dass es die meisten, wenn nicht sogar alle verfügbaren Funktionen aufnehmen kann
    herkömmlicher Speicher, so dass PSAs nicht trocknen können. Wenn ein alternativer Benutzer
    LMRs werden von der PPE-Anwendung nicht benötigt, dann können Sie diese verwenden
    Anweisung, um PCBoard anzuweisen, die LMR-Daten nicht zu laden.

    Siehe auch die FUNCTION USELMRS, diese gibt den aktuellen Status zurück
    von USELMRS. Wenn beispielsweise die Funktion USELMRS TRUE zurückgibt, dann ein GETALTUSEr
    lädt LMRS. Wenn es FALSE zurückgibt, wird LMRS nicht geladen.
hint-function-confinfo=Mit dieser Anweisung kann auf ein Feld in der Konferenz zugegriffen werden
    Konfiguration.

    ### Parameter
    @1 = Die Konferenznummer, über die Informationen abgerufen werden sollen
    @2 = Zu änderndes Konferenzfeld. (Siehe Hinweis)

    { Conference_access_constants }
hint-function-tinkey=### Parameter
    @1 = Anzahl der Takte, die auf die Eingabe gewartet werden sollen.

    ### Rückgabewert
    Vom Benutzer eingegebene Eingabe

    ### Hinweise
    Y1 ist die Anzahl der Taktimpulse, die `TINEKY` auf eine Eingabe warten soll
    bevor das Zeitlimit überschritten wird. 1 Sekunde = 18 Ticks (ungefähr)

    Ein Tick-Wert von 0 führt dazu, dass `TINKEY` unbegrenzt auf die Eingabe mit wartet
    eine maximale Timeout-Zeit von ca. 4 Stunden. Der Verlust des Mobilfunkanbieters endet ebenfalls
    `TINKEY`.
hint-function-cwd=### Rückgabewert
    Das aktuelle Arbeitsverzeichnis
hint-function-instrr=Gibt die Position ganz rechts von @2 in @1 `(1-LEN(@1))` oder `0` zurück, wenn @2 nicht in @1 ist
hint-function-base64enc=Kodiert die Bytes von @1 als Base64-Text. Ein String-Argument steuert seine UTF-8-Bytes bei.
hint-function-base64dec=Dekodiert Base64-Text in @1 in einen Byte-Blob. Fehlerhafte Eingabeberichte `ErrCode.Format`.
hint-function-tobytes=Die binäre Darstellung von @1 als Byte-Blob. Zeichenfolgen verwenden UTF-8; Numerische Skalare verwenden Little-Endian-Speicher mit fester Breite.
hint-statement-on-error=ON ERROR GOTO label | GOSUB-Label | Vorgehensweise | AUS – wohin ein fehlgeschlagener Vorgang das Programm sendet.
hint-function-fdordaka=Gibt die Adresse zurück, auf die dieses Board antwortet, als zone:net/node mit dem Punkt
    angehängt, wenn ein solcher Datensatz vorhanden ist, oder eine leere Zeichenfolge, wenn kein solcher Datensatz vorhanden ist

    @1 = Datensatznummer, gezählt von eins
hint-function-fdordorg=Gibt die Ursprungszeile zurück, die an die hier geschriebene Echomail angehängt ist

    @1 = Datensatznummer, gezählt von eins. Es ist nur eine Ursprungslinie konfiguriert,
    also ist jede zweite Zahl leer
hint-function-fdordarea=Gibt das Tag eines Nachrichtenbereichs zurück, der am Netzwerk teilnimmt, oder eines
    leere Zeichenfolge, wenn kein solcher Datensatz vorhanden ist

    @1 = Datensatznummer, gezählt von eins
hint-function-fdoqrd=Gibt die Datei zurück, die unter dieser Nummer in der Ausgangswarteschlange wartet, oder eine
    leere Zeichenfolge, wenn dort nichts wartet

    @1 = Datensatznummer, gezählt von eins
hint-function-getdrive=### Rückgabewert
    Der aktuelle Laufwerksbuchstabe

    ### Hinweise
    Laufwerksnummern entsprechen Laufwerksbuchstaben auf folgende Weise
    A: = 0
    B: = 1
    C: = 2
    …
hint-function-setdrive=Wählt die DOS-Laufwerksnummer `@1` und liefert die gewählte Nummer. IcyBoard besitzt keinen DOS-Laufwerkszustand; diese Kompatibilitätsfunktion gibt daher ihr Argument zurück, ohne die Pfadauflösung zu ändern.
hint-function-bs2i=Konvertiert einen 4-Byte-BSreal in eine PPL-Ganzzahl.

    ### Parameter
    @1 ist ein BIGSTR-Typ, da BIGSTR-Typen enthalten können
    Binärdaten. Für diese Funktion konvertiert PPL die erste
    4 Bytes des BIGSTR in eine INTEGER-Variable konvertieren und zurücksenden
    es.

    ### Rückgabewert
    Gibt einen konvertierten 4-Byte-BSrealwert in Form einer 4-Byte-Ganzzahl zurück.
hint-function-bd2i=Konvertiert ein 8-Byte-Bdreal in eine PPL-Ganzzahl.
hint-function-i2bs=Konvertiert einen 4-Byte-PPL-INTEGER in einen 4-Byte-BSreal und speichert ihn in einem BIGSTR.
hint-function-i2bd=Konvertiert einen 4-Byte-PPL-INTEGER in einen 8-Byte-Bdrealwert und speichert ihn.
hint-function-ftell=`FTELL` gibt den aktuellen Dateizeiger-Offset für die angegebene Datei zurück
        Dateikanal. Wenn der Kanal nicht geöffnet ist, wird 0 zurückgegeben.
        Andernfalls wird die aktuelle Position in der geöffneten Datei zurückgegeben.

        ### Parameter
        @1 – Der zu verarbeitende Dateikanal

        ### Rückgabewert
        4-Byte-Ganzzahl mit Vorzeichen, die den Dateizeiger-Offset enthält
        der an den Kanal angehängten Datei.
hint-function-os=### Rückgabewert
        Eine Ganzzahl, die angibt, welches Betriebssystem/ welche Platinenversion verwendet wird
        die PPE läuft derzeit unter.
        1=DOS, 2 = OS2, 0 = unbekannt.
hint-function-short_desc=### Rückgabewert
    TRUE, wenn der Benutzer kurze Dateibeschreibungen aktiviert hat, andernfalls wird FALSE zurückgegeben.
hint-function-getbankbal=### Parameter
    @1 Das abzurufende Feld.

    ### Rückgabewert
    Gibt den Wert eines angegebenen Feldes zurück.

    ### Felder

    Zeitfelder (in Minuten)
    ------------------------
        0 = Datum der letzten Einzahlung
        1 = Letztes Auszahlungsdatum
        2 = Letzter Transaktionsbetrag (in Minuten)
        3 = Gesparter Betrag (Zeitguthaben auf dem Konto)
        4 = Max. Auszahlung (das Maximum, das ein Benutzer an einem Tag abheben kann)
        5 = Max Stored Amount (maximal zulässige Speicherzeit)

    Bytefelder (in K Bytes)
    ------------------------
        6 = Letztes Einzahlungsdatum
        7 = Letztes Auszahlungsdatum
        8 = Letzter Transaktionsbetrag (in K Bytes)
        9 = Gesparter Betrag (ihr K-Byte-Saldo auf ihrem Konto)
        10 = Max. Auszahlung (das Maximum, das ein Benutzer an einem Tag abheben kann)
        11 = Max Stored Amount (maximal zulässige K-Bytes, die gespeichert werden dürfen)
hint-function-getmsghdr=### Parameter
    @1 = Konferenznummer der Nachrichtenbasis
    @2 = Ein doppelter Ausdruck, der die Nachrichtennummer der Nachricht angibt, um den Nachrichtenheaderwert zu erhalten.
    @3 = Das abzurufende Feld.

    ### Rückgabewert
    Gibt den Wert des angegebenen Felds zurück.

    { message_header_constants }
hint-function-setmsghdr=### Parameter
    @1 = Ein ganzzahliger Ausdruck, der die Konferenznummer der Nachrichtenbasis angibt.
    @2 = Ein doppelter Ausdruck, der die Nachrichtennummer der Nachricht angibt, um den Nachrichtenheaderwert festzulegen.
    @3 = Ein ganzzahliger Ausdruck zwischen 1 und 5, der das abzurufende Feld darstellt.
    @4 = Ein Zeichenfolgenausdruck, der die Daten enthält, die in das angegebene Feld eingefügt werden sollen.

    ### Felder
    1 = Feld „An“.
    2 = Feld „Von“.
    3 = Feld „Betreff“.
    4 = Feld „Passwort“.
    5 = „Echo“-Flag

    ### Rückgabewert
    Gibt den Wert der Nachrichtennummer zurück.  Wenn die Nachricht so ist
    Passt es an die gleiche Stelle wie das Original, dann ist es das Gleiche.
    Eine geänderte Kopfzeile passt nicht in die ursprüngliche Nachricht
    Header, dann wird die Nachricht am Ende der Nachricht eingefügt
    Basis.
hint-function-areaid=Erzeugt eine Tupelkonferenz/einen Tupelbereich zur Identifizierung einer Nachrichtenbasis.
hint-function-len_dim=@1 = Das Array, dessen Elementanzahl ermittelt werden soll
    @2 = Nullbasierte Dimensionsnummer (`0`, `1` oder `2`)
    ### Rückgabewert
        Liefert die Elementanzahl in Dimension @2, nicht ihren höchsten Index. Ein mit Grenze `[10]` deklariertes Array hat beispielsweise Länge 11. Eine ungültige Dimension liefert 0.