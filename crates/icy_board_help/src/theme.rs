use serde::{Deserialize, Serialize};

use crate::{Result, document::Role, invalid};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct HelpTheme {
    pub title: u8,
    pub heading: u8,
    pub body: u8,
    pub emphasis: u8,
    pub code: u8,
    pub note: u8,
    pub border: u8,
    /// Left and right inset, measured in terminal cells.
    pub margin: usize,
    /// Draw renderer-owned heading underlines.
    pub decoration: bool,
}

impl Default for HelpTheme {
    fn default() -> Self {
        Self {
            title: 0x0F,
            heading: 0x0B,
            body: 0x07,
            emphasis: 0x0E,
            code: 0x0A,
            note: 0x0D,
            border: 0x03,
            margin: 0,
            decoration: true,
        }
    }
}

impl HelpTheme {
    pub fn preset(name: &str) -> Result<Self> {
        match name {
            "classic" => Ok(Self::default()),
            "minimal" => Ok(Self {
                title: 0x0F,
                heading: 0x0F,
                body: 0x07,
                emphasis: 0x0F,
                code: 0x07,
                note: 0x07,
                border: 0x07,
                margin: 0,
                decoration: false,
            }),
            _ => Err(invalid(format!("Unknown help theme preset: {name}"))),
        }
    }

    pub fn validate(&self) -> Result<()> {
        for (name, attr) in [
            ("title", self.title),
            ("heading", self.heading),
            ("body", self.body),
            ("emphasis", self.emphasis),
            ("code", self.code),
            ("note", self.note),
            ("border", self.border),
        ] {
            if attr == 0 || attr >= 0x80 {
                return Err(invalid(format!("Help theme {name} attribute {attr:02X} is special or blinking; use 01..7F")));
            }
        }
        if self.margin > 19 {
            return Err(invalid("Help theme margin must be in 0..=19"));
        }
        Ok(())
    }

    pub fn attribute(&self, role: Role) -> u8 {
        match role {
            Role::Title => self.title,
            Role::Heading => self.heading,
            Role::Body => self.body,
            Role::Emphasis => self.emphasis,
            Role::Code => self.code,
            Role::Note => self.note,
            Role::Border => self.border,
        }
    }
}
