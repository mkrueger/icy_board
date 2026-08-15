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

## Installing it locally

Zed compiles the extension itself, so a Rust toolchain installed through rustup
is enough. In Zed open the extensions page, choose `Install Dev Extension` and
select this directory. Open a `.pps` file afterwards.

## After a grammar change

The grammar is pulled from this repository by commit. When `crates/tree-sitter-ppl`
changes, push the commit and update `rev` in `extension.toml`.
