about = Decompiler für die PCBoard-Programmiersprache
raw = PPE-Rohdarstellung ohne Rekonstruktion der Kontrollstrukturen
disassemble = Disassemblierung statt PPL ausgeben
output = Quelltext auf stdout statt in eine Datei schreiben; Banner, Fortschritt und Warnungen gehen an stderr
check = Laufzeitkompatibilität prüfen; Funde liefern Exit-Code 0, außer mit --strict; Lese-, Analyse- oder Berichtsfehler liefern Exit-Code 1
strict = erfordert --check; Exit-Code 1 bei jeder nicht unterstützten, nicht implementierten oder teilweise implementierten Referenz, sonst 0; Fehler liefern weiterhin Exit-Code 1
cp437 = Quelltext für die ursprünglichen Werkzeuge als CP437 statt UTF-8 schreiben
style = Schreibweise der Schlüsselwörter, gültig: u=Großbuchstaben (Standard), l=Kleinbuchstaben, c=CamelCase
lang-version = Sprachversion des Quelltexts, standardmäßig PPL_LANG_VERSION und danach die neueste Version
version = Version ausgeben und beenden
file = zu dekompilierende Datei[.ppe]