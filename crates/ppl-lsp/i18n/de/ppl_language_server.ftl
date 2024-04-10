hint-type-boolean=
    BOOLEAN
    
    Unsigned Char (1 Byte) 0 = FALSCH, sonst TRUE
hint-type-date=
    DATE
    
    Unsigned Integer (2 Bytes) PCBoard julianisches Datum (Anzahl der Tage seit 1/1/1900) 
hint-type-ddate=
    DDATE

    Long Int mit Vorzeichen für julianisches Datum. DDATE ist für die Verwendung mit DBase-Datumsfeldern.
    Es hält einen langen Integer für julianische Daten. Wenn es in den Zeichenfolgentyp gezwungen wird, ist es im Format CCYYMMDD oder 19940527
hint-type-integer=
    INTEGER / SDWORD / LONG

    Signed long Integer (4 Bytes) Bereich: -2,147,483,648 → +2,147,483,647
hint-type-money=
    MONEY
    
    Signed long Integer (4 Bytes) Bereich: -$21,474,836.48 → +$21,474,836.47
hint-type-string=
    STRING
    
    Zeichenfolge mit maximaler Länge von 256 Zeichen
hint-type-time=
    TIME
    
    Signed long Integer (4 Bytes) Anzahl der Sekunden seit Mitternacht
hint-type-bigstr=
    BIGSTR
    
    Zeichenfolge mit maximaler Länge von 2048 Zeichen. Kann auch CHR(0) Zeichen enthalten.
hint-type-edate=
    EDATE
    
    Julianisches Datum im Earth Datum Format YYMM.DD. Gleicher Bereich wie DATE.
hint-type-float=
    REAL / FLOAT

    4-Byte Fließkommazahl Bereich: +/-3.4E-38 - +/-3.4E+38 (7-Stellen Präzision)
hint-type-double=
    DREAL / DOUBLE

    8-Byte Fließkommazahl Bereich: +/-1.7E-308 - +/-1.7E+308 (15-Stellen Präzision)
hint-type-unsigned=
    UNSIGNED / DWORD / UDWORD

    4-Byte unsigned Integer Bereich: 0 - 4,294,967,295
hint-type-Byte=
    Byte / UByte

    1-Byte unsigned Integer Bereich: 0 - 255
hint-type-word=
    WORD / UWORD

    2-Byte unsigned Integer Bereich: 0 - 65,535
hint-type-sByte=
    SByte / SHORT

    1-Byte signed Integer Bereich: -128 - 127
hint-type-sword=
    SWORD / INT

    2-Byte signed Integer Bereich: -32,768 - 32,767
