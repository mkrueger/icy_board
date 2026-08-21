#!/bin/sh
# Sets up PPL support in an editor: the language server for anything that
# speaks LSP, and the tree-sitter grammar for Helix and Neovim.
#
#   tools/setup-editor.sh              installs the server and every editor found
#   tools/setup-editor.sh server       only the language server
#   tools/setup-editor.sh helix        only Helix
#   tools/setup-editor.sh neovim       only Neovim
#
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
grammar="$root/crates/tree-sitter-ppl"
[ -d "$grammar" ] || grammar="$root/tree-sitter-ppl"
config_home=${XDG_CONFIG_HOME:-$HOME/.config}
data_home=${XDG_DATA_HOME:-$HOME/.local/share}

say() { printf '%s\n' "$*"; }
note() { printf '  %s\n' "$*"; }

fail() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

# --------------------------------------------------------------------------
# The language server, which every editor with LSP support needs.
# --------------------------------------------------------------------------
install_server() {
    command -v cargo >/dev/null 2>&1 || fail "cargo is needed to build the language server - see https://rustup.rs"

    say "Building and installing ppl-lsp ..."
    cargo install --path "$root/crates/ppl-lsp" --quiet
    say "Installed ppl-lsp in ${CARGO_HOME:-$HOME/.cargo}/bin"

    if ! command -v ppl-lsp >/dev/null 2>&1; then
        note "That directory is not in your PATH yet. Add it, or point your editor at the binary."
    fi
    say ""
}

# --------------------------------------------------------------------------
# The parser, built the way an editor loads it.
# --------------------------------------------------------------------------
build_parser() {
    target=$1
    mkdir -p "$(dirname "$target")"
    "${CC:-cc}" -shared -fPIC -O1 -I "$grammar/src" "$grammar/src/parser.c" -o "$target"
}

copy_queries() {
    target=$1
    # A link means someone pointed the editor at a working copy on purpose.
    if [ -L "$target" ]; then
        return
    fi
    mkdir -p "$target"
    cp "$grammar"/queries/*.scm "$target"
}

# --------------------------------------------------------------------------
# Helix
# --------------------------------------------------------------------------
install_helix() {
    runtime="$config_home/helix/runtime"
    languages="$config_home/helix/languages.toml"

    say "Setting up Helix ..."
    build_parser "$runtime/grammars/ppl.so"
    note "parser  $runtime/grammars/ppl.so"
    copy_queries "$runtime/queries/ppl"
    note "queries $runtime/queries/ppl"

    if [ -f "$languages" ] && grep -q 'name *= *"ppl"' "$languages"; then
        note "languages.toml already knows PPL, leaving it alone"
    else
        mkdir -p "$(dirname "$languages")"
        [ -f "$languages" ] && cp "$languages" "$languages.backup"
        cat >>"$languages" <<'TOML'

[language-server.ppl-lsp]
command = "ppl-lsp"

[[language]]
name = "ppl"
scope = "source.ppl"
injection-regex = "^ppl$"
file-types = ["pps"]
comment-token = ";"
indent = { tab-width = 4, unit = "    " }
language-servers = ["ppl-lsp"]
roots = ["ppl.toml"]
TOML
        note "wrote  $languages"
    fi

    if command -v hx >/dev/null 2>&1; then
        say ""
        hx --health ppl | sed 's/^/  /'
    fi
    say ""
}

# --------------------------------------------------------------------------
# Neovim
# --------------------------------------------------------------------------
install_neovim() {
    site="$data_home/nvim/site"

    say "Setting up Neovim ..."
    build_parser "$site/parser/ppl.so"
    note "parser  $site/parser/ppl.so"
    copy_queries "$site/queries/ppl"
    note "queries $site/queries/ppl"

    ftdetect="$config_home/nvim/ftdetect/ppl.lua"
    if [ -f "$ftdetect" ]; then
        note "$ftdetect exists, leaving it alone"
    else
        mkdir -p "$(dirname "$ftdetect")"
        cat >"$ftdetect" <<'LUA'
vim.filetype.add({ extension = { pps = "ppl" } })
LUA
        note "wrote  $ftdetect"
    fi

    ftplugin="$config_home/nvim/ftplugin/ppl.lua"
    if [ -f "$ftplugin" ]; then
        note "$ftplugin exists, leaving it alone"
    else
        mkdir -p "$(dirname "$ftplugin")"
        cat >"$ftplugin" <<'LUA'
vim.treesitter.start()
vim.bo.commentstring = "; %s"
vim.lsp.start({
    name = "ppl-lsp",
    cmd = { "ppl-lsp" },
    root_dir = vim.fs.root(0, { "ppl.toml", ".git" }),
})
LUA
        note "wrote  $ftplugin"
    fi
    say ""
}

# --------------------------------------------------------------------------

what=${1:-all}
case "$what" in
server)
    install_server
    ;;
helix)
    install_helix
    ;;
neovim | nvim)
    install_neovim
    ;;
all)
    install_server
    command -v hx >/dev/null 2>&1 && install_helix
    command -v nvim >/dev/null 2>&1 && install_neovim
    say "For VS Code install the .vsix from the release page; it uses the server that was just installed."
    ;;
*)
    fail "unknown target '$what' - use server, helix, neovim or all"
    ;;
esac
