# Help: Set Conference Registrations

## Subcommands

- `S` Selects ALL conferences
- `D` Deselect ALL conferences
- `#` Selects a specific conference
- `#-#` Selects a range of conferences

## Description

Last question when modifing user records is 'Select Conferences' that
brings up up a conference registration screen

After selecting the conference(s) to act upon the board wil ask for flags
you whish to use for that conference.

### Registration Flags

- `R` User is registered if subscription isn't expired
- `X` User is registered regardless of expiration
- `L` User is locked out of that conference
- `S` User chose to scan this conference
- `C` User is temporary Sysop in this conference

Note that PUBLIC conferences that the user can access 'R' and 'X' do
not matter unless the 'L' flag is turned on which forces a lock out.

## Examples

To register a user in a conference and mark the conference for scanning:

```text
RS
```

To register a user even if they are expired enter the following:

```text
RX
```
