# Help: (2) View User File

List the board's user records for administration. Command 2 permission is
required. This is the full user-file listing, not the public `USERS` search.

## Subcommands

- `V` View the user-file listing on screen.
- `P` Use the legacy print selection, also displayed on screen.
- Press Enter at the selection prompt to return.

## Description

Each line contains the user's name, city or state, security level, and last
logon date and time. Records marked deleted or disabled are not excluded.
Treat this information as private administrative data.

PCBoard could send this listing to a printer. IcyBoard has no printer output
here. Although `P` requests nonstop output initially, the listing enables
line counting again, so pauses may still occur.

This command does not change records. Use command `7` for the available
online maintenance actions.

## Examples

```text
2 V
```
