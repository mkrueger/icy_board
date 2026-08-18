use std::{path::PathBuf, sync::Arc};

use dizbase::file_base::{
    FileBase,
    file_header::FileHeader,
    metadata::{MetadataHeader, MetadataType},
};
use humanize_bytes::humanize_bytes_decimal;
use tokio::sync::Mutex;

use crate::{
    Res,
    icy_board::{icb_text::IceText, state::IcyBoardState},
    vm::TerminalTarget,
};

/// Where a description starts, once the name, size and date columns are written.
const DESCRIPTION_COLUMN: usize = 33;

/// Keeps a description inside its column.
///
/// A FILE_ID.DIZ is drawn for a screen that starts at column 0, and the ANSI ones
/// usually open with a cursor-back to get there - `ESC[255D` is common. Printed as
/// it stands, that walks over the name and size of the file it belongs to. The
/// movement is shortened rather than dropped, so the art keeps its shape and its
/// colours and only loses the part that would leave the column.
fn clamp_to_column(line: &str, left: usize) -> String {
    let mut out = String::with_capacity(line.len());
    let mut column = left;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\r' {
            // A carriage return is a cursor-back to column zero by another name.
            if column > left {
                out.push_str(&format!("\x1b[{}D", column - left));
                column = left;
            }
            continue;
        }
        if ch != '\x1b' || chars.peek() != Some(&'[') {
            out.push(ch);
            if !ch.is_control() {
                column += 1;
            }
            continue;
        }

        chars.next();
        let mut params = String::new();
        let mut final_byte = None;
        for c in chars.by_ref() {
            if c.is_ascii_digit() || c == ';' || c == '?' {
                params.push(c);
            } else {
                final_byte = Some(c);
                break;
            }
        }
        let Some(final_byte) = final_byte else {
            break;
        };
        let first = || params.split(';').next().unwrap_or("").parse::<usize>().unwrap_or(1).max(1);

        match final_byte {
            'D' => {
                let wanted = first();
                let room = column - left;
                if room > 0 {
                    out.push_str(&format!("\x1b[{}D", wanted.min(room)));
                    column -= wanted.min(room);
                }
            }
            'C' => {
                let by = first();
                out.push_str(&format!("\x1b[{by}C"));
                column += by;
            }
            'G' => {
                let wanted = first().max(left + 1);
                out.push_str(&format!("\x1b[{wanted}G"));
                column = wanted - 1;
            }
            _ => {
                out.push('\x1b');
                out.push('[');
                out.push_str(&params);
                out.push(final_byte);
            }
        }
    }
    out
}

/// Takes the colours out of a description and leaves everything else standing.
///
/// A DIZ carries whatever colours its author liked, including a reset that puts
/// the caller back to the terminal default and so loses the colour the board
/// chose for the column. A sysop who wants the listing to look like their board
/// rather than like 1994 turns `strip_colors_in_descriptions` on.
fn strip_colors(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '\x1b' || chars.peek() != Some(&'[') {
            out.push(ch);
            continue;
        }
        chars.next();
        let mut params = String::new();
        let mut final_byte = None;
        for c in chars.by_ref() {
            if c.is_ascii_digit() || c == ';' || c == '?' {
                params.push(c);
            } else {
                final_byte = Some(c);
                break;
            }
        }
        let Some(final_byte) = final_byte else {
            break;
        };
        if final_byte != 'm' {
            out.push('\x1b');
            out.push('[');
            out.push_str(&params);
            out.push(final_byte);
        }
    }
    out
}

/// Decides which files a listing shows.
///
/// The first stage only looks at the index entry, which is already in memory. The second
/// stage is optional and is the only reason to touch a file's description, so a plain
/// listing or a name search never pays for reading - or scanning - metadata it discards.
pub struct FileFilter {
    accepts: Box<dyn Fn(&FileHeader) -> bool>,
    accepts_described: Option<Box<dyn Fn(&FileHeader, &[MetadataHeader]) -> bool>>,
}

impl FileFilter {
    /// Shows everything in the area.
    pub fn all() -> Self {
        Self::header(|_| true)
    }

    /// Decides from the index entry alone.
    pub fn header(accepts: impl Fn(&FileHeader) -> bool + 'static) -> Self {
        Self {
            accepts: Box::new(accepts),
            accepts_described: None,
        }
    }

    /// Narrows by index entry first and consults the description only for what survives.
    pub fn with_description(
        accepts: impl Fn(&FileHeader) -> bool + 'static,
        accepts_described: impl Fn(&FileHeader, &[MetadataHeader]) -> bool + 'static,
    ) -> Self {
        Self {
            accepts: Box::new(accepts),
            accepts_described: Some(Box::new(accepts_described)),
        }
    }
}

pub struct FileList {
    pub path: PathBuf,
    pub files: Arc<Mutex<FileBase>>,
}

impl FileList {
    pub fn new(path: PathBuf, files: Arc<Mutex<FileBase>>) -> Self {
        Self { path, files }
    }

    pub async fn display_file_list(&mut self, cmd: &mut IcyBoardState, filter: FileFilter) -> Res<()> {
        let short_header = if let Some(user) = &cmd.session.current_user {
            user.flags.use_short_filedescr
        } else {
            false
        };
        cmd.session.disp_options.in_file_list = Some(self.path.clone());
        let colors = cmd.get_board().await.config.color_configuration.clone();
        let dir = self.files.lock().await.dir().to_path_buf();
        let show_uploader = cmd.board.lock().await.config.file_transfer.display_uploader;
        let strip_description_colors = cmd.board.lock().await.config.file_transfer.strip_colors_in_descriptions;
        let sysop_name = cmd.board.lock().await.config.sysop.name.clone();
        let headers = self.files.lock().await.clone();
        for entry in headers.iter() {
            if !(filter.accepts)(entry) {
                continue;
            }
            let full_path = dir.join(entry.name());
            // Only reached by files that are candidates, so an area is not scanned wholesale
            // just to list a handful of matches.
            let meta_data = self.files.lock().await.read_metadata(&full_path)?;
            if let Some(accepts_described) = &filter.accepts_described {
                if !accepts_described(entry, &meta_data) {
                    continue;
                }
            }
            if cmd.session.request_logoff {
                break;
            }
            if cmd.session.disp_options.abort_printout {
                break;
            }
            let date = entry.date();
            let size = entry.size();
            let name = entry.name();
            cmd.set_color(TerminalTarget::Both, colors.file_name.clone()).await?;
            if cmd.session.search_pattern.is_some() {
                cmd.print_found_text(TerminalTarget::Both, &format!("{:<12} ", name)).await?;
            } else {
                cmd.print(TerminalTarget::Both, &format!("{:<12} ", name)).await?;
            }
            if name.len() > 12 {
                cmd.new_line().await?;
            }

            if dir.join(entry.name()).exists() {
                cmd.set_color(TerminalTarget::Both, colors.file_size.clone()).await?;
                cmd.print(TerminalTarget::Both, &format!("{:>8}  ", humanize_bytes_decimal!(size).to_string()))
                    .await?;
            } else {
                cmd.set_color(TerminalTarget::Both, colors.file_offline.clone()).await?;
                cmd.print(TerminalTarget::Both, &format!("{:>8}  ", "Offline".to_string())).await?;
            }

            cmd.set_color(TerminalTarget::Both, colors.file_date.clone()).await?;
            cmd.print(TerminalTarget::Both, &format!("{}", date.format("%m/%d/%y"))).await?;
            if false {
                cmd.set_color(TerminalTarget::Both, colors.file_new_file.clone()).await?;
                cmd.print(TerminalTarget::Both, "*").await?;
                cmd.reset_color(TerminalTarget::Both).await?;
                cmd.print(TerminalTarget::Both, " ").await?;
            } else {
                cmd.print(TerminalTarget::Both, "  ").await?;
            }

            let mut printed_lines = false;
            let mut first_line = true;
            for m in &meta_data {
                if m.get_type() == MetadataType::FileID {
                    let description = std::str::from_utf8(&m.data)?;
                    cmd.set_color(TerminalTarget::Both, colors.file_description.clone()).await?;
                    for line in description.lines() {
                        if cmd.session.disp_options.abort_printout {
                            break;
                        }
                        if first_line {
                            first_line = false;
                        } else {
                            cmd.print(TerminalTarget::Both, &format!("{:33}", " ")).await?;
                        }
                        let line = clamp_to_column(line, DESCRIPTION_COLUMN);
                        let line = if strip_description_colors { strip_colors(&line) } else { line };
                        if cmd.session.search_pattern.is_some() {
                            cmd.print_found_text(TerminalTarget::Both, &line).await?;
                        } else {
                            cmd.print(TerminalTarget::Both, &line).await?;
                        }
                        cmd.new_line().await?;
                        printed_lines = true;
                        if short_header {
                            break;
                        }
                        cmd.set_color(TerminalTarget::Both, colors.file_description_low.clone()).await?;
                    }
                }
            }
            if show_uploader {
                if !first_line {
                    cmd.print(TerminalTarget::Both, &format!("{:33}", " ")).await?;
                }
                let mut uploader = None;
                for m in &meta_data {
                    if m.get_type() == MetadataType::Uploader {
                        uploader = Some(std::str::from_utf8(&m.data)?.to_string());
                        break;
                    }
                }

                if let Ok(line) = cmd.get_display_text(IceText::UploadedBy) {
                    cmd.set_color(TerminalTarget::Both, colors.file_description.clone()).await?;
                    cmd.session.op_text = uploader.unwrap_or_else(|| sysop_name.clone());
                    if cmd.session.search_pattern.is_some() {
                        cmd.print_found_text(TerminalTarget::Both, &line).await?;
                    } else {
                        cmd.print(TerminalTarget::Both, &line).await?;
                    }
                    cmd.new_line().await?;
                }
            }
            if !printed_lines {
                cmd.new_line().await?;
            }
        }
        cmd.session.disp_options.in_file_list = None;
        Ok(())
    }
}

#[cfg(test)]
mod description_tests {
    use super::{DESCRIPTION_COLUMN, clamp_to_column};

    /// The line that started this: the first line of the FILE_ID.DIZ in
    /// 3nt1094.zip walks back 255 columns before writing anything.
    #[test]
    fn a_cursor_back_cannot_leave_the_column() {
        let line = "\x1b[255D\x1b[0m \x1b[CTHE iNCREDiBlE WASTE oF TiME";
        let clamped = clamp_to_column(line, DESCRIPTION_COLUMN);

        assert!(!clamped.contains("255D"), "the walk back is still there: {clamped:?}");
        assert!(!clamped.contains("\x1b[D"), "an empty move should be dropped: {clamped:?}");
        assert!(clamped.contains("\x1b[0m"), "colour must survive: {clamped:?}");
        assert!(clamped.contains("\x1b[1C"), "forward movement must survive: {clamped:?}");
    }

    /// Walking back over text that was written inside the column is fine.
    #[test]
    fn a_cursor_back_inside_the_column_is_left_alone() {
        let clamped = clamp_to_column("abcdef\x1b[3D", DESCRIPTION_COLUMN);
        assert_eq!(clamped, "abcdef\x1b[3D");
    }

    /// Only the part that would leave the column is taken off.
    #[test]
    fn a_cursor_back_is_shortened_rather_than_dropped() {
        let clamped = clamp_to_column("abc\x1b[10D", DESCRIPTION_COLUMN);
        assert_eq!(clamped, "abc\x1b[3D");
    }

    /// A carriage return is the same move under another name.
    #[test]
    fn a_carriage_return_becomes_a_move_to_the_column() {
        assert_eq!(clamp_to_column("abcde\r", DESCRIPTION_COLUMN), "abcde\x1b[5D");
        assert_eq!(clamp_to_column("\rx", DESCRIPTION_COLUMN), "x");
    }

    /// An absolute column cannot point outside the column either.
    #[test]
    fn an_absolute_column_is_pushed_into_the_column() {
        assert_eq!(clamp_to_column("\x1b[1G", DESCRIPTION_COLUMN), "\x1b[34G");
        assert_eq!(clamp_to_column("\x1b[50G", DESCRIPTION_COLUMN), "\x1b[50G");
    }

    /// Anything that is not movement is passed through untouched.
    #[test]
    fn other_sequences_are_untouched() {
        let line = "\x1b[1;33mhello\x1b[0m";
        assert_eq!(clamp_to_column(line, DESCRIPTION_COLUMN), line);
    }
}

#[cfg(test)]
mod strip_color_tests {
    use super::strip_colors;

    #[test]
    fn colours_go_and_the_text_stays() {
        assert_eq!(strip_colors("\x1b[1;36mhello\x1b[0m"), "hello");
    }

    /// Only colour goes: the art needs its spacing to keep its shape.
    #[test]
    fn movement_survives() {
        assert_eq!(strip_colors("\x1b[0m\x1b[5Cart\x1b[3D"), "\x1b[5Cart\x1b[3D");
    }

    #[test]
    fn a_line_without_colour_is_unchanged() {
        assert_eq!(strip_colors("plain text"), "plain text");
    }
}
