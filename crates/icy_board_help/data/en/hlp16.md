# Help: (16) List Directory

List entries in a directory relative to the board directory. This sysop
command uses the same access permission as command `6`. It is a filesystem
listing, not the public file-directory catalog displayed by `F`.

## Subcommands

- `16` Ask for a directory path.
- `16 .` List the board directory.
- `16 art` List the board's art directory if it exists.
- Press Enter at the directory prompt to cancel.

## File Paths

Supply a directory, not a file mask or a complete DOS command. Relative
subdirectories and `./` are accepted. Absolute paths and parent directory
components such as `../` are rejected. Use forward slashes and avoid
spaces in stacked paths. The interactive path prompt allows 30 characters.

PCBoard accepted DOS drive paths and wildcard file specifications here.
IcyBoard does not expand `*` or `?`, interpret DOS drives, recurse through
subdirectories, or execute shell commands or switches.

The path restriction checks components rather than resolved symbolic-link
targets. It is not a filesystem sandbox; administrators must control links
inside the board directory and the host permissions of the server account.

## Display

Directories appear first, followed by files, with case-insensitive name
sorting within each group. Lines show the name, byte size or `<DIR>`, and
modification date and time when available.

The final count includes directory entries despite its file-count label;
the byte total includes files only. An empty or unreadable directory can
produce the same No Files Found response. Listing does not modify files.

## Examples

```text
16 .
```
