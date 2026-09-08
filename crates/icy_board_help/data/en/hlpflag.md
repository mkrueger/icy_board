# Help: (FLAG) a File for Download

Flagging puts a file on your download list without transferring it.
The board searches the file directories in the current conference.
Use `D` later to download the flagged files.

## Subcommands

- `[filename]` Flag a file by name. Matching ignores letter case.
- `*` Match any sequence of characters in a filename.
- `?` Match one character in a filename.
- `Enter` At an empty filename prompt, return without adding a file.

Enter one filename or pattern per `FLAG` command. A pattern may select
several files. The board displays a number, filename and size for each
file added. Files already flagged are not added twice. Missing files,
the batch limit and insufficient account funds can prevent additions.

To review or remove flagged files, enter `D`, keep the flagged list,
and finish any filename prompts. At the remote download confirmation,
`L` lists files and `E` edits the list. Within `E`, use `R` and the
displayed file numbers to remove entries.

## Examples

Flag a single file:

```text
FLAG GUIDE.ZIP
```

Flag ZIP files whose names begin with GUIDE:

```text
FLAG GUIDE*.ZIP
```

Start downloading the files you have flagged:

```text
D
```
