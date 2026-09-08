//! Records the commit the binary was built from. A release tarball has no
//! checkout, so an empty hash is a normal result, never a build failure.
fn main() {
    icy_board_buildinfo::emit_git_hash();
}
