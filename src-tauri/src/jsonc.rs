//! Pragmatic JSONC helpers: mod.json / lib.json / chapterN.json are JSON
//! with comments and trailing commas.

/// Strip // comments, /* */ blocks and trailing commas, keeping strings
/// intact — enough to feed serde_json.
pub fn strip(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    let mut in_str = false;
    let mut esc = false;
    while i < bytes.len() {
        let c = bytes[i];
        if in_str {
            out.push(c as char);
            if esc {
                esc = false;
            } else if c == b'\\' {
                esc = true;
            } else if c == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match c {
            b'"' => {
                in_str = true;
                out.push('"');
                i += 1;
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                out.push('\n');
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                i += 2;
                while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i += 1; // skip the closing '/'
                i += 1;
            }
            b'}' | b']' => {
                // drop a trailing comma (and whitespace) before the close
                while out.ends_with(' ') || out.ends_with('\t') || out.ends_with('\n') || out.ends_with('\r') {
                    out.pop();
                }
                if out.ends_with(',') {
                    out.pop();
                }
                out.push(c as char);
                i += 1;
            }
            _ => {
                out.push(c as char);
                i += 1;
            }
        }
    }
    out
}

/// Locate the value object after `"key" :` — returns the byte index of
/// the opening '{', or None. A plain byte scan (keys appear before their
/// value; the first `"key"` followed by `:` and `{` wins).
pub fn find_object_start(text: &str, key: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let key_bytes = format!("\"{}\"", key);
    let mut pos = 0;
    while pos + key_bytes.len() <= bytes.len() {
        if &bytes[pos..pos + key_bytes.len()] == key_bytes.as_bytes() {
            let mut j = pos + key_bytes.len();
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b':' {
                j += 1;
                while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == b'{' {
                    return Some(j);
                }
            }
        }
        pos += 1;
    }
    None
}

/// Return the range [start, end) of the object whose '{' is at `start`,
/// handling strings and nesting.
pub fn object_end(text: &str, start: usize) -> usize {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut in_str = false;
    let mut esc = false;
    let mut i = start;
    while i < bytes.len() {
        let c = bytes[i];
        if in_str {
            if esc {
                esc = false;
            } else if c == b'\\' {
                esc = true;
            } else if c == b'"' {
                in_str = false;
            }
        } else {
            match c {
                b'"' => in_str = true,
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return i + 1;
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    text.len()
}
