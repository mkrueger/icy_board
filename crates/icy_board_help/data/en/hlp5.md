# Help: (5) List Message Headers

List message headers in the currently selected message area. Command 5
permission is required. The listing includes message and reply numbers,
recipient, sender, subject, and a status prefix. It does not enter the
message-reading loop afterward.

## Subcommands

- `5` Open the message-scan prompt.
- Enter a message number at that prompt to scan forward from that number.
- Press Enter at the scan prompt to return without scanning.

## Compatibility

PCBoard's header scan included active and inactive messages. IcyBoard has
an active/inactive column, but its current header-loading and access-filter
path skips deleted messages. Do not rely on this command to locate all
deleted records for recovery. An `A` prefix identifies an active entry.

The scan honors message access checks; command 5 permission alone does not
grant permission to read every private message. The current scan runs
forward to the end of this area. Do not rely on reverse scans, range end
limits, or the full `R` filtering language here.

If a base cannot be opened, this handler attempts to create a new base at
the same path. Investigate damaged bases from a backup before scanning;
this is not a non-writing forensic recovery tool.

## Examples

Start the scan:

```text
5
```

Then enter this at the message-scan prompt to begin at message 100:

```text
100
```
