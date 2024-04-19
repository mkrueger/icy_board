# icy_board

Aim is to make PCBoard clone that's finished on PCboards 100th birthday - for sure it'll take much time.
I've started to port a PPL decompiler 2022 and it's now a compiler, decompiler and runtime.
(https://github.com/mkrueger/PPLEngine)

It works quite well but the PPEs require a BBS to be running to be useful. First approach was to make a general runtime, however PPEs are too PCboard specific.
So all it's needed is a new PCBoard.
There are data structures for almost all PCBoard data structures so making a BBS is the next logical step. Don't expect anything to be runnable soon.

![Login screen](assets/login_screen.png?raw=true "Login screen")

## Goals

* Have a modernized version of PCBoard that runs on modern systems - esp. linux/Raspberry Pi
* Be as compatible as possible (PPE/Handling)
* Provide the whole PCBoard eco system - including config tools
* Maintain the spirit/look & feel of PCBoard (at least for a while)
* Make it as easy as possible to run existing PCBoard installations

### Non Goals

* GUI config tools - it's ment for running on a SSH session :)
  * However if one will invest the time for making a UI it's welcome but the goal is to have modern TUIs.

## Differences to PCBoard

The goal is to be as compatible as possible but enhancing PCBoard for the moderen age implies changes that may break compatibiltiy.

* All config files are in .toml format and can be changed with any text editor
  * Old config files are created automatically for old .PPE compatibility files but some PPEs might break. (Report please)
  * TOML formats include: Bullettins, Script Questionnaires and Menus
  * Exceptions so far: Trashcan file 
* Long file name support! yeah! (However old config files have limits)
  * Applies to old PPEs as well they got their limits extended, unless they check for file lengths themself.
* Message base format changed to JAM
  * Planned is to support multiple message base formats
  * Means all PPEs/tools break that may read the old format
* No security level centralization. Security levels are now on command basis. 

### CP437 is dead

All files should be UTF-8. The importer automatically converts files to UTF-8.

However it's possible to use CP437 files with IcyBoard. Icy Board differenciates UTF-8 from CP437 with the UTF8 BOM.
All files without  FF EF FE at the start are treated as UTF8. So it's no hard requirement. Copy old DOS files and they will show up fine regardless of CP437 or UTF-8 used.

So it's possible to use a modern (non CP437) text editor to alter the BBS files.

* Config files, Menus, (in general: non display text files) are ALWAYS UTF-8.
* UTF8->CodePage 437 chars will be translated by the table found here: <https://en.wikipedia.org/wiki/Code_page_437>
  
### Command changes

* No more hard coded commands. They're now all configurable

### PPE/Menus

* `.MNU` files are converted to a new .toml format (in the assumption that no `.PPE` will handle `.MNU` files)
* PCBoard is now case sensitive on unix but that does NOT apply to `.PPE` files. Note: May change for new `.PPE` files but will always remain for old ones for compatiblity reasons.

### Trashcan

A simple text file read line by line. If a line starts with '#' it's treated as a comment.
Leading & traling whitespaces are ignored


### Enhancements

* Access System is more complex. The old one had user levels. An access now consists of a combination of:
  * security level
  * group (like unix groups)
  * age 
* Added more trashcans: bad_email, bad_passwords
* New 'vip' users vip_users.txt (same as trashcan). But a list of users which the sysop gets a notification for logon (from RemoteAccess)
* 

### Planned Enhancements/Discussion 

* Access system time limit - so it's only open at certain days & times - however only command where that makes sense to me is the sysop page.
But I like the RA "DayTimes" system.
* 

# PPLEngine

An engine for PPL/PPE PCBoard handling - just for fun.

Features:

* A new decompiler/disassembler engine (ppld)
* A compiler (pplc) that compiles UTF-8/CP437 files to output CP437 PPEs
* A runtime (pplx) that runs .PPE files on console
* A language server that provides developer functionality in any editor that supports lsp

## Why are you doing this?

Just for fun. Started this project initially to learn rust.

## What works

* Decompiler is pretty complete (report bugs!). I would say it's better than everything we had back in the 90'.
* Compiler should be able to parse a PPS and generate running PPE files
* Runner should basically work.
* Started to implement a LSP to provide syntax highlighting and tooltips.

### Decompiler

Decompiler is completely rewritten and can disassemble now on top of recompilation.

* PPE 3.40 Support
* Reconstruction of control structures currently is broken due of a rewrite of the decompiler
  if you want then/elseif/else, while…endwhile, for…next, break & continue, select case support
  go back to b67e861a734c57c1a7b2fb891725260ae6d7f343
  The new decompiler infrastructure is much better and the decompilation result should be 100% correct. (The old one may contain bugs)
  And it supports 15.4 but the AST is a bit different so the reconstruction needs to be rewritten.

  The old source could be used as starting point - but the new one has way better tools for AST analyzation that should be used instead of the old hacky approach.

* It tries to do some name guessing based on variable usage.

```text
PCBoard Programming Language Decompiler

Usage: ppld [OPTIONS] <FILE>

Arguments:
  <FILE>  file[.ppe] to decompile

Options:
      --raw            raw ppe without reconstruction control structures
  -d, --disassemble    output the disassembly instead of ppl
      --output         output to console instead of writing to file
      --style <STYLE>  keyword casing style, valid values are u=upper (default), l=lower, c=camel
  -h, --help           Print help
  -V, --version        Print version
```

The dissamble output can be used to see what the compilers are generating and for debugging purposes.

### Compiler

Supports up to 15.4 PPL (1.0 -> 3.40 PPE format)

Should be compatible to the old PCB compiler with some slight differences (see PPL differences)

The compiler automatically generates user variables, if needed but behavior can be changed with command line options. It does some optimizations so it should produce smaller & faster exectuables than the original one.

pplc has following options:

```text
PCBoard Programming Language Compiler

Usage: pplc [OPTIONS] <FILE>

Arguments:
  <FILE>  file[.pps] to compile (extension defaults to .pps if not specified)

Options:
  -d, --disassemble                output the disassembly instead of compiling
      --nouvar                     force no user variables
      --forceuvar                  force user variables
      --nowarnings                 don't report any warnings
      --ppl-version <PPL_VERSION>  version number for the compiler, valid: 100, 200, 300, 310, 330 (default), 340
      --dos                        input file is CP437
  -h, --help                       Print help
  -V, --version                    Print version

As default the compiler takes UTF8 input - DOS special chars are translated to CP437 in the output.
```

Note:  All old DOS files are usually CP437 - so it's recommended to use --dos for compiling these.

#### PPL differences

The aim is to be as compatible as possible.

* Added keywords that are invalid as identifiers (but are ok for labels):
  ```LET```, ```IF```, ```ELSE```, ```ELSEIF```, ```ENDIF```, ```WHILE```, ```ENDWHILE```, ```FOR```, ```NEXT```, ```BREAK```, ```CONTINUE```, ```RETURN```, ```GOSUB```, ```GOTO```, ```SELECT```, ```CASE```, ```DEFAULT```, ```ENDSELECT```

I think it improves the language and it's open for discussion. Note that some aliases like "quit" for the break keyword is not a keyword but is recognized as 'break' statement. I can change the status of a keyword so it's not a hard limit - as said "open for discussion".

* Added ```€``` as valid identifier character. (for UTF8 files)
* Return type differences in function declaration/implementation is an error, original compiler didn't care.


#### PPL 4.0

New Constructs (Language Version 400):

New loops
``` REPEAT ... UNTIL [CONDITION] ``` Statement
``` LOOP ... ENDLOOP``` Statement

Variable initializers:

``` TYPE VAR=[INITIALIZER]``` Statement

Operator Assignment for binary (non condition operators):

``` A += 1``` Statement

Works for ```+-*/%``` and ```&|```

Return can now return values inside functions:

``` RETURN expr``` Statement

1:1 same semantic as:

```PPL
FUNC_NAME = expr
RETURN 
```

No more brackets needed for if or while statements!

Example:
```PPL
IF A <> B THEN
...
ENDIF

WHILE IsValid() PRINTLN "Success."
```

Note: With "lang" version >=400 'Quit' and 'Loop' are no longer synonyms for 'break' and 'continue'. Existing sources should be easily adapted.
But never saw them in the wild.

### Runner

* pplx is able to run several PPEs on command line still has many limits but it's usable
* Feel free to request support for missing PPEs there are still bugs

```text
PCBoard Programming Language Execution Environment

Usage: pplx [OPTIONS] <FILE>

Arguments:
  <FILE>  file[.ppe] to run

Options:
  -s, --sysop    if set, the executable will run in sysop mode
  -h, --help     Print help
  -V, --version  Print version
```

#### Runner differences

Fixed an recursion bug:
  
```PPL
DECLARE PROCEDURE FOO(VAR BYTE X)
BYTE C
FOO(C)
PRINTLN "End value:", C

PROCEDURE FOO(VAR BYTE X)
  BYTE LOC
  LOC = X + 1
  if (X > 2) RETURN
  X = X * 2
  PRINTLN LOC ,":", X
  FOO(LOC)
  PRINTLN LOC ,":", X
  X = LOC
ENDPROC
```

The value of LOC changes between prints but does not in PCBoard.
pcboard prints:

```TEXT
1:0
2:2
3:4
3:4
2:2
1:0
End value:1
```

Correct is:

```TEXT
1:0
2:2
3:4
3:4
3:2
3:0
End value:3
```

Would be easy to simulate the bug (just swap local write back & variable write back in endproc handling).
But I don't assume that this bug affects any existing PPEs.

## TODO

* Execution engine needs to be completed.
* Real Compiler/Decompiler support for 15.4 (debugging the old dos pplc/pcobard is the way to go here)
* Possible rewrite the expression decompilation part - it's hard to follow atm.
* LSP needs to be extended - find references, rename, code completion etc.

## Building & Running

* Get rust on your system <https://www.rust-lang.org/tools/install>

```bash
cd PPLEngine
cargo build -r
```

running in release mode:

```bash
cd target/release
./ppld [PPEFILE]
./pplx [PPEFILE]
./pplc [PPLFILE]
```

Result is printed to stdout

Testing the lsp using vs code:

```bash
cd ppl-lsp
pnpm i
cargo build 
```

then start vs code with

```bash
code .
```

And run from inside vs code the project with F5. A new vs code opens with PPL support.
(Still need to figure out how to plug in the lsp for other editors)
