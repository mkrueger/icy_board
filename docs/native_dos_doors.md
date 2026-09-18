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

From an Icy Board build, run:

```bash
icbsetup dos-image /path/to/board
```

This reproducibly downloads and verifies:

- FreeDOS 1.4 LiteUSB from the official FreeDOS site
- SeaBIOS and the VGA BIOS used by upstream v86

It creates:

```text
assets/dos/freedos.img
assets/dos/seabios.bin
assets/dos/vgabios.bin
```

The command patches the image to call `C:\ICB\RUN.BAT` and then execute the
small ACPI shutdown helper injected by Icy Board. The image remains a normal
MBR/FAT16 raw disk and can also be opened by standard image tools.

## Install a door

Install the game's files in a host directory such as `doors/lord`, matching
the door's `path` setting. Icy Board imports that directory into `C:\DOOR`
when it first creates the per-door image. To update an existing image without
mounting it, stop the door and use, for example:

```bash
icbsetup dos-copy assets/dos/doors/lord.img LORD.EXE DOOR/LORD.EXE
```

Doors that require a FOSSIL driver need one installed separately. FreeDOS does
not ship X00, and Icy Board does not redistribute it. Install a driver whose
license permits your use under `C:\FOSSIL`. `X00.SYS` is a device driver, not
an executable: do not put `C:\FOSSIL\X00.SYS` in `START.BAT` or `dos_command`.
Load it at boot by adding this line to the image's `C:\FDCONFIG.SYS`, preserving
the existing configuration, and follow your driver's documentation for options:

```dos
DEVICE=C:\FOSSIL\X00.SYS
```

Use a DOS-aware image tool to edit that file, or `dos-copy` to install a
complete replacement. Configure `assets/dos/freedos.img` before creating any
per-door images, or modify the existing per-door image. Changes to the base
image do not update images already created. A driver distributed as an
executable TSR may instead be loaded by the batch file according to its own
instructions. Doors that support direct serial I/O can use COM1 without a
FOSSIL.

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
file, and COM1. The literal TOML string also permits DOS backslashes without
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
Delete the per-door image to reinstall it from the host directory. A normal
guest shutdown saves the modified image. Cancellation or a runtime timeout
does not save that run's disk changes.

Before every launch Icy Board refreshes the selected drop file in both
`C:\ICB` and `C:\DOOR`, along with `RUN.BAT`, `FDAUTO.BAT`, and
`POWEROFF.COM`. Changes to those session settings therefore do not require an
image rebuild. Changes to installed game files require either `dos-copy` or
deleting the per-door image; deleting it also resets game state.

## Run game setup outside Icy Board

The image is a bootable raw hard disk, not an emulator-specific container.
For interactive VGA/keyboard setup, it can be booted with QEMU. This is an
external maintenance option; Icy Board itself still uses `x86-native`.

1. Stop Icy Board before modifying or booting an image. Back up the per-door
   image, for example `assets/dos/doors/dostest.img` for a door named `dostest`.
   Do not use the base `freedos.img` to configure an already installed game.
2. Temporarily replace that image's `FDAUTO.BAT`. Its normal startup redirects
   DOS to COM1, runs the door, and powers off, so it is unsuitable for a local
   setup session. Create a host file named `SETUP.BAT` containing:

   ```dos
   @ECHO OFF
   SET DOSDIR=C:\FREEDOS
   SET PATH=%DOSDIR%\BIN
   CTTY CON
   CD C:\DOOR
   ```

   Copy it into the image as the startup file:

   ```bash
   icbsetup dos-copy assets/dos/doors/dostest.img SETUP.BAT FDAUTO.BAT
   ```

3. With QEMU installed, boot the image in its graphical console:

   ```bash
   qemu-system-i386 -m 64 -boot c -nic none \
     -drive file=assets/dos/doors/dostest.img,format=raw,if=ide
   ```

   Run the setup executable supplied with your LORD distribution at the
   `C:\DOOR>` prompt. Save the configuration and exit setup before shutting
   down QEMU. Do not use `-snapshot` if you want these changes saved.
4. Close QEMU before restarting Icy Board. The next door launch automatically
   restores its normal `FDAUTO.BAT`; game configuration and `FDCONFIG.SYS`
   remain in the persistent image.

The QEMU procedure is not an assertion of compatibility with every door or
setup program; it has not been validated with the reported LORD installation.

## x86-native development

The workspace depends on the Git repository at
<https://github.com/mkrueger/x86>; `Cargo.lock` records the selected revision.
The x86 repository also provides a standalone `x86-console` diagnostic host;
Icy Board does not bundle it as an interactive DOS setup command. It contains an
asset-gated FreeDOS smoke test:

```bash
X86_BIOS=assets/dos/seabios.bin \
X86_VGA_BIOS=assets/dos/vgabios.bin \
X86_DISK=assets/dos/freedos.img \
X86_EXPECT_SERIAL='No DOS door configured.' \
cargo test --test freedos_boot -- --ignored
```
