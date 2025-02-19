## PPLC

PPLC works basically like the old one - specify file name and you're all set. 
But it has way more to offer now:

```
Usage: pplc [<file>] [-d] [--nowarnings] [--version <version>] [--lang-version <lang-version>] [--cp437 <cp437>] [--init]

PCBoard Programming Language Compiler

Positional Arguments:
  file              file[.pps] to compile (extension defaults to .pps if not
                    specified)

Options:
  -d, --disassemble output the disassembly instead of compiling
  --nowarnings      don't report any warnings
  --version         version number for the compiled PPE, valid: 100, 200, 300,
                    310, 320, 330, 340, 400 (default)
  --lang-version    version number for the language (defaults to version)
  --cp437           specify the encoding of the file, defaults to autodetection
  --init            create & init new ppl package in target directory
  --help, help      display usage information
```

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

### Supported versions

PPLC is designed to generate valid output files PCBoard 15.0-15.4 and icy board. Using `--version` changes the container format and sets the language version to that value.
With `--langversion` it's possible to specify a special language version. This is useful for using the new PPL4 features for old PCBoard versions.
Recommened is language verison 350.

Versions 400+ is just for icy board. PPE files are no longer crypted and some additional features are implemented making version 400 incompatible with PCBoard.

### Packages
Packages are a new feature. They help to create bigger projects and to distribute/generate PPLs with different versions.

A package is basically:

```
ppl.toml
src/main.pps
```

All .pps files in "src" and subdirs are taken for compilation. With "main.pps" being the 1st one. Since procedure/function declarations are no longer required in language version >= 350 this makes it easier to break down a ppl project in many logicial parts.

Just compile 'pplc ppl.toml' and the executable is generated in `target/icy_board`. If another `--version` number is given the target directory changes to according pcboard version. Let's say `--version 330` produces `target/pcboard-15.30`.

It's possible to generate/copy more files to handle CP437 for targetting DOS/PCBOARD.


### PPL.TOML

Let's take a look at the toml:

```toml
[package]
name = "lread"
version = "0.1.0"
language_version = 350
authors = ["Mike Krüger <mkrueger@posteo.de>"]

[data]
text_files = ["lread.cfg"]
art_files = ["data/screen.icy"]
```

`name` specifies the name of the output ppe file.
`text_files` specify a number of text files that are copied to the target directory and converted to CP437 if needed.
`art_files` specify a number of files that are copied to the target directory and converted to CP437 if needed. In case of `icy` files they're converted to `pcb` so in that case in the target is a file data/screen.pcb with the correct encoding.
`files` just get copied

This all can change in the future. This is the first implementation and the Software is still in alpha stage. 
Esp the split of text/art files is a point to discuss. 

Another thing is release packing with FILE_ID.DIZ generation, src.zip packaging etc. this needs to be worked out.