hint-type-boolean=Unsigned Char (1 Byte) 0 = `FALSE`, sonst `TRUE`
hint-type-date=Unsigned Integer (2 Bytes) PCBoard julianisches Datum (Anzahl der Tage seit 1/1/1900) 
hint-type-ddate=
    Long Int mit Vorzeichen für julianisches Datum. DDATE ist für die Verwendung mit DBase-Datumsfeldern.
    Es hält einen langen Integer für julianische Daten. Wenn es in den Zeichenfolgentyp gezwungen wird, ist es im Format CCYYMMDD oder 19940527
hint-type-integer=Signed long Integer (4 Bytes) Bereich: -2,147,483,648 → +2,147,483,647
hint-type-money=Signed long Integer (4 Bytes) Bereich: -$21,474,836.48 → +$21,474,836.47
hint-type-string=Zeichenfolge mit maximaler Länge von 256 Zeichen
hint-type-time=Signed long Integer (4 Bytes) Anzahl der Sekunden seit Mitternacht
hint-type-bigstr=Zeichenfolge mit maximaler Länge von 2048 Zeichen. Kann auch CHR(0) Zeichen enthalten.
hint-type-edate=Julianisches Datum im Earth Datum Format YYMM.DD. Gleicher Bereich wie DATE.
hint-type-float=4-Byte Fließkommazahl Bereich: +/-3.4E-38 - +/-3.4E+38 (7-Stellen Präzision)
hint-type-double=8-Byte Fließkommazahl Bereich: +/-1.7E-308 - +/-1.7E+308 (15-Stellen Präzision)
hint-type-unsigned=4-Byte unsigned Integer Bereich: 0 - 4,294,967,295
hint-type-long=8-Byte signed Integer Bereich: -9,223,372,036,854,775,808 - 9,223,372,036,854,775,807
hint-type-ulong=8-Byte unsigned Integer Bereich: 0 - 18,446,744,073,709,551,615
hint-type-users=Eine schreibgeschützte Momentaufnahme aller registrierten Benutzer des Boards. Einträge werden mit einem nullbasierten Index gelesen oder mit `FOREACH` durchlaufen.
hint-member-board-users=Die beim ersten Lesen von `Board` registrierten Benutzer als schreibgeschützte `USER`-Momentaufnahmen.
hint-member-users-count=Die Anzahl registrierter Benutzer in dieser Board-Momentaufnahme.
hint-member-user-valid=Gibt an, ob dieses `USER`-Objekt einen vorhandenen Datensatz darstellt. Ein ungültiger `Board.Users`-Index liefert einen leeren Benutzer mit `Valid` gleich false.
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
