# PPL for Zed

Editor support for the PCBoard Programming Language: the tree-sitter grammar
from this repository and the `icyboard-ppl` language server.

## What you get

- Syntax highlighting, bracket matching, auto-indent, outline and text objects
- Diagnostics, completion, hover, signature help, go to definition and
  references through the language server

## Requirements

The extension does not ship a binary. Build the tools of this repository and put
them on your `PATH`:

```sh
cargo build --release
```

The server is `target/release/icyboard-ppl`. If you keep it somewhere else, name
the path in your Zed settings:

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
