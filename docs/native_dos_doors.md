# Native DOS doors

Icy Board can run classic DOS doors in-process through the native Rust
[`x86-native`](https://github.com/mkrueger/x86) library. The runtime does not
invoke QEMU, DOSBox, DOSEMU, Node.js, a browser, or WebAssembly.

The design follows ENiGMA BBS's v86 door integration:

1. Icy Board creates the configured drop file.
2. The configured host door directory is copied to `C:\DOOR` when its
   persistent image is first created.
3. The drop file and a door-specific `RUN.BAT` are written to `C:\ICB` in the
   per-door FreeDOS image.
4. SeaBIOS boots FreeDOS from the image.
5. `FDAUTO.BAT` calls `C:\ICB\RUN.BAT`, which changes to `C:\DOOR` first.
6. COM1 is bridged to the caller with DCD, DSR, and CTS asserted.
7. Icy Board's `C:\ICB\POWEROFF.COM` signals ACPI shutdown and the modified
   image is saved.

The current native x86 core uses process-global machine state. Icy Board
therefore serializes DOS sessions. This is safe for persistent game data but
means only one native DOS door can run at a time until x86-native supports
isolated machine instances.

## Prepare FreeDOS

The first DOS-door launch automatically downloads and prepares any missing
FreeDOS and BIOS assets. The caller sees a preparation message; the first
launch needs Internet access and can take a few minutes. Subsequent launches
use the installed files without downloading them again.

To prepare these files ahead of time, run:

```bash
icbsetup dos-image /path/to/board
```

Both paths download and verify pinned SHA-256 checksums for:

- FreeDOS 1.4 LiteUSB from the official FreeDOS site
- SeaBIOS and the VGA BIOS used by upstream v86

It creates:

```text
assets/dos/freedos.img
assets/dos/seabios.bin
assets/dos/vgabios.bin
```

New files are staged before publication. Existing files, including customized
base images, are never replaced. Failed downloads can be retried by launching
the door again or running `dos-image`; the error details are logged. Existing
files are trusted, so damaged assets require manual repair or removal while
the board is stopped.

The new base image is patched to call `C:\ICB\RUN.BAT` and then execute the
small ACPI shutdown helper injected by Icy Board. The image remains a normal
MBR/FAT16 raw disk and can also be opened by standard image tools. Door game
files and licensed third-party drivers are not downloaded automatically.

## Install a door

Install the game's files in a host directory such as `doors/lord`, matching
the door's `path` setting. Icy Board imports that directory into `C:\DOOR`
when it first creates the per-door image. To update an existing image without
mounting it, stop the door and use, for example:

```bash
icbsetup dos-copy assets/dos/doors/lord.img LORD.EXE DOOR/LORD.EXE
```

### Optional FOSSIL driver

Doors that support direct serial I/O, including the tested LORD 4.06 setup,
can use COM1 without a FOSSIL driver. Install one only when your door needs it.
FreeDOS Lite does not include X00, and Icy Board does not redistribute it.

For X00, obtain the complete original ZIP distribution and read its license.
Only proceed if the terms permit your intended use. Stop Icy Board, then run:

```bash
icbsetup dos-fossil /path/to/board /path/to/x00153a.zip --accept-license
```

This prepares missing base assets and installs X00 into the **base image**
for future doors. For a door whose persistent image already exists, use:

```bash
icbsetup dos-fossil /path/to/board /path/to/x00153a.zip --door LORD --accept-license
```

The command takes the board lock and stages changes in a temporary image. It
keeps the original image as `<image>.pre-fossil.bak`, preserves the existing
configuration, copies the archive's files (including license and documentation)
into `C:\FOSSIL`, and appends this boot entry to `C:\FDCONFIG.SYS`:

```dos
DEVICE=C:\FOSSIL\X00.SYS E B,0,57600
```

Select **FOSSIL**, **COM1**, and a locked baud rate of **57600** in the game's
own setup. X00 numbers COM1 as port zero. The helper supports flat X00 ZIP
distributions; it does not configure arbitrary FOSSIL drivers. Existing
FOSSIL directories, driver references, or backup files cause a refusal rather
than being overwritten. Restore the backup while the board is stopped to undo
the installation. X00 1.53a initialization and serial output have been tested
with the native emulator; compatibility with every door is not implied.

Changes to the base image do not update existing per-door images. Installing
into one door image does not affect other doors. Do not redistribute an image
containing X00 without checking its distribution terms.

Other drivers still require their documented installation procedure, using a
DOS-aware image tool or `dos-copy`. `X00.SYS` is loaded as a device driver by
the helper; do not invoke that filename as a command in `START.BAT`.

## Configure a door

A door entry in the conference door TOML can use:

```toml
[[door]]
name = "LORD"
description = "Legend of the Red Dragon"
password = ""
securiy_level = "10"
door_type = "Dos"
path = "doors/lord"
use_shell_execute = false
drop_file = "DoorSys"
dos_memory_mb = 64
dos_command = '''
@ECHO OFF
CALL START.BAT {node}
'''
```

`START.BAT` must be configured for the game's launch syntax, selected drop
file, and COM1. For `PCBoard`, Icy Board supplies both `PCBOARD.SYS` and
`USERS.SYS`; `{dropFile}` selects `PCBOARD.SYS`. The literal TOML string also permits DOS backslashes without
escaping them.

The setup editor exposes the security expression, door type, host path, shell
execution, drop-file format, DOS command, and DOS memory size.
The DOS command supports these case-sensitive substitutions:

| Variable | Value |
| --- | --- |
| `{dropFile}` | Generated drop-file name |
| `{node}` | One-based node number: the first node is `1` |
| `{baud}` | `57600` |

`path` is the door's host directory, not a disk image. On first launch Icy Board
copies `assets/dos/freedos.img` to `assets/dos/doors/<door-name>.img`, imports the
directory as `C:\DOOR`, and then preserves that image as the door's game state.
Deleting the per-door image reinstalls from the host directory and loses its
saved game state; it is not necessary for routine configuration. A normal
guest shutdown saves the modified image. Cancellation or a runtime timeout
does not save that run's disk changes.

Before every launch Icy Board refreshes the selected drop file in both
`C:\ICB` and `C:\DOOR`, along with `RUN.BAT`, `FDAUTO.BAT`, and
`POWEROFF.COM`. Changes to those session settings therefore do not require an
image rebuild. Use `dos-copy` for updated host files and `dos-console` for
configuration inside the installed game's image.

## Run game setup from the command line

Stop Icy Board, then open the door's persistent image with the same native
emulator used by the BBS:

```bash
icbsetup dos-console /path/to/board LORD
```

Use the configured door name, not an image filename. The command takes the
board lock and boots a temporary copy of `assets/dos/doors/lord.img`. It does
not modify the base image or reimport an existing game from the host.

For a game that has not yet been launched, supply its host directory once:

```bash
icbsetup dos-console /path/to/board LORD --source /path/to/board/doors/lord
```

This creates the persistent image and imports the game into `C:\DOOR` before
starting the temporary session. `--source` is ignored when the image already
exists. Discarding the first session leaves that initial imported image intact.

The console starts at `C:\DOOR>`. Run the game's setup program there, for
example `LORDCFG.EXE`. Configure game paths as `C:\DOOR`, so programs that
validate paths see exactly the directory used during normal door launches.
Select the BBS drop-file format configured in Icy Board, COM1, and 57600 baud.
The console does not generate a fresh caller drop file.

Save and close the game's setup program, then type `EXIT` at the DOS prompt.
This shuts down the guest, restores the image's original `FDAUTO.BAT`, and
atomically saves the modified image. A uniquely named
`<image>.pre-console-<suffix>.bak` beside it preserves the original image.
Game files and `FDCONFIG.SYS` changes remain in the saved image; manual changes
to the temporary `FDAUTO.BAT` do not. Restore a backup only with the board stopped.

Press **Ctrl+Q** to discard the session instead, including when a guest program
hangs. Escape, Ctrl+C, arrows, and function keys are passed to DOS. An emulator
error also discards the temporary session. Memory defaults to 64 MiB; use
`--memory 32`, for example, to match a differently configured door.

This is a text-mode VGA/keyboard console for a terminal of at least 80x25.
CP437 characters, colors, and the cursor are rendered in the host terminal.
Keyboard text uses the US ASCII mapping; graphics modes and mouse input are
not supported. Larger guest text modes are clipped to the host terminal size.
Pasted input is paced and capped at 4096 queued characters. Real FreeDOS shell
save/discard and the LORD 4.06 setup menu have been tested; compatibility with
every setup program is not implied.

The image remains a standard MBR/FAT16 raw disk for external maintenance tools.
Do not boot or modify it concurrently with Icy Board or the native console.

## x86-native development

The workspace depends on the Git repository at
<https://github.com/mkrueger/x86>; the workspace manifest pins a tested revision.
The x86 repository also provides a standalone `x86-console` diagnostic host;
use `icbsetup dos-console` for Icy Board's persistent door images. The emulator
repository contains asset-gated firmware tests:

```bash
X86_BIOS=assets/dos/seabios.bin \
X86_VGA_BIOS=assets/dos/vgabios.bin \
X86_DISK=assets/dos/freedos.img \
cargo test --test freedos_boot -- --ignored --test-threads=1
```
