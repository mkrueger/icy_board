Integration
-----------

IcyBoard ships a language server and a tree-sitter grammar for PPL, so an editor
can highlight, complete, navigate and check a source the same way the compiler
reads it.

============================  ==========================================================
Editor                        What it uses
============================  ==========================================================
Visual Studio Code            The extension, which starts the language server
Helix, Neovim, Zed, Emacs     The tree-sitter grammar, plus the language server over LSP
Anything else with LSP        The language server
============================  ==========================================================

The one command
~~~~~~~~~~~~~~~

From a checkout of the repository:

.. code-block:: bash

   tools/setup-editor.sh

That builds and installs ``ppl-language-server``, and sets up every editor it
finds: the parser and the queries land where Helix and Neovim look for them, and
the configuration is written unless there already is one. Run it again after
pulling; it leaves anything you changed alone.

To do one at a time:

.. code-block:: bash

   tools/setup-editor.sh server    # only the language server
   tools/setup-editor.sh helix
   tools/setup-editor.sh neovim

It needs `rustup <https://rustup.rs>`_ and a C compiler. The parser it builds is
the one checked into the repository, so no Node.js is involved.

What the language server offers
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

* Errors and warnings while typing, from the same compiler front end as ``pplc``
* Completion, including the fields of a record after a ``.``, the members of a
  board object such as ``CONFERENCE``, and the fields a record literal has not
  named yet
* Signature help for user routines, built-in functions and built-in statements
* Hover: what a built-in does, and the type of a variable, the signature of a
  routine, the fields of a record or the type of one of them
* Goto definition, find all references, rename, and the occurrences of the name
  under the cursor
* An outline of the file - its types with their fields, its routines and its
  variables
* Formatting, of the document or of a selection

Highlighting is left to the editor's own grammar - the tree-sitter grammar in
Helix and Neovim, the TextMate grammar the VS Code extension carries - so a file
stays coloured while it is half written and while the server is busy.

A directory with a ``ppl.toml`` is read as one package, so a type declared in one
file is known in the next.

Visual Studio Code
~~~~~~~~~~~~~~~~~~

Download ``ppl-language-server-X.X.X.vsix`` from
`the release page <https://github.com/mkrueger/icy_board/releases/latest>`_ and
drag it onto the VS Code window, or open it from ``Extensions: Install from
VSIX``.

The extension needs the ``ppl-language-server`` binary. Either put it on your
PATH - ``tools/setup-editor.sh server`` does - or point the setting
``ppl.serverPath`` at it. If it cannot be found, the extension says so and offers
to open the setting.

Helix
~~~~~

``tools/setup-editor.sh helix`` does all of this. By hand:

**Step 1: build the parser and copy the queries**

.. code-block:: bash

   cd crates/tree-sitter-ppl
   mkdir -p ~/.config/helix/runtime/grammars ~/.config/helix/runtime/queries/ppl
   cc -shared -fPIC -O1 -I src src/parser.c -o ~/.config/helix/runtime/grammars/ppl.so
   cp queries/*.scm ~/.config/helix/runtime/queries/ppl/

Helix can fetch and build the grammar itself, but it does not bring the queries
along, so highlighting stays off until they are copied.

**Step 2: describe the language**

Add to ``~/.config/helix/languages.toml``:

.. code-block:: toml

   [language-server.ppl-lsp]
   command = "ppl-language-server"

   [[language]]
   name = "ppl"
   scope = "source.ppl"
   injection-regex = "^ppl$"
   file-types = ["pps"]
   comment-token = ";"
   indent = { tab-width = 4, unit = "    " }
   language-servers = ["ppl-lsp"]
   roots = ["ppl.toml"]

**Step 3: check**

.. code-block:: bash

   hx --health ppl

which should answer:

.. code-block:: text

   Configured language servers:
     ✓ ppl-lsp: /home/you/.cargo/bin/ppl-language-server
   Tree-sitter parser: ✓
   Highlight queries: ✓
   Textobject queries: ✓
   Indent queries: ✓

Neovim
~~~~~~

``tools/setup-editor.sh neovim`` builds the parser into
``~/.local/share/nvim/site/parser/ppl.so``, copies the queries next to it and
writes two small files, unless you already have them:

``~/.config/nvim/ftdetect/ppl.lua``

.. code-block:: lua

   vim.filetype.add({ extension = { pps = "ppl" } })

``~/.config/nvim/ftplugin/ppl.lua``

.. code-block:: lua

   vim.treesitter.start()
   vim.bo.commentstring = "; %s"
   vim.lsp.start({
       name = "ppl-language-server",
       cmd = { "ppl-language-server" },
       root_dir = vim.fs.root(0, { "ppl.toml", ".git" }),
   })

With `nvim-treesitter <https://github.com/nvim-treesitter/nvim-treesitter>`_ the
parser can be installed from the repository instead; see
``crates/tree-sitter-ppl/README.md``.

Another editor
~~~~~~~~~~~~~~

Anything that speaks LSP only needs the binary and a file type for ``.pps``:

.. code-block:: bash

   cargo install --path crates/ppl-lsp

The grammar in ``crates/tree-sitter-ppl`` follows the usual tree-sitter layout,
so Zed, Emacs and the tree-sitter CLI read it as it is.
