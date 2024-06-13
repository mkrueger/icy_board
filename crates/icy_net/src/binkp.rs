use std::fmt::Display;

pub enum Commands {
    NUL = 0,
    ADR = 1,
    PWD = 2,
    FILE = 3,
    OK = 4,
    EOB = 5,
    GOT = 6,
    ERR = 7,
    BSY = 8,
    GET = 9,
    SKIP = 10,
}

impl Display for Commands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Commands::NUL => write!(f, "M_NUL"),
            Commands::ADR => write!(f, "M_ADR"),
            Commands::PWD => write!(f, "M_PWD"),
            Commands::FILE => write!(f, "M_FILE"),
            Commands::OK => write!(f, "M_OK"),
            Commands::EOB => write!(f, "M_EOB"),
            Commands::GOT => write!(f, "M_GOT"),
            Commands::ERR => write!(f, "M_ERR"),
            Commands::BSY => write!(f, "M_BSY"),
            Commands::GET => write!(f, "M_GET"),
            Commands::SKIP => write!(f, "M_SKIP"),
        }
    }
}
/*
M_NUL "SYS ..."                | M_NUL "SYS ..."                |
| M_NUL "ZYZ ..."                | M_NUL "ZYZ ..."                |
| M_NUL "LOC ..."                | M_NUL "LOC ..."                |
| M_NUL "VER ..."                | M_NUL "VER ..."                |
| M_NUL "OPT ..."                | M_NUL "OPT ..."                |
| M_ADR "1:1/1.1@fidonet"        | M_ADR "2:2/2.2@fidonet"        |
| M_PWD "password"              */
