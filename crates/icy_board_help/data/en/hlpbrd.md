# Help: (BR)oadcast Another Node

This command sends a brief message to all or a single node.

## Subcommands

- `(node #)` Node number. This can be any active node number or
  ALL for broadcasting the message to all nodes.
- `(message)` Message to send.

## Description

Allows to display a messag any or all active nodes. This command is useful
to notify users that the system may be going down.

When the message is displayed to the user it will also beep to get the
attention.

Note:
If a user is not in IcyBoard (running a DOOR) the message won't be seen.

## Examples

To broadcast a message to node 1 which says PLEASE LOG OFF ASAP type:

```text
BR 1 PLEASE LOG OFF ASAP
```

To broadcast this to all active nodes use ALL:

```text
BR ALL PLEASE LOG OFF ASAP
```
