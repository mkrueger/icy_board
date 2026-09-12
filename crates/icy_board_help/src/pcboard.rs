use codepages::tables::CP437_TO_UNICODE;
use serde::{Deserialize, Serialize};

use crate::{HelpTheme, Result, document, invalid};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Encoding {
    #[default]
    Utf8,
    Cp437,
}

#[derive(Clone, Debug)]
pub struct RenderOptions {
    pub width: usize,
    pub encoding: Encoding,
    pub theme: HelpTheme,
    pub clear_screen: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            width: 79,
            encoding: Encoding::Utf8,
            theme: HelpTheme::default(),
            clear_screen: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderedHelp {
    pub bytes: Vec<u8>,
    /// Wrapped semantic text with LF endings, excluding decorative underlines.
    pub plain_text: String,
}

pub fn render(markdown: &str, options: &RenderOptions) -> Result<RenderedHelp> {
    let document = document::compile(markdown, options.width, &options.theme)?;
    let plain_text = document.plain_text();
    let mut output = String::new();
    if options.clear_screen {
        output.push_str(&format!("@X{:02X}@CLS@", options.theme.body));
    }
    for line in &document.lines {
        // Prefix even empty lines before the display-file dispatcher sees literal !/$/%.
        let mut attribute = options.theme.attribute(line.spans.first().map_or(document::Role::Body, |span| span.role));
        output.push_str(&format!("@X{attribute:02X}"));
        for span in &line.spans {
            let next = options.theme.attribute(span.role);
            if next != attribute {
                output.push_str(&format!("@X{next:02X}"));
                attribute = next;
            }
            for ch in span.text.chars() {
                if ch == '@' {
                    // write_raw leaves @@ in GotAt; XFF consumes it without changing color.
                    output.push_str("@@XFF");
                } else {
                    output.push(ch);
                }
            }
        }
        output.push_str("\r\n");
    }
    output.push_str("@X07");
    let bytes = match options.encoding {
        Encoding::Utf8 => {
            let mut bytes = vec![0xEF, 0xBB, 0xBF];
            bytes.extend_from_slice(output.as_bytes());
            bytes
        }
        Encoding::Cp437 => output.chars().map(encode_cp437).collect::<Result<Vec<_>>>()?,
    };
    Ok(RenderedHelp { bytes, plain_text })
}

fn encode_cp437(ch: char) -> Result<u8> {
    CP437_TO_UNICODE
        .iter()
        .position(|&candidate| candidate == ch)
        .map(|index| index as u8)
        .ok_or_else(|| invalid(format!("Character {ch:?} (U+{:04X}) is not representable in CP437", ch as u32)))
}
