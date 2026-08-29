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

Copy each door file into the image without mounting it:

```bash
icbsetup dos-copy assets/dos/freedos.img LORD.EXE DOORS/LORD/LORD.EXE
icbsetup dos-copy assets/dos/freedos.img LORD.CFG DOORS/LORD/LORD.CFG
```

Doors that require a FOSSIL driver need one installed separately. FreeDOS does
not ship X00, and Icy Board does not redistribute it. Install a driver whose
license permits your use under `C:\FOSSIL`, then load it from the door's DOS
command. Doors that support direct serial I/O can use COM1 without a FOSSIL.

## Configure a door

A door entry in the conference door TOML can use:

```toml
[[door]]
name = "LORD"
description = "Legend of the Red Dragon"
password = ""
securiy_level = "10"
door_type = "Dos"
path = "door/lord"
use_shell_execute = false
drop_file = "DoorSys"
dos_memory_mb = 64
dos_command = """
@ECHO OFF
C:\FOSSIL\X00.SYS
LORD.EXE {node}
"""
```

The setup editor exposes the security expression, door type, host path, shell
execution, drop-file format, DOS command, and DOS memory size.
The DOS command supports these case-sensitive substitutions:

| Variable | Value |
| --- | --- |
| `{dropFile}` | Generated drop-file name |
| `{node}` | Current Icy Board node number |
| `{baud}` | `57600` |

`path` is the door's host directory, not a disk image. On first launch Icy Board
copies `assets/dos/freedos.img` to `assets/dos/doors/<door-name>.img`, imports the
directory as `C:\DOOR`, and then preserves that image as the door's game state.
Delete the per-door image to reinstall it from the host directory. A caller
disconnect cancels the emulator and still saves completed guest writes.

Before every launch Icy Board refreshes the selected drop file in both
`C:\ICB` and `C:\DOOR`, along with `RUN.BAT`, `FDAUTO.BAT`, and
`POWEROFF.COM`. Changes to those session settings therefore do not require an
image rebuild. Changes to installed game files require either `dos-copy` or
deleting the per-door image; deleting it also resets game state.

## x86-native development

During joint development this workspace uses the sibling checkout at `../x86`.
Before release, publish the x86-native changes and replace the path dependency
with a pinned released version or Git revision. The x86 repository contains an
asset-gated FreeDOS smoke test:

```bash
X86_BIOS=assets/dos/seabios.bin \
X86_VGA_BIOS=assets/dos/vgabios.bin \
X86_DISK=assets/dos/freedos.img \
X86_EXPECT_SERIAL='No DOS door configured.' \
cargo test --test freedos_boot -- --ignored
```
