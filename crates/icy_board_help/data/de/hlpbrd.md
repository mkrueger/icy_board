# Hilfe: (BR)oadcast Nachricht an Node

Dieser Befehl sendet eine kurze Nachricht an einen oder alle Nodes.

## Unterkommandos

- `(node #)` Node Nummer oder ALL fuer alle Nodes.
- `(text)` Der Text zum Senden

## Beschreibung

Sendet eine Nachricht an eine oder alle Nodes. Dieser Befehl ist nuetzlich,
um Benutzer zu informieren, dass das System runterfaehrt.

Wenn die Nachricht angezeigt wird, wird auch ein Beep gesendet.

Achtung:
Die Nachricht wird nicht gesehen, wenn ein DOOR ausgefuehrt wird.

## Beispiel

Um eine Nachricht an Node 1 zu senden:

```text
BR 1 PLEASE LOG OFF ASAP
```

Diese Nachricht wird an alle Nodes gesendet mit •ALL":

```text
BR ALL PLEASE LOG OFF ASAP
```