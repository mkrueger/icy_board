# Help: (U)pload File(s)

Upload sends files from your computer to the current conference.
Enter `U` to be asked for a filename, or put filenames after the
command. `BU` and `UB` request batch uploading when permitted.

If downloads are flagged, the board first asks whether to continue.
The default is `N`, which cancels the upload and keeps your flags.
Answering `Y` continues and clears the flagged download list.

## Subcommands

- `[filename]` Name a file to upload. Use the filename only, without
  a directory path or wildcards. Existing filenames are rejected.
- `[protocol letter]` Select an enabled protocol by its letter on
  the command line. Use one supported by your terminal program.
- `GB` or `BYE` On the command line, request logoff after uploading.
- `Enter` At an empty filename prompt, finish choosing files. If
  none have been accepted, return without starting a transfer.

On the command line, separate multiple filenames with spaces or
semicolons. At an interactive filename prompt, enter one name only;
protocol letters and `GB`/`BYE` are not options at that prompt.
Batch availability and the number of files allowed depend on the board.

## Descriptions

The board asks for a description before transferring each named file.
Use up to 45 characters per line and at least five characters on the
first line. The board sets the maximum number of description lines.

- `/` or `\` At the beginning of the first description line, request
  private screening instead of ordinary public posting.
- `Enter` On a blank first description line, abandon this file before
  transfer. On a later blank line, finish the description.

Follow any further filename prompts, then start your terminal program's
send function when the board is ready. Extra files accepted in a batch
may need descriptions after transfer if no embedded description is
found. Upload processing or sysop review may delay public availability.

## Batch Confirmation

At the remote batch upload confirmation prompt:

- `G` Continue and log off after the upload.
- `A` Abort before starting the transfer.
- `P` Change the transfer protocol.
- `Enter` Continue and remain online afterward.

If the board asks for a usable upload protocol, `N` cancels. Local
sessions choose source files with a picker instead of accepting local
paths from the command line, and do not use remote protocol prompts.

## Examples

Upload one file and answer the description prompts:

```text
U GUIDE.ZIP
```

Request a batch upload of two files:

```text
BU GUIDE.ZIP TOOLS.ZIP
```

Request logoff after uploading:

```text
U GUIDE.ZIP BYE
```
