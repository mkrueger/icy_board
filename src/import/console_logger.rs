use std::io::stdout;

use crossterm::{
    execute,
    style::{Attribute, Color, Print, SetAttribute, SetForegroundColor},
};
use icy_ppe::Res;

use super::OutputLogger;

#[derive(Default)]
pub struct ConsoleLogger {}

impl OutputLogger for ConsoleLogger {
    fn start_action(&self, message: String) {
        execute!(
            stdout(),
            SetAttribute(Attribute::Bold),
            Print(message),
            SetAttribute(Attribute::Reset)
        )
        .unwrap();
        println!();
    }

    fn check_error(&self, res: Option<std::io::Error>) -> Res<()> {
        match res {
            None => Ok(()),
            Some(e) => {
                print_error(e.to_string());
                Err(e.into())
            }
        }
    }

    fn warning(&self, message: String) {
        execute!(
            stdout(),
            SetAttribute(Attribute::Bold),
            SetForegroundColor(Color::Yellow),
            Print("Warning:".to_string()),
            SetAttribute(Attribute::Reset),
            Print(format!(" {}\n", message))
        )
        .unwrap();
    }
}

pub fn print_error(e: impl Into<String>) {
    execute!(
        stdout(),
        SetAttribute(Attribute::Bold),
        SetForegroundColor(Color::Red),
        Print("Error:".to_string()),
        SetAttribute(Attribute::Reset),
        Print(format!(" {}\n", e.into()))
    )
    .unwrap();
}
