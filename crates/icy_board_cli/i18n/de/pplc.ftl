about = Compiler für die PCBoard-Programmiersprache
disassemble = Disassemblierung ausgeben, statt zu kompilieren
nowarnings = keine Warnungen ausgeben
version = Version ausgeben und beenden
mono = Klartext ohne ANSI-Escapesequenzen zur farbigen Ausgabe schreiben
runtime = Versionsnummer der kompilierten PPE-Datei, gültig: 100, 200, 300, 310, 320, 330, 340, 400 (Standard)
lang-version = Sprachversion (Standard: Manifest, PPL_LANG_VERSION, dann Laufzeitversion bis höchstens 400)
compression = PPE-400-Sektionskompression: none (Standard) oder zstd
debug = optionale Quellsymbolnamen in PPE-400-Debugdaten aufnehmen
cp437 = Dateikodierung angeben (cp437 = true, utf8 = false), standardmäßig automatisch erkennen
init = neues PPL-Paket im Zielverzeichnis erstellen und initialisieren
defines = durch Semikolon getrennte Liste von Präprozessorvariablen
format = Quelldatei formatieren, statt zu kompilieren
stdout = mit --format das Ergebnis auf stdout ausgeben und die Datei unverändert lassen
check = Quelldatei oder Paket auf Fehler prüfen, ohne zu kompilieren
print-config = wirksame Compilerkonfiguration ausgeben, ohne zu kompilieren
print-config-json = wirksame Compilerkonfiguration als JSON ausgeben, ohne zu kompilieren
file = zu kompilierende Datei[.pps] (ohne Angabe einer Erweiterung wird .pps verwendet)