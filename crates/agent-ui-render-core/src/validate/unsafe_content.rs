use serde_json::Value;

#[must_use]
pub fn collect_unsafe_string_paths(value: &Value, max_paths: usize) -> Vec<String> {
    let mut paths = Vec::new();
    collect(value, "$".to_owned(), max_paths, &mut paths);
    paths
}

fn collect(value: &Value, path: String, max_paths: usize, paths: &mut Vec<String>) {
    if paths.len() >= max_paths {
        return;
    }
    match value {
        Value::String(text) => {
            if is_unsafe_text(text) {
                paths.push(path);
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                collect(item, format!("{path}[{index}]"), max_paths, paths);
                if paths.len() >= max_paths {
                    break;
                }
            }
        }
        Value::Object(map) => {
            for (key, item) in map {
                collect(item, format!("{path}.{key}"), max_paths, paths);
                if paths.len() >= max_paths {
                    break;
                }
            }
        }
        _ => {}
    }
}

#[must_use]
pub fn is_unsafe_text(value: &str) -> bool {
    has_html_tag(value)
        || has_event_handler_assignment(value)
        || value.to_ascii_lowercase().contains("javascript:")
        || contains_case_insensitive(value, "dangerouslySetInnerHTML")
        || has_forbidden_ui_assignment(value)
        || has_camel_event_assignment(value)
}

fn has_html_tag(value: &str) -> bool {
    let bytes = value.as_bytes();
    for index in 0..bytes.len() {
        if bytes[index] != b'<' {
            continue;
        }
        let mut cursor = index + 1;
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor < bytes.len() && bytes[cursor] == b'/' {
            cursor += 1;
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
        }
        if cursor < bytes.len()
            && bytes[cursor].is_ascii_alphabetic()
            && value[index..].contains('>')
        {
            return true;
        }
    }
    false
}

fn has_event_handler_assignment(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut index = 0;
    while index + 2 < bytes.len() {
        let boundary = index == 0 || !bytes[index - 1].is_ascii_alphanumeric();
        if boundary
            && bytes[index] == b'o'
            && bytes[index + 1] == b'n'
            && bytes[index + 2].is_ascii_alphabetic()
        {
            let mut cursor = index + 3;
            while cursor < bytes.len() && bytes[cursor].is_ascii_alphabetic() {
                cursor += 1;
            }
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor < bytes.len() && bytes[cursor] == b'=' {
                return true;
            }
        }
        index += 1;
    }
    false
}

fn has_forbidden_ui_assignment(value: &str) -> bool {
    for token in [
        "className",
        "style",
        "action",
        "actionHandler",
        "actionName",
        "component",
        "componentName",
    ] {
        for sep in [':', '='] {
            if has_assignment_token(value, token, sep) {
                return true;
            }
        }
    }
    false
}

fn has_camel_event_assignment(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index + 3 < bytes.len() {
        let boundary = index == 0 || !bytes[index - 1].is_ascii_alphanumeric();
        if boundary
            && bytes[index] == b'o'
            && bytes[index + 1] == b'n'
            && bytes[index + 2].is_ascii_uppercase()
        {
            let mut cursor = index + 3;
            while cursor < bytes.len()
                && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
            {
                cursor += 1;
            }
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor < bytes.len() && (bytes[cursor] == b':' || bytes[cursor] == b'=') {
                return true;
            }
        }
        index += 1;
    }
    false
}

fn has_assignment_token(value: &str, token: &str, sep: char) -> bool {
    let mut rest = value;
    while let Some(pos) = rest.find(token) {
        let after = &rest[pos + token.len()..];
        let after = after.trim_start();
        if after.starts_with(sep) {
            return true;
        }
        rest = &after[after.len().min(1)..];
    }
    false
}

fn contains_case_insensitive(value: &str, needle: &str) -> bool {
    value
        .to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}
