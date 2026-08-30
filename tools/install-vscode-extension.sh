#!/bin/sh
# Builds the local PPL VS Code extension, bundles the language server and
# installs the resulting VSIX in VS Code.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
extension="$root/editors/vscode"

fail() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

command -v cargo >/dev/null 2>&1 || fail "cargo is required"
command -v node >/dev/null 2>&1 || fail "node is required"
command -v pnpm >/dev/null 2>&1 || fail "pnpm is required"
command -v code >/dev/null 2>&1 || fail "the VS Code command-line tool 'code' is required"

printf '%s\n' "Building ppl-lsp ..."
cargo build --manifest-path "$root/Cargo.toml" --release --package ppl-lsp

printf '%s\n' "Bundling ppl-lsp ..."
rm -rf "$extension/server"
mkdir -p "$extension/server"
cp "$root/target/release/ppl-lsp" "$extension/server/ppl-lsp"
chmod +x "$extension/server/ppl-lsp"

printf '%s\n' "Building the VS Code extension ..."
cd "$extension"
pnpm install --frozen-lockfile
pnpm run compile

version=$(node -p "require('./package.json').version")
vsix="ppl-vscode-$version.vsix"
rm -f "$vsix"
pnpm exec vsce package --no-dependencies -o "$vsix"

printf '%s\n' "Installing $vsix ..."
code --install-extension "$extension/$vsix" --force

printf '%s\n' "Installed $extension/$vsix"