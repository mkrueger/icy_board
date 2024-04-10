use std::env;

use jamjam::jam::JamMessageBase;

pub fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: read_jam <filename>");
        std::process::exit(1);
    }

    let base = JamMessageBase::open(&args[1]).unwrap();

    for header in base.iter().flatten() {
        println!(
            "From:{:40}  Number:{}/{}",
            header.get_from().unwrap(),
            header.message_number - base.base_messagenumber() + 1,
            base.active_messages()
        );
        println!("From:{}", header.get_to().unwrap());
        println!("Subj:{}", header.get_subject().unwrap());
        println!("--------------------------------------------------");
        println!("{}", base.read_msg_text(&header).unwrap());
    }
}
