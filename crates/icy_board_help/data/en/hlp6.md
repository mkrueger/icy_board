# Help: (6) View Text File

Display a board file using IcyBoard's normal display-file handling.
Command 6 permission is required. PCBoard allowed a sysop to view host
files; IcyBoard restricts the path syntax accepted by this command.

## Subcommands

- `6` Ask for a relative file name.
- `6 art/welcome` Display that board-relative file if it exists.
- Press Enter at the file-name prompt to cancel.

## File Names

Use a relative path from the board directory. Absolute paths and parent
directory components such as `../` are rejected. Ordinary subdirectories
and `./` are accepted. Use forward slashes, and do not supply DOS drive
names, shell commands, wildcards, or command-line switches.

The interactive file-name prompt allows 30 characters and uppercases typed
input. On a case-sensitive host, supplying the correctly cased path with
the command avoids that prompt conversion. Avoid spaces in stacked paths.

This uses the normal board display machinery, not a raw file dump. Display
formatting and normal file selection rules apply. Only display trusted
board content.

The restriction checks path components, not resolved symbolic-link
targets. It is not a filesystem sandbox. Administrators must control board
directory links and host file permissions.

## Examples

If the board has a welcome display at this relative path:

```text
6 art/welcome
```
