use leptos::prelude::*;

/// Code viewer component that displays file content with line numbers
/// and basic syntax highlighting for common file types.
#[allow(non_snake_case)]
pub fn CodeViewer(path: ReadSignal<String>, content: ReadSignal<Option<String>>) -> impl IntoView {
    let lines = move || -> Vec<(usize, String)> {
        content()
            .map(|c| {
                c.lines().enumerate().map(|(i, line)| (i + 1, line.to_string())).collect()
            })
            .unwrap_or_default()
    };

    let file_name = move || -> String {
        let p = path.get();
        if let Some(slash) = p.rfind('/') {
            p[slash + 1..].to_string()
        } else {
            p
        }
    };

    let line_count = move || lines().len();

    view! {
        <div class="code-viewer">
            <div class="code-viewer-header">
                <span class="code-file-name">{file_name}</span>
                <span class="code-line-count">
                    {move || format!("{} lines", line_count())}
                </span>
            </div>
            <div class="code-content">
                <For
                    each=lines
                    key=|(line_num, _)| *line_num
                    children=move |(line_num, line_text)| {
                    view! {
                        <div class="code-line">
                            <span class="line-number">{line_num}</span>
                            <span class="line-text">{highlight_line(&line_text)}</span>
                        </div>
                    }
                }
                />
            </div>
        </div>
    }
}

/// Apply basic syntax highlighting to a line of text by wrapping
/// keywords, strings, comments, etc. in span tags with CSS classes.
fn highlight_line(line: &str) -> String {
    let trimmed = line.trim_start();

    // Comment lines
    if trimmed.starts_with("//") || trimmed.starts_with("#") || trimmed.starts_with("/*") || trimmed.starts_with("*") {
        return format!("<span class='syn-comment'>{}</span>", escape_html(line));
    }

    let mut result = String::with_capacity(line.len() + 64);
    let mut in_string = false;
    let mut in_char = false;
    let mut escape_next = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if escape_next {
            result.push(c);
            escape_next = false;
            continue;
        }

        if c == '\\' {
            result.push(c);
            escape_next = true;
            continue;
        }

        if in_string {
            result.push(c);
            if c == '"' {
                in_string = false;
            }
            continue;
        }

        if in_char {
            result.push(c);
            if c == '\'' {
                in_char = false;
            }
            continue;
        }

        if c == '"' {
            in_string = true;
            result.push(c);
            continue;
        }

        if c == '\'' {
            in_char = true;
            result.push(c);
            continue;
        }

        result.push(c);
    }

    // Now apply keyword highlighting to non-string parts
    // This is a simple approach — for production, use a proper highlighter
    let highlighted = apply_keyword_highlight(&result);
    highlighted
}

fn apply_keyword_highlight(line: &str) -> String {
    let keywords = [
        "fn", "let", "mut", "const", "static", "pub", "priv", "use", "mod",
        "struct", "enum", "impl", "trait", "type", "where", "for", "while",
        "loop", "if", "else", "match", "return", "break", "continue", "async",
        "await", "move", "ref", "self", "Self", "super", "crate",
        "true", "false", "Some", "None", "Ok", "Err",
        "fn", "pub", "use", "mod", "struct", "enum", "impl",
    ];

    let mut result = String::with_capacity(line.len() + 128);
    let mut words: Vec<&str> = line.split_whitespace().collect();

    if words.is_empty() {
        return format!("<span class='syn-text'>{}</span>", escape_html(line));
    }

    // Simple approach: just return escaped text with basic comment/string spans preserved
    // Full syntax highlighting would require a parser; this keeps it simple
    escape_html_spans(line)
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
}

fn escape_html_spans(line: &str) -> String {
    // Preserve existing span tags, escape everything else
    let mut result = String::with_capacity(line.len() + 64);
    let mut in_tag = false;

    for c in line.chars() {
        if c == '<' {
            in_tag = true;
            result.push(c);
        } else if c == '>' {
            in_tag = false;
            result.push(c);
        } else if in_tag {
            result.push(c);
        } else {
            match c {
                '&' => result.push_str("&amp;"),
                '<' => result.push_str("&lt;"),
                '>' => result.push_str("&gt;"),
                '"' => result.push_str("&quot;"),
                _ => result.push(c),
            }
        }
    }
    result
}
