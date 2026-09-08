# Help: (7) User Maintenance

Browse user records and perform limited online maintenance. Command 7
permission is required. The initial record is the current user's record;
record numbers shown at the prompt start at one.

## Subcommands

- `12` Display record 12. Numbers beyond the end select the last record.
- `12+` Display record 12 and set forward browsing.
- `12-` Display record 12 and set backward browsing.
- `+` Move forward one record and keep that direction.
- `-` Move backward one record and keep that direction.
- Press Enter to move one record in the current direction. Moving past
  either end leaves maintenance.
- `F` Find the first name or alias containing the text entered next,
  ignoring letter case.
- `D` Confirm marking this record deleted and setting both normal and
  expired security levels to zero. The default answer is No.
- `U` Clear this record's deletion flag without another confirmation.
- `C` or `E` Set the expiration date. Use month/day/four-digit-year order,
  such as `12/31/2027`. An empty or all-zero answer clears expiration.
- `L` or `S` Open the full user listing; choose `V` or `P` there.
- `Q` Leave user maintenance.

## Safety and Compatibility

Changes are saved as they are made. `Q` does not undo them. Make a separate
backup before maintenance; this command does not create one automatically.

Record 1 cannot be marked deleted or undeleted here. For other records,
`U` only clears the deletion flag: it does not restore either security
level changed by `D`. Correct those levels in the administrative user
editor if the account is to regain access.

Deletion is initially a flag, not physical removal. Command `8` can remove
flagged records permanently. After packing, record numbers may change.

PCBoard offered broader record editing. IcyBoard's `C` changes expiration
only; `S` is not a search here, and the old add-user and printer actions
are not implemented in this menu. Use the administrative user editor for
other account fields. Access to this command is powerful: it does not
enforce PCBoard's per-record security-level editing restrictions.

## Examples

Open maintenance at record 12:

```text
7 12
```

Then use `F` to search, or `Q` to leave.
