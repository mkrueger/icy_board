
# PPL

IcyBoard features a rewriten engine for ppl execution, a compiler and a decompiler.

Features:

* A compiler (pplc) that compiles UTF-8/CP437 files to output CP437 PPEs
* A decompiler (ppld) that decompiles all old .PPE files (up to PCBoard 15.4 PPEs) 
  - Reconstructs PPL all old statements 
* A language server that provides developer functionality in any editor that supports lsp
  - Included VS Code Extension (.vsix) for easy installation - just dnd it into the vs code extensions panel.

## What works

* Both compiler & decompiler is DONE. I would say it's better than everything we had back in the 90'.
Everything that doesn't work is a bug - please report issues.
* The decompiler should be able to decompile any existing PPE file and trick out the anti decompiliation tricks that were common these days :). Note that it's not designed to decompile the new 400 PPEs.
* Compiler should be able to parse a PPS and generate running PPE files
  - There are slight differences to PPLC - the new one is more strict. Issues should be easy fixable
  - Be prepared for tons of warnings of non trivial .PPS files. The old PPLC hasn't had much error checks. In doubt I added a warning instead.
* IcyBoard should be able to run most PPE files
  - PPE data files can be converted to UTF8 with (icbsetup ppe-convert <PATH>) but backup all files first
  - ppe-convert can take a <FILENAME> to convert the single file to UTF8
  - WARNING: Handle ppe-convert with care - can potentially destroy things. Convert one PPE after another.
  - No need to convert PPE - CP437 works, just consider that - I do it because no modern editor supports CP437 anymore.
* LSP should provide highlighting, help, find all refs/goto definition and a basic code completion 

### Decompiler

First Decompiler was based upon ppld. Find the original code here:
https://github.com/astuder/ppld

Much effort was done for implementing the decompiler. Existing PPEs may need to be altered for IcyBoard or at least analyzed so being able to decompile
the old PPEs is important for the project.

The current Decompiler is completely rewritten and uses a ppl machine language - which it can disassemble - to reconstruct a PPL AST.

* PPE 3.40 Support
* Full reconstruction of IF/THEN, SWITH, WHEN etc.
* It tries to do some name guessing based on variable usage.

```text
Usage: ppld <file> [-r] [-d] [-o] [--style <style>]

PCBoard Programming Language Decompiler

Positional Arguments:
  file              file[.ppe] to decompile

Options:
  -r, --raw         raw ppe without reconstruction control structures
  -d, --disassemble output the disassembly instead of ppl
  -o, --output      output to console instead of writing to file
  --style           keyword casing style, valid values are u=upper (default),
                    l=lower, c=camel
  --help, help      display usage information
```

The dissamble output can be used to see what the compilers are generating and for debugging purposes.

### Compiler

Supports up to 15.4 PPL (1.0 -> 3.40 PPE format)

Should be compatible to the old PCB compiler with some slight differences (see PPL differences)

The compiler decides itself if uservars are generated or not (so --novars is no longer needed)

pplc has following options:

```text
Usage: pplc <file> [-d] [--nowarnings] [--version <version>] [--lang-version <lang-version>] [--cp437 <cp437>]

PCBoard Programming Language Compiler

Positional Arguments:
  file              file[.pps] to compile (extension defaults to .pps if not
                    specified)

Options:
  -d, --disassemble output the disassembly instead of compiling
  --nowarnings      don't report any warnings
  --version         version number for the compiler, valid: 100, 200, 300, 310,
                    330, 340, 400 (Default)
  --lang-version    version number for the language (defaults to `version`)
  --cp437           specify the encoding of the file, defaults to autodetection
  --help, help      display usage information

As default the compiler takes UTF8 input - DOS special chars are translated to CP437 in the output.
```

Note:  All old DOS files are usually CP437 - so it's recommended to use --cp437 for compiling these.

#### PPL differences

The aim is to be as compatible as possible.

* Added keywords that are invalid as identifiers (but are ok for labels):
  ```LET```, ```IF```, ```ELSE```, ```ELSEIF```, ```ENDIF```, ```WHILE```, ```ENDWHILE```, ```FOR```, ```NEXT```, ```BREAK```, ```CONTINUE```, ```RETURN```, ```GOSUB```, ```GOTO```, ```SELECT```, ```CASE```, ```DEFAULT```, ```ENDSELECT```

I think it improves the language and it's open for discussion. Note that some aliases like "quit" for the break keyword is not a keyword but is recognized as 'break' statement. I can change the status of a keyword so it's not a hard limit - as said "open for discussion".

* Added ```€``` as valid identifier character. (for UTF8 files)
* Return type differences in function declaration/implementation is an error, original compiler didn't care.


#### PPL 4.0

DECLARE FUNCTION/PROCEDURE is now no longer needed anymore.

New Constructs (Language Version 350):

New loops
``` REPEAT ... UNTIL [CONDITION] ``` Statement
``` LOOP ... ENDLOOP``` Statement

Variable initializers:

``` TYPE VAR=[INITIALIZER]``` Statement

It's possible to initialize dim expressions as well:

``` TYPE VAR={ expr1, expr2, ..., exprn }``` means:

```PPL
TYPE VAR(n)
VAR(0) = expr1
...
VAR(n - 1) = exprn
```

Operator Assignment for binary (non condition operators):

Example:
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

Note: With "lang" version >=350 'Quit' and 'Loop' are no longer synonyms for 'break' and 'continue'. Existing sources should be easily adapted.
But never saw them in the wild.

##### Procedure parameters

It's now possible to use procedures as parameters.

Example:
```
PROCEDURE PrintHello(PROCEDURE f())
    PRINT "Hello "
    f()
ENDPROC
```
##### Pre Processor

You use four preprocessor directives to control conditional compilation:

`;$DEFINE` Defines a pre processor variable
`;$IF` Opens a conditional compilation, where code is compiled only if the specified expression is true
`;$ELIF` Closes the preceding conditional compilation and opens a new conditional compilation based on the specified expression
`;$ELSE` Closes the preceding conditional compilation and opens a new conditional compilation if the previous specified expression was true or false
`;$ENDIF` Closes the preceding conditional compilation.


Tokens:

`;#NAME` Replaces the directive with the evaluated token

Example with the predefined defines:
```
PrintLn "Version:", ;#Version
PrintLn "Runtime:", ;#Runtime
PrintLn "Language:", ;#LangVersion
```

Would print:

```
Version:0.1.0                                                                   
Runtime:400                                                                     
Language:400     
```

Would be possible to define conditional compilation based on language or runtime version
For example:
```
;$IF VERSION <= 340
    PrintLn "World"
;$ELIF VERSION < 200
    PrintLn "Old World"
;$ELSE
    PrintLn "New World"
;$ENDIF
```

##### Language Version 400

Language Version breaks compatibility with older PCBoards.

WARNING: 400 is not yet finished so expect to have to recompile in the final version.

Changes:

* (), {} and [] is different. [] is for indexer expressions it's encouraged to use that for arrays. And '{', '}' is exclusive for array initializers.

Member references. 400 introduces new BBS types:
* Conference, MessageArea, FileArea

For example:
```PPL
CONFERENCE CUR = CONFINFO(i) 

IF CUR.HasAccess() 
  PRINTLN CUR.Name
```

It's basically pre defined objects. This change requires a slight change in PPE.

* Pre defined functions can now have overloads. See "CONFINFO" from above. CONFINFO has two versions: One old one with 2 parameters and the new one with 1.
The new version returns a CONFERENCE object where the old one basically an "Object" of varying types, depending on the requested conference field.

API still TBD. CONFINFO(i) which returns the CONFERENCE is the only new function so far. However it contains some member functions for message & file areas. All BBS "objects" should be accessible through PPL. It should no longer be needed to gather information through manual reading of config files anymore.

## Building & Running

* Get rust on your system <https://www.rust-lang.org/tools/install>

```bash
cd PPLEngine
cargo build -r
```

```bash
cd target/release
./ppld [PPEFILE]
./pplc [PPLFILE]
```
