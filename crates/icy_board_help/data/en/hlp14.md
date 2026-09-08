# Help: (14) Drop Another Node to DOS - Compatibility

PCBoard command `14` instructed another network node to leave the BBS for
DOS. With a caller present, its immediate/deferred choice allowed dropping
the node now or waiting until the session ended. This changed the node's
operating state, not just the caller's login state.

## Current Availability

IcyBoard does not implement built-in command `14`. Neither immediate nor
deferred DOS exit is available through this number. The legacy security
configuration entry is not an implementation of remote host control.

## Subcommands and Alternatives

- `11` Inspect the current node list.
- `12` Request forced logoff of another caller, if authorized. This has
  no additional confirmation once a valid node is chosen and is not a DOS
  exit or a way to take the server offline.
- Use the host's trusted service-management controls for maintenance or
  shutdown, after notifying callers and arranging downtime.

There are no supported DOS-exit subcommands here. Do not use command `12`
as though it reserves a node against subsequent logons.

## Examples

Inspect nodes without changing their state:

```text
11
```
