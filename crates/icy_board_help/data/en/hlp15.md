# Help: (15) Recycle Another Node - Compatibility

PCBoard command `15` requested that another network node recycle through
DOS and its BBS startup sequence. It was a node-restart operation, distinct
from logging off a caller or dropping a node to DOS for maintenance.

## Current Availability

IcyBoard does not implement built-in command `15`. No node recycle or
server restart is performed by this number. A custom menu may provide a
separate administrative action, but that is not built-in compatibility.

## Subcommands and Alternatives

- `11` View node status before or after maintenance.
- `12` Request another caller's logoff, if authorized. It does not restart
  the node process or the server, and it has no extra confirmation after
  a valid node is selected.
- Use the trusted host console or service manager to restart IcyBoard.
  Notify callers and plan downtime first; a server restart may affect
  all sessions rather than one selected node.

There are no supported recycle options at the BBS prompt. Do not assume
that a successful caller logoff has restarted any service.

## Examples

Check the node list without requesting a disconnect:

```text
11
```
