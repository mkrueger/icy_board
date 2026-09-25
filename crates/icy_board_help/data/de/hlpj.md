# Hilfe: (J) Konferenz betreten

Konferenzen separieren Informationen, Nachrichten und Dateien. Konferenzen
teilen diese in Kategorien, um die Suche zu vereinfachen.

## Subkommandos

- `[conf. name]` Konferenzname. Spezifiziert den Konferenznamen
- `[conf. num.]` Konferenznummer. Spezifiziert die Konferenznummer
- `Q` Schnellbeitritt. Wird nach der Konferenznummer oder dem Namen
  angegeben, zB. `J 13 Q` oder `J;13;Q`. News- und Intro-Dateien werden
  übersprungen, außer das System zeigt News immer an oder erzwingt das Intro.
- `S` Suche nach Namen. Filtert den Konferenznamen nach Muster
  es werden nur passende Konferenzen angezeigt.

## Beschreibung

Konferenzen sind ein Weg wie Nachrichten & Files in Areas aufgeteilt werden
Jede Konferenz hat eigene Datei- & Nachrichtenbereiche, Bullettins, News,
Umfragen und Doors oder kann diese mit anderen Konferenzen teilen.

## Beispiele

Um die Konferenz mit Nummer 13 zu betreten, kann man diesen Befehl eingeben

```text
J 13
```

Wenn man den Konferenznamen kennt, kann man diesen verwenden.
zB. 'BBS' betreten:

```text
J BBS
```

Wenn man nach einem Teilnamen suchen will zb. nach 'GAMES' kann man
diesen Befehl eingeben, der alle Konferenzen mit 'GAMES' im Namen listet:

```text
J S GAMES
```

IcyBoard sucht nach passenden Konferenzen und zeigt zb. das hier an:

```text
   12) Amiga Games
   42) C64 Games
    1) PC Games
```

Mit der Nummer kann die Konferenz betreten werden.