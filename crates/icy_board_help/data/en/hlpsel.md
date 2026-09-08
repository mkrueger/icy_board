# Help: (SELECT) Conferences for scanning or reading

To specify which conferences you want to scan or read mail in you must select
the conference using this command.

## Subcommands

- `S` Selects ALL conferences
- `D` Deselect ALL conferences
- `#` Selects/Deselects a specific conference
- `#-#` Selects/Deselects a range of conferences

## Description

Often you want to deselect conferences you do not want to participate in.
These are skipped in all scanning operations.

## Examples

If you want to enable confonferences 3 to 7 and 14 you could enter:

```text
SELECT 3-7;14
```

To deselect the conferences you can add a D:

```text
SELECT 3-7;14;D
```
