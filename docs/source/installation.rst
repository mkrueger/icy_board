Installation
============

IcyBoard is a set of command-line programs. There is no installer and nothing
is written outside the directory into which the release is unpacked. A board
needs a terminal of at least 80 columns by 25 rows for the setup tools.

Supported systems
-----------------

==============================  ==========  ===========
System                          Prebuilt    From source
==============================  ==========  ===========
Linux x86_64                    yes         yes
Windows x86_64                  yes         yes
macOS, Apple Silicon and Intel  yes         yes
Raspberry Pi and other Linux    no          yes
==============================  ==========  ===========

Installing a release
--------------------

Download the archive for your system from the `IcyBoard releases`_. The archive
contains a ``bin/`` directory with every program. Unpack it wherever you like
and put ``bin/`` on your ``PATH``, so the tools can find one another and can be
called from a board directory.

.. csv-table:: Release archives
   :header: "Release file", "System"
   :widths: 65, 35

   "``icy_board_linux_<version>.zip``", "Linux x86_64"
   "``icy_board_windows_<version>.zip``", "Windows x86_64"
   "``icy_board_osx_aarch64-apple-darwin_<version>.zip``", "macOS, Apple Silicon"
   "``icy_board_osx_x86_64-apple-darwin_<version>.zip``", "macOS, Intel"

.. _IcyBoard releases: https://github.com/mkrueger/icy_board/releases

Building from source
--------------------

Building needs a current `Rust toolchain`_. On Raspberry Pi and similar Linux
systems the OpenSSL development package may be required as well.

.. code-block:: bash

   git clone https://github.com/mkrueger/icy_board
   cd icy_board
   cargo build --release

The programs are written to ``target/release/``.

.. _Rust toolchain: https://rustup.rs

Creating the first board
------------------------

.. code-block:: bash

   icbsetup create mybbs
   cd mybbs
   icboard

``icbsetup`` writes a complete board into ``mybbs/`` and prints the randomly
generated initial sysop password once. Keep it until the first login. ``icboard``
reads ``icboard.toml`` in the current directory; alternatively, ``ICB_PATH`` may
name the board directory. ``icboard --localon`` opens a local sysop session
immediately. Telnet is enabled on port 1337 in a newly created board.

Importing an existing board
---------------------------

An existing installation can be imported from its ``PCBOARD.DAT``. The source
installation is read but never changed, and the destination must not already
exist.

.. code-block:: bash

   icbsetup import /path/to/PCBOARD.DAT mybbs

The result is a starting point rather than a finished migration. Read
``mybbs/importlog.txt``, check paths that referred to the old installation, test
a migrated user and inspect third-party PPE configuration files before opening
the network listeners.

Converting PPE data files
-------------------------

Legacy PPE binaries can usually run unchanged. Their configuration and display
files often contain DOS paths, uppercase names or CP437 text, however. Back up
the directory, then let ``icbsetup`` lowercase its names and convert recognized
text files to UTF-8 with a BOM:

.. code-block:: bash

   icbsetup ppe-convert /path/to/ppe-directory

Passing one file converts that file only. Files with unusual extensions may
need to be converted manually because the tool deliberately avoids guessing
that arbitrary binary data is text.

Installed programs
------------------

=================  ============================================================
Program            Purpose
=================  ============================================================
``icboard``         Board server and local call-waiting screen
``icbsetup``        Board creation, import and interactive configuration
``icbsm``           User and group management, packing and bulk changes
``mkicbtxt``        System-message editor
``mkicbmnu``        Menu editor
``icbfile``         File-base maintenance and import
``icbmailer``       FTN mail scanning, polling and tossing
``pplc``            PPL compiler, package builder and formatter
``ppld``            PPE decompiler and compatibility checker
``icyboard-ppl``    PPL language server for editors
=================  ============================================================

PPL editor support
------------------

Editor support consists of a grammar for highlighting, folding and indentation,
plus the ``icyboard-ppl`` language server for diagnostics, completion, hover,
signature help, definitions and references.

* **VS Code:** install the platform-specific ``.vsix`` from the IcyBoard release.
* **Zed:** install the **PPL** extension; it obtains the language server from the
  IcyBoard releases. Until it is in the registry, install the extension from
  https://github.com/mkrueger/zed-ppl as a development extension.
* **Helix and Neovim:** from a source checkout, run
  ``tools/setup-editor.sh helix`` or ``tools/setup-editor.sh neovim``.
* **Other LSP editors:** run ``icyboard-ppl`` over standard input/output for
  ``.pps`` files. The server takes no arguments.
