Integration
-----------

IcyBoard comes with a Tree-sitter grammar and a Language Server for PPL.
This allows you to get syntax highlighting, code completion, and other IDE-like features 
in editors that support Tree-sitter and/or the Language Server Protocol (LSP).

Put `ppl-language-server` somewhere in your PATH. This is the easiest way to make sure your editor can find it.

Visual Studio Code
~~~~~~~~~~~~~~~~~~

Download and install the PPL extension from `https://github.com/mkrueger/icy_board/releases/latest`

Extract the VSIX file (ppl-language-server-X.X.X.vsix) and install it.
Just open the VSIX file with VS Code and it will prompt you to install the extension.
Or drag and drop the VSIX file onto the VS Code window.

Helix
~~~~~

Helix has built-in support for both Tree-sitter grammars and language servers. Here's how to configure PPL support:

**Step 1: Configure the Language**

Add the following to your ``~/.config/helix/languages.toml``:

.. code-block:: toml

   [[grammar]]
   name = "ppl"
   source = { git = "https://github.com/mkrueger/icy_board/", rev = "main", subpath = "crates/tree-sitter-ppl" }

   [language-server.ppl-lsp]
   command = "ppl-language-server"

   [[language]]
   name = "ppl"
   scope = "source.ppl"
   injection-regex = "^ppl$"
   grammar = "ppl"
   file-types = ["pps", "ppx", "ppd"]
   comment-token = ";"
   indent = { tab-width = 4, unit = "    " }
   language-servers = ["ppl-lsp"]


**Step 2: Build and Install the Grammar**

.. code-block:: bash

    hx --grammar fetch
    hx --grammar build

**Step 3: Verify the Installation**

.. code-block:: bash

    helix --health ppl

This should show:

`Tree-sitter parser: ✓
Highlight queries: ✓`

