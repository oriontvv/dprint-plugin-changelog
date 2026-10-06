//! Keep a Changelog specific knowledge: release headings, dates, versions and section names.

use std::cmp::Ordering;

pub const SECTION_NAMES: [&str; 6] = [
    "Added",
    "Changed",
    "Deprecated",
    "Removed",
    "Fixed",
    "Security",
];

/// Index of the canonical section name matching `name` (case-insensitive, optional trailing colon).
pub fn canonical_section(name: &str) -> Option<usize> {
    let name = name.trim().trim_end_matches(':').trim();
    SECTION_NAMES
        .iter()
        .position(|s| s.eq_ignore_ascii_case(name))
}

/// A hint for commonly used wrong section names.
pub fn section_suggestion(name: &str) -> Option<&'static str> {
    let lower = name.trim().trim_end_matches(':').trim().to_lowercase();
    let idx = match lower.as_str() {
        "add" | "adds" | "new" | "features" | "feature" | "additions" => 0,
        "change" | "changes" | "update" | "updates" | "updated" | "improvements" => 1,
        "deprecate" | "deprecations" | "deprecation" => 2,
        "remove" | "removals" | "removal" | "deleted" => 3,
        "fix" | "fixes" | "bugfix" | "bugfixes" | "bug fix" | "bug fixes" => 4,
        "security fixes" | "vulnerabilities" => 5,
        _ => return None,
    };
    Some(SECTION_NAMES[idx])
}

#[derive(Debug, Clone)]
pub struct ReleaseHeading {
    /// The version, or `Unreleased`.
    pub version: String,
    /// The label as it should be rendered: `[1.0.0]`, `[1.0.0](url)` or `1.0.0`.
    pub label: String,
    /// The text inside the label, without brackets.
    pub inner: String,
    /// Whether the label is written in brackets.
    pub bracketed: bool,
    /// Whether the label already carries its own link target (`[x](url)` / `[x][ref]`).
    pub linked: bool,
    /// Whitespace separated words after the label (date, `[YANKED]`).
    pub tail: Vec<String>,
}

impl ReleaseHeading {
    pub fn is_unreleased(&self) -> bool {
        self.version == "Unreleased"
    }

    pub fn render(&self) -> String {
        if self.tail.is_empty() {
            format!("## {}", self.label)
        } else {
            format!("## {} - {}", self.label, self.tail.join(" "))
        }
    }
}

pub enum HeadingParse {
    NotRelease,
    Invalid(String),
    Release(ReleaseHeading),
}

/// Interprets the text of a level 2 heading. Version recognition is delegated to `parse-changelog`
/// so that the plugin agrees with it on what a release heading is.
pub fn parse_heading(title: &str) -> HeadingParse {
    let title = title.trim();
    let (inner, extra, bracketed, linked, rest) = split_label(title);
    let inner = inner.trim();

    let (version, canon_inner) = if inner.eq_ignore_ascii_case("unreleased") {
        ("Unreleased".to_string(), "Unreleased".to_string())
    } else {
        let unprefixed = inner.strip_prefix('v').unwrap_or(inner);
        if !unprefixed.starts_with(|c: char| c.is_ascii_digit()) {
            return HeadingParse::NotRelease;
        }
        match parse_changelog::parse(&format!("## {inner}\n")) {
      Ok(changelog) => match changelog.into_values().next() {
        Some(release) => (release.version.to_string(), inner.to_string()),
        None => return HeadingParse::NotRelease,
      },
      Err(_) => {
        return HeadingParse::Invalid(format!(
          "'{inner}' is not a valid semantic version (expected MAJOR.MINOR.PATCH, see https://semver.org)"
        ))
      }
    }
    };

    let label = if bracketed {
        format!("[{canon_inner}]{extra}")
    } else {
        canon_inner.clone()
    };

    let rest = rest.trim().trim_start_matches(['-', '–', '—']).trim();
    let tail = rest
        .split_whitespace()
        .map(|word| {
            if word.eq_ignore_ascii_case("[yanked]") {
                "[YANKED]".to_string()
            } else {
                word.to_string()
            }
        })
        .collect();

    HeadingParse::Release(ReleaseHeading {
        version,
        label,
        inner: canon_inner,
        bracketed,
        linked,
        tail,
    })
}

/// Returns (inner label text, extra text after `]`, bracketed, linked, remaining text).
fn split_label(title: &str) -> (&str, &str, bool, bool, &str) {
    if title.starts_with('[') {
        if let Some(close) = title.find(']') {
            let inner = &title[1..close];
            let after = &title[close + 1..];
            let mut end = close + 1;
            let mut linked = false;
            if after.starts_with('(') {
                if let Some(p) = after.find(')') {
                    end += p + 1;
                    linked = true;
                }
            } else if after.starts_with('[') {
                if let Some(p) = after.find(']') {
                    end += p + 1;
                    linked = true;
                }
            }
            return (inner, &title[close + 1..end], true, linked, &title[end..]);
        }
    }
    let end = title.find(char::is_whitespace).unwrap_or(title.len());
    (&title[..end], "", false, false, &title[end..])
}

/// Validates an ISO 8601 calendar date (`YYYY-MM-DD`).
pub fn check_date(word: &str) -> Result<(), String> {
    let b = word.as_bytes();
    let shaped = b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
    if !shaped {
        return Err(format!(
            "'{word}' is not an ISO 8601 date (expected YYYY-MM-DD)"
        ));
    }
    let year: u32 = word[0..4].parse().unwrap();
    let month: u32 = word[5..7].parse().unwrap();
    let day: u32 = word[8..10].parse().unwrap();
    let leap = (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        _ => 0,
    };
    if day == 0 || day > days_in_month {
        return Err(format!("'{word}' is not a valid calendar date"));
    }
    Ok(())
}

/// Compares two semantic versions by precedence (build metadata is ignored).
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let (core_a, pre_a) = split_version(a);
    let (core_b, pre_b) = split_version(b);
    let ord = core_a.cmp(&core_b);
    if ord != Ordering::Equal {
        return ord;
    }
    match (pre_a, pre_b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(pa), Some(pb)) => compare_prerelease(pa, pb),
    }
}

fn split_version(v: &str) -> ([u128; 3], Option<&str>) {
    let v = v.strip_prefix('v').unwrap_or(v);
    let v = v.split('+').next().unwrap_or(v);
    let (core, pre) = match v.split_once('-') {
        Some((core, pre)) => (core, Some(pre)),
        None => (v, None),
    };
    let mut nums = [0u128; 3];
    for (i, part) in core.split('.').take(3).enumerate() {
        nums[i] = part.parse().unwrap_or(u128::MAX);
    }
    (nums, pre)
}

fn compare_prerelease(a: &str, b: &str) -> Ordering {
    let mut ia = a.split('.');
    let mut ib = b.split('.');
    loop {
        match (ia.next(), ib.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => {
                let ord = match (x.parse::<u128>(), y.parse::<u128>()) {
                    (Ok(nx), Ok(ny)) => nx.cmp(&ny),
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => x.cmp(y),
                };
                if ord != Ordering::Equal {
                    return ord;
                }
            }
        }
    }
}
