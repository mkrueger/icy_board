use std::{iter::Peekable, vec::IntoIter};

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use unicode_width::UnicodeWidthChar;

use crate::{HelpTheme, Result, invalid};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Title,
    Heading,
    Body,
    Emphasis,
    Code,
    Note,
    Border,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StyledSpan {
    pub role: Role,
    pub text: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StyledLine {
    pub spans: Vec<StyledSpan>,
    pub decoration: bool,
}

impl StyledLine {
    pub fn plain_text(&self) -> String {
        self.spans.iter().map(|span| span.text.as_str()).collect()
    }

    pub fn width(&self) -> usize {
        self.spans.iter().map(|span| span.text.chars().count()).sum()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Document {
    pub lines: Vec<StyledLine>,
}

impl Document {
    pub fn plain_text(&self) -> String {
        let mut result = String::new();
        for line in self.lines.iter().filter(|line| !line.decoration) {
            result.push_str(&line.plain_text());
            result.push('\n');
        }
        result
    }
}

type Inline = Vec<StyledSpan>;

enum Block {
    Paragraph(Inline),
    Heading(Inline, Role),
    List(Option<u64>, Vec<Vec<Block>>),
    Note(Vec<Block>),
    Code(String),
    Table(Vec<Vec<Inline>>),
}

struct Reader<'a> {
    events: Peekable<IntoIter<Event<'a>>>,
}

impl<'a> Reader<'a> {
    fn blocks(&mut self, end: Option<TagEnd>) -> Result<Vec<Block>> {
        let mut blocks = Vec::new();
        while let Some(event) = self.events.peek() {
            if matches!(event, Event::End(tag) if Some(*tag) == end) {
                self.events.next();
                return Ok(blocks);
            }
            let block = match self.events.next().unwrap() {
                Event::Start(Tag::Paragraph) => Block::Paragraph(self.inline(Some(TagEnd::Paragraph), Role::Body)?),
                Event::Start(Tag::Heading { level, id, classes, attrs }) => {
                    if id.is_some() || !classes.is_empty() || !attrs.is_empty() {
                        return Err(invalid("Heading attributes are not supported"));
                    }
                    let role = if level == HeadingLevel::H1 { Role::Title } else { Role::Heading };
                    Block::Heading(self.inline(Some(TagEnd::Heading(level)), role)?, role)
                }
                Event::Start(Tag::BlockQuote(kind)) => Block::Note(self.blocks(Some(TagEnd::BlockQuote(kind)))?),
                Event::Start(Tag::List(start)) => {
                    let mut items = Vec::new();
                    loop {
                        match self.events.next() {
                            Some(Event::Start(Tag::Item)) => items.push(self.blocks(Some(TagEnd::Item))?),
                            Some(Event::End(TagEnd::List(_))) => break,
                            other => return Err(invalid(format!("Unsupported list content: {other:?}"))),
                        }
                    }
                    Block::List(start, items)
                }
                Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(_))) => {
                    let mut text = String::new();
                    loop {
                        match self.events.next() {
                            Some(Event::Text(part)) => text.push_str(&part),
                            Some(Event::End(TagEnd::CodeBlock)) => break,
                            other => return Err(invalid(format!("Unsupported code content: {other:?}"))),
                        }
                    }
                    Block::Code(text)
                }
                Event::Start(Tag::Table(_)) => Block::Table(self.table()?),
                event if is_inline(&event) => {
                    // Tight list items omit paragraph tags.
                    let mut spans = self.inline_event(event, Role::Body)?;
                    spans.extend(self.inline(None, Role::Body)?);
                    Block::Paragraph(spans)
                }
                other => return Err(invalid(format!("Unsupported Markdown construct: {other:?}"))),
            };
            blocks.push(block);
        }
        if end.is_some() {
            return Err(invalid("Unterminated Markdown block"));
        }
        Ok(blocks)
    }

    fn inline(&mut self, end: Option<TagEnd>, role: Role) -> Result<Inline> {
        let mut spans = Vec::new();
        while let Some(event) = self.events.peek() {
            if matches!(event, Event::End(tag) if Some(*tag) == end) {
                self.events.next();
                return Ok(spans);
            }
            if end.is_none() && !is_inline(event) {
                return Ok(spans);
            }
            let event = self.events.next().unwrap();
            spans.extend(self.inline_event(event, role)?);
        }
        if end.is_some() {
            return Err(invalid("Unterminated Markdown inline element"));
        }
        Ok(spans)
    }

    fn inline_event(&mut self, event: Event<'a>, role: Role) -> Result<Inline> {
        match event {
            Event::Text(text) => Ok(vec![span(role, &text)?]),
            Event::Code(text) => Ok(vec![span(Role::Code, &text.replace('\t', "    "))?]),
            Event::SoftBreak => Ok(vec![span(role, " ")?]),
            Event::HardBreak => Ok(vec![StyledSpan { role, text: "\n".into() }]),
            Event::Start(Tag::Emphasis) => self.inline(Some(TagEnd::Emphasis), Role::Emphasis),
            Event::Start(Tag::Strong) => self.inline(Some(TagEnd::Strong), Role::Emphasis),
            Event::Start(Tag::Link { dest_url, .. }) => {
                validate_text(&dest_url)?;
                if dest_url.chars().any(char::is_whitespace) {
                    return Err(invalid("Link destinations must be printable URLs without whitespace"));
                }
                let mut label = self.inline(Some(TagEnd::Link), role)?;
                let label_text: String = label.iter().map(|span| span.text.as_str()).collect();
                if label_text != dest_url.as_ref() && !dest_url.is_empty() {
                    label.push(span(role, &format!(" ({dest_url})"))?);
                }
                Ok(label)
            }
            other => Err(invalid(format!("Unsupported Markdown inline construct: {other:?}"))),
        }
    }

    fn table(&mut self) -> Result<Vec<Vec<Inline>>> {
        let mut rows = Vec::new();
        loop {
            let end = match self.events.next() {
                Some(Event::Start(Tag::TableHead)) => TagEnd::TableHead,
                Some(Event::Start(Tag::TableRow)) => TagEnd::TableRow,
                Some(Event::End(TagEnd::Table)) => return Ok(rows),
                other => return Err(invalid(format!("Unsupported table content: {other:?}"))),
            };
            let mut row = Vec::new();
            loop {
                match self.events.next() {
                    Some(Event::Start(Tag::TableCell)) => row.push(self.inline(Some(TagEnd::TableCell), Role::Body)?),
                    Some(Event::End(tag)) if tag == end => break,
                    other => return Err(invalid(format!("Unsupported table row: {other:?}"))),
                }
            }
            rows.push(row);
        }
    }
}

fn is_inline(event: &Event<'_>) -> bool {
    matches!(
        event,
        Event::Text(_) | Event::Code(_) | Event::SoftBreak | Event::HardBreak | Event::Start(Tag::Emphasis | Tag::Strong | Tag::Link { .. })
    )
}

fn validate_text(text: &str) -> Result<()> {
    for ch in text.chars() {
        if ch.is_control() || UnicodeWidthChar::width(ch) != Some(1) {
            return Err(invalid(format!("Character {ch:?} (U+{:04X}) must be printable and exactly one terminal cell wide", ch as u32)));
        }
    }
    Ok(())
}

fn span(role: Role, text: &str) -> Result<StyledSpan> {
    validate_text(text)?;
    Ok(StyledSpan { role, text: text.into() })
}

#[derive(Clone)]
struct Prefix {
    first: String,
    rest: String,
    used: bool,
}

impl Prefix {
    fn take(&mut self) -> String {
        let text = if self.used { &self.rest } else { &self.first }.clone();
        self.used = true;
        text
    }
}

struct Layout<'a> {
    width: usize,
    theme: &'a HelpTheme,
    document: Document,
}

impl Layout<'_> {
    fn gap(&mut self) {
        if self.document.lines.last().is_some_and(|line| !line.spans.is_empty()) {
            self.document.lines.push(StyledLine::default());
        }
    }

    fn blocks(&mut self, blocks: &[Block], prefix: &mut Prefix, base: Role) -> Result<()> {
        for (index, block) in blocks.iter().enumerate() {
            if index > 0 {
                self.gap();
            }
            match block {
                Block::Paragraph(spans) => self.prose(spans, prefix, base)?,
                Block::Heading(spans, role) => {
                    self.prose(spans, prefix, *role)?;
                    if self.theme.decoration {
                        let text = format!("{}{}", prefix.rest, if *role == Role::Title { "=" } else { "-" }.repeat(self.available(&prefix.rest)?));
                        self.document.lines.push(StyledLine { spans: vec![span(Role::Border, &text)?], decoration: true });
                    }
                }
                Block::Note(children) => {
                    let mut nested = Prefix { first: format!("{}> ", prefix.take()), rest: format!("{}> ", prefix.rest), used: false };
                    self.blocks(children, &mut nested, Role::Note)?;
                }
                Block::List(start, items) => {
                    for (index, item) in items.iter().enumerate() {
                        let marker = match start {
                            Some(start) => format!("{}. ", start.checked_add(index as u64).ok_or_else(|| invalid("List number overflow"))?),
                            None => "- ".into(),
                        };
                        let mut nested = Prefix {
                            first: format!("{}{}", prefix.take(), marker),
                            rest: format!("{}{}", prefix.rest, " ".repeat(marker.len())),
                            used: false,
                        };
                        if item.is_empty() {
                            self.push_line(&nested.take(), &[], base)?;
                        } else {
                            self.blocks(item, &mut nested, base)?;
                        }
                    }
                }
                Block::Code(text) => {
                    for raw in text.split_terminator('\n') {
                        let mut expanded = String::new();
                        let mut column = 0;
                        for ch in raw.chars() {
                            if ch == '\t' {
                                let count = 4 - column % 4;
                                expanded.push_str(&" ".repeat(count));
                                column += count;
                            } else {
                                expanded.push(ch);
                                column += 1;
                            }
                        }
                        validate_text(&expanded)?;
                        let lead = prefix.take();
                        if column > self.available(&lead)? {
                            return Err(invalid(format!("Code line has {column} cells but only {} fit; shorten the line or increase width", self.available(&lead)?)));
                        }
                        self.push_line(&lead, &expanded.chars().map(|ch| (ch, Role::Code)).collect::<Vec<_>>(), base)?;
                    }
                }
                Block::Table(rows) => {
                    // Stacked fields remain readable at every supported width without truncation.
                    if let Some(headers) = rows.first() {
                        if rows.len() == 1 {
                            for header in headers {
                                self.prose(header, prefix, Role::Heading)?;
                            }
                        }
                        for (index, row) in rows.iter().skip(1).enumerate() {
                            if index > 0 {
                                self.gap();
                            }
                            for (header, cell) in headers.iter().zip(row) {
                                let mut field: Inline = header.iter().map(|s| StyledSpan { role: Role::Heading, text: s.text.clone() }).collect();
                                field.push(span(Role::Body, ": ")?);
                                field.extend(cell.iter().cloned());
                                self.prose(&field, prefix, base)?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn available(&self, prefix: &str) -> Result<usize> {
        self.width
            .checked_sub(self.theme.margin + prefix.chars().count())
            .filter(|&width| width > 0)
            .ok_or_else(|| invalid("Margins or nested blocks leave no room for help text"))
    }

    fn push_line(&mut self, prefix: &str, glyphs: &[(char, Role)], base: Role) -> Result<()> {
        if glyphs.len() > self.available(prefix)? {
            return Err(invalid("Rendered line exceeds the requested width"));
        }
        let mut spans = Vec::<StyledSpan>::new();
        for (ch, role) in prefix.chars().map(|ch| (ch, base)).chain(glyphs.iter().copied()) {
            if let Some(last) = spans.last_mut().filter(|last| last.role == role) {
                last.text.push(ch);
            } else {
                spans.push(StyledSpan { role, text: ch.to_string() });
            }
        }
        self.document.lines.push(StyledLine { spans, decoration: false });
        Ok(())
    }

    fn prose(&mut self, spans: &[StyledSpan], prefix: &mut Prefix, base: Role) -> Result<()> {
        let mut lead = prefix.take();
        let mut limit = self.available(&lead)?;
        let mut line = Vec::new();
        let mut word = Vec::new();
        let mut pending_space = false;
        let glyphs = spans.iter().flat_map(|span| {
            let role = if span.role == Role::Body { base } else { span.role };
            span.text.chars().map(move |ch| (ch, role))
        });
        for (ch, role) in glyphs.chain(std::iter::once(('\n', base))) {
            let separator = ch == '\n' || (ch.is_whitespace() && role != Role::Code);
            if !separator {
                word.push((ch, role));
                continue;
            }
            if !word.is_empty() {
                let space = usize::from(pending_space && !line.is_empty());
                if !line.is_empty() && line.len() + space + word.len() > limit {
                    self.push_line(&lead, &line, base)?;
                    line.clear();
                    lead = prefix.take();
                    limit = self.available(&lead)?;
                } else if space > 0 {
                    line.push((' ', base));
                }
                for glyph in word.drain(..) {
                    if line.len() == limit {
                        self.push_line(&lead, &line, base)?;
                        line.clear();
                        lead = prefix.take();
                        limit = self.available(&lead)?;
                    }
                    line.push(glyph);
                }
            }
            pending_space = true;
            if ch == '\n' {
                self.push_line(&lead, &line, base)?;
                line.clear();
                lead = prefix.rest.clone();
                limit = self.available(&lead)?;
                pending_space = false;
            }
        }
        Ok(())
    }
}

pub fn compile(markdown: &str, width: usize, theme: &HelpTheme) -> Result<Document> {
    if !(40..=79).contains(&width) {
        return Err(invalid("Help width must be in 40..=79 cells"));
    }
    theme.validate()?;
    for (offset, ch) in markdown.char_indices() {
        if ch.is_control() && !matches!(ch, '\r' | '\n' | '\t') {
            return Err(invalid(format!("Source control character U+{:04X} at byte {offset}", ch as u32)));
        }
    }
    let normalized = markdown.replace("\r\n", "\n").replace('\r', "\n");
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES
        | Options::ENABLE_DEFINITION_LIST
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS
        | Options::ENABLE_MATH;
    let mut code_ranges = Vec::new();
    let mut events = Vec::new();
    let mut depth = 0usize;
    for (event, range) in Parser::new_ext(&normalized, options).into_offset_iter() {
        match &event {
            Event::Start(_) => {
                depth += 1;
                if depth > 64 {
                    return Err(invalid("Markdown nesting exceeds 64 levels"));
                }
            }
            Event::End(_) => depth -= 1,
            _ => {}
        }
        if matches!(event, Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(_))) | Event::Code(_)) {
            code_ranges.push(range);
        }
        events.push(event);
    }
    for (offset, _) in normalized.match_indices('\t') {
        if !code_ranges.iter().any(|range| range.contains(&offset)) {
            return Err(invalid(format!("Tabs are allowed only in code, at byte {offset}")));
        }
    }
    let blocks = Reader { events: events.into_iter().peekable() }.blocks(None)?;
    let margin = " ".repeat(theme.margin);
    let mut prefix = Prefix { first: margin.clone(), rest: margin, used: false };
    let mut layout = Layout { width, theme, document: Document::default() };
    layout.blocks(&blocks, &mut prefix, Role::Body)?;
    Ok(layout.document)
}