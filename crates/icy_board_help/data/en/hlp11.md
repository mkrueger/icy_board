# Help: (11) View Nodes

Display the administrative node list. Command 11 permission is required.
This corresponds to PCBoard's expanded node display, including nodes
without a caller.

## Subcommands

- `11` List all configured node slots. No additional option is required.

## Description

Node numbers start at one. Occupied slots show the current operation and,
when a user record is available, the user's name. The city or state is
included only if the board's node-display configuration enables it.

Slots without a caller state are shown as Available. This is a snapshot,
not continuous monitoring, and Available does not establish that an
external listener or host service is healthy.

Use this list to identify node numbers before commands `12` or `13`.
Viewing the list does not disconnect, restart, or change a node.

## Examples

```text
11
```
