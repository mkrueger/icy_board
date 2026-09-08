# Help: (10) Shelled DOS Function - Compatibility

PCBoard command `10` prompted for an operating-system command, ran it
through the command interpreter, and returned to the BBS afterward. It
differed from command `9`, which exited into the configured remote DOS
environment.

## Current Availability

IcyBoard does not implement built-in command `10`. It cannot be used to
execute shell commands, DOS utilities, batch files, or host administration
commands. Keeping the old security setting does not provide a shell.

## Subcommands and Alternatives

- `6` Display a trusted board-relative file, subject to file-view access.
- `16` List a board-relative directory, using the same permission as `6`.
- `OPEN` Run a configured door if the board grants access to it.
- Use a trusted host console for operating-system commands. There is no
  equivalent unrestricted shell at the IcyBoard caller prompt.

The board may also provide configured scripts or custom menu actions.
Those are separate features, not implementations of command `10`.

## Examples

List the board directory without starting a shell:

```text
16 .
```
