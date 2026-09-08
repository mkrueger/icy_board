# Hilfe: (M)odus

Dieser Befehl erlaubt es den Grafikmodus zu wechseln zwischen kein ANSI,
ANSI, RIP, AVATAR und GrafikMode

## Subkommandos

- `CTTY` Kein ANSI. Weder Farben noch ANSI Steurcodes. Nuur ASCII
  Höchste Kompatibilität.
- `ANSI` ANSI Steuercodes. Benutzt ANSI Steuercodes, aber keine
  Farben. Schnellerer Bildschirmaufbau bei langsamen g
  Verbindungen.
- `GRAPH` ANSI Farben. Benutzt ANSI Farb- und Steuercodes. Dieser
  Modus ist der Standard.
- `AVATAR` AVATAR. Wie GRAPH nur werden AVATAR Codes benutzt.
- `RIP` RIPscrip Grafikmodus. Wenn RIPscrip Displayfiles vorhanden
  sind werden diese benutzt.

## Beschreibung

Ohne Subkommandos wechselt dieser Befehl zwischen ANSI Farben und kein ANSI
Mit Subkommando wird der entsprechende Modus aktiviert. Falls auf dem
Schirm unlesbare Codes auftauchen, einfach 'M' nochmal ausführen, um in den
kein ANSI Modus zu wechseln.

## Über RIPscrip/AVATAR

Terminals verwenden üblicherweise ANSI. RIPscrip/Avatar werden nur von
wenigen Terminals (zb. Icy Term) unterstützt. Bitte sicherstellen, dass
dein Terminal diese unterstützt. Falls nicht, werden unlesbare Zeichen auf
dem Bildschirm erscheinen.

## Beispiel

Um den AVATAR Modus zu aktivieren, kann dieser Befehl verwendet werden:

```text
M AVT
```