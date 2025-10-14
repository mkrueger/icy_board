Introduction
------------

What is PPL? 
~~~~~~~~~~~~

PPL (PCBoard Programming Language) is a scripting language used to extend
the functionality of PCBoard Bulletin Board System (BBS) software. It allows
system operators (sysops) to create custom menus, prompts, and other interactive
features for their BBS installations. PPL is known for its flexibility and
power, enabling sysops to tailor the user experience to their specific needs.

It's basically a BASIC dialect with some Pascal-like ideas. It has a rich set of built-in functions
and commands to interact with the BBS system, manage user sessions, and handle
data.

PPL Compiler
------------

Overview
~~~~~~~~

The IcyBoard PPL compiler (`pplc`) compiles PCBoard Programming Language source files (.pps) into 
executable PPE files that can be run on PCBoard-compatible BBS systems. The compiler supports all 
PPL versions from 1.00 through 4.00, maintaining backward compatibility while adding modern features.

Version Support
~~~~~~~~~~~~~~~

The compiler supports multiple PPE format versions:

* **PPL 1.00** - Original PCBoard 15.0 format
* **PPL 2.00** - PCBoard 15.2 enhancements
* **PPL 3.00** - PCBoard 15.3 with database support
* **PPL 3.10** - Minor updates and fixes
* **PPL 3.30** - Additional string handling
* **PPL 3.40** - PCBoard 15.4 final version
* **PPL 4.00** - IcyBoard extensions (default)

Command Line Usage
~~~~~~~~~~~~~~~~~~

Basic syntax::

    pplc [options] <file> 

The compiler automatically adds the `.pps` extension if not specified.

**Positional Arguments**
  * ``file`` - Source file to compile (e.g., ``myscript`` or ``myscript.pps``)

**Options**
  * ``-d, --disassemble`` - Output disassembly instead of compiling to PPE
  * ``--nowarnings`` - Suppress warning messages (errors still shown)
  * ``--version <ver>`` - Target PPE version (100, 200, 300, 310, 330, 340, 400)
  * ``--lang-version <ver>`` - Language version (defaults to ``--version`` value)
  * ``--cp437`` - Force CP437 encoding for DOS source files
  * ``--help`` - Display usage information

**User Variables**
  The compiler automatically determines whether user variables are needed based on your code. 
  The old ``--novars`` flag is no longer required - the compiler handles this optimization 
  automatically.

Character Encoding
~~~~~~~~~~~~~~~~~~

The compiler handles multiple character encodings to support both modern and legacy source files:

* **Default**: UTF-8 input with automatic conversion to CP437 for BBS display
* **DOS/Legacy**: Use ``--cp437`` flag for original DOS source files
* **Auto-detection**: The compiler attempts to detect encoding automatically

.. warning::
   All original DOS PPL files use CP437 encoding. It's recommended to convert them to 
   UTF-8 for modern development.

Examples
~~~~~~~~

**Compile a modern PPL script**::

    pplc myscript.pps

**Compile a legacy DOS script**::

    pplc --cp437 legacy.pps 

**Target specific PPE version**::

    pplc --version 340 myscript.pps 

Compatibility Notes
~~~~~~~~~~~~~~~~~~~

While maintaining high compatibility with the original PCBoard compiler, there are some 
minor differences:

* More permissive syntax in some areas
* Better error messages with more context 
* Automatic optimization of user variables
* Support for UTF-8 source files
* Extended functionality in version 4.00
* Some legacy .pps files may not compile due errors that were ignored by the original compiler.
  Most quirks from the old pplc are now treated as warnings. But the old compiler didn't find all 
  errors and produced broken PPE files (which may work because the invalid part is unused) in some cases.
  But they should be easy to fix.

PPL Projects
------------
PPL Projects are new in icy_board and provide a way to manage larger PPL codebases with multiple source files,
shared resources, and build configurations. 

**Creating a Project**

pplc --init new_project
    
    Initializes a new PPL project in the specified directory.

**Project Layout**

Projects using ``ppl.toml`` follow a standard directory structure::

    my_project/
    ├── ppl.toml           # Project configuration
    ├── src/               # Source files
    │   ├── main.pps       # Main entry point (sorted first)
    │   ├── utils.pps      # Additional modules
    │   └── menus.pps
    ├── target/            # Build output (auto-generated)
    │   ├── pcboard_15.40/ # Version-specific builds
    │   └── icboard/       # Default IcyBoard build
    ├── docs/              # Documentation
    └── art/               # ANSI art files

ppl.toml
~~~~~~~~

The ``ppl.toml`` file is the project configuration file for PPL projects, defining package metadata, 
compiler settings, and associated data files. It uses the TOML format for easy editing and version 
control.

**File Structure**

The configuration file consists of three main sections: ``[package]``, ``[compiler]``, and ``[data]``.

.. code-block:: toml

    [package]
    name = "my_ppe_project"
    version = "1.0.0"
    runtime = 400                    # Target PPE runtime version (optional)
    authors = ["Your Name"]          # List of authors (optional)

    [compiler]
    language_version = 400           # PPL language version (optional)
    defines = ["FEATURE_X", "DEBUG"] # Preprocessor defines (optional)

    [data]
    text_files = ["docs/readme.txt", "docs/help.txt"]   # Text files to include
    art_files = ["art/welcome.ans", "art/menu.ans"]     # ANSI art files

    [formatting]
    # Code formatting options for the PPL formatter (optional)
    indent_size = 4
    space_around_binop = true
    use_tabs = false

**Configuration Sections**

``[package]`` Section (Required)
  The package section defines the project's basic metadata:

  * ``name`` (string, required) - The name of your PPE project
  * ``version`` (string, required) - Semantic version (e.g., "1.0.0", "2.1.3")
  * ``runtime`` (integer, optional) - Target PPE runtime version:
    
    * ``100`` - PCBoard 15.0
    * ``200`` - PCBoard 15.1
    * ``300`` - PCBoard 15.2
    * ``310`` - PCBoard 15.21
    * ``320`` - PCBoard 15.22
    * ``330`` - PCBoard 15.3
    * ``340`` - PCBoard 15.4
    * ``400`` - IcyBoard (default)

  * ``authors`` (array of strings, optional) - List of project authors

``[compiler]`` Section (Optional)
  Controls compiler behavior:

  * ``language_version`` (integer, optional) - PPL language version to use (defaults to latest)
  * ``defines`` (array of strings, optional) - Preprocessor definitions for conditional compilation

``[data]`` Section (Optional)
  Specifies additional files to include with the compiled PPE:

  * ``text_files`` (array of strings, optional) - Text files to bundle
  * ``art_files`` (array of strings, optional) - ANSI art files to include (icy_draw *.icy files are converted automatically)

**Source File Discovery**

The compiler automatically discovers all ``.pps`` files in the ``src/`` directory:

* ``main.pps`` is always compiled first if present
* Subdirectories are included recursively

**Build Output**

Compiled files are placed in version-specific directories under ``target/``:

* ``target/pcboard_15.0/`` - For runtime version 100
* ``target/pcboard_15.10/`` - For runtime version 200
* ``target/pcboard_15.20/`` - For runtime version 300
* ``target/pcboard_15.21/`` - For runtime version 310
* ``target/pcboard_15.22/`` - For runtime version 320
* ``target/pcboard_15.30/`` - For runtime version 330
* ``target/pcboard_15.40/`` - For runtime version 340
* ``target/icboard/`` - For runtime version 400 (IcyBoard)

PPL Decompiler
--------------

Overview
~~~~~~~~
The decompiler (`ppld`) converts compiled PPE binaries back into readable PPL source
or a low-level disassembly. It’s useful for:

* Auditing legacy PPEs before migrating to IcyBoard
* Recovering lost source (best-effort reconstruction)
* Running a compatibility assessment (`--check`) against the current runtime
* Inspecting variable usage and low-level opcodes (disassembly mode)
* Analyzing PPE behavior without original source
* Security auditing of third-party PPEs

The decompiler attempts to reconstruct higher-level control structures (IF / WHILE /
SELECT / blocks) unless raw mode is requested.

Features
~~~~~~~~
* Structured source reconstruction (default)
* Raw linear form (`--raw`) with minimal structural recovery
* Disassembly mode (`-d`) showing opcodes & variable table
* Optional stdout output (`-o`) instead of writing a `.ppd` file
* Keyword casing styles: upper (default), lower, camel
* Compatibility scanner (`--check`) reporting unimplemented / unsupported usage

Command Line Usage
~~~~~~~~~~~~~~~~~~
Basic syntax::

    ppld [options] <file>

If `<file>` has no extension, `.ppe` is assumed.

Options
~~~~~~~
``-r, --raw``  
  Disable reconstruction of structured control flow (emit a direct linear form).

``-d, --disassemble``  
  Output a disassembly instead of regenerated PPL source. Shows:
  * Raw script buffer dump
  * Variable table (with inferred names if possible)
  * Instruction listing

``-o, --output``  
  Write decompiled PPL to stdout instead of creating a ``.ppd`` file.

``--check``  
  Perform a compatibility analysis and exit. Does not emit decompiled source.
  Reports:
  * Unimplemented (stubbed) statements/functions
  * Unsupported (intentionally not implemented) items
  * Partially implemented features (placeholder category)

``--style <u|l|c>``  
  Keyword casing: ``u`` = UPPER (default), ``l`` = lower, ``c`` = CamelCase.

``file``  
  PPE file to decompile (with or without ``.ppe``).

Generated Output
~~~~~~~~~~~~~~~~
Default (no ``-o`` / no disassembly) creates a sibling file with ``.ppd`` extension, e.g.::

    LOGIN.PPE  →  LOGIN.ppd

With ``-o`` the reconstructed source is printed to the console.

Disassembly Mode
~~~~~~~~~~~~~~~~
When ``-d`` is used:

1. Script buffer dump (hex/offset view)
2. Variable table (usage-analyzed, auto-named)
3. Instruction listing with addresses

This mode is ideal for debugging malformed or partially corrupt PPEs.

Example disassembly output::

    Script Buffer:
    0000: 50 50 45 00 03 40 00 00 ...
    
    Variable Table:
    [0000] STRING user_name
    [0001] INTEGER menu_choice
    [0002] BOOLEAN flag_1
    
    Instructions:
    0000: PUSH    "Welcome"
    0002: PCALL   PRINTLN
    0004: LET     [0000], U_NAME()
    ...

Raw Mode vs Structured
~~~~~~~~~~~~~~~~~~~~~~
``--raw`` disables control flow reconstruction heuristics. Use it when:

* The structured version looks wrong (edge cases in nested GOTOs)
* You want a representation closer to original opcode sequencing
* Debugging compiler output differences

Compatibility Checking
~~~~~~~~~~~~~~~~~~~~~~
The ``--check`` flag runs a static scan of all predefined statement/function calls
against internally curated status sets:

* Unimplemented: Compiles but runtime stub (calls log or does nothing)
* Unsupported: Intentionally not provided (e.g., obsolete hardware ops)
* Partial: Works but missing edge cases / minor behaviors

Example output::

    ppld --check DOORBANK.PPE

    Checking compatibility for: DOORBANK.ppe
    PPE Version: 400

    Compatibility Report
    --------------------------------------
    Unimplemented:
      [012A] STATEMENT DLOCK
      [045C] FUNCTION DRIVESPACE

    Summary: 2 references -> 2 unimplemented 0 unsupported 0 partial

Return code is non-zero only if an internal error occurs (not due to findings).  
Use this before deploying legacy PPEs into an IcyBoard runtime.

Example Workflows
~~~~~~~~~~~~~~~~~
Decompile to file::

    ppld login.ppe

Print to stdout in lower-case style::

    ppld --style l -o stats

Run compatibility audit only::

    ppld --check doors/league.ppe

View disassembly (no source reconstruction)::

    ppld -d puzzles

Combine disassembly with stdout (both honored)::

    ppld -d -o logic

(If ``--check`` is present it runs first and exits before other modes.)

Limitations & Notes
~~~~~~~~~~~~~~~~~~~
* Some complex GOTO / FALLTHROUGH patterns may not fully reconstruct; use ``--raw`` if unsure.
* Formatting is intentionally normalized (does not preserve original spacing).
* Variable names are inferred when possible; otherwise generic placeholders appear.
* If you add new runtime implementations, update the internal classification sets so the report stays accurate.
