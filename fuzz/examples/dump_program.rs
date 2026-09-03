//! Renders a fuzz artifact back into the PPL source the target saw.

use arbitrary::{Arbitrary, Unstructured};
use icy_board_fuzz::Program;

fn main() {
    let path = std::env::args().nth(1).expect("usage: dump_program <artifact>");
    let bytes = std::fs::read(&path).expect("artifact");
    let unstructured = Unstructured::new(&bytes);
    let program = Program::arbitrary_take_rest(unstructured).expect("program");
    let source = program.render();
    eprintln!("language version: {}", program.language_version());
    eprintln!("source bytes: {}", source.len());
    eprintln!("lines: {}", source.lines().count());
    print!("{source}");
}
