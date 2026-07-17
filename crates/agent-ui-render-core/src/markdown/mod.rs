use crate::render::escape_html;

#[must_use]
pub fn markdown_to_html(source: &str) -> String {
    let mut parts = Vec::new();
    let mut paragraph = Vec::new();
    let mut list_items: Vec<String> = Vec::new();
    let mut list_ordered = false;
    let mut in_code = false;
    let mut code_lang = String::new();
    let mut code_lines = Vec::new();

    for raw_line in source.lines() {
        let line = raw_line.trim_end();
        if in_code {
            if line.trim_start().starts_with("```") {
                parts.push(code_block_html(&code_lang, &code_lines.join("\n")));
                in_code = false;
                code_lang.clear();
                code_lines.clear();
            } else {
                code_lines.push(line.to_owned());
            }
            continue;
        }

        if line.trim_start().starts_with("```") {
            flush_paragraph(&mut parts, &mut paragraph);
            flush_list(&mut parts, &mut list_items, &mut list_ordered);
            in_code = true;
            code_lang = line
                .trim_start()
                .trim_start_matches("```")
                .trim()
                .to_owned();
            continue;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            flush_paragraph(&mut parts, &mut paragraph);
            flush_list(&mut parts, &mut list_items, &mut list_ordered);
            continue;
        }
        if trimmed == "---" || trimmed == "***" {
            flush_paragraph(&mut parts, &mut paragraph);
            flush_list(&mut parts, &mut list_items, &mut list_ordered);
            parts.push("<hr>".to_owned());
            continue;
        }
        if let Some((level, text)) = heading(trimmed) {
            flush_paragraph(&mut parts, &mut paragraph);
            flush_list(&mut parts, &mut list_items, &mut list_ordered);
            let level = (level + 2).min(6);
            parts.push(format!(
                "<h{level}>{}</h{level}>",
                inline_markdown_to_html(text.trim())
            ));
            continue;
        }
        if let Some(text) = trimmed.strip_prefix("> ") {
            flush_paragraph(&mut parts, &mut paragraph);
            flush_list(&mut parts, &mut list_items, &mut list_ordered);
            parts.push(format!(
                "<blockquote><p>{}</p></blockquote>",
                inline_markdown_to_html(text)
            ));
            continue;
        }
        if let Some(text) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            flush_paragraph(&mut parts, &mut paragraph);
            if list_ordered {
                flush_list(&mut parts, &mut list_items, &mut list_ordered);
            }
            list_items.push(inline_markdown_to_html(text));
            continue;
        }
        if let Some(text) = ordered_list_item(trimmed) {
            flush_paragraph(&mut parts, &mut paragraph);
            if !list_ordered && !list_items.is_empty() {
                flush_list(&mut parts, &mut list_items, &mut list_ordered);
            }
            list_ordered = true;
            list_items.push(inline_markdown_to_html(text));
            continue;
        }
        paragraph.push(trimmed.to_owned());
    }

    if in_code {
        parts.push(code_block_html(&code_lang, &code_lines.join("\n")));
    }
    flush_paragraph(&mut parts, &mut paragraph);
    flush_list(&mut parts, &mut list_items, &mut list_ordered);
    parts.join("\n")
}

fn code_block_html(language: &str, text: &str) -> String {
    if language.is_empty() {
        format!("<pre><code>{}</code></pre>", escape_html(text))
    } else {
        format!(
            "<pre data-language=\"{}\"><code>{}</code></pre>",
            escape_html(language),
            escape_html(text)
        )
    }
}

fn ordered_list_item(line: &str) -> Option<&str> {
    let dot = line.find('.')?;
    if dot == 0 || !line[..dot].bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    line[dot + 1..].strip_prefix(' ')
}

fn heading(line: &str) -> Option<(usize, &str)> {
    let count = line.chars().take_while(|ch| *ch == '#').count();
    if count == 0 || count > 3 {
        return None;
    }
    let rest = &line[count..];
    rest.starts_with(' ').then_some((count, rest))
}

fn flush_paragraph(parts: &mut Vec<String>, paragraph: &mut Vec<String>) {
    if paragraph.is_empty() {
        return;
    }
    parts.push(format!(
        "<p>{}</p>",
        inline_markdown_to_html(&paragraph.join(" "))
    ));
    paragraph.clear();
}

fn flush_list(parts: &mut Vec<String>, list_items: &mut Vec<String>, ordered: &mut bool) {
    if list_items.is_empty() {
        *ordered = false;
        return;
    }
    let items = list_items
        .iter()
        .map(|item| format!("<li>{item}</li>"))
        .collect::<Vec<_>>()
        .join("");
    let tag = if *ordered { "ol" } else { "ul" };
    parts.push(format!("<{tag}>{items}</{tag}>"));
    list_items.clear();
    *ordered = false;
}

#[must_use]
pub fn inline_markdown_to_html(source: &str) -> String {
    // Escape first, then apply a small safe subset. This intentionally avoids raw HTML.
    let escaped = escape_html(source);
    let escaped = replace_semantic_tokens(&escaped);
    let escaped = replace_inline_code(&escaped);
    let escaped = replace_double_delimited(&escaped, "**", "strong");
    let escaped = replace_double_delimited(&escaped, "__", "strong");
    let escaped = replace_double_delimited(&escaped, "*", "em");
    replace_links(&escaped)
}

fn replace_semantic_tokens(source: &str) -> String {
    let mut output = String::new();
    let mut rest = source;
    while let Some(start) = rest.find('{') {
        output.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(colon) = after.find(':') else {
            output.push('{');
            rest = after;
            continue;
        };
        let tone = &after[..colon];
        let Some(end) = after[colon + 1..].find('}') else {
            output.push('{');
            rest = after;
            continue;
        };
        let content = after[colon + 1..colon + 1 + end].trim();
        if matches!(
            tone,
            "critical" | "error" | "warning" | "success" | "info" | "muted"
        ) {
            output.push_str(&format!(
                "<span class=\"semantic semantic-{tone}\">{content}</span>"
            ));
            rest = &after[colon + 1 + end + 1..];
        } else {
            output.push('{');
            rest = after;
        }
    }
    output.push_str(rest);
    output
}

fn replace_inline_code(source: &str) -> String {
    replace_double_delimited(source, "`", "code")
}

fn replace_double_delimited(source: &str, delimiter: &str, tag: &str) -> String {
    let mut output = String::new();
    let mut rest = source;
    loop {
        let Some(start) = rest.find(delimiter) else {
            output.push_str(rest);
            break;
        };
        let after_start = &rest[start + delimiter.len()..];
        let Some(end) = after_start.find(delimiter) else {
            output.push_str(rest);
            break;
        };
        output.push_str(&rest[..start]);
        output.push_str(&format!("<{tag}>{}</{tag}>", &after_start[..end]));
        rest = &after_start[end + delimiter.len()..];
    }
    output
}

fn replace_links(source: &str) -> String {
    let mut output = String::new();
    let mut rest = source;
    while let Some(start) = rest.find('[') {
        output.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(label_end) = after.find("](") else {
            output.push('[');
            rest = after;
            continue;
        };
        let label = &after[..label_end];
        let href_start = label_end + 2;
        let Some(href_end) = after[href_start..].find(')') else {
            output.push('[');
            rest = after;
            continue;
        };
        let href = &after[href_start..href_start + href_end];
        if is_safe_link(href) {
            let external = href
                .get(..7)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
                || href
                    .get(..8)
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"));
            let attrs = if external {
                " target=\"_blank\" rel=\"noopener noreferrer\""
            } else {
                ""
            };
            // The full inline source was escaped before link parsing, so the
            // href is already attribute-safe and must not be escaped twice.
            output.push_str(&format!("<a href=\"{href}\"{attrs}>{label}</a>"));
            rest = &after[href_start + href_end + 1..];
        } else {
            output.push_str(label);
            rest = &after[href_start + href_end + 1..];
        }
    }
    output.push_str(rest);
    output
}

fn is_safe_link(href: &str) -> bool {
    if href.is_empty()
        || href.chars().any(char::is_whitespace)
        || href.chars().any(char::is_control)
    {
        return false;
    }
    let lower = href.to_ascii_lowercase();
    lower.starts_with("https://")
        || lower.starts_with("http://")
        || lower.starts_with("mailto:")
        || href.starts_with('#')
        || (href.starts_with('/') && !href.starts_with("//"))
}
