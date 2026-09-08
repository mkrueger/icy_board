# Help: (QWK) Mail Packets

QWK packets let you read messages with an offline mail reader and send
replies back to the board. Enter `QWK` for the QWK command prompt, or
put a subcommand after it.

## Subcommands

- `D` Create and download a QWK packet from included message areas.
- `U` Upload a reply packet produced by your offline reader. Choose
  the transfer protocol when prompted, then send the packet using
  your terminal program.
- `S` Display the scanned message areas and change their inclusion.
- `Enter` At an empty QWK command prompt, return to the board.

Downloading or uploading ends the QWK command session. Leaving the
area selection screen returns to the QWK command prompt.

## Selecting Areas

Use the area numbers displayed by `S`. An `X` marks an included area.
These are QWK area numbers, not main-menu conference numbers.

- `[number]` Toggle inclusion of one area.
- `[first-last]` Toggle the areas in an inclusive numeric range.
- `[number]S` or `[first-last]S` Include those areas.
- `[number]D` or `[first-last]D` Exclude those areas.
- `S` Select all using the selection screen's bulk operation.
- `D` Deselect all using the selection screen's bulk operation.
- `L` Prompt for an area and a last-read message number.
- `Enter` At an empty selection prompt, leave this screen.

Separate entries with spaces. For widely spaced area numbers, use
explicit numbers or ranges: the current bulk `S` and `D` operations
can miss high-numbered areas. The `L` prompt currently changes a
different read pointer from the one used to start QWK packet creation;
do not rely on it to reset the packet's starting message.

## Reply Packets

Use a ZIP-format reply packet for this board. The importer expects a
message member named with the board's configured ID followed by
`.MSG`. Use the same board profile in your offline reader; reply area
mapping currently has limitations, so check important replies online.

## Examples

Choose areas before downloading:

```text
QWK S
```

If areas 1 through 3 and area 5 are listed, include the first three
and exclude area 5 at the selection prompt:

```text
1-3S 5D
```

Download a packet or upload your replies from the main command prompt:

```text
QWK D
QWK U
```
