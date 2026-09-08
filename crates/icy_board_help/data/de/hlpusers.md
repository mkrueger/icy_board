# Hilfe: (USER) Auflisten

Dieser Befehl erlaubt es die Benutzerdatenbank nach einem Text zu filtern.
IcyBoard zeigt alle passenden Eintraege an.

## Unterbefehl

- `(text)` Suchtext

## Beschreibung

Dieser Befehl zeigt Benuternamen, Ort und letztes Einlogdatum aller
gefunden Benutzer an.

## Beispiel

Um alle User aus Berlin zu suchen, muss man das Wort 'Berlin' angeben:

```text
USER BERLIN
```

Jetzt werden alle Eintraege die 'BERLIN' enthalten aufgelistet:

```text
        User Name                  Location              Last On
-------------------------  ------------------------  ---------------
OMNIBRAIN                  BERLIN                    06-13-24 13:37
SYSOP                      BERLIN                    07-23-24 21:12
BERLIN JOE                 ZOSSEN                    12-13-23 14:21
```