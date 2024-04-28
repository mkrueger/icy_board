
(*
 * COPYRIGHT 1993 SAMUEL H. SMITH
 * ALL RIGHTS RESERVED
 *
 *)

{$IFDEF LANG_ENGLISH}

TX_COPYRIGHT = 'HS/Link Configuration Manager v1.21·; Copyright (C) 1993 Samuel H. Smith';

TX_PGDN = ' PgDn [next page] ';
TX_PGUP = ' PgUp [prior page] ';
TX_HELP = ' F1 [help] ';
TX_EXIT = ' ESC [exit] ';

TX_PG1  = 'Basic Settings [Page 1 of 5]';
TX_SUB1 = 'These are the most important settings for proper HS/Link operation';
TX_P    = 'COM port number (1-8) . . . . . . . . . . . ';
TX_U    = 'Directory for downloaded files. . . . . . . ';
TX_HS   = 'Use "SLOW HANDSHAKE" during disk I/O. . . . ';
TX_WELCOME = 'Welcome!';

TX_PG2  = 'Advanced COM Settings [Page 2 of 5]';
TX_SUB2 = 'These settings control advanced communication features';
TX_PB   = 'Non-Standard COM base (in HEX, 0 = standard COM port) . . . . . . ';
TX_PI   = 'Non-Standard COM IRQ (1-15, 0 = standard COM port). . . . . . . . ';
TX_B    = 'COM port open speed (300-115200, 0 = automatic) . . . . . . . . . ';
TX_E    = 'Effective modem-to-modem speed (0 = same as open speed) . . . . . ';
TX_HC   = 'Enable CTS hardware handshake . . . . . . . . . . . . . . . . . . ';
TX_HR   = 'Enable RTS hardware handshake . . . . . . . . . . . . . . . . . . ';
TX_HX   = 'Enable XON/XOFF software handshake. . . . . . . . . . . . . . . . ';
TX_C    = 'Enable Carrier Detect checking. . . . . . . . . . . . . . . . . . ';
TX_NT   = 'Preserve original time/date stamp on received files . . . . . . . ';

TX_PG3  = 'Advanced Optional Features 1 [Page 3 of 5]';
TX_SUB3 = 'These settings control various optional features';
TX_R    = 'Attempt to Resume aborted transfers . . . . . . . . . . . . . . . ';
TX_K    = 'Keep partial files from aborted transfers . . . . . . . . . . . . ';
TX_O    = 'Allow received files to overwrite/modify existing files . . . . . ';
TX_NB   = 'Enable file read/write buffering. . . . . . . . . . . . . . . . . ';
TX_NC   = 'Enable "Dynamic-Code-Substitution" logic. . . . . . . . . . . . . ';
TX_S    = 'Size of transmitted data blocks (64-4096 bytes/block) . . . . . . ';
TX_W    = 'Number of blocks in transit before ACK is required (0-1000) . . . ';
TX_A    = 'Disable transmission of ACK for each block received . . . . . . . ';
TX_BANG = 'Force remote unit to use local option settings. . . . . . . . . . ';

TX_PG4  = 'Advanced Optional Features 2 [Page 4 of 5]';
TX_SUB4 = 'These settings control more optional features';
TX_I    = 'Idle method (0=none, 1=BIOS, 2=DesqView, 3=DDOS, 4=WIN/OS2) . . . ';
TX_NM   = 'Enable "Minimal-Blocks" logic (requires MNP). . . . . . . . . . . ';
TX_N5   = 'Disable NS16550AN buffering logic . . . . . . . . . . . . . . . . ';
TX_FT   = 'NS16550 Fifo Threshold (1-15) . . . . . . . . . . . . . . . . . . ';
TX_LF   = 'DSZLOG status logging file. . . . . . . . . ';
TX_NO   = 'Incoming file duplication check list. . . . ';
TX_FC   = 'Force CTS handshake even when CTS is initially missing. . . . . . ';
TX_NA   = 'Use Alternate XON/XOFF method during initial handshake. . . . . . ';
TX_NE   = 'Maximum number of repeated errors before aborting transfer. . . . ';

TX_PG5  = 'Screen Settings [Page 5 of 5]';
TX_SUB5 = 'These settings control the screen format and colors';
TX_NF   = 'Enable Full-Screen status display . . . . . . . . . . . . . . . . ';
TX_NV   = 'Enable direct video writing (turn off in DesqView/DoubleDOS/etc). ';
TX_CP   = 'Position of status display on screen (0-2). . . . . . . . . . . . ';
TX_CB   = 'Color of BORDER in full screen mode . . . . . . . . . . . . . . . ';
TX_CM   = 'Color of PROMPTS in full screen mode. . . . . . . . . . . . . . . ';
TX_CT   = 'Color of WINDOW TITLE in full screen mode . . . . . . . . . . . . ';
TX_CW   = 'Color of WINDOW CONTENTS in full screen mode. . . . . . . . . . . ';
TX_CG   = 'Color of PROGRESS GRAPHS in full screen mode. . . . . . . . . . . ';
TX_COLORS = ' Color Table ';

TX_CANTFIND = 'Cannot find: ';
TX_NEWCONFIG = 'New Configuration';
TX_CANTOPEN = 'Cannot open: ';
TX_OPENERROR = 'Open Error';
TX_SAVING = 'Saving configuration: ';
TX_CANTCREATE = 'Cannot create: ';
TX_CREATEERROR = 'Create Error';

TX_DEFAULT = 'All options were set to default values.';
TX_NOCONFIG = 'No configuration file needed.';
TX_EMPTY = 'Empty file ';
TX_DELETED = ' deleted.';

{$ENDIF}

{$IFDEF LANG_GERMAN}

TX_COPYRIGHT = 'HS/Link Configuration Manager v1.21·; Copyright (C) 1993 Samuel H. Smith';

TX_PGDN = ' PgDn [nÑchste Seite] ';
TX_PGUP = ' PgUp [Seite zurÅck] ';
TX_HELP = ' F1 [Hilfe] ';
TX_EXIT = ' ESC [Beenden] ';

TX_PG1  = 'Grund-Einstellungen [Seite 1 von 5]';
TX_SUB1 = 'Die wichtigsten Einstellungen fÅr den korrekten Lauf von HS/Link';
TX_P    = 'COM Port (1-8). . . . . . . . . . . . . . . ';
TX_U    = 'Verzeichnis fÅr zu empfangene Dateien . . . ';
TX_HS   = '"LANGS. HANDSHAKE" wÑhrend Disk I/O . . . . ';
TX_WELCOME = 'Willkommen!';

TX_PG2  = 'Fortgeschrittene COM-Einstellungen [Seite 2 von 5]';
TX_SUB2 = 'Einstellungen bestimmen fortgeschrittene Kommunikationseigenschaften';
TX_PB   = 'Nicht-Standard COM-Basis (in Hex, 0 = Standard COM-Port)  . . . . ';
TX_PI   = 'Nicht-Standard COM IRQ (1-15, 0 = standard COM-Port)  . . . . . . ';
TX_B    = 'COM-Port bei Geschw. îffnen (300-115200,0=automatisch)  . . . . . ';
TX_E    = 'Effektive Modem-zu-Modem Geschw. (0=gleiche als îffn.Geschw.) . . ';
TX_HC   = 'Erlaube CTS Hardware Handshake  . . . . . . . . . . . . . . . . . ';
TX_HR   = 'Erlaube RTS Hardware Handshake  . . . . . . . . . . . . . . . . . ';
TX_HX   = 'Erlaube XON/XOFF Software Handshake . . . . . . . . . . . . . . . ';
TX_C    = 'Erlaube Carrier Detect-Check  . . . . . . . . . . . . . . . . . . ';
TX_NT   = 'Behalten Original Zeit/Datum fÅr empfangene Dateien . . . . . . . ';

TX_PG3  = 'Fortgeschrittene optionale Eigenschaften 1 [Seite 3 von 5]';
TX_SUB3 = 'Diese Einstellungen bestimmen diverse optionale Eigenschaften';
TX_R    = 'Versuche abgebrochene DateiÅbertragungen wiederherzustellen . . . ';
TX_K    = 'Behalten abgebrochene DateiÅbertragungen  . . . . . . . . . . . . ';
TX_O    = 'Erlauben existierende Dateien zu Åberschreiben  . . . . . . . . . ';
TX_NB   = 'Erlauben Datei Lesen/Schreiben-Pufferung  . . . . . . . . . . . . ';
TX_NC   = 'Erlauben "Dynamische-Code-Ersetzung"-Logik  . . . . . . . . . . . ';
TX_S    = 'Grîsse der Åbertragenen Datenblîcke (64-4096 Bytes/Block) . . . . ';
TX_W    = 'Anzahl der Blîcke im verkehr bevor ACK ist benîtigt (0-1000)  . . ';
TX_A    = 'Abstellung der ACK-öbertragung fÅr jeden empfangenen Block  . . . ';
TX_BANG = 'Zwinge Gegensystem Lokale Optionseinstellungen zu brauchen. . . . ';

TX_PG4  = 'Fortgeschrittene optionale Eigenschaften 2 [Seite 4 von 5]';
TX_SUB4 = 'Diese Einstellungen bestimmen mehr optionale Eigenschaften';
TX_I    = 'Idle-Methode (0=keine,1=BIOS,2=DesqView,3=DDOS,4=WIN/OS2) . . . . ';
TX_NM   = 'Erlaube "Minimal-Blîcke"-Logik (benîtigt MNP) . . . . . . . . . . ';
TX_N5   = 'Abstellen NS16550AN Pufferungslogik . . . . . . . . . . . . . . . ';
TX_FT   = 'NS16550 Fifo Schwelle (1-15)  . . . . . . . . . . . . . . . . . . ';
TX_LF   = 'DSZLOG Status Logging-Datei . . . . . . . . ';
TX_NO   = 'Eingehende Datei Doppelte-Checklist . . . .. ';
TX_FC   = 'Zwinge CTS handshake auch wenn CTS ursprÅnglich fehlt . . . . . . ';
TX_NA   = 'Gebrauche Zweitmethode XON/XOFF wÑhrend erst-handshake. . . . . . ';
TX_NE   = 'Maximale Anzahl wiederholter Fehler vor abbruch öbertragung . . . ';

TX_PG5  = 'Fenster-Einstellungen [Seite 5 von 5]';
TX_SUB5 = 'Diese Einstellungen bestimmen das Bildschirmformat und Farben';
TX_NF   = 'Erlaube Voll-Bildschirm Statusanzeige . . . . . . . . . . . . . . ';
TX_NV   = 'Erlaube Direktes Bildschirmschreiben (abstellen fÅr DV/DD/etc)  . ';
TX_CP   = 'Position des Status-Displays auf dem Bildschirm (0-2) . . . . . . ';
TX_CB   = 'Farbe des RANDES im Vollbildschirm-Modus  . . . . . . . . . . . . ';
TX_CM   = 'Farbe des PROMPTS im Vollbildschirm-Modus . . . . . . . . . . . . ';
TX_CT   = 'Farbe des FENSTER TITELS im Vollbildschirm-Modus  . . . . . . . . ';
TX_CW   = 'Farbe des FENSTER INHALTES im Vollbildschirm-Modus  . . . . . . . ';
TX_CG   = 'Farbe des PROZESSINDIKATORS im Vollbildschirm-Modus . . . . . . . ';
TX_COLORS = ' Farbtabelle ';

TX_CANTFIND = 'Kann nicht finden: ';
TX_NEWCONFIG = 'Neue Konfiguration';
TX_CANTOPEN = 'Kann nicht îffnen: ';
TX_OPENERROR = 'ôffnen-Fehler';
TX_SAVING = 'Speichere Konfiguration: ';
TX_CANTCREATE = 'Kann nicht erstellen: ';
TX_CREATEERROR = 'Erstell-Fehler';

TX_DEFAULT = 'Alle Optionen wurden auf die Standardwerte gesetzt.';
TX_NOCONFIG = 'Keine Konfigurationsdatei benîtigt.';
TX_EMPTY = 'Leere Datei ';
TX_DELETED = ' gelîscht.';

{$ENDIF}

