# Help: (9) Exit to DOS - Compatibility

PCBoard command `9` left the current node for a configured remote DOS
environment. It asked for confirmation and depended on the legacy remote
batch-file setup. This was an administrative host-control function, not
ordinary caller logoff.

## Current Availability

IcyBoard does not implement built-in command `9`. There is no remote DOS
prompt or host shell behind this number. Retained compatibility security
settings do not enable the missing function. A custom menu can assign its
own action, but that is a board-specific extension.

## Subcommands and Alternatives

- `G` End the current caller session normally; it does not stop the server
  or enter DOS.
- `OPEN` Select a configured door, if allowed. A DOS door is a separately
  configured application, not general access to the host operating system.
- Use the host's trusted administrative console and service controls for
  server maintenance. They are outside the BBS command prompt.

## Examples

To log off rather than request the unavailable DOS function:

```text
G
```
