# Help: (D)ownload A File -also- (DB) Download Batch

Download files from the current conference to your computer. Enter `D`
to be asked for a filename, or put filenames after the command. `BD`
and `DB` request batch downloading when your access permits it.

If files are already flagged, the board first asks whether to download
them. `Y` keeps the list; `N` clears it. In batch mode, press `Enter`
at an empty filename prompt when you have finished adding files. If
the list is empty, this returns without starting a transfer.

## Subcommands

- `[filename]` Add a file by name. Matching ignores letter case.
- `*` Match any sequence of characters in a filename.
- `?` Match one character in a filename.
- `[protocol letter]` Select an enabled protocol by its letter on the
  command line. At a filename prompt, a single letter is a filename,
  not a protocol selection.
- `GB` or `BYE` Request logoff after the transfer. These work on the
  command line and at the initial filename prompts.

Separate multiple filenames with spaces or semicolons. The board
checks batch size, available time, download limits and any applicable
charges. Multiple files require a batch-capable transfer protocol.

## Before the Transfer

At the download confirmation prompt:

- `A` Abort without starting the transfer.
- `E` Edit the flagged file list.
- `G` Start the transfer and log off afterward.
- `L` List the selected files.
- `P` Change the transfer protocol.
- `Enter` Start the transfer and remain online afterward.

Within `E`, use `A` to add filenames, `R` to remove files by their
displayed numbers, `L` to list files, and `Enter` to return. Separate
removal numbers with spaces; ranges are not supported there.

Select a protocol supported by your terminal program and start its
receive function when the board sends the files. If no usable protocol
is selected, the board asks for one; `N` cancels. Local sessions use a
destination picker instead of the remote protocol prompts. Completed
files leave the flagged list; failed transfers can leave files flagged.

## Examples

Download one file:

```text
D README.ZIP
```

Select two files for a batch:

```text
DB GUIDE.ZIP TOOLS.ZIP
```

Select matching files and request logoff after the transfer:

```text
D GUIDE*.ZIP BYE
```
