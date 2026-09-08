# Help: (8) Pack User File

Permanently remove selected user records and compact the user file.
Command 8 permission is required, and only user record 1 may run it.
IcyBoard performs this operation directly rather than invoking PCBoard's
external system manager.

## Subcommands

- `8` Start the interactive packing questions.
- `N` At the initial confirmation, cancel without packing.
- `Y` At the initial confirmation, continue to the selection questions.

These letters describe an English session. Follow the displayed Yes/No
choices in other languages.

## Selection Questions

- Keep locked-out users: defaults to Yes. This protects security-zero
  records that are not marked deleted. No removes that protection; it does
  not by itself select every security-zero record for deletion.
- Purge older than: enter a last-logon cutoff in `MMDDYYYY` order, or leave
  blank for no date cutoff. Use all eight digits for a modern year.
- Keep security: enter a level from `0` to `255` to protect users at or
  above it. Leave blank for no security threshold. A threshold of `0`
  protects every record.

Records are selected if marked deleted OR last logged on before the given
cutoff. Record 1 and the selected keep rules take precedence over removal.
Keeping locked-out users does not protect a delete-flagged record, but a
matching keep-security threshold does.

## Safety

The command refuses to start while another node has a caller state.
Arrange a maintenance window and prevent new logons; the initial check
does not reserve all nodes for the duration of the operation.

The initial confirmation defaults to No. There is no final preview or
confirmation after the selection questions. The names printed afterward
are records already removed, not candidates awaiting approval.

Before packing, IcyBoard copies the configured user file to a sibling
backup whose name has `.bak` appended. An older backup at that name is
replaced. A backup failure aborts the pack. Keep an independent backup
before starting, especially before repeated runs.

Removed records include their stored conference flags and read pointers.
Surviving records are compacted, so their record numbers can change. Review
external tools that retain record numbers. Command `7 U` cannot recover a
record after it has been physically removed; use a backup offline.

## Examples

Start without stacking confirmation or selection answers:

```text
8
```
