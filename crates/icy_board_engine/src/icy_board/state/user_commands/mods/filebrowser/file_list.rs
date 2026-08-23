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
use std::fmt::Write as _;

/// Where a description starts, once the name, size and date columns are written.
const DESCRIPTION_COLUMN: usize = 33;

/// Keeps a description inside its column.
///
/// A `FILE_ID.DIZ` is drawn for a screen that starts at column 0, and the ANSI ones
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
                let _ = write!(out, "\x1b[{}D", column - left);
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
                    let _ = write!(out, "\x1b[{}D", wanted.min(room));
                    column -= wanted.min(room);
                }
            }
            'C' => {
                let by = first();
                let _ = write!(out, "\x1b[{by}C");
                column += by;
            }
            'G' => {
                let wanted = first().max(left + 1);
                let _ = write!(out, "\x1b[{wanted}G");
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

/// Reduces a description to plain text.
///
/// A DIZ carries whatever its author liked - colours, cursor movement, and on a
/// `PCBoard` the `@X` pairs on top. A reset among them puts the caller back to the
/// terminal default and so loses the colour the board lists files in. A sysop who
/// wants the listing to read as their board rather than as 1994 turns
/// `strip_colors_in_descriptions` on and gets the words and nothing else.
fn strip_descriptions_markup(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // Every escape goes, not only the colours: what is left has to sit
            // in the column as written.
            if chars.peek() == Some(&'[') {
                chars.next();
                for c in chars.by_ref() {
                    if !(c.is_ascii_digit() || c == ';' || c == '?') {
                        break;
                    }
                }
            } else {
                chars.next();
            }
            continue;
        }
        if ch == '@' {
            // `@X` plus two hex digits is PCBoard's colour pair.
            let mut look = chars.clone();
            if matches!(look.next(), Some('X' | 'x')) {
                let first = look.next();
                let second = look.next();
                if first.is_some_and(|c| c.is_ascii_hexdigit()) && second.is_some_and(|c| c.is_ascii_hexdigit()) {
                    chars = look;
                    continue;
                }
            }
        }
        out.push(ch);
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
    marks_new: Option<Box<dyn Fn(&FileHeader) -> bool>>,
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
            marks_new: None,
        }
    }

    pub fn new_files_since(since: chrono::DateTime<chrono::Utc>) -> Self {
        Self {
            accepts: Box::new(move |file| file.date() >= since),
            accepts_described: None,
            marks_new: Some(Box::new(move |file| file.date() >= since)),
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
            marks_new: None,
        }
    }
}

pub struct FileList {
    pub path: PathBuf,
    pub files: Arc<Mutex<FileBase>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DateFieldState {
    Date,
    Offline,
    Deleted,
}

fn date_field_state(deleted: bool, exists: bool) -> DateFieldState {
    if deleted {
        DateFieldState::Deleted
    } else if !exists {
        DateFieldState::Offline
    } else {
        DateFieldState::Date
    }
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
        for entry in &headers {
            if !(filter.accepts)(entry) {
                continue;
            }
            let full_path = dir.join(entry.name());
            // Only reached by files that are candidates, so an area is not scanned wholesale
            // just to list a handful of matches.
            let meta_data = self.files.lock().await.read_metadata(&full_path)?;
            if let Some(accepts_described) = &filter.accepts_described
                && !accepts_described(entry, &meta_data)
            {
                continue;
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
            let exists = full_path.exists();
            cmd.set_color(TerminalTarget::Both, colors.file_name.clone()).await?;
            if cmd.session.search_pattern.is_some() {
                cmd.print_found_text(TerminalTarget::Both, &format!("{name:<12} ")).await?;
            } else {
                cmd.print(TerminalTarget::Both, &format!("{name:<12} ")).await?;
            }
            if name.len() > 12 {
                cmd.new_line().await?;
                cmd.print(TerminalTarget::Both, &" ".repeat(13)).await?;
            }

            cmd.set_color(TerminalTarget::Both, colors.file_size.clone()).await?;
            cmd.print(TerminalTarget::Both, &format!("{:>8}  ", humanize_bytes_decimal!(size).to_string()))
                .await?;

            match date_field_state(entry.is_deleted(), exists) {
                DateFieldState::Deleted => {
                    cmd.set_color(TerminalTarget::Both, colors.file_deleted.clone()).await?;
                    cmd.print(TerminalTarget::Both, " DELETED").await?;
                }
                DateFieldState::Offline => {
                    cmd.set_color(TerminalTarget::Both, colors.file_offline.clone()).await?;
                    cmd.print(TerminalTarget::Both, "OFF-LINE").await?;
                }
                DateFieldState::Date => {
                    cmd.set_color(TerminalTarget::Both, colors.file_date.clone()).await?;
                    cmd.print(TerminalTarget::Both, &format!("{}", date.format("%m/%d/%y"))).await?;
                }
            }

            if filter.marks_new.as_ref().is_some_and(|marks_new| marks_new(entry)) {
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
                        // Plain text has nothing left that could leave the column.
                        let line = if strip_description_colors {
                            strip_descriptions_markup(line)
                        } else {
                            clamp_to_column(line, DESCRIPTION_COLUMN)
                        };
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
                        cmd.set_color(TerminalTarget::Both, colors.file_description.clone()).await?;
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
    use chrono::{Duration, Utc};
    use dizbase::file_base::file_header::{FileAttributes, FileHeader};

    use super::{DESCRIPTION_COLUMN, DateFieldState, FileFilter, clamp_to_column, date_field_state};

    #[test]
    fn deleted_and_offline_states_replace_the_date_in_pcboard_order() {
        assert_eq!(date_field_state(false, true), DateFieldState::Date);
        assert_eq!(date_field_state(false, false), DateFieldState::Offline);
        assert_eq!(date_field_state(true, false), DateFieldState::Deleted);
    }

    #[test]
    fn a_new_file_filter_marks_every_file_it_accepts() {
        let since = Utc::now();
        let filter = FileFilter::new_files_since(since);
        let old = FileHeader {
            id: 0,
            name: "OLD.ZIP".to_string(),
            date: since - Duration::days(1),
            size: 1,
            dl_counter: 0,
            attribute: FileAttributes::NONE,
        };
        let new = FileHeader {
            date: since + Duration::seconds(1),
            name: "NEW.ZIP".to_string(),
            ..old.clone()
        };

        assert!(!(filter.accepts)(&old));
        assert!((filter.accepts)(&new));
        assert!(filter.marks_new.as_ref().is_some_and(|marks| marks(&new)));
    }

    /// The line that started this: the first line of the `FILE_ID.DIZ` in
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
    use super::strip_descriptions_markup;

    #[test]
    fn colours_go_and_the_text_stays() {
        assert_eq!(strip_descriptions_markup("\x1b[1;36mhello\x1b[0m"), "hello");
    }

    /// Everything goes, movement included - what is left has to sit where it is
    /// written.
    #[test]
    fn movement_goes_too() {
        assert_eq!(strip_descriptions_markup("\x1b[0m\x1b[5Cart\x1b[3D"), "art");
        assert_eq!(strip_descriptions_markup("\x1b[255Dtitle"), "title");
    }

    /// `PCBoard` wrote its colours as `@X` and two hex digits.
    #[test]
    fn pcboard_colour_pairs_go() {
        assert_eq!(strip_descriptions_markup("@X0Fbright@X07plain"), "brightplain");
        assert_eq!(strip_descriptions_markup("@x1elower case too"), "lower case too");
    }

    /// An `@` that is not a colour pair is part of the description.
    #[test]
    fn an_ordinary_at_sign_stays() {
        assert_eq!(strip_descriptions_markup("mail@example.com"), "mail@example.com");
        assert_eq!(strip_descriptions_markup("@XZZ"), "@XZZ");
    }

    #[test]
    fn a_line_without_markup_is_unchanged() {
        assert_eq!(strip_descriptions_markup("plain text"), "plain text");
    }
}
