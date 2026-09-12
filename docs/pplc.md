## PPLC

PPLC works basically like the old one - specify file name and you're all set. 
But it has way more to offer now:

```
Usage: pplc [-d] [--nowarnings] [--version] [--mono] [--runtime <runtime>] [--lang-version <lang-version>] [--cp437] [--init] [--defines <defines>] [--format] [--stdout] [--check] [--compression <compression>] [--debug] [--print-config] [--print-config-json] [--] [<file>]

PCBoard Programming Language Compiler

Positional Arguments:
  file              file[.pps] to compile (extension defaults to .pps if not
                    specified)

Options:
  -d, --disassemble output the disassembly instead of compiling
  --nowarnings      don't report any warnings
  --version         print the version and exit
  --mono            write plain text, without the ansi escapes that colour the
                    output
  --runtime         version number for the compiled PPE, valid: 100, 200, 300,
                    310, 320, 330, 340, 400 (default)
  --lang-version    language version (defaults to the manifest,
                    PPL_LANG_VERSION, then runtime capped at 400)
  --cp437           specify the encoding of the file (cp437 = true, utf8 =
                    false), defaults to autodetection
  --init            create & init new ppl package in target directory
  --defines         semicolon separated list of pre processor variables
  --format          formats source file instead of compile
  --stdout          with --format, write the result to stdout and leave the file
                    alone
  --check           checks source/package for errors without compiling
  --compression     section compression for runtime 400: none (default), zstd
       --debug           keep optional source symbol names in PPE-400 debug data
  --print-config    prints the effective compiler configuration without
                    compiling
  --print-config-json
                    prints the effective compiler configuration as json without
                    compiling
  --help, help      display usage information
```

### Coloured output

Diagnostics are coloured only when someone is looking at a terminal. When the
output is piped, redirected or read by an editor, `pplc` writes plain text, so
escape sequences never end up in a log or an output pane. `--mono` forces plain
text even on a terminal, `NO_COLOR` does the same through the environment, and
`CLICOLOR_FORCE` keeps the colour when piping into a pager such as `less -R`.

### Check mode

`--check` validates source diagnostics and formatting without writing a PPE or
changing the source. It returns a nonzero status for invalid code **or** a
formatting difference. A formatting-only diff does not mean a program cannot
compile. Use `--format` to apply formatting, or `--format --stdout` to inspect
the formatted source without changing the file.

### Effective configuration

`--print-config` reads a source or `ppl.toml`, including `;$LANGVERSION`, and
shows the settings the compiler would use without creating an executable or a
target directory:

```text
PPL compilation configuration

Source                 /home/mike/doors/hello.pps
Project                none
Sources                1
Encoding               detect

Language version       350
       From                 environment
       Command line         not set
       Manifest             not set
       Environment          350

Runtime version        400
       From                 default

Output                 /home/mike/doors/hello.ppe
Defines                none
```

`--print-config-json` emits only JSON, with absolute source and output paths, so
an editor can discover a package's actual output instead of reproducing the
compiler's rules.

### Compression and debug names

Runtime 400 stores the program in sections, and each section can be packed with
Zstd. `--compression zstd` turns that on; the default is `none`. The choice is
recorded in the file, so the loader never has to guess from a length difference.
Compression is a runtime-400 feature; asking for it on an older target is an
error rather than a silent fallback.

`--debug` keeps variable, record, field, enum and enum-member names in optional
PPE-400 debug data. Without it those names are left out, and nothing else about
the program changes: stripped and unstripped builds have the same content
identity. Qualified host API names remain necessary for binding in both forms.
There is no source path, source text or line number in either form.

### Disassembling

Instead of creating a .PPE executable it can print a disassembler. This is useful to find out what the compiler does with the input code.

 A .PPE executable basically consits out of a variable table that cotains all variables and constants used in the .PPE file.
 And a machine code that is basically a simple version of PPL. Doesn't have any other control constructs than `IF !COND GOTO` but if you're familiar with PPL the disassembler should be easy to understand.

 At least it is designed to be. A simple hello world would look like:

```
Variable Table 1 variables

   # Type         Flags Role           Name        Value
---------------------------------------------------------------------------------------
0001 String       0     Constant       CONST_2     "Hello, World!"


Offset  # OpCode      Parameters
---------------------------------------------------------------------------------------
       [000A 0001 0001 0000 0000 ]
00000: 0A 'PrintLn'   [CONST_2 0001]
       [0001 ]
0000A: 01 END        


Generated:
Real uncompressed script buffer size: 12 bytes

00000: 000A 0001 0001 0000 0000 0001 
```

That example targets a PCBoard runtime, where each instruction has a word
encoding to show. Runtime 400 has none: its instructions are stored in their own
section and jumps address instruction indices. The listing there prints the
instructions and their operands without the word dump, and ends with the
instruction count instead of a script buffer size.

### Supported versions

PPLC targets PCBoard 15.0-15.4 and Icy Board. `--runtime` selects the target
container; `--version` only prints the compiler version and exits.
`--lang-version` selects the source language independently. Language 350 syntax
can target a classic runtime when it lowers to classic instructions; nominal
enum storage and explicit conversions still require runtime 400. Language 400
APIs and records cannot be made available on PCBoard by changing a CLI flag.

Versions 400+ is just for icy board. Runtime 400 writes a different container
entirely — sectioned, unencrypted and validated before it runs, described in
[ppe_format.md](ppe_format.md) — which makes it incompatible with PCBoard.
PPEs from the unreleased 400 beta use the old container and have to be
recompiled; the loader says so instead of misreading them.

### Packages
Packages are a new feature. They help to create bigger projects and to distribute/generate PPLs with different versions.

A package is basically:

```
ppl.toml
src/main.pps
```

All `.pps` files in `src` and its subdirectories are compiled together, with
`main.pps` first. Explicit `DECLARE` statements are optional at every supported
language version because the compiler collects routine signatures package-wide.

Run `pplc ppl.toml` to generate the executable in `target/icboard`, relative to
the package. A classic `--runtime` selects a version-specific directory;
`pplc --runtime 330 --lang-version 350 ppl.toml` uses `target/pcboard_15.30`.
An explicit `;$LANGVERSION` in a source still takes precedence over the CLI.

It's possible to generate/copy more files to handle CP437 for targetting DOS/PCBOARD.


### PPL.TOML

Let's take a look at the toml:

```toml
[package]
name = "lread"
version = "0.1.0"
authors = ["Mike Krüger <mkrueger@posteo.de>"]

[compiler]
language_version = 350

[data]
text_files = ["lread.cfg"]
art_files = ["data/screen.icy"]
```

`name` specifies the name of the output ppe file.
`language_version` is the project's source language. Without one,
`PPL_LANG_VERSION` supplies a personal default; `--lang-version` and a
``;$LANGVERSION`` source directive take precedence over both.
`text_files` specify a number of text files that are copied to the target directory and converted to CP437 if needed.
`art_files` specify a number of files that are copied to the target directory and converted to CP437 if needed. In case of `icy` files they're converted to `pcb` so in that case in the target is a file data/screen.pcb with the correct encoding.
`files` just get copied

This all can change in the future. This is the first implementation and the Software is still in alpha stage. 
Esp the split of text/art files is a point to discuss. 

Another thing is release packing with FILE_ID.DIZ generation, src.zip packaging etc. this needs to be worked out.