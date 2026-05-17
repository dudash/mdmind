use crate::model::{Document, ExternalRef, ExternalRefKind, MetadataEntry, Node, TaskState};
use crate::parser::parse_node_fragment;

pub fn import_document(source: &str, format: &str) -> Result<Document, String> {
    match format {
        "freemind" => import_freemind(source),
        "html" => import_html(source),
        "markdown" => import_markdown(source),
        "opml" => import_opml(source),
        _ => Err(format!(
            "Unsupported import format '{format}'. Choose one of: freemind, html, markdown, opml."
        )),
    }
}

fn import_html(source: &str) -> Result<Document, String> {
    let source = html_primary_content_region(source);
    let mut roots = Vec::new();
    let mut stack: Vec<StackNode> = Vec::new();
    let mut captures: Vec<HtmlCapture> = Vec::new();
    let mut current_heading_level = None;
    let mut list_depth = 0usize;
    let mut skip_stack: Vec<String> = Vec::new();
    let mut saw_content = false;
    let mut cursor = 0usize;

    while let Some(relative_start) = source[cursor..].find('<') {
        let start = cursor + relative_start;
        if start > cursor && skip_stack.is_empty() {
            push_html_text(&mut captures, &source[cursor..start]);
        }

        let Some(relative_end) = source[start..].find('>') else {
            return Err(format!(
                "HTML input contains an unterminated tag near line {}.",
                line_number(source, start)
            ));
        };
        let end = start + relative_end;
        let raw_tag = source[start + 1..end].trim();
        cursor = end + 1;

        if raw_tag.starts_with('!') || raw_tag.starts_with('?') {
            continue;
        }

        let lower_tag = raw_tag.to_ascii_lowercase();
        let closing = lower_tag.starts_with('/');
        let self_closing = lower_tag.ends_with('/');
        let name = if closing {
            html_tag_name(lower_tag.trim_start_matches('/'))
        } else {
            html_tag_name(lower_tag.trim_end_matches('/').trim_end())
        };

        if !skip_stack.is_empty() {
            if !closing && html_starts_chrome_region(&lower_tag, &name) && !self_closing {
                skip_stack.push(name);
            } else if closing {
                close_html_skip_region(&mut skip_stack, &name);
            }
            continue;
        }

        if lower_tag.starts_with("script") || lower_tag.starts_with("style") {
            let closing = if lower_tag.starts_with("script") {
                "</script>"
            } else {
                "</style>"
            };
            if let Some(close_relative) = source[cursor..].to_ascii_lowercase().find(closing) {
                cursor += close_relative + closing.len();
            }
            continue;
        }

        if !closing && html_starts_chrome_region(&lower_tag, &name) {
            if !self_closing {
                skip_stack.push(name);
            }
            continue;
        }

        if closing {
            match name.as_str() {
                "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "li" | "p" => {
                    saw_content |= finish_html_capture(
                        name.as_str(),
                        &mut captures,
                        &mut stack,
                        &mut roots,
                        &mut current_heading_level,
                    )?;
                }
                "ul" | "ol" => {
                    list_depth = list_depth.saturating_sub(1);
                }
                _ => {}
            }
            continue;
        }

        match name.as_str() {
            "br" => push_html_text(&mut captures, "\n"),
            "ul" | "ol" => {
                saw_content |= finish_html_capture(
                    "li",
                    &mut captures,
                    &mut stack,
                    &mut roots,
                    &mut current_heading_level,
                )?;
                list_depth += 1;
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => captures.push(HtmlCapture {
                kind: HtmlCaptureKind::Heading(html_heading_level(&name)),
                text: String::new(),
                line: line_number(source, start),
            }),
            "li" => captures.push(HtmlCapture {
                kind: HtmlCaptureKind::ListItem(list_depth.saturating_sub(1)),
                text: String::new(),
                line: line_number(source, start),
            }),
            "p" => captures.push(HtmlCapture {
                kind: HtmlCaptureKind::Paragraph,
                text: String::new(),
                line: line_number(source, start),
            }),
            _ => {}
        }

        if self_closing {
            saw_content |= finish_html_capture(
                name.as_str(),
                &mut captures,
                &mut stack,
                &mut roots,
                &mut current_heading_level,
            )?;
        }
    }

    if cursor < source.len() {
        push_html_text(&mut captures, &source[cursor..]);
    }

    while let Some(capture) = captures.pop() {
        saw_content |=
            process_html_capture(capture, &mut stack, &mut roots, &mut current_heading_level)?;
    }

    finish_stack(&mut stack, &mut roots);
    if !saw_content || roots.is_empty() {
        return Err(
            "HTML input did not contain any importable headings or list items.".to_string(),
        );
    }

    Ok(Document { nodes: roots })
}

fn import_freemind(source: &str) -> Result<Document, String> {
    let mut roots = Vec::new();
    let mut stack: Vec<Node> = Vec::new();
    let mut saw_node = false;
    let mut cursor = 0usize;

    while let Some(relative_start) = source[cursor..].find('<') {
        let start = cursor + relative_start;
        let Some(relative_end) = source[start..].find('>') else {
            return Err(format!(
                "FreeMind input contains an unterminated tag near line {}.",
                line_number(source, start)
            ));
        };
        let end = start + relative_end;
        let raw_tag = source[start + 1..end].trim();
        cursor = end + 1;

        if raw_tag.starts_with('?') || raw_tag.starts_with('!') {
            continue;
        }

        if raw_tag.eq_ignore_ascii_case("/node") {
            let Some(node) = stack.pop() else {
                return Err(format!(
                    "FreeMind input contains a </node> close tag without a matching start tag near line {}.",
                    line_number(source, start)
                ));
            };
            attach_node(node, &mut stack, &mut roots);
            continue;
        }

        if let Some(rest) = tag_start_rest(raw_tag, "node") {
            let self_closing = rest.trim_end().ends_with('/');
            let attributes_source = if self_closing {
                rest.trim_end().trim_end_matches('/').trim_end()
            } else {
                rest
            };
            let node = freemind_node_from_attrs(
                &parse_attributes(attributes_source, line_number(source, start))?,
                line_number(source, start),
            )?;
            saw_node = true;
            stack.push(node);
            if self_closing {
                let node = stack
                    .pop()
                    .expect("self-closing start should have just pushed a node");
                attach_node(node, &mut stack, &mut roots);
            }
            continue;
        }

        if let Some(rest) = tag_start_rest(raw_tag, "attribute") {
            let Some(node) = stack.last_mut() else {
                continue;
            };
            let attributes_source = rest.trim_end().trim_end_matches('/').trim_end();
            let attributes = parse_attributes(attributes_source, line_number(source, start))?;
            if let (Some(name), Some(value)) =
                (attr(&attributes, "NAME"), attr(&attributes, "VALUE"))
            {
                push_imported_metadata_or_detail(node, name, value);
            }
            continue;
        }

        if let Some(rest) = tag_start_rest(raw_tag, "icon") {
            let Some(node) = stack.last_mut() else {
                continue;
            };
            let attributes_source = rest.trim_end().trim_end_matches('/').trim_end();
            let attributes = parse_attributes(attributes_source, line_number(source, start))?;
            if let Some(value) = attr(&attributes, "BUILTIN") {
                push_imported_metadata_or_detail(node, "icon", value);
            }
            continue;
        }

        if let Some(rest) = tag_start_rest(raw_tag, "richcontent") {
            let Some(node) = stack.last_mut() else {
                continue;
            };
            let attributes_source = rest.trim_end().trim_end_matches('/').trim_end();
            let attributes = parse_attributes(attributes_source, line_number(source, start))?;
            if attr(&attributes, "TYPE").is_some_and(|value| value.eq_ignore_ascii_case("NOTE")) {
                let Some(close_relative) =
                    source[cursor..].to_ascii_lowercase().find("</richcontent>")
                else {
                    return Err(format!(
                        "FreeMind NOTE richcontent is missing a closing tag near line {}.",
                        line_number(source, start)
                    ));
                };
                let close_start = cursor + close_relative;
                let note = richcontent_to_detail_lines(&source[cursor..close_start]);
                node.detail.extend(note);
                cursor = close_start + "</richcontent>".len();
            }
        }
    }

    if !stack.is_empty() {
        return Err("FreeMind input contains unclosed <node> elements.".to_string());
    }
    if !saw_node {
        return Err("FreeMind input did not contain any <node> elements.".to_string());
    }

    Ok(Document { nodes: roots })
}

fn import_markdown(source: &str) -> Result<Document, String> {
    let mut roots = Vec::new();
    let mut stack: Vec<StackNode> = Vec::new();
    let mut current_heading_level = None;

    for (index, raw_line) in source.lines().enumerate() {
        let line = index + 1;
        if raw_line.trim().is_empty() {
            continue;
        }
        if raw_line.contains('\t') {
            return Err(format!(
                "Markdown import does not support tabs for indentation near line {line}."
            ));
        }

        if let Some((heading_level, label)) = parse_markdown_heading(raw_line) {
            let node = node_from_markdown_label(label, line)?;
            append_stack_node(
                StackNode {
                    level: heading_level,
                    node,
                },
                &mut stack,
                &mut roots,
            );
            current_heading_level = Some(heading_level);
            continue;
        }

        if let Some((indent, label)) = parse_markdown_bullet(raw_line) {
            if indent % 2 != 0 {
                return Err(format!(
                    "Markdown bullet indentation must use multiples of two spaces near line {line}."
                ));
            }
            let base_level = current_heading_level.map_or(0, |level| level + 1);
            let node = node_from_markdown_label(label, line)?;
            append_stack_node(
                StackNode {
                    level: base_level + (indent / 2),
                    node,
                },
                &mut stack,
                &mut roots,
            );
            continue;
        }

        let detail_target = ensure_markdown_detail_target(&mut stack, &mut roots);
        detail_target.detail.push(raw_line.trim().to_string());
    }

    finish_stack(&mut stack, &mut roots);
    if roots.is_empty() {
        return Err("Markdown input did not contain any importable content.".to_string());
    }

    Ok(Document { nodes: roots })
}

#[derive(Debug, Clone)]
struct StackNode {
    level: usize,
    node: Node,
}

fn append_stack_node(next: StackNode, stack: &mut Vec<StackNode>, roots: &mut Vec<Node>) {
    while stack
        .last()
        .is_some_and(|previous| previous.level >= next.level)
    {
        pop_stack_node(stack, roots);
    }
    stack.push(next);
}

fn finish_stack(stack: &mut Vec<StackNode>, roots: &mut Vec<Node>) {
    while !stack.is_empty() {
        pop_stack_node(stack, roots);
    }
}

fn pop_stack_node(stack: &mut Vec<StackNode>, roots: &mut Vec<Node>) {
    let finished = stack.pop().expect("stack should not be empty").node;
    if let Some(parent) = stack.last_mut() {
        parent.node.children.push(finished);
    } else {
        roots.push(finished);
    }
}

#[derive(Debug, Clone)]
struct HtmlCapture {
    kind: HtmlCaptureKind,
    text: String,
    line: usize,
}

#[derive(Debug, Clone)]
enum HtmlCaptureKind {
    Heading(usize),
    ListItem(usize),
    Paragraph,
}

fn html_primary_content_region(source: &str) -> &str {
    let article = largest_html_region(source, "article");
    let main = largest_html_region(source, "main");
    match (article, main) {
        (Some(article), Some(main)) if main.len() > article.len() => main,
        (Some(article), _) => article,
        (None, Some(main)) => main,
        (None, None) => source,
    }
}

fn largest_html_region<'a>(source: &'a str, tag_name: &str) -> Option<&'a str> {
    let mut best = None;
    let mut cursor = 0usize;
    while let Some((start, end)) = next_html_region(source, tag_name, cursor) {
        let candidate = &source[start..end];
        if best.is_none_or(|current: &str| candidate.len() > current.len()) {
            best = Some(candidate);
        }
        cursor = end;
    }
    best
}

fn next_html_region(source: &str, tag_name: &str, cursor: usize) -> Option<(usize, usize)> {
    let mut cursor = cursor;
    let mut region_start = None;
    let mut depth = 0usize;

    while let Some(relative_start) = source[cursor..].find('<') {
        let start = cursor + relative_start;
        let relative_end = source[start..].find('>')?;
        let end = start + relative_end;
        let raw_tag = source[start + 1..end].trim();
        cursor = end + 1;

        if raw_tag.starts_with('!') || raw_tag.starts_with('?') {
            continue;
        }

        let lower_tag = raw_tag.to_ascii_lowercase();
        let closing = lower_tag.starts_with('/');
        let self_closing = lower_tag.ends_with('/');
        let name = if closing {
            html_tag_name(lower_tag.trim_start_matches('/'))
        } else {
            html_tag_name(lower_tag.trim_end_matches('/').trim_end())
        };
        if name != tag_name {
            continue;
        }

        if closing {
            if depth > 0 {
                depth -= 1;
                if depth == 0 {
                    return region_start.map(|region_start| (region_start, cursor));
                }
            }
        } else if !self_closing {
            if depth == 0 {
                region_start = Some(start);
            }
            depth += 1;
        }
    }

    None
}

fn html_tag_name(raw_tag: &str) -> String {
    raw_tag
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .trim_matches('/')
        .to_string()
}

fn html_heading_level(name: &str) -> usize {
    name.strip_prefix('h')
        .and_then(|level| level.parse::<usize>().ok())
        .unwrap_or(1)
        .saturating_sub(1)
}

fn is_html_chrome_tag(name: &str) -> bool {
    matches!(
        name,
        "nav"
            | "aside"
            | "footer"
            | "form"
            | "button"
            | "select"
            | "option"
            | "textarea"
            | "noscript"
            | "iframe"
            | "svg"
            | "canvas"
    )
}

fn html_starts_chrome_region(lower_tag: &str, name: &str) -> bool {
    is_html_chrome_tag(name) || html_has_chrome_attributes(lower_tag, name)
}

fn close_html_skip_region(skip_stack: &mut Vec<String>, name: &str) {
    if let Some(index) = skip_stack.iter().rposition(|open_name| open_name == name) {
        skip_stack.truncate(index);
    }
}

fn html_has_chrome_attributes(lower_tag: &str, name: &str) -> bool {
    let rest = lower_tag
        .strip_prefix(name)
        .unwrap_or(lower_tag)
        .trim_end_matches('/')
        .trim();
    html_attribute_marks_chrome(rest, "class")
        || html_attribute_marks_chrome(rest, "id")
        || html_attribute_marks_chrome(rest, "role")
        || html_attribute_marks_chrome(rest, "aria-label")
}

fn html_attribute_marks_chrome(source: &str, wanted_key: &str) -> bool {
    let bytes = source.as_bytes();
    let mut cursor = 0usize;

    while cursor < bytes.len() {
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() {
            break;
        }

        let key_start = cursor;
        while cursor < bytes.len()
            && !bytes[cursor].is_ascii_whitespace()
            && bytes[cursor] != b'='
            && bytes[cursor] != b'/'
            && bytes[cursor] != b'>'
        {
            cursor += 1;
        }
        let key = &source[key_start..cursor];

        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] != b'=' {
            while cursor < bytes.len() && !bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            continue;
        }
        cursor += 1;

        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() {
            break;
        }

        let value_start;
        let value_end;
        if bytes[cursor] == b'"' || bytes[cursor] == b'\'' {
            let quote = bytes[cursor];
            cursor += 1;
            value_start = cursor;
            while cursor < bytes.len() && bytes[cursor] != quote {
                cursor += 1;
            }
            value_end = cursor;
            if cursor < bytes.len() {
                cursor += 1;
            }
        } else {
            value_start = cursor;
            while cursor < bytes.len()
                && !bytes[cursor].is_ascii_whitespace()
                && bytes[cursor] != b'>'
            {
                cursor += 1;
            }
            value_end = cursor;
        }

        if key == wanted_key && html_attribute_value_marks_chrome(&source[value_start..value_end]) {
            return true;
        }
    }

    false
}

fn html_attribute_value_marks_chrome(value: &str) -> bool {
    value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .any(|token| {
            matches!(
                token,
                "nav"
                    | "navbar"
                    | "navigation"
                    | "menu"
                    | "sidebar"
                    | "widget"
                    | "footer"
                    | "breadcrumb"
                    | "pagination"
                    | "related"
                    | "search"
                    | "contentinfo"
                    | "complementary"
            ) || token.starts_with("share")
                || token.starts_with("social")
                || token.starts_with("comment")
                || token.starts_with("reply")
                || token.starts_with("subscribe")
                || token.starts_with("newsletter")
        })
}

fn push_html_text(captures: &mut [HtmlCapture], text: &str) {
    let Some(capture) = captures.last_mut() else {
        return;
    };
    capture.text.push_str(&decode_xml_entities(text));
    capture.text.push(' ');
}

fn finish_html_capture(
    tag_name: &str,
    captures: &mut Vec<HtmlCapture>,
    stack: &mut Vec<StackNode>,
    roots: &mut Vec<Node>,
    current_heading_level: &mut Option<usize>,
) -> Result<bool, String> {
    let Some(index) = captures.iter().rposition(|capture| {
        matches!(
            (&capture.kind, tag_name),
            (
                HtmlCaptureKind::Heading(_),
                "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
            ) | (HtmlCaptureKind::ListItem(_), "li")
                | (HtmlCaptureKind::Paragraph, "p")
        )
    }) else {
        return Ok(false);
    };

    let capture = captures.remove(index);
    process_html_capture(capture, stack, roots, current_heading_level)
}

fn process_html_capture(
    capture: HtmlCapture,
    stack: &mut Vec<StackNode>,
    roots: &mut Vec<Node>,
    current_heading_level: &mut Option<usize>,
) -> Result<bool, String> {
    let text = normalize_html_text(&capture.text);
    if text.is_empty() {
        return Ok(false);
    }
    if is_html_chrome_text(&text) {
        return Ok(false);
    }

    match capture.kind {
        HtmlCaptureKind::Heading(level) => {
            let node = node_from_markdown_label(&text, capture.line)?;
            append_stack_node(StackNode { level, node }, stack, roots);
            *current_heading_level = Some(level);
        }
        HtmlCaptureKind::ListItem(list_level) => {
            let base_level = current_heading_level.as_ref().map_or(0, |level| level + 1);
            let node = node_from_markdown_label(&text, capture.line)?;
            append_stack_node(
                StackNode {
                    level: base_level + list_level,
                    node,
                },
                stack,
                roots,
            );
        }
        HtmlCaptureKind::Paragraph => {
            if let Some((label, detail)) = numbered_html_paragraph_parts(&text) {
                let base_level = current_heading_level.as_ref().map_or(0, |level| level + 1);
                let mut node = plain_node(&label, capture.line);
                if !detail.is_empty() {
                    node.detail.push(detail);
                }
                append_stack_node(node_stack_entry(base_level, node), stack, roots);
                return Ok(true);
            }
            let detail_target = ensure_markdown_detail_target(stack, roots);
            detail_target.detail.push(text);
        }
    }

    Ok(true)
}

fn node_stack_entry(level: usize, node: Node) -> StackNode {
    StackNode { level, node }
}

fn normalize_html_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_html_chrome_text(value: &str) -> bool {
    let lower = value
        .trim()
        .trim_matches(':')
        .trim_matches('|')
        .to_ascii_lowercase();
    lower == "share this"
        || lower == "like this"
        || lower == "post navigation"
        || lower == "related posts"
        || lower == "leave a comment"
        || lower.starts_with("share on ")
}

fn numbered_html_paragraph_parts(value: &str) -> Option<(String, String)> {
    let trimmed = value.trim();
    let mut chars = trimmed.char_indices();
    let mut digit_end = None;
    for (index, ch) in &mut chars {
        if ch.is_ascii_digit() {
            digit_end = Some(index + ch.len_utf8());
        } else if ch == '.' {
            break;
        } else {
            return None;
        }
    }

    let digit_end = digit_end?;
    let after_digits = &trimmed[digit_end..];
    let after_dot = after_digits.strip_prefix('.')?;
    if !after_dot.starts_with(char::is_whitespace) {
        return None;
    }

    let (label, detail) = split_long_html_node_label(trimmed);
    Some((label, detail))
}

fn split_long_html_node_label(value: &str) -> (String, String) {
    const MAX_LABEL_CHARS: usize = 120;
    if value.chars().count() <= MAX_LABEL_CHARS {
        return (value.to_string(), String::new());
    }

    let mut split_at = 0usize;
    for (chars, (index, ch)) in value.char_indices().enumerate() {
        if chars > MAX_LABEL_CHARS {
            break;
        }
        if ch.is_whitespace() {
            split_at = index;
        }
    }

    if split_at == 0 {
        return (value.to_string(), String::new());
    }

    let label = value[..split_at].trim().to_string();
    let detail = value[split_at..].trim().to_string();
    (label, detail)
}

fn ensure_markdown_detail_target<'a>(
    stack: &'a mut Vec<StackNode>,
    roots: &mut Vec<Node>,
) -> &'a mut Node {
    if stack.is_empty() {
        append_stack_node(
            StackNode {
                level: 0,
                node: plain_node("Imported Markdown", 1),
            },
            stack,
            roots,
        );
    }
    &mut stack.last_mut().expect("detail target should exist").node
}

fn parse_markdown_heading(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim_start();
    let level = trimmed.chars().take_while(|ch| *ch == '#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let rest = &trimmed[level..];
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    Some((level - 1, rest.trim()))
}

fn parse_markdown_bullet(line: &str) -> Option<(usize, &str)> {
    let indent = line
        .as_bytes()
        .iter()
        .take_while(|byte| **byte == b' ')
        .count();
    let trimmed = &line[indent..];
    for marker in ["- ", "* ", "+ "] {
        if let Some(label) = trimmed.strip_prefix(marker) {
            return Some((indent, label.trim()));
        }
    }
    None
}

fn node_from_markdown_label(label: &str, line: usize) -> Result<Node, String> {
    if label.trim().is_empty() {
        return Err(format!(
            "Markdown import found an empty heading or bullet near line {line}."
        ));
    }
    parse_node_fragment(label).map_err(|diagnostics| {
        let messages = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>()
            .join("; ");
        format!("Markdown import could not parse node syntax near line {line}: {messages}")
    })
}

fn plain_node(text: &str, line: usize) -> Node {
    Node {
        text: text.to_string(),
        task: None,
        detail: Vec::new(),
        tags: Vec::new(),
        metadata: Vec::new(),
        id: None,
        references: Vec::new(),
        relations: Vec::new(),
        children: Vec::new(),
        line,
    }
}

fn freemind_node_from_attrs(attributes: &[(String, String)], line: usize) -> Result<Node, String> {
    let label = attr(attributes, "TEXT")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("FreeMind node near line {line} is missing TEXT."))?;

    let mut node = plain_node(label, line);
    node.id = attr(attributes, "ID")
        .map(str::trim)
        .filter(|value| is_safe_id(value))
        .map(str::to_string);
    if let Some(link) = attr(attributes, "LINK").filter(|value| !value.trim().is_empty()) {
        node.references.push(ExternalRef {
            label: "link".to_string(),
            target: link.to_string(),
            kind: ExternalRefKind::Link,
        });
    }
    for (key, value) in attributes {
        if matches!(key.as_str(), "TEXT" | "ID" | "LINK") || value.trim().is_empty() {
            continue;
        }
        push_imported_metadata_or_detail(&mut node, key, value);
    }

    Ok(node)
}

fn import_opml(source: &str) -> Result<Document, String> {
    let events = opml_outline_events(source)?;
    if events.is_empty() {
        return Err("OPML input did not contain any <outline> nodes.".to_string());
    }

    let mut roots = Vec::new();
    let mut stack: Vec<Node> = Vec::new();

    for event in events {
        match event.kind {
            OutlineEventKind::Start { self_closing } => {
                let node = node_from_attrs(&event.attributes, event.line)?;
                stack.push(node);

                if self_closing {
                    let node = stack
                        .pop()
                        .expect("self-closing start should have just pushed a node");
                    attach_node(node, &mut stack, &mut roots);
                }
            }
            OutlineEventKind::End => {
                let Some(node) = stack.pop() else {
                    return Err(format!(
                        "OPML contains an </outline> close tag without a matching start tag near line {}.",
                        event.line
                    ));
                };
                attach_node(node, &mut stack, &mut roots);
            }
        }
    }

    if !stack.is_empty() {
        return Err("OPML contains unclosed <outline> nodes.".to_string());
    }

    Ok(Document { nodes: roots })
}

fn attach_node(node: Node, stack: &mut [Node], roots: &mut Vec<Node>) {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(node);
    } else {
        roots.push(node);
    }
}

#[derive(Debug, Clone)]
struct OutlineEvent {
    kind: OutlineEventKind,
    attributes: Vec<(String, String)>,
    line: usize,
}

#[derive(Debug, Clone)]
enum OutlineEventKind {
    Start { self_closing: bool },
    End,
}

fn opml_outline_events(source: &str) -> Result<Vec<OutlineEvent>, String> {
    let mut events = Vec::new();
    let mut cursor = 0usize;

    while let Some(relative_start) = source[cursor..].find('<') {
        let start = cursor + relative_start;
        let Some(relative_end) = source[start..].find('>') else {
            return Err(format!(
                "OPML contains an unterminated tag near line {}.",
                line_number(source, start)
            ));
        };
        let end = start + relative_end;
        let raw_tag = source[start + 1..end].trim();
        cursor = end + 1;

        if raw_tag.starts_with('?') || raw_tag.starts_with('!') {
            continue;
        }

        if raw_tag.eq_ignore_ascii_case("/outline") {
            events.push(OutlineEvent {
                kind: OutlineEventKind::End,
                attributes: Vec::new(),
                line: line_number(source, start),
            });
            continue;
        }

        let Some(rest) = outline_start_rest(raw_tag) else {
            continue;
        };
        let self_closing = rest.trim_end().ends_with('/');
        let attributes_source = if self_closing {
            rest.trim_end().trim_end_matches('/').trim_end()
        } else {
            rest
        };

        events.push(OutlineEvent {
            kind: OutlineEventKind::Start { self_closing },
            attributes: parse_attributes(attributes_source, line_number(source, start))?,
            line: line_number(source, start),
        });
    }

    Ok(events)
}

fn outline_start_rest(raw_tag: &str) -> Option<&str> {
    tag_start_rest(raw_tag, "outline")
}

fn tag_start_rest<'a>(raw_tag: &'a str, expected_name: &str) -> Option<&'a str> {
    if raw_tag.len() < expected_name.len() {
        return None;
    }
    let (name, rest) = raw_tag.split_at(expected_name.len());
    if !name.eq_ignore_ascii_case(expected_name) {
        return None;
    }
    if rest.is_empty() || rest.starts_with(char::is_whitespace) || rest.starts_with('/') {
        Some(rest.trim_start())
    } else {
        None
    }
}

fn parse_attributes(source: &str, line: usize) -> Result<Vec<(String, String)>, String> {
    let bytes = source.as_bytes();
    let mut attributes = Vec::new();
    let mut cursor = 0usize;

    while cursor < bytes.len() {
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() {
            break;
        }

        let key_start = cursor;
        while cursor < bytes.len()
            && !bytes[cursor].is_ascii_whitespace()
            && bytes[cursor] != b'='
            && bytes[cursor] != b'/'
        {
            cursor += 1;
        }
        let key = source[key_start..cursor].trim();
        if key.is_empty() {
            return Err(format!(
                "Import XML has an invalid attribute near line {line}."
            ));
        }

        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] != b'=' {
            return Err(format!(
                "Import XML attribute '{key}' is missing a value near line {line}."
            ));
        }
        cursor += 1;

        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() {
            return Err(format!(
                "Import XML attribute '{key}' is missing a value near line {line}."
            ));
        }

        let value = if bytes[cursor] == b'"' || bytes[cursor] == b'\'' {
            let quote = bytes[cursor];
            cursor += 1;
            let value_start = cursor;
            while cursor < bytes.len() && bytes[cursor] != quote {
                cursor += 1;
            }
            if cursor >= bytes.len() {
                return Err(format!(
                    "Import XML attribute '{key}' has an unterminated quoted value near line {line}."
                ));
            }
            let value = source[value_start..cursor].to_string();
            cursor += 1;
            value
        } else {
            let value_start = cursor;
            while cursor < bytes.len() && !bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            source[value_start..cursor].to_string()
        };

        attributes.push((key.to_string(), decode_xml_entities(&value)));
    }

    Ok(attributes)
}

fn node_from_attrs(attributes: &[(String, String)], line: usize) -> Result<Node, String> {
    let label = attr(attributes, "text")
        .or_else(|| attr(attributes, "title"))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("OPML outline node near line {line} is missing text/title."))?;

    let mut detail = Vec::new();
    if let Some(value) = attr(attributes, "mdm_detail") {
        detail.extend(value.lines().map(str::to_string));
    }
    if let Some(value) = attr(attributes, "_note").or_else(|| attr(attributes, "note")) {
        detail.extend(value.lines().map(str::to_string));
    }

    let mut metadata = Vec::new();
    let mut references = Vec::new();
    if let Some(value) = attr(attributes, "mdm_refs") {
        match references_from_mdm_refs(value) {
            Ok(imported_references) => references.extend(imported_references),
            Err(message) => detail.push(format!("mdm_refs: {value} ({message})")),
        }
    }
    for (key, value) in attributes {
        if value.trim().is_empty() {
            continue;
        }
        match key.as_str() {
            "text" | "title" | "mdm_detail" | "_note" | "note" | "mdm_tags" | "mdm_id"
            | "mdm_task" | "mdm_task_progress" | "mdm_task_blocked" | "mdm_refs" => {}
            "url" | "htmlUrl" | "xmlUrl" => references.push(ExternalRef {
                label: key.clone(),
                target: value.clone(),
                kind: ExternalRefKind::Link,
            }),
            _ if is_safe_metadata_key(key) && is_safe_metadata_value(value) => {
                metadata.push(MetadataEntry {
                    key: key.clone(),
                    value: value.clone(),
                });
            }
            _ => detail.push(format!("{key}: {value}")),
        }
    }

    Ok(Node {
        text: label.to_string(),
        task: attr(attributes, "mdm_task").and_then(parse_task_state),
        detail,
        tags: parse_tags(attr(attributes, "mdm_tags")),
        metadata,
        id: attr(attributes, "mdm_id")
            .map(str::trim)
            .filter(|value| is_safe_id(value))
            .map(str::to_string),
        references,
        relations: Vec::new(),
        children: Vec::new(),
        line,
    })
}

fn attr<'a>(attributes: &'a [(String, String)], name: &str) -> Option<&'a str> {
    attributes
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
}

fn push_imported_metadata_or_detail(node: &mut Node, key: &str, value: &str) {
    let key = normalize_metadata_key(key);
    if is_safe_metadata_key(&key) && is_safe_metadata_value(value) {
        node.metadata.push(MetadataEntry {
            key,
            value: value.to_string(),
        });
    } else {
        node.detail.push(format!("{key}: {value}"));
    }
}

fn normalize_metadata_key(value: &str) -> String {
    let normalized = value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    if normalized.is_empty() {
        "source".to_string()
    } else {
        normalized
    }
}

fn parse_task_state(value: &str) -> Option<TaskState> {
    match value.trim().to_ascii_lowercase().as_str() {
        "open" | "todo" | "unchecked" => Some(TaskState::Open),
        "done" | "complete" | "completed" | "checked" => Some(TaskState::Done),
        _ => None,
    }
}

fn parse_tags(value: Option<&str>) -> Vec<String> {
    value
        .into_iter()
        .flat_map(str::split_whitespace)
        .filter_map(|tag| {
            let tag = tag.trim();
            if tag.is_empty() {
                None
            } else if tag.starts_with('#') {
                Some(tag.to_string())
            } else {
                Some(format!("#{tag}"))
            }
        })
        .collect()
}

fn references_from_mdm_refs(value: &str) -> Result<Vec<ExternalRef>, String> {
    let fragment = format!("Imported refs {value}");
    parse_node_fragment(&fragment)
        .map(|node| node.references)
        .map_err(|diagnostics| {
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>()
                .join("; ")
        })
}

fn is_safe_metadata_key(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
}

fn is_safe_metadata_value(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| !ch.is_whitespace() && !matches!(ch, '[' | ']' | '(' | ')'))
}

fn is_safe_id(value: &str) -> bool {
    is_safe_metadata_value(value)
}

fn decode_xml_entities(value: &str) -> String {
    let named = value
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");

    let mut decoded = String::new();
    let mut cursor = 0usize;
    while let Some(relative_start) = named[cursor..].find("&#") {
        let start = cursor + relative_start;
        decoded.push_str(&named[cursor..start]);
        let Some(relative_end) = named[start..].find(';') else {
            decoded.push_str(&named[start..]);
            return decoded;
        };
        let end = start + relative_end;
        let entity = &named[start + 2..end];
        let codepoint = if let Some(hex) = entity.strip_prefix(['x', 'X']) {
            u32::from_str_radix(hex, 16).ok()
        } else {
            entity.parse::<u32>().ok()
        };
        match codepoint.and_then(char::from_u32) {
            Some(ch) => decoded.push(ch),
            None => decoded.push_str(&named[start..=end]),
        }
        cursor = end + 1;
    }
    decoded.push_str(&named[cursor..]);
    decoded
}

fn richcontent_to_detail_lines(source: &str) -> Vec<String> {
    let mut text = String::new();
    let mut cursor = 0usize;
    while let Some(relative_start) = source[cursor..].find('<') {
        let start = cursor + relative_start;
        text.push_str(&source[cursor..start]);
        let Some(relative_end) = source[start..].find('>') else {
            text.push_str(&source[start..]);
            return detail_lines_from_text(&decode_xml_entities(&text));
        };
        let end = start + relative_end;
        let tag = source[start + 1..end].trim().to_ascii_lowercase();
        if tag.starts_with("br")
            || tag.starts_with("/p")
            || tag.starts_with("/li")
            || tag.starts_with("/div")
        {
            text.push('\n');
        } else {
            text.push(' ');
        }
        cursor = end + 1;
    }
    text.push_str(&source[cursor..]);
    detail_lines_from_text(&decode_xml_entities(&text))
}

fn detail_lines_from_text(value: &str) -> Vec<String> {
    value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

fn line_number(source: &str, offset: usize) -> usize {
    source[..offset]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_document;
    use crate::serializer::serialize_document;
    use crate::validate::validate_document;

    #[test]
    fn imports_nested_opml_outline_nodes() {
        let source = r##"<?xml version="1.0"?>
<opml version="2.0">
  <body>
    <outline text="Project" mdm_id="project" mdm_tags="#idea" owner="jason">
      <outline text="MVP Scope" mdm_task="open" mdm_detail="First line&#10;Second line" status="active" />
      <outline text="External" url="https://example.com" />
    </outline>
  </body>
</opml>"##;

        let document = import_document(source, "opml").expect("opml should import");
        assert_eq!(document.nodes[0].text, "Project");
        assert_eq!(document.nodes[0].id.as_deref(), Some("project"));
        assert_eq!(document.nodes[0].metadata[0].key, "owner");
        assert_eq!(document.nodes[0].children[0].task, Some(TaskState::Open));
        assert_eq!(
            document.nodes[0].children[0].detail,
            vec!["First line".to_string(), "Second line".to_string()]
        );
        assert_eq!(
            document.nodes[0].children[1].references[0].target,
            "https://example.com"
        );

        let serialized = serialize_document(&document);
        let parsed = parse_document(&serialized);
        assert!(
            parsed.diagnostics.is_empty(),
            "serialized import should parse cleanly: {:?}\n{}",
            parsed.diagnostics,
            serialized
        );
        assert!(
            validate_document(&parsed.document).is_empty(),
            "serialized import should validate cleanly"
        );
    }

    #[test]
    fn imports_mdm_refs_from_round_trip_opml() {
        let source = r#"<opml version="2.0">
  <body>
    <outline text="Research" mdm_refs="[brief](docs/brief.md) ![diagram](assets/diagram.png)" />
  </body>
</opml>"#;

        let document = import_document(source, "opml").expect("opml should import");
        let references = &document.nodes[0].references;
        assert_eq!(references.len(), 2);
        assert_eq!(references[0].label, "brief");
        assert_eq!(references[0].target, "docs/brief.md");
        assert_eq!(references[0].kind, ExternalRefKind::Link);
        assert_eq!(references[1].label, "diagram");
        assert_eq!(references[1].target, "assets/diagram.png");
        assert_eq!(references[1].kind, ExternalRefKind::Image);
    }

    #[test]
    fn imports_markdown_headings_bullets_and_details() {
        let source = "# Project #idea [id:project]\n\nIntro detail.\n\n## Tasks\n\n- [ ] First task #todo @status:active\n  - Nested task note\n";

        let document = import_document(source, "markdown").expect("markdown should import");
        let root = &document.nodes[0];
        assert_eq!(root.text, "Project");
        assert_eq!(root.tags, vec!["#idea"]);
        assert_eq!(root.id.as_deref(), Some("project"));
        assert_eq!(root.detail, vec!["Intro detail."]);
        assert_eq!(root.children[0].text, "Tasks");
        assert_eq!(root.children[0].children[0].task, Some(TaskState::Open));
        assert_eq!(root.children[0].children[0].text, "First task");
        assert_eq!(
            root.children[0].children[0].children[0].text,
            "Nested task note"
        );

        let serialized = serialize_document(&document);
        let parsed = parse_document(&serialized);
        assert!(
            parsed.diagnostics.is_empty(),
            "serialized import should parse cleanly: {:?}\n{}",
            parsed.diagnostics,
            serialized
        );
    }

    #[test]
    fn imports_bullet_only_markdown_as_roots() {
        let source = "- Root\n  - Child\n- Other Root\n";
        let document = import_document(source, "markdown").expect("markdown should import");
        assert_eq!(document.nodes.len(), 2);
        assert_eq!(document.nodes[0].text, "Root");
        assert_eq!(document.nodes[0].children[0].text, "Child");
        assert_eq!(document.nodes[1].text, "Other Root");
    }

    #[test]
    fn imports_loose_markdown_text_as_detail_under_default_root() {
        let source = "Loose paragraph.\nAnother line.\n";
        let document = import_document(source, "markdown").expect("markdown should import");
        assert_eq!(document.nodes[0].text, "Imported Markdown");
        assert_eq!(
            document.nodes[0].detail,
            vec!["Loose paragraph.".to_string(), "Another line.".to_string()]
        );
    }

    #[test]
    fn rejects_empty_markdown_headings() {
        let error = import_document("# \n", "markdown").expect_err("empty heading should fail");
        assert!(error.contains("empty heading or bullet"));
    }

    #[test]
    fn imports_html_headings_lists_and_paragraph_details() {
        let source = r#"
<!doctype html>
<html>
  <head><title>Ignored</title><style>.hidden { display: none; }</style></head>
  <body>
    <h1>Imported Web Page [id:web/page]</h1>
    <p>Intro &amp; context.</p>
    <h2>Actions</h2>
    <ul>
      <li>[ ] Capture summary #todo @status:active</li>
      <li>Nested
        <ul>
          <li>Child item</li>
        </ul>
      </li>
    </ul>
  </body>
</html>"#;

        let document = import_document(source, "html").expect("html should import");
        let root = &document.nodes[0];
        assert_eq!(root.text, "Imported Web Page");
        assert_eq!(root.id.as_deref(), Some("web/page"));
        assert_eq!(root.detail, vec!["Intro & context."]);
        assert_eq!(root.children[0].text, "Actions");
        assert_eq!(root.children[0].children[0].task, Some(TaskState::Open));
        assert_eq!(root.children[0].children[0].tags, vec!["#todo"]);
        assert_eq!(root.children[0].children[1].text, "Nested");
        assert_eq!(root.children[0].children[1].children[0].text, "Child item");

        let serialized = serialize_document(&document);
        let parsed = parse_document(&serialized);
        assert!(
            parsed.diagnostics.is_empty(),
            "serialized import should parse cleanly: {:?}\n{}",
            parsed.diagnostics,
            serialized
        );
    }

    #[test]
    fn imports_html_primary_content_and_numbered_article_paragraphs() {
        let source = r#"
<!doctype html>
<html>
  <body>
    <nav><h1>Site Menu</h1><ul><li>Home</li></ul></nav>
    <main>
      <aside><h2>Archives</h2><ul><li>January</li></ul></aside>
      <article>
        <h1>Ranked Article</h1>
        <p>Intro paragraph.</p>
        <p>2. Second entry title with enough extra words to become its own node and carry remaining text as detail because imported article list paragraphs are often long.</p>
        <p>Supporting context for second entry.</p>
        <p>1. First entry title.</p>
        <div class="sharedaddy"><h2>Share this:</h2><ul><li>Share on Email</li></ul></div>
        <section id="comments"><h2>Comments</h2><p>Reader note.</p></section>
      </article>
    </main>
    <footer><h2>Footer Links</h2></footer>
  </body>
</html>"#;

        let document = import_document(source, "html").expect("html should import");
        assert_eq!(document.nodes.len(), 1);
        let root = &document.nodes[0];
        assert_eq!(root.text, "Ranked Article");
        assert_eq!(root.detail, vec!["Intro paragraph."]);
        assert_eq!(root.children.len(), 2);
        assert!(root.children[0].text.starts_with("2. Second entry title"));
        assert!(!root.children[0].detail[0].is_empty());
        assert!(
            root.children[0]
                .detail
                .contains(&"Supporting context for second entry.".to_string())
        );
        assert_eq!(root.children[1].text, "1. First entry title.");
        assert!(!serialize_document(&document).contains("Site Menu"));
        assert!(!serialize_document(&document).contains("Archives"));
        assert!(!serialize_document(&document).contains("Footer Links"));
        assert!(!serialize_document(&document).contains("Share on Email"));
        assert!(!serialize_document(&document).contains("Reader note"));
    }

    #[test]
    fn imports_freemind_nodes_notes_attributes_and_links() {
        let source = r#"<map version="1.0.1">
  <node TEXT="Project" ID="project" CREATED="123">
    <richcontent TYPE="NOTE"><html><body><p>First note</p><p>Second note</p></body></html></richcontent>
    <attribute NAME="owner" VALUE="jason" />
    <node TEXT="Reference" LINK="https://example.com" />
  </node>
</map>"#;

        let document = import_document(source, "freemind").expect("freemind should import");
        let root = &document.nodes[0];
        assert_eq!(root.text, "Project");
        assert_eq!(root.id.as_deref(), Some("project"));
        assert!(root.metadata.iter().any(|entry| entry.key == "created"));
        assert!(
            root.metadata
                .iter()
                .any(|entry| entry.key == "owner" && entry.value == "jason")
        );
        assert_eq!(root.detail, vec!["First note", "Second note"]);
        assert_eq!(root.children[0].text, "Reference");
        assert_eq!(root.children[0].references[0].target, "https://example.com");

        let serialized = serialize_document(&document);
        let parsed = parse_document(&serialized);
        assert!(
            parsed.diagnostics.is_empty(),
            "serialized import should parse cleanly: {:?}\n{}",
            parsed.diagnostics,
            serialized
        );
    }

    #[test]
    fn rejects_freemind_nodes_without_text() {
        let error = import_document(r#"<map><node ID="missing-text" /></map>"#, "freemind")
            .expect_err("missing TEXT should fail");
        assert!(error.contains("missing TEXT"));
    }

    #[test]
    fn rejects_opml_without_outline_nodes() {
        let error =
            import_document("<opml><body /></opml>", "opml").expect_err("empty OPML should fail");
        assert!(error.contains("did not contain any <outline>"));
    }

    #[test]
    fn imports_title_fallback_notes_entities_and_quoted_attributes() {
        let source = r#"<opml version="2.0">
  <body>
    <outline title='Roadmap &amp; Research' _note="Alpha&#10;Beta &#x2713;" priority="high" owner='jason' unsafe_name="two words">
      <outline text="Child &lt;One&gt;" note='Detail &amp; context' />
    </outline>
  </body>
</opml>"#;

        let document = import_document(source, "opml").expect("opml should import");
        let root = &document.nodes[0];
        assert_eq!(root.text, "Roadmap & Research");
        assert_eq!(root.detail[0], "Alpha");
        assert_eq!(root.detail[1], "Beta ✓");
        assert_eq!(
            root.metadata,
            vec![
                MetadataEntry {
                    key: "priority".to_string(),
                    value: "high".to_string(),
                },
                MetadataEntry {
                    key: "owner".to_string(),
                    value: "jason".to_string(),
                }
            ]
        );
        assert!(
            root.detail
                .iter()
                .any(|line| line == "unsafe_name: two words")
        );
        assert_eq!(root.children[0].text, "Child <One>");
        assert_eq!(root.children[0].detail, vec!["Detail & context"]);

        let serialized = serialize_document(&document);
        let parsed = parse_document(&serialized);
        assert!(
            parsed.diagnostics.is_empty(),
            "serialized import should parse cleanly: {:?}\n{}",
            parsed.diagnostics,
            serialized
        );
    }

    #[test]
    fn rejects_outline_nodes_without_text_or_title() {
        let error = import_document(
            r#"<opml version="2.0"><body><outline mdm_id="missing-label" /></body></opml>"#,
            "opml",
        )
        .expect_err("missing text/title should fail");
        assert!(error.contains("missing text/title"));
    }

    #[test]
    fn rejects_extra_outline_close_tags() {
        let error = import_document(
            r#"<opml version="2.0"><body><outline text="Root" /></outline></body></opml>"#,
            "opml",
        )
        .expect_err("extra close tag should fail");
        assert!(error.contains("without a matching start tag"));
    }

    #[test]
    fn rejects_unclosed_outline_tags() {
        let error = import_document(
            r#"<opml version="2.0"><body><outline text="Root"><outline text="Child" /></body></opml>"#,
            "opml",
        )
        .expect_err("unclosed tag should fail");
        assert!(error.contains("unclosed <outline>"));
    }
}
