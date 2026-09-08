# Help: (Y)our Mail Scan

Use `Y` to locate mail to and from your name or alias
without reading message bodies. Choose options after the
command or at its scan prompt. A blank prompt answer
leaves without scanning.

By default, the scan covers the current conference and
uses the board's preferred quick or long display format.
An existing personal e-mail base is also scanned and
reported separately, even with a current-conference scan.

## Scope Subcommands

- `C` Scan the current conference.
- `A` Scan selected conferences you may scan, also
  including the current conference.
- `ALL` Include conferences not selected for scanning,
  subject to the scan's registration checks.
- `W` Scan conferences flagged as having mail waiting.
  This also selects messages after each area's last-read
  mark, forward order, and omission of empty results.

The conference scan combines results from its message
areas. Numbers in a long listing are local to their area;
choose the corresponding area before reading a number.

## Selection and Display Subcommands

- `S` or `*` Scan after each area's saved last-read number
  and set forward order.
- `+` List numbers in forward order.
- `-` List numbers in reverse order, the initial default.
- `C+` or `C-` Select the current conference and direction.
- `Q` Show a compact line of counts per conference.
- `L` List message numbers to you and from you, with a
  total count. A `+` after a number means marked read.
- `Z` Omit results whose total found is zero. With `W`,
  also omit results with no messages addressed to you.

Separate options with spaces.
The `+` read marker reflects recipient-read status, not
whether a message lies before your last-read position.

## Understanding the Counts

The quick display counts messages addressed to you and
the total found. The total includes public messages and
other mail visible to this scan, not just messages to or
from you. Mail addressed to `ALL` contributes to the total
but not to your personal count. `Z` alone therefore does
not restrict the display to conferences with personal mail.

Without `S` or `W`, old messages and messages already
marked read are included. Scanning does not mark messages
read or move your last-read position.

## Examples

```text
Y C L
Y A S Q
Y ALL S - L
Y W Q
```

These list current-conference mail numbers, summarize
newer mail in selected conferences, list newer mail in
reverse order across conferences, and summarize mail in
conferences flagged as having mail waiting.
