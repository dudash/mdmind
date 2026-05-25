use std::{env, io::IsTerminal};

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownDocument {
    pub blocks: Vec<MarkdownBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkdownBlock {
    FrontMatter(String),
    Heading {
        level: u8,
        text: String,
    },
    Paragraph(String),
    ListItem {
        depth: usize,
        marker: ListMarker,
        text: String,
    },
    BlockQuote(String),
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    Table {
        rows: Vec<Vec<String>>,
    },
    Rule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListMarker {
    Bullet,
    Ordered(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Auto,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderTarget {
    CliAnsi,
    Plain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderOptions {
    pub width: usize,
    pub color: ColorMode,
    pub target: RenderTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Styles {
    color: bool,
}

impl Styles {
    fn from_options(options: RenderOptions) -> Self {
        let color = matches!(options.target, RenderTarget::CliAnsi)
            && matches!(options.color, ColorMode::Auto)
            && env::var_os("NO_COLOR").is_none()
            && std::io::stdout().is_terminal();
        Self { color }
    }

    fn heading1(self, text: &str) -> String {
        self.wrap("1;38;5;81", text)
    }

    fn heading2(self, text: &str) -> String {
        self.wrap("1;38;5;110", text)
    }

    fn heading(self, text: &str) -> String {
        self.wrap("1;38;5;109", text)
    }

    fn accent(self, text: &str) -> String {
        self.wrap("38;5;110", text)
    }

    fn dim(self, text: &str) -> String {
        self.wrap("38;5;244", text)
    }

    fn code(self, text: &str) -> String {
        self.wrap("38;5;229", text)
    }

    fn wrap(self, code: &str, text: &str) -> String {
        if self.color {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            width: 88,
            color: ColorMode::Auto,
            target: RenderTarget::CliAnsi,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CurrentBlock {
    Heading {
        level: u8,
        text: String,
    },
    Paragraph(String),
    ListItem {
        depth: usize,
        marker: ListMarker,
        text: String,
    },
    BlockQuote(String),
    CodeBlock {
        language: Option<String>,
        code: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ListState {
    ordered: bool,
    next: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LinkState {
    target: String,
    image: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct TableBuilder {
    rows: Vec<Vec<String>>,
}

pub fn parse_markdown(markdown: &str) -> MarkdownDocument {
    let (frontmatter, body) = split_frontmatter(markdown);
    let parser = Parser::new_ext(body, parser_options());
    let mut blocks = Vec::new();
    if let Some(frontmatter) = frontmatter {
        if !frontmatter.trim().is_empty() {
            blocks.push(MarkdownBlock::FrontMatter(frontmatter));
        }
    }

    let mut current: Option<CurrentBlock> = None;
    let mut lists = Vec::<ListState>::new();
    let mut quote_depth = 0_usize;
    let mut links = Vec::<LinkState>::new();
    let mut table: Option<TableBuilder> = None;
    let mut table_row: Option<Vec<String>> = None;
    let mut table_cell: Option<String> = None;

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Table(_) => {
                    flush_current(&mut current, &mut blocks);
                    table = Some(TableBuilder::default());
                }
                Tag::TableHead | Tag::TableRow => {
                    table_row = Some(Vec::new());
                }
                Tag::TableCell => {
                    table_cell = Some(String::new());
                }
                Tag::Heading { level, .. } => {
                    flush_current(&mut current, &mut blocks);
                    current = Some(CurrentBlock::Heading {
                        level: heading_level(level),
                        text: String::new(),
                    });
                }
                Tag::Paragraph if table_cell.is_none() && current.is_none() => {
                    current = Some(if quote_depth > 0 {
                        CurrentBlock::BlockQuote(String::new())
                    } else {
                        CurrentBlock::Paragraph(String::new())
                    });
                }
                Tag::List(start) => {
                    lists.push(ListState {
                        ordered: start.is_some(),
                        next: start.unwrap_or(1),
                    });
                }
                Tag::Item => {
                    flush_current(&mut current, &mut blocks);
                    let depth = lists.len().saturating_sub(1);
                    let marker = match lists.last_mut() {
                        Some(list) if list.ordered => {
                            let number = list.next;
                            list.next += 1;
                            ListMarker::Ordered(number)
                        }
                        _ => ListMarker::Bullet,
                    };
                    current = Some(CurrentBlock::ListItem {
                        depth,
                        marker,
                        text: String::new(),
                    });
                }
                Tag::BlockQuote(_) => {
                    flush_current(&mut current, &mut blocks);
                    quote_depth += 1;
                }
                Tag::CodeBlock(kind) => {
                    flush_current(&mut current, &mut blocks);
                    current = Some(CurrentBlock::CodeBlock {
                        language: code_block_language(kind),
                        code: String::new(),
                    });
                }
                Tag::Link {
                    link_type: _,
                    dest_url,
                    ..
                } => {
                    links.push(LinkState {
                        target: dest_url.to_string(),
                        image: false,
                    });
                }
                Tag::Image {
                    link_type: _,
                    dest_url,
                    ..
                } => {
                    links.push(LinkState {
                        target: dest_url.to_string(),
                        image: true,
                    });
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Table => {
                    flush_current(&mut current, &mut blocks);
                    if let Some(table) = table.take() {
                        if !table.rows.is_empty() {
                            blocks.push(MarkdownBlock::Table { rows: table.rows });
                        }
                    }
                }
                TagEnd::TableHead | TagEnd::TableRow => {
                    if let (Some(table), Some(row)) = (table.as_mut(), table_row.take()) {
                        if row.iter().any(|cell| !cell.is_empty()) {
                            table.rows.push(row);
                        }
                    }
                }
                TagEnd::TableCell => {
                    if let (Some(row), Some(cell)) = (table_row.as_mut(), table_cell.take()) {
                        row.push(clean_text(&cell));
                    }
                }
                TagEnd::Heading(_) | TagEnd::Paragraph | TagEnd::Item | TagEnd::CodeBlock => {
                    flush_current(&mut current, &mut blocks);
                }
                TagEnd::List(_) => {
                    flush_current(&mut current, &mut blocks);
                    lists.pop();
                }
                TagEnd::BlockQuote(_) => {
                    flush_current(&mut current, &mut blocks);
                    quote_depth = quote_depth.saturating_sub(1);
                }
                TagEnd::Link | TagEnd::Image => {
                    if let Some(link) = links.pop() {
                        append_link_target(&mut current, table_cell.as_mut(), &link);
                    }
                }
                _ => {}
            },
            Event::Text(text) => append_markdown_text(&mut current, table_cell.as_mut(), &text),
            Event::Code(code) => {
                append_markdown_text(&mut current, table_cell.as_mut(), &format!("`{code}`"))
            }
            Event::Html(_) | Event::InlineHtml(_) => {}
            Event::SoftBreak => append_markdown_text(&mut current, table_cell.as_mut(), " "),
            Event::HardBreak => append_markdown_text(&mut current, table_cell.as_mut(), "\n"),
            Event::Rule => {
                flush_current(&mut current, &mut blocks);
                blocks.push(MarkdownBlock::Rule);
            }
            Event::TaskListMarker(checked) => {
                append_markdown_text(
                    &mut current,
                    table_cell.as_mut(),
                    if checked { "[x] " } else { "[ ] " },
                );
            }
            Event::FootnoteReference(reference) => {
                append_markdown_text(
                    &mut current,
                    table_cell.as_mut(),
                    &format!("[^{reference}]"),
                );
            }
            _ => {}
        }
    }

    flush_current(&mut current, &mut blocks);
    MarkdownDocument { blocks }
}

pub fn render_markdown(markdown: &str, options: RenderOptions) -> String {
    let document = parse_markdown(markdown);
    render_markdown_document(&document, options)
}

pub fn render_markdown_document(document: &MarkdownDocument, options: RenderOptions) -> String {
    let width = options.width.max(24);
    let styles = Styles::from_options(options);
    let mut lines = Vec::<String>::new();

    for block in &document.blocks {
        match block {
            MarkdownBlock::FrontMatter(frontmatter) => {
                push_blank_if_needed(&mut lines);
                lines.push(styles.dim("╭─ metadata"));
                for line in frontmatter.lines() {
                    let line = line.trim();
                    if !line.is_empty() {
                        push_wrapped(&mut lines, line, "│ ", "│ ", width);
                    }
                }
                lines.push(styles.dim("╰─"));
            }
            MarkdownBlock::Heading { level, text } => {
                push_blank_if_needed(&mut lines);
                let text = text.trim();
                if *level == 1 {
                    lines.push(styles.heading1(text));
                    lines.push(styles.accent(&"━".repeat(text.chars().count().max(6).min(width))));
                } else if *level == 2 {
                    lines.push(styles.heading2(text));
                    lines.push(styles.dim(&"─".repeat(text.chars().count().max(6).min(width))));
                } else {
                    lines.push(styles.heading(&format!(
                        "{} {text}",
                        "▸".repeat((*level as usize).saturating_sub(2))
                    )));
                }
            }
            MarkdownBlock::Paragraph(text) => {
                push_blank_if_needed(&mut lines);
                push_wrapped(&mut lines, text.trim(), "", "", width);
            }
            MarkdownBlock::ListItem {
                depth,
                marker,
                text,
            } => {
                let indent = "  ".repeat(*depth);
                let (marker, text) = list_marker_and_text(marker, text);
                let marker = styles.accent(&marker);
                let first = format!("{indent}{marker}");
                let rest = " ".repeat(visible_width(&first));
                push_wrapped(&mut lines, text, &first, &rest, width);
            }
            MarkdownBlock::BlockQuote(text) => {
                push_blank_if_needed(&mut lines);
                let quote = styles.dim("│ ");
                push_wrapped(&mut lines, text.trim(), &quote, &quote, width);
            }
            MarkdownBlock::CodeBlock { language, code } => {
                push_blank_if_needed(&mut lines);
                push_code_block(&mut lines, language.as_deref(), code, width, styles);
            }
            MarkdownBlock::Table { rows } => {
                push_blank_if_needed(&mut lines);
                push_table(&mut lines, rows, styles);
            }
            MarkdownBlock::Rule => {
                push_blank_if_needed(&mut lines);
                lines.push(styles.dim(&"─".repeat(width.min(72))));
            }
        }
    }

    trim_trailing_blank_lines(&mut lines);
    if lines.is_empty() {
        "(empty markdown)".to_string()
    } else {
        lines.join("\n")
    }
}

fn parser_options() -> Options {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options
}

fn split_frontmatter(markdown: &str) -> (Option<String>, &str) {
    let Some(first_line_end) = markdown.find('\n') else {
        return (None, markdown);
    };
    if markdown[..first_line_end].trim_end_matches('\r') != "---" {
        return (None, markdown);
    }

    let content_start = first_line_end + 1;
    let mut line_start = content_start;
    for line in markdown[content_start..].split_inclusive('\n') {
        let marker = line.trim_end_matches(['\r', '\n']);
        if marker == "---" || marker == "..." {
            let body_start = line_start + line.len();
            let frontmatter = markdown[content_start..line_start]
                .trim_end_matches(['\r', '\n'])
                .to_string();
            return (Some(frontmatter), &markdown[body_start..]);
        }
        line_start += line.len();
    }

    (None, markdown)
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn code_block_language(kind: CodeBlockKind<'_>) -> Option<String> {
    match kind {
        CodeBlockKind::Indented => None,
        CodeBlockKind::Fenced(language) => {
            let language = language.trim();
            (!language.is_empty()).then(|| language.to_string())
        }
    }
}

fn flush_current(current: &mut Option<CurrentBlock>, blocks: &mut Vec<MarkdownBlock>) {
    let Some(block) = current.take() else {
        return;
    };

    match block {
        CurrentBlock::Heading { level, text } => {
            let text = clean_text(&text);
            if !text.is_empty() {
                blocks.push(MarkdownBlock::Heading { level, text });
            }
        }
        CurrentBlock::Paragraph(text) => {
            let text = clean_text(&text);
            if !text.is_empty() {
                blocks.push(MarkdownBlock::Paragraph(text));
            }
        }
        CurrentBlock::ListItem {
            depth,
            marker,
            text,
        } => {
            let text = clean_text(&text);
            if !text.is_empty() {
                blocks.push(MarkdownBlock::ListItem {
                    depth,
                    marker,
                    text,
                });
            }
        }
        CurrentBlock::BlockQuote(text) => {
            let text = clean_text(&text);
            if !text.is_empty() {
                blocks.push(MarkdownBlock::BlockQuote(text));
            }
        }
        CurrentBlock::CodeBlock { language, code } => {
            blocks.push(MarkdownBlock::CodeBlock { language, code });
        }
    }
}

fn append_markdown_text(
    current: &mut Option<CurrentBlock>,
    table_cell: Option<&mut String>,
    text: &str,
) {
    if let Some(table_cell) = table_cell {
        table_cell.push_str(text);
    } else {
        append_text(current, text);
    }
}

fn append_text(current: &mut Option<CurrentBlock>, text: &str) {
    if current.is_none() {
        *current = Some(CurrentBlock::Paragraph(String::new()));
    }

    match current.as_mut().expect("current block should exist") {
        CurrentBlock::Heading { text: value, .. }
        | CurrentBlock::Paragraph(value)
        | CurrentBlock::ListItem { text: value, .. }
        | CurrentBlock::BlockQuote(value)
        | CurrentBlock::CodeBlock { code: value, .. } => value.push_str(text),
    }
}

fn append_link_target(
    current: &mut Option<CurrentBlock>,
    table_cell: Option<&mut String>,
    link: &LinkState,
) {
    if link.target.is_empty() || link.target.starts_with('#') {
        return;
    }
    if link.image {
        append_markdown_text(current, table_cell, &format!(" [image: {}]", link.target));
    } else {
        append_markdown_text(current, table_cell, &format!(" ({})", link.target));
    }
}

fn clean_text(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

fn push_blank_if_needed(lines: &mut Vec<String>) {
    if lines.last().is_some_and(|line| !line.is_empty()) {
        lines.push(String::new());
    }
}

fn trim_trailing_blank_lines(lines: &mut Vec<String>) {
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
}

fn push_wrapped(
    lines: &mut Vec<String>,
    text: &str,
    first_prefix: &str,
    rest_prefix: &str,
    width: usize,
) {
    if text.is_empty() {
        lines.push(first_prefix.trim_end().to_string());
        return;
    }

    let first_width = width.saturating_sub(visible_width(first_prefix)).max(12);
    let rest_width = width.saturating_sub(visible_width(rest_prefix)).max(12);
    let mut current = String::new();
    let mut limit = first_width;
    let mut prefix = first_prefix;

    for word in text.split_whitespace() {
        let separator = usize::from(!current.is_empty());
        if !current.is_empty() && current.chars().count() + separator + word.chars().count() > limit
        {
            lines.push(format!("{prefix}{current}"));
            current.clear();
            prefix = rest_prefix;
            limit = rest_width;
        }

        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }

    if !current.is_empty() {
        lines.push(format!("{prefix}{current}"));
    }
}

fn list_marker_and_text<'a>(marker: &ListMarker, text: &'a str) -> (String, &'a str) {
    match marker {
        ListMarker::Ordered(number) => (format!("{number}. "), text.trim()),
        ListMarker::Bullet => {
            let text = text.trim();
            if let Some(rest) = text.strip_prefix("[x] ") {
                ("☑ ".to_string(), rest.trim())
            } else if let Some(rest) = text.strip_prefix("[ ] ") {
                ("☐ ".to_string(), rest.trim())
            } else {
                ("• ".to_string(), text)
            }
        }
    }
}

fn push_code_block(
    lines: &mut Vec<String>,
    language: Option<&str>,
    code: &str,
    width: usize,
    styles: Styles,
) {
    let label = language
        .filter(|language| !language.is_empty())
        .map(|language| format!(" code · {language} "))
        .unwrap_or_else(|| " code ".to_string());
    let rule_width = width.saturating_sub(2).max(12);
    let header_fill = rule_width.saturating_sub(label.chars().count()).max(1);
    lines.push(styles.dim(&format!("╭─{label}{}", "─".repeat(header_fill))));

    let code = code.trim_matches('\n');
    if code.is_empty() {
        lines.push(styles.dim("│"));
    } else {
        for line in code.lines() {
            lines.push(format!("{}{}", styles.dim("│ "), styles.code(line)));
        }
    }

    lines.push(styles.dim(&format!("╰{}", "─".repeat(rule_width + 1))));
}

fn push_table(lines: &mut Vec<String>, rows: &[Vec<String>], styles: Styles) {
    let column_count = rows.iter().map(Vec::len).max().unwrap_or_default();
    if column_count == 0 {
        return;
    }

    let mut widths = vec![3_usize; column_count];
    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            widths[index] = widths[index].max(cell.chars().count());
        }
    }

    lines.push(styles.dim(&format_table_border("┌", "┬", "┐", &widths)));
    for (row_index, row) in rows.iter().enumerate() {
        lines.push(format_table_row(row, &widths, styles));
        if row_index == 0 && rows.len() > 1 {
            lines.push(styles.dim(&format_table_border("├", "┼", "┤", &widths)));
        }
    }
    lines.push(styles.dim(&format_table_border("└", "┴", "┘", &widths)));
}

fn format_table_row(row: &[String], widths: &[usize], styles: Styles) -> String {
    let mut line = styles.dim("│");
    for (index, width) in widths.iter().enumerate() {
        let cell = row.get(index).map(String::as_str).unwrap_or_default();
        line.push(' ');
        line.push_str(cell);
        line.push_str(&" ".repeat(width.saturating_sub(cell.chars().count())));
        line.push(' ');
        line.push_str(&styles.dim("│"));
    }
    line
}

fn format_table_border(left: &str, middle: &str, right: &str, widths: &[usize]) -> String {
    let mut line = String::from(left);
    for (index, width) in widths.iter().enumerate() {
        if index > 0 {
            line.push_str(middle);
        }
        line.push_str(&"─".repeat(width + 2));
    }
    line.push_str(right);
    line
}

fn visible_width(value: &str) -> usize {
    let mut width = 0;
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            for ch in chars.by_ref() {
                if ch.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            width += 1;
        }
    }
    width
}

#[cfg(test)]
mod tests {
    use super::{RenderOptions, parse_markdown, render_markdown};

    #[test]
    fn parses_headings_lists_links_and_code_blocks() {
        let document = parse_markdown(
            "# Title\n\nA [link](docs/readme.md).\n\n- [x] Done\n- Open\n\n```rust\nlet x = 1;\n```\n",
        );

        assert_eq!(document.blocks.len(), 5);
        let rendered = render_markdown(
            "# Title\n\nA [link](docs/readme.md).\n\n- [x] Done\n- Open\n\n```rust\nlet x = 1;\n```\n",
            RenderOptions::default(),
        );

        assert!(rendered.contains("Title\n━━━━━━"));
        assert!(rendered.contains("A link (docs/readme.md)."));
        assert!(rendered.contains("☑ Done"));
        assert!(rendered.contains("╭─ code · rust"));
        assert!(rendered.contains("│ let x = 1;"));
    }

    #[test]
    fn wraps_paragraphs_to_requested_width() {
        let rendered = render_markdown(
            "This paragraph is deliberately long enough to wrap across several terminal lines.",
            RenderOptions {
                width: 32,
                ..RenderOptions::default()
            },
        );

        assert!(rendered.lines().count() > 1);
    }

    #[test]
    fn renders_frontmatter_and_simple_tables() {
        let rendered = render_markdown(
            "---\ntitle: Weekly Report\nowner: product\n---\n\n| Area | Status |\n| --- | --- |\n| CLI | Done |\n| TUI | Later |\n",
            RenderOptions::default(),
        );

        assert!(rendered.contains("╭─ metadata\n│ title: Weekly Report\n│ owner: product\n╰─"));
        assert!(rendered.contains("┌──────┬────────┐"));
        assert!(rendered.contains("│ Area │ Status │"));
        assert!(rendered.contains("│ CLI  │ Done   │"));
    }

    #[test]
    fn omits_raw_html_noise_but_keeps_markdown_images() {
        let rendered = render_markdown(
            "<p align=\"center\"><img src=\"badge.svg\"></p>\n\n![Diagram](diagram.png)\n",
            RenderOptions::default(),
        );

        assert!(!rendered.contains("<p"));
        assert!(rendered.contains("Diagram [image: diagram.png]"));
    }
}
