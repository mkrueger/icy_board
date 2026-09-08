# Help: (4) Recover Deleted Message

Clear the deleted flag on a message that still exists in a JAM message
base. Command 4 permission is required. This corresponds to PCBoard's
message recovery command, not restoration from a backup.

## Subcommands

- `4` Ask for one message number.
- `4 123` Attempt to restore message 123 immediately.
- Press Enter at the number prompt to return without selecting a message.

## Description

Only a single numeric message number is accepted; ranges are not supported.
There is no additional Yes/No confirmation before the restore attempt.

Important: the current implementation uses the first configured message
area of the current conference, not the currently selected area. Verify the
conference, area, and original message number before using this command.

Recovery only works while the stored message remains present. A full pack
can permanently remove deleted content. After that, recovery requires an
appropriate backup rather than command `4`.

Do not use this as a repair tool for a damaged base. If opening the base
fails, the handler attempts to create a new base at that path and enters
message reading if creation succeeds. Back up and investigate damaged
bases offline first.

## Examples

Ask for a message number without supplying one in advance:

```text
4
```
