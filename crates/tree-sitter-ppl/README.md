# tree-sitter-ppl

A [tree-sitter](https://tree-sitter.github.io) grammar for PPL, the PCBoard
Programming Language, as IcyBoard implements it.

It covers PPL 1.00 through 4.01: the classic PCBoard statements and the later
additions - `REPEAT`/`LOOP`, bracket indexing, brace array initializers, the dot
operator with the board objects, `TYPE ... ENDTYPE`, record literals and
routines passed as parameters.

Every PPL source in the IcyBoard repository parses without an error; the
`repository_sources` test keeps it that way.

## Files

| Path | Contents |
| :--- | :--- |
| `grammar.js` | The grammar |
| `queries/highlights.scm` | Syntax highlighting |
| `queries/locals.scm` | Scopes, definitions and references |
| `queries/folds.scm` | Foldable regions |
| `queries/indents.scm` | Indentation |
| `test/corpus/` | Parser tests |

## Building

```bash
npm install -g tree-sitter-cli   # or: cargo install tree-sitter-cli
tree-sitter generate
tree-sitter test
```

`tree-sitter generate` runs `grammar.js`, so it needs Node.js. The generated
parser in `src/` is checked in, so an editor does not.

## Neovim

With [nvim-treesitter](https://github.com/nvim-treesitter/nvim-treesitter):

```lua
vim.filetype.add({ extension = { pps = "ppl" } })

require("nvim-treesitter.parsers").get_parser_configs().ppl = {
  install_info = {
    url = "https://github.com/mkrueger/icy_board",
    location = "crates/tree-sitter-ppl",
    files = { "src/parser.c" },
    branch = "main",
  },
  filetype = "ppl",
}
```

Then `:TSInstall ppl` and copy the `queries/` directory to
`~/.config/nvim/queries/ppl/`.

## Helix

Add to `~/.config/helix/languages.toml`:

```toml
[[language]]
name = "ppl"
scope = "source.ppl"
file-types = ["pps"]
comment-token = ";"
indent = { tab-width = 4, unit = "    " }

[[grammar]]
name = "ppl"
source = { git = "https://github.com/mkrueger/icy_board", subpath = "crates/tree-sitter-ppl", rev = "main" }
```

Then `hx --grammar fetch && hx --grammar build` and copy `queries/` to
`~/.config/helix/runtime/queries/ppl/`.

## Rust

```rust
let mut parser = tree_sitter::Parser::new();
parser.set_language(&tree_sitter_ppl::LANGUAGE.into())?;
let tree = parser.parse(source, None).unwrap();
```

`HIGHLIGHTS_QUERY`, `LOCALS_QUERY`, `FOLDS_QUERY` and `INDENTS_QUERY` carry the
query files.

## What the grammar cannot know

PPL is not line oriented in the grammar, the way the compiler is not either:
`WHILE cond` may be followed by its single statement on the next line. Two
places pay for that:

* A built-in statement that stands alone and is followed by a line naming
  another built-in - `CLS` then `NEWLINE` - reads as two statements, which is
  what a program means. A built-in name used as the only argument of another -
  `PRINTLN color` with a variable called `color` - reads as two statements too,
  which is not. Give the argument a companion (`PRINTLN color, ""`) or rename
  the variable.
* `RETURN` followed by a line that could be an expression prefers to leave the
  line alone, so `RETURN` and a following call stay separate statements.

`*` comments are not supported. The compiler only reads them at the start of a
line, which a tree-sitter grammar cannot tell apart from a multiplication.
