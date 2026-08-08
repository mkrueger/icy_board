use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use codepages::tables::get_utf8;

/// One file as it appeared in a PCBoard DIR listing or a FILES.BBS.
#[derive(Debug, PartialEq)]
pub struct Entry {
    pub name: String,
    pub size: Option<u64>,
    pub date: Option<DateTime<Utc>>,
    pub description: String,
    /// PCBoard marked files that cost no download time with a `*` in the size column.
    pub free: bool,
}

impl Entry {
    fn new(name: String) -> Self {
        Self {
            name,
            size: None,
            date: None,
            description: String::new(),
            free: false,
        }
    }

    fn push_line(&mut self, line: &str) {
        let line = line.trim_end();
        if line.is_empty() && self.description.is_empty() {
            return;
        }
        if !self.description.is_empty() {
            self.description.push('\n');
        }
        self.description.push_str(line);
    }

    fn finish(mut self) -> Self {
        self.description = self.description.trim_end().to_string();
        self
    }
}

/// The column layout PCBoard writes: name, right aligned size, `MM-DD-YY`, then the text.
const SIZE_COLUMN: usize = 13;
const DATE_COLUMN: usize = 23;
const DESCRIPTION_COLUMN: usize = 33;
/// Continuation lines carry this at column 31.
const CONTINUATION_COLUMN: usize = 31;
const CONTINUATION_MARKER: char = '|';

pub fn parse_pcboard_dir(data: &[u8]) -> Vec<Entry> {
    let mut entries: Vec<Entry> = Vec::new();

    for line in get_utf8(data).lines() {
        let chars: Vec<char> = line.chars().collect();
        if is_separator(line) {
            continue;
        }

        if let Some(entry) = entries.last_mut() {
            if is_continuation(&chars) {
                entry.push_line(take(&chars, DESCRIPTION_COLUMN, usize::MAX).trim_end());
                continue;
            }
        } else if is_continuation(&chars) {
            // A listing that opens with a continuation is damaged; there is nothing to attach it to.
            continue;
        }

        let Some(mut entry) = parse_header_line(&chars) else {
            continue;
        };
        entry.push_line(take(&chars, DESCRIPTION_COLUMN, usize::MAX).trim_end());
        entries.push(entry);
    }

    entries.into_iter().map(Entry::finish).collect()
}

/// The two banner lines at the top of every DIR file, plus any rule the sysop added.
fn is_separator(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return true;
    }
    trimmed.chars().all(|c| c == '=' || c == '-' || c == ' ')
}

/// Marked with a `|` at column 31, though some boards just leave the name column blank.
fn is_continuation(chars: &[char]) -> bool {
    if chars.get(CONTINUATION_COLUMN) == Some(&CONTINUATION_MARKER) {
        return true;
    }
    chars.len() > DESCRIPTION_COLUMN && chars[..DESCRIPTION_COLUMN].iter().all(|c| c.is_whitespace())
}

fn parse_header_line(chars: &[char]) -> Option<Entry> {
    let name = take(chars, 0, SIZE_COLUMN - 1);
    let name = name.trim();
    if name.is_empty() || name.contains(' ') {
        return None;
    }
    // The header line of the listing itself looks close enough to an entry to slip through.
    if !name.chars().next().is_some_and(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }

    let mut entry = Entry::new(name.to_ascii_uppercase());
    let size = take(chars, SIZE_COLUMN, DATE_COLUMN - 2);
    let size = size.trim();
    entry.free = size.starts_with('*');
    entry.size = size.trim_start_matches(['*', '+']).replace(',', "").parse().ok();
    entry.date = parse_date(take(chars, DATE_COLUMN, DESCRIPTION_COLUMN - 2).trim());
    // Without a size or a date this is the banner PCBoard puts above the listing.
    if entry.size.is_none() && entry.date.is_none() {
        return None;
    }
    Some(entry)
}

fn take(chars: &[char], from: usize, to: usize) -> String {
    if from >= chars.len() {
        return String::new();
    }
    chars[from..to.min(chars.len())].iter().collect()
}

/// PCBoard wrote two digit years; it read anything below 80 as being past 2000.
fn parse_date(text: &str) -> Option<DateTime<Utc>> {
    let mut parts = text.split(['-', '/', '.']);
    let month: u32 = parts.next()?.trim().parse().ok()?;
    let day: u32 = parts.next()?.trim().parse().ok()?;
    let year: i32 = parts.next()?.trim().parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    let year = match year {
        0..=79 => 2000 + year,
        80..=99 => 1900 + year,
        _ => year,
    };
    let date = NaiveDate::from_ymd_opt(year, month, day)?.and_hms_opt(0, 0, 0)?;
    Utc.from_utc_datetime(&date).into()
}

/// FILES.BBS has no fixed columns: a name in the first column starts an entry and every
/// indented line after it continues the description.
pub fn parse_files_bbs(data: &[u8]) -> Vec<Entry> {
    let mut entries: Vec<Entry> = Vec::new();

    for line in get_utf8(data).lines() {
        if line.trim().is_empty() || is_separator(line) {
            continue;
        }
        if line.starts_with([' ', '\t']) {
            if let Some(entry) = entries.last_mut() {
                entry.push_line(line.trim_start().trim_start_matches(['|', '\\']).trim_start());
            }
            continue;
        }

        let mut parts = line.splitn(2, char::is_whitespace);
        let Some(name) = parts.next() else {
            continue;
        };
        let mut entry = Entry::new(name.to_ascii_uppercase());
        let rest = parts.next().unwrap_or_default();
        entry.push_line(strip_leading_size_and_date(rest));
        entries.push(entry);
    }

    entries.into_iter().map(Entry::finish).collect()
}

/// Some generators put the size and date between the name and the text; they are already
/// known from the file itself, so they are dropped rather than parsed.
fn strip_leading_size_and_date(rest: &str) -> &str {
    let mut remainder = rest.trim_start();
    for _ in 0..2 {
        let Some((field, tail)) = remainder.split_once(char::is_whitespace) else {
            break;
        };
        let is_size = field.chars().all(|c| c.is_ascii_digit() || c == ',') && field.chars().any(|c| c.is_ascii_digit());
        let is_date = parse_date(field).is_some();
        if !is_size && !is_date {
            break;
        }
        remainder = tail.trim_start();
    }
    remainder
}

pub fn format_files_bbs(entries: &[(String, String)]) -> String {
    let mut result = String::new();
    for (name, description) in entries {
        result.push_str(&format!("{:<12} ", name));
        for (i, line) in description.lines().enumerate() {
            if i > 0 {
                result.push_str(&" ".repeat(13));
            }
            result.push_str(line);
            result.push('\n');
        }
        if description.is_empty() {
            result.push('\n');
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a_dir_listing_with_a_wrapped_description() {
        let listing = concat!(
            "Filename       Size      Date    Description of File Contents\r\n",
            "============ ========  ========  ==========================================\r\n",
            "ALLFILES.ZIP      744  02-14-94  A listing of all files available on this\r\n",
            "                               | bulletin board system.  Use this list for\r\n",
            "                               | browsing the file lists off-line.\r\n",
            "RULES.TXT         768  02-14-94  ASCII file listing the rules.\r\n",
        );

        let entries = parse_pcboard_dir(listing.as_bytes());
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "ALLFILES.ZIP");
        assert_eq!(entries[0].size, Some(744));
        assert_eq!(entries[0].date.unwrap().format("%Y-%m-%d").to_string(), "1994-02-14");
        assert_eq!(
            entries[0].description,
            "A listing of all files available on this\nbulletin board system.  Use this list for\nbrowsing the file lists off-line."
        );
        assert_eq!(entries[1].name, "RULES.TXT");
        assert_eq!(entries[1].description, "ASCII file listing the rules.");
    }

    #[test]
    fn test_a_free_file_is_recognised() {
        let listing = "FREE.ZIP        *1024  01-02-03  No download time.\r\n";
        let entries = parse_pcboard_dir(listing.as_bytes());
        assert_eq!(entries.len(), 1);
        assert!(entries[0].free);
        assert_eq!(entries[0].size, Some(1024));
    }

    #[test]
    fn test_an_offline_file_keeps_its_description() {
        let listing = "GONE.ZIP      OFFLINE  01-02-03  Ask the sysop for this one.\r\n";
        let entries = parse_pcboard_dir(listing.as_bytes());
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].size, None);
        assert_eq!(entries[0].description, "Ask the sysop for this one.");
    }

    #[test]
    fn test_the_two_digit_year_pivots_at_eighty() {
        assert_eq!(parse_date("01-02-79").unwrap().format("%Y").to_string(), "2079");
        assert_eq!(parse_date("01-02-80").unwrap().format("%Y").to_string(), "1980");
        assert_eq!(parse_date("01-02-1994").unwrap().format("%Y").to_string(), "1994");
        assert_eq!(parse_date("not a date"), None);
    }

    #[test]
    fn test_a_files_bbs_with_indented_continuations() {
        let listing = concat!(
            "ALPHA.ZIP    The first file\r\n",
            "             which wraps onto a second line\r\n",
            "BETA.ZIP     The second file\r\n",
        );
        let entries = parse_files_bbs(listing.as_bytes());
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "ALPHA.ZIP");
        assert_eq!(entries[0].description, "The first file\nwhich wraps onto a second line");
        assert_eq!(entries[1].description, "The second file");
    }

    #[test]
    fn test_a_files_bbs_that_repeats_size_and_date() {
        let listing = "ALPHA.ZIP  12,345  01-02-94  The first file\r\n";
        let entries = parse_files_bbs(listing.as_bytes());
        assert_eq!(entries[0].description, "The first file");
    }

    #[test]
    fn test_a_description_starting_with_a_number_is_kept() {
        let listing = "ALPHA.ZIP  3d rendering tools\r\n";
        let entries = parse_files_bbs(listing.as_bytes());
        assert_eq!(entries[0].description, "3d rendering tools");
    }

    #[test]
    fn test_export_wraps_continuations_under_the_name() {
        let exported = format_files_bbs(&[("ALPHA.ZIP".to_string(), "first line\nsecond line".to_string())]);
        assert_eq!(exported, "ALPHA.ZIP    first line\n             second line\n");
    }
}
