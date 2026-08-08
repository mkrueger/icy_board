# icbfile

`icbfile` is the maintenance tool for Icy Board file areas. It creates and repairs the
database behind an area, imports descriptions from an existing PCBoard board, and lets you
edit entries without starting the BBS.

## How file handling works

### Areas

A conference points at a list of file areas through its `dir_file` setting. That file is a
TOML document with one `[[area]]` block per area:

```toml
[[area]]
name = "General Files"
path = "uploads"
metadata_path = "uploads/dir"
password = ""
sort_order = "FileName"
sort_direction = "Ascending"
```

Two paths matter, and they are deliberately separate:

* `path` is the directory the files actually live in. This is what users download from.
* `metadata_path` names the area's bookkeeping. Icy Board derives the database location
  from it, so the two can sit on different volumes.

Keeping them apart is what makes read-only areas possible: point `path` at a mounted
CD-ROM image and `metadata_path` at writable storage, and the area works normally.

### The database

The database is placed in a `.icy` directory beside `metadata_path`, named after it:

```
uploads/
  ALLFILES.ZIP
  CALGUIDE.ZIP
  RULES.TXT
  .icy/
    dir.db
    dir.db-wal        while a node has the area open
    dir.db-shm
```

It is a plain SQLite database. Nothing needs to be installed — SQLite is compiled into
Icy Board. It runs in WAL mode so several nodes can read an area while one of them writes.

`.icy` exists so that nothing in it can ever collide with a file a user uploads, and so
the download directory contains only downloadable files.

### What is authoritative

The **directory is the source of truth for which files exist.** Every time an area is
opened, new files found in it are added to the database automatically. You do not have to
tell Icy Board about a file you copied in.

Entries whose file has disappeared are *kept*, not deleted, so that a temporarily offline
volume does not throw away descriptions and counters. Use `icbfile check --prune` to
remove them for good.

The **database is the source of truth for descriptions.** There is no `FILES.BBS` or `DIR`
text file being read at runtime.

### Where descriptions come from

A description is either *derived* or *authored*.

Derived descriptions are pulled out of the file itself the first time anything asks for
one. For archives, `icbfile` looks inside for a description member and takes the best one
it finds, in this order of preference:

1. `FILE_ID.PCB`
2. `FILE_ID.ANS`
3. `FILE_ID.DIZ`
4. `DESC.SDI`

For `.ANS`, `.NFO`, `.TXT`, `.XB`, `.PCB` and `.ASC` files the SAUCE record is read
instead. `.EXE`, `.COM`, `.BAT`, `.BMP`, `.GIF` and `.JPG` are left alone.

These archive formats are understood:

```
7z  ace  arc  arj  bz2  gz  ha  hyp  ice  lha/lzh  pi9  qqq  rar  sq/sq2  sqz
tar  tar.bz2  tar.gz  tar.Z  tbz  tgz  uc2/ue2  Z  zip  zoo
```

Authored descriptions are the ones you supply — through `icbfile import` or
`icbfile set --desc`. They are marked, and a later scan will **not** overwrite them. Only
`icbfile scan --force` discards them.

Alongside the description, a hash of each file is stored. The BBS `T` command uses it to
tell a user whether a file has changed since it was scanned.

## Addressing an area

Every command takes a target, which is one of:

* **a directory** — used directly, with the database in `<dir>/.icy/dir.db`.
  Convenient for a quick look at a directory that is not wired into a board yet.
* **a `file_areas.toml` plus `--area`** — resolves `path` and `metadata_path` exactly the
  way the BBS does, so the database ends up where the board will look for it.
  `--area` takes either the area name (case-insensitive) or its index.

Use the second form for anything on a live board. Use `icbfile areas` to see the indices:

```
$ icbfile areas config/file_areas.toml
  0  General Files                       uploads
  1  Utilities                           utils
```

## Commands

```
icbfile areas  <file_areas.toml>
icbfile list   <target> [-a AREA] [-l]
icbfile scan   <target> [-a AREA] [--force]
icbfile check  <target> [-a AREA] [--prune]
icbfile import <target> <listing>... [-a AREA] [-f FORMAT] [-n] [--overwrite] [--keep-missing]
icbfile export <target> [-a AREA] [-o FILE]
icbfile set    <target> <file> [-a AREA] [--desc TEXT] [--free BOOL] [--locked BOOL]
```

Run any command with `--help` for the full option list.

## Common scenarios

### Creating a new area

There is no `create` command, because there is nothing to create. Make the directory, copy
files into it, and point an `[[area]]` at it. The database appears on first use:

```sh
mkdir -p uploads
cp *.zip uploads/
icbfile list uploads
```

The first `list` picks up the files, builds the database, and shows whatever descriptions
could be derived from the archives.

### Importing an existing PCBoard board

PCBoard keeps its file listings as text files, usually `DIR01`, `DIR02` … in the
conference's directory. Those descriptions are hand-curated and worth keeping.

First look at what would happen. `--dry-run` writes nothing:

```sh
icbfile import config/file_areas.toml ~/pcb/GEN/MAINCONF/DIR01 --area 0 --dry-run
```

```
/home/you/pcb/GEN/MAINCONF/DIR01: 3 entries
  would set ALLFILES.ZIP: A listing of all files available on this
  would set CALGUIDE.ZIP: An ASCII version of the Caller's Guide for
  would set RULES.TXT: ASCII file listing the rules of this bulletin

dry run: 3 description(s) would be set, 0 kept, 0 not in the directory
```

Then do it for real by dropping `--dry-run`. Each PCBoard DIR file corresponds to one
area, so repeat with the matching `--area` for each.

The parser handles the usual PCBoard layout: name, size, `MM-DD-YY` date and text in fixed
columns, with wrapped lines marked `|`. Two-digit years pivot at 80, so `94` is 1994 and
`05` is 2005. A `*` in the size column marks a free file. An entry whose size column reads
`OFFLINE` keeps its description.

Input is decoded as CP437, so box drawing and accented characters survive.

### Importing a FILES.BBS

Same command — the format is detected automatically:

```sh
icbfile import uploads FILES.BBS
```

Force it with `-f filesbbs` or `-f pcboard` if detection guesses wrong.

### Entries whose file is not there

By default, a listing entry with no matching file is reported and skipped:

```
  skipped, not in the directory: ALLFILES.ZIP
```

That is usually what you want. If you are importing descriptions before the files are in
place, `--keep-missing` records the entry anyway, taking the size and date from the
listing. `icbfile list -l` marks those entries `MISSING` until the file appears.

### Re-importing

An import will not overwrite a description that was imported or edited earlier; those are
counted as "kept". Pass `--overwrite` to replace them.

### After adding files by hand

Nothing is needed. Opening the area picks up new files and derives their descriptions on
first read.

To force the work to happen now, rather than when the first user lists the area:

```sh
icbfile scan uploads
```

`scan` keeps authored descriptions. Use `--force` only when you want to throw away
everything you imported or edited and go back to what the archives say.

### Editing a single entry

```sh
icbfile set uploads RULES.TXT --desc "Board rules, please read before posting"
icbfile set uploads RULES.TXT --free true
```

A description set this way is authored, so scans leave it alone.

### Checking an area

```sh
icbfile check config/file_areas.toml --area 0
```

Reports entries whose file has gone and entries whose size no longer matches what was
recorded. Add `--prune` to drop the missing ones.

### Exporting

```sh
icbfile export uploads -o FILES.BBS
```

Writes the descriptions back out as a CP437 `FILES.BBS`. Without `-o` it goes to stdout,
which is handy for a quick look or for piping somewhere else.

### Backing up an area

Copying `dir.db` while a node is running is not safe, because recent writes may still be
in the `-wal` file. Use SQLite's own snapshot instead:

```sh
sqlite3 uploads/.icy/dir.db "VACUUM INTO '/backup/dir.db'"
```

That is consistent and needs no downtime. You can also inspect an area with the same tool:

```sh
sqlite3 uploads/.icy/dir.db "SELECT name, size FROM files ORDER BY name"
```

The `sqlite3` command line tool is not required to run Icy Board and is not installed with
it; it is packaged separately on every distribution if you want it. Stopping the board and
copying `dir.db` together with its `-wal` and `-shm` files works too.

## Current limitations

* **Download counters are not maintained.** The database stores `dl_counter` and `list -l`
  shows it, but nothing in the BBS increments it yet, so it stays at zero.
* **File flags are stored but unused.** `--free` and `--locked` are recorded and survive,
  but no command handler checks them yet.
* **Descriptions are not full-text indexed.** The `Z` scan searches the descriptions the
  area has loaded, which is fine at normal area sizes.
* **Only one description per file.** PCBoard's separate long description is not modelled.
