Installation  
============

Getting started
~~~~~~~~~~~~~~~

Grab a binary distribution from https://github.com/mkrueger/icy_board/releases or build from source (see above).

I recommend putting the bin/ (or target/release if you build from source) directory in the path but you can just `cd bin` for now.

First create a new BBS: `./icbsetup create FOO`
Then start it: `./icboard FOO`

This will fire up a new call waiting screen where you can log in as sysop. By defaulut telnet is enabled on port 1337.

NOTE: Ensure that your terminal screen is big enough - 80x25 at least.

Building
~~~~~~~~

Prerequisites:
  * Rust toolchain (stable) — https://www.rust-lang.org/tools/install
  * A UTF-8 capable terminal (most modern terminals)
  * (Optional) VS Code for PPL editing

Build everything:

.. code-block:: bash

   git clone https://github.com/mkrueger/icy_board.git
   cd icy_board
   cargo build --release

This will create a target/release/ directory with all executables.


Create new BBS installations
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

1. Pick an identifier (letters / digits / underscore). Example: ``FOO``  
2. Create the instance:

   .. code-block:: bash

      ./icbsetup create FOO

3. Start it:

   .. code-block:: bash

      ./icboard FOO

Then the call waiting screen appears. You can access the setup or log in as user or sysop.

Import legacy PCBoard systems
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Icy Board can ingest an existing PCBoard installation directly from your original
``PCBOARD.DAT`` (plus the related files it references). The importer converts
binary/text formats into structured TOML, normalizes encodings to UTF-8 (with BOM
for display files), hashes passwords, and recreates conferences, commands,
security levels, protocols, colors, text resources, and user base metadata.

.. code-block:: bash

   ./icbsetup import /path/to/pcb /path/to/NEW_BBS_DIR

On success:

* Converted files populate ``NEW_BBS_DIR/``
* A log file is written to ``NEW_BBS_DIR/importlog.txt``
* You can start the board:

  .. code-block:: bash

     ./icboard /path/to/NEW_BBS_DIR

Limitations are that the importer may import wrong/old paths - they may need to be manually adjusted.
PPE plugins need to be manually converted as well.

Post-import tasks
~~~~~~~~~~~~~~~~~

1. See ``importlog.txt`` for warnings/errors (missing files, malformed records).
2. Manual convert PPE plugins (see below).
3. Test a migrated user:
   * Login
   * Read mail/conferences
   * Post a test message
4. Enable network services (telnet/ssh) only after verifying console launch works.


Converting PPE plugins to modern systems
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Even if .PPE files don't need to be recompiled (they may work as-is) they may need to be adjusted to work with Icy Board.
Most of them have a configuration that hint to old paths or files that don't exist anymore. So they need to be manually adjusted.

I recommend lowercasing all filenames and paths - Icy Board is case-sensitive as well as converting all text files to UTF-8 with BOM.

WARNING: Backup your original PPE files before conversion!

For that icbsetup has a PPE conversion assistant:

.. code-block:: bash
   
   ./icbsetup ppe-convert /path/to/ppe


This will lowercase all files and convert most fils from CP437 to UTF-8 with BOM. If a file is CP437 and is not converted.
(This is likely the case because there are plenty of text files with strange extensions).

Manual convert a single file with

.. code-block:: bash
   
   ./icbsetup ppe-convert /path/to/file.nfo

This will convert a single CP437 file to UTF-8 with BOM. The PPE engine will automatically detect the encoding and convert to CP437 if needed.
