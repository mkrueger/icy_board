# Icy Board

A re-creation of PCBoard, the DOS bulletin board system, for machines that are
still running. Same commands, same `@` macros, same PPEs — on Linux, macOS,
Windows and the Raspberry Pi, over telnet, SSH and websockets.

![Main menu](assets/main_menu.png?raw=true "Main menu")

## Status

Beta. A board runs, takes callers, carries message and file areas, runs PPEs and
speaks FTN as a leaf. What it does not do yet is written down rather than left
to be discovered:

* [Feature status](docs/feature_parity.md) — every PCBoard command and how far it got
* [Known limitations](docs/known_limitations.md) — read this before moving a board over
* [Differences](docs/differences.md) — where icy_board departs from the original on purpose

There is no modem and no serial support, and there never will be. Everything
else that PCBoard did is either here or on the list.

## What it looks like

| | | |
| :---: | :---: | :---: |
| ![Reading a message](assets/message_reader.png?raw=true) | ![File listing](assets/file_list.png?raw=true) | ![Call waiting screen](assets/call_waiting_screen.png?raw=true) |
| The message reader | A file listing, descriptions read out of each archive | The call waiting screen the sysop sees |
| ![icbsetup](assets/icbsetup.png?raw=true) | ![icbsm](assets/icbsm.png?raw=true) | ![mkicbtxt](assets/mkicbtxt.png?raw=true) |
| `icbsetup` — the board | `icbsm` — users and groups | `mkicbtxt` — every prompt |

The configuration tools are TUIs, so they work over SSH.

## Try it

```sh
icbsetup create mybbs     # writes a complete board into mybbs/
cd mybbs
icboard --localon         # logs you in as the sysop, telnet on port 1337
```

Prebuilt archives are on the
[releases page](https://github.com/mkrueger/icy_board/releases); building from
source needs nothing but a [Rust toolchain](https://rustup.rs). See
[INSTALL.md](INSTALL.md) for both, and for the list of programs — the board is a
set of command line tools, not one binary.

An existing PCBoard installation can be brought over. The original is only read,
never written:

```sh
icbsetup import /path/to/PCBOARD.DAT mybbs
```

The importer is the part that most needs real installations to test against. If
yours does not come over, that is worth a bug report more than anything else.

## Documentation

The **handbook** is the long form — installation, running a board, events, the
FTN mailer, customizing, and a complete PPL reference. It is built from
[docs/source](docs/source) and ships as a PDF with each release.

```sh
cd docs && make latexpdf     # or: make html
```

Shorter pieces live beside it in the repository:

| | |
| :--- | :--- |
| [Getting started](docs/gettingstarted.md) | What to do after `icbsetup create` |
| [File areas](docs/icbfile.md) | Bringing a file base into shape |
| [PPL](docs/ppl.md) · [PPLC](docs/pplc.md) · [New in PPL](docs/new_ppl.md) | The language, its compiler, and what 4.0 added |
| [New @ macros](docs/new_macros.md) | Beyond PCBoard's set |
| [PPE format](docs/ppe_format.md) | The executable format, for tooling |
| [Roadmap](docs/roadmap.md) | What is done and what is next |

PPL has editor support for [VS Code](editors/vscode),
[Zed](https://github.com/mkrueger/zed-ppl) and, through the
[tree-sitter grammar](crates/tree-sitter-ppl), Neovim and Helix.

## What it is trying to be

* PCBoard on a machine you can still buy, especially Linux and the Raspberry Pi
* As compatible as a rewrite can be — PPEs are the hard case and the interesting one
* The whole ecosystem, config tools included, not just the board
* PCBoard's look and feel, kept on purpose
* Extended where it helps, without breaking what already ran

And what it is not trying to be:

* A board that is easy out of the box. PCBoard was not, and neither is this. Every
  modern BBS looks the same because it ships one way of doing things; here you
  configure it, and it looks like yours.
* A GUI. The tools are TUIs so they work over SSH. A GUI is welcome if someone
  writes one, but nothing waits for it.

## Where it came from

It started as a port of Adrian Studer's [ppld](https://github.com/astuder/ppld),
a PPE decompiler, written to learn Rust. Decompiling PPEs turns out to need a
board to run them against, and a general-purpose runtime does not help because
PPEs are specific to PCBoard down to the last data structure. So the decompiler
needed a BBS, and the BBS is this.

The leaked PCBoard 15.4 sources and a copy of the real thing running under
DOSBox settle the arguments about what correct means — see
[compat/](compat/README.md) for how that oracle is driven.

## License

Apache 2.0, see [LICENSE](LICENSE).
