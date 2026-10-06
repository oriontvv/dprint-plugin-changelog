//! Line-level helpers: splitting text into lines while tracking fenced code blocks,
//! ATX heading detection and link reference definitions.

#[derive(Clone, Debug)]
pub struct Line {
    pub text: String,
    /// 1-based line number in the original file.
    pub no: usize,
    /// True for every line that belongs to a fenced code block, including the fence delimiters.
    pub in_fence: bool,
}

impl Line {
    pub fn synthetic(text: String, no: usize) -> Line {
        Line {
            text,
            no,
            in_fence: false,
        }
    }

    pub fn is_blank(&self) -> bool {
        !self.in_fence && self.text.trim().is_empty()
    }
}

/// Splits text into lines (normalizing `\r\n`), flagging fenced code blocks.
/// The second value is the line number of a code fence that is never closed.
pub fn split(text: &str) -> (Vec<Line>, Option<usize>) {
    let text = text.replace("\r\n", "\n");
    if text.is_empty() {
        return (Vec::new(), None);
    }
    let body = text.strip_suffix('\n').unwrap_or(&text);

    let mut lines = Vec::new();
    let mut fence: Option<(char, usize, usize)> = None; // (char, length, opening line number)

    for (i, raw) in body.split('\n').enumerate() {
        let no = i + 1;
        let trimmed = raw.trim_start();
        let in_fence = match fence {
            None => {
                if let Some((ch, len)) = fence_open(trimmed) {
                    fence = Some((ch, len, no));
                    true
                } else {
                    false
                }
            }
            Some((ch, len, _)) => {
                if fence_close(trimmed, ch, len) {
                    fence = None;
                }
                true
            }
        };
        lines.push(Line {
            text: raw.to_string(),
            no,
            in_fence,
        });
    }

    (lines, fence.map(|(_, _, no)| no))
}

fn fence_open(trimmed: &str) -> Option<(char, usize)> {
    let ch = trimmed.chars().next()?;
    if ch != '`' && ch != '~' {
        return None;
    }
    let len = trimmed.chars().take_while(|&c| c == ch).count();
    if len < 3 {
        return None;
    }
    // the info string of a backtick fence may not contain backticks
    if ch == '`' && trimmed[len..].contains('`') {
        return None;
    }
    Some((ch, len))
}

fn fence_close(trimmed: &str, ch: char, open_len: usize) -> bool {
    let len = trimmed.chars().take_while(|&c| c == ch).count();
    len >= open_len && trimmed[len..].trim().is_empty()
}

/// Returns the level and text of an ATX heading line.
pub fn heading(line: &Line) -> Option<(usize, String)> {
    if line.in_fence {
        return None;
    }
    let t = line.text.trim_end();
    let indent = t.len() - t.trim_start_matches(' ').len();
    if indent > 3 {
        return None;
    }
    let s = &t[indent..];
    let level = s.chars().take_while(|&c| c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }
    let rest = &s[level..];
    if !rest.is_empty() && !rest.starts_with(' ') && !rest.starts_with('\t') {
        return None;
    }
    let mut text = rest.trim();
    // optional closing sequence of hashes
    let stripped = text.trim_end_matches('#');
    if stripped.len() != text.len()
        && (stripped.is_empty() || stripped.ends_with(' ') || stripped.ends_with('\t'))
    {
        text = stripped.trim_end();
    }
    Some((level, text.to_string()))
}

/// Returns the label of a link reference definition line (`[label]: url`).
pub fn ref_def_label(line: &Line) -> Option<String> {
    if line.in_fence {
        return None;
    }
    let t = line.text.trim_end();
    let indent = t.len() - t.trim_start_matches(' ').len();
    if indent > 3 {
        return None;
    }
    let s = &t[indent..];
    if !s.starts_with('[') {
        return None;
    }
    let idx = s.find("]:")?;
    let label = &s[1..idx];
    if label.trim().is_empty()
        || label.contains('[')
        || label.contains(']')
        || label.starts_with('^')
    {
        return None;
    }
    if s[idx + 2..].trim().is_empty() {
        return None;
    }
    Some(label.to_string())
}

/// Normalizes a link label the way CommonMark matches them: case folded, whitespace collapsed.
pub fn normalize_label(label: &str) -> String {
    label
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
