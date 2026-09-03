//! Renders a fuzz artifact of any generated kind back into the PPL source the target saw.

use arbitrary::{Arbitrary, Unstructured};
use icy_board_fuzz::{MutatedSource, Preprocessed, Program};

fn main() {
    let mut args = std::env::args().skip(1);
    let kind = args.next().expect("usage: dump_source <program|preprocessed|mutated> <artifact>");
    let path = args.next().expect("usage: dump_source <program|preprocessed|mutated> <artifact>");
    let bytes = std::fs::read(&path).expect("artifact");
    let unstructured = Unstructured::new(&bytes);

    let (version, source) = match kind.as_str() {
        "program" => {
            let value = Program::arbitrary_take_rest(unstructured).expect("program");
            (value.language_version(), value.render())
        }
        "preprocessed" => {
            let value = Preprocessed::arbitrary_take_rest(unstructured).expect("preprocessed");
            (value.language_version(), value.render())
        }
        "mutated" => {
            let value = MutatedSource::arbitrary_take_rest(unstructured).expect("mutated");
            (value.language_version(), value.render())
        }
        other => panic!("unknown kind {other}"),
    };

    eprintln!("language version: {version}");
    eprintln!("source bytes: {}", source.len());
    eprintln!("lines: {}", source.lines().count());
    print!("{source}");
}
