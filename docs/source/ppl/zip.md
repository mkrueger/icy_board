# Writing ZIP archives

PPL language/runtime **4.00** provides [Zip](api/zip.md) and
[ZipWriter](api/zipwriter.md) for writing archives. For reading ZIP and other
formats, use [Archive](api/archive.md) instead.

## Create, add, finish

`Zip.Create(path [, overwrite])` creates a temporary archive alongside its
destination. `overwrite` defaults to `FALSE`; replacing an existing file
requires `TRUE`. The destination is not published until `Finish()` succeeds.
Check the writer's `Valid` property before using it.

```PPL
;$LANGVERSION 400
ZIPWRITER writer
ERROR failure

writer = Zip.Create("export.zip")
IF !writer.Valid THEN
    PRINTLN Error.Last().Message
    EXIT
ENDIF

IF !writer.AddFile("readme.txt", "docs/readme.txt") THEN
    failure = Error.Last()
    writer.Abort()
    PRINTLN failure.Message
    EXIT
ENDIF

IF !writer.Finish() PRINTLN Error.Last().Message
```

Compile with `pplc --lang-version 400 --runtime 400 example.pps`. Running this
example needs a readable `readme.txt` and a writable destination directory in
the PPE's file context. It creates `export.zip`; it is not a read-only example.

`AddFile` defaults its archive entry name to the source's filename.
`AddDirectory` adds a directory entry, while `AddBytes` writes an in-memory
`BYTES` value. `AddTree` defaults to an empty prefix and recursive traversal.
Place the destination outside the source tree.

Archive entry names are relative paths using `/`. Absolute paths, `.`/`..`
components, backslashes, drive separators, duplicate names and file/directory
collisions are rejected. Source files must be regular files; this is not an API
for archiving arbitrary device files or following user-created symbolic links.

## Options and defaults

| Setting | Default | Contract |
| --- | --- | --- |
| Compression | `ZipMethod.Deflate`, level `6` | `SetCompression` changes subsequent entries. Deflate accepts levels `0` through `9`; `Stored` must omit the level. |
| ZIP64 | `Zip64Mode.Auto` | Call `SetZip64` before adding the first entry. `Never` rejects sizes or entry counts needing ZIP64. |
| Comment encoding | `ZipEncoding.Utf8` | `SetComment` also accepts CP437; unrepresentable text is rejected. Encoded comments may not exceed 65,535 bytes. |
| Timestamp override | None | File additions use available source modification times; otherwise the writer uses the current time. `SetTimestamp()` clears the override. |
| Permissions override | None | Default Unix modes are 0755 for directories and 0644 for files. `SetPermissions(mode)` accepts decimal `0` through `511`; omit the argument to reset it. |

`SetTimestamp(date, time)` requires both nonempty native values, or no arguments
to reset the override. Although the signature marks both arguments optional,
passing only one is invalid. `SetTimestampUtc(timestamp)` uses UTC components;
an empty timestamp clears the override. ZIP timestamps support years
1980 through 2107 and two-second precision: odd seconds and subseconds are
discarded. Invalid options leave the previous setting unchanged.

The writer limits an archive to 100,000 entries. Disabling ZIP64 imposes the
additional classic ZIP size/count limits. These limits do not make archive
creation a general CPU, storage or permissions sandbox.

## Failure and ownership

Methods return `BOOLEAN` and report failures through [Error.Last](api/error.md#last).
An entry-addition failure aborts the temporary archive and invalidates the
writer; do not attempt to continue adding entries. `Abort()` discards pending
output without publishing it. `Finish()` closes the writer and publishes the
archive only on success.

Save the error before calling cleanup if you need to report it afterward: a
successful cleanup operation can clear the published error. Copies of a writer
refer to the same underlying archive, not independently writable copies.

The implementation contract is in the
[ZIP runtime](../../../crates/icy_board_engine/src/icy_board/state/ppl_zip.rs);
the generated member pages list every available operation and its exact types.
