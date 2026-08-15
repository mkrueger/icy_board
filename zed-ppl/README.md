# PPL for Zed

Editor support for the PCBoard Programming Language: the tree-sitter grammar
from this repository and the `icyboard-ppl` language server.

## What you get

- Syntax highlighting, bracket matching, auto-indent, outline and text objects
- Diagnostics, completion, hover, signature help, go to definition and
  references through the language server

## The language server

Nothing to install. On the first PPL file the extension fetches `icyboard-ppl`
from the newest IcyBoard release and keeps it until a release brings a newer one.

An `icyboard-ppl` on your `PATH` is used instead, so a local build wins over the
downloaded one:

```sh
cargo build --release --package icyboard-ppl
```

To point at one particular build, name it in your Zed settings:

```json
{
  "lsp": {
    "icyboard-ppl": {
      "binary": {
        "path": "/opt/icyboard/bin/icyboard-ppl"
      }
    }
  }
}
```

## Installing it in Zed

The extension is not in the Zed registry yet, so it is installed from this
directory as a dev extension.

1. Have a Rust toolchain from [rustup](https://rustup.rs). Zed compiles the
   extension and the grammar itself and needs nothing else; the `wasm32-wasip2`
   target and the wasi-sdk are fetched by Zed on the first build.
2. Clone this repository, if you have not already:

   ```sh
   git clone https://github.com/mkrueger/icy_board
   ```

3. In Zed open the command palette with <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>P</kbd>
   and run `zed: install dev extension`, or open `Extensions` and press
   `Install Dev Extension`.
4. Select the `zed-ppl` directory of the clone. The first build takes a few
   seconds; `Extensions` then lists `PPL` as a dev extension.
5. Open a `.pps` file. The status bar shows the language `PPL`, and the language
   server is downloaded on first use.

### Checking that it works

- Syntax colouring and the outline (<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>O</kbd>)
  come from the grammar.
- Hover, go to definition and diagnostics come from the language server. While it
  is being fetched the status bar says so.
- `zed: open log` shows what happened. A working start is silent; a failure names
  the reason, for example a platform without a prebuilt server.

### After changing the extension

Run `zed: reload extensions` to rebuild it. Changes to the grammar need a pushed
commit and a new `rev`, see below.

## After a grammar change

The grammar is pulled from this repository by commit. When `crates/tree-sitter-ppl`
changes, push the commit and update `rev` in `extension.toml`.
