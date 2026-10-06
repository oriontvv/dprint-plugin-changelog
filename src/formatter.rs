//! The changelog formatter and linter.
//!
//! dprint has no channel for lint diagnostics, so a violation that cannot be fixed safely
//! (a bad date, a duplicate version, ...) is reported by failing the format with a list of
//! `line N: message` entries. Everything that can be fixed mechanically is fixed.

use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;

use dprint_core::configuration::resolve_new_line_kind;

use crate::configuration::Configuration;
use crate::lines as md;
use crate::lines::Line;
use crate::release;
use crate::release::HeadingParse;
use crate::release::ReleaseHeading;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// 1-based line number in the original file.
    pub line: usize,
    pub message: String,
}

/// The changelog violates the Keep a Changelog structure in a way that can't be fixed automatically.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatTextError {
    pub diagnostics: Vec<Diagnostic>,
}

impl std::fmt::Display for FormatTextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, d) in self.diagnostics.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "line {}: {}", d.line, d.message)?;
        }
        Ok(())
    }
}

impl std::error::Error for FormatTextError {}

struct Release {
    no: usize,
    heading: ReleaseHeading,
    body: Vec<Line>,
}

struct Section {
    no: usize,
    name: String,
    canonical: Option<usize>,
    lines: Vec<Line>,
}

struct Doc {
    preamble: Vec<Line>,
    releases: Vec<Release>,
    /// Link reference definitions collected from the end of the file.
    links: Vec<String>,
    /// Normalized labels of every link reference definition in the file.
    defs: HashSet<String>,
}

/// Lints and formats a changelog. Returns `Ok(None)` when the text is already formatted.
pub fn format_text(
    _path: &Path,
    text: &str,
    config: &Configuration,
) -> Result<Option<String>, FormatTextError> {
    let (body, bom) = match text.strip_prefix('\u{feff}') {
        Some(rest) => (rest, true),
        None => (text, false),
    };

    let (lines, unclosed_fence) = md::split(body);
    let mut diags = Vec::new();
    if let Some(no) = unclosed_fence {
        diags.push(Diagnostic {
            line: no,
            message: "code fence is never closed".to_string(),
        });
    }

    let doc = parse(lines, &mut diags);
    lint_releases(&doc, config, &mut diags);

    let mut flat = doc.preamble;
    for release in doc.releases {
        flat.push(Line::synthetic(release.heading.render(), release.no));
        flat.extend(process_release(&release, config, &mut diags));
    }

    if !diags.is_empty() {
        diags.sort_by_key(|d| d.line);
        return Err(FormatTextError { diagnostics: diags });
    }

    let mut out = normalize(&flat);
    if !doc.links.is_empty() {
        if !out.is_empty() {
            out.push(String::new());
        }
        out.extend(doc.links);
    }

    let newline = resolve_new_line_kind(body, config.new_line_kind);
    let mut result = String::new();
    if bom {
        result.push('\u{feff}');
    }
    for line in &out {
        result.push_str(line);
        result.push_str(newline);
    }

    Ok(if result == text { None } else { Some(result) })
}

fn parse(lines: Vec<Line>, diags: &mut Vec<Diagnostic>) -> Doc {
    let defs: HashSet<String> = lines
        .iter()
        .filter_map(md::ref_def_label)
        .map(|l| md::normalize_label(&l))
        .collect();

    // the trailing run of link reference definitions (and blank lines) is the links block
    let mut cut = lines.len();
    while cut > 0 {
        let line = &lines[cut - 1];
        if line.is_blank() || md::ref_def_label(line).is_some() {
            cut -= 1;
        } else {
            break;
        }
    }
    let links: Vec<String> = lines[cut..]
        .iter()
        .filter(|l| md::ref_def_label(l).is_some())
        .map(|l| l.text.trim().to_string())
        .collect();

    let mut preamble = Vec::new();
    let mut releases: Vec<Release> = Vec::new();

    for line in lines.into_iter().take(cut) {
        if let Some((level, title)) = md::heading(&line) {
            match (level, release::parse_heading(&title)) {
                (2, HeadingParse::Release(heading)) => {
                    releases.push(Release {
                        no: line.no,
                        heading,
                        body: Vec::new(),
                    });
                    continue;
                }
                (2, HeadingParse::Invalid(message)) => diags.push(Diagnostic {
                    line: line.no,
                    message,
                }),
                (_, HeadingParse::Release(_)) => diags.push(Diagnostic {
                    line: line.no,
                    message: format!(
                        "release heading '{}' must be a level 2 heading (##)",
                        line.text.trim()
                    ),
                }),
                _ => {}
            }
        }
        match releases.last_mut() {
            Some(release) => release.body.push(line),
            None => preamble.push(line),
        }
    }

    Doc {
        preamble,
        releases,
        links,
        defs,
    }
}

fn lint_releases(doc: &Doc, config: &Configuration, diags: &mut Vec<Diagnostic>) {
    let mut seen: HashMap<&str, usize> = HashMap::new();

    if config.require_unreleased && !doc.releases.iter().any(|r| r.heading.is_unreleased()) {
        diags.push(Diagnostic {
            line: 1,
            message: "missing '## [Unreleased]' section".to_string(),
        });
    }

    for (i, release) in doc.releases.iter().enumerate() {
        let heading = &release.heading;

        match seen.get(heading.version.as_str()) {
            Some(first) => diags.push(Diagnostic {
                line: release.no,
                message: format!(
                    "duplicate release '{}' (first defined on line {})",
                    heading.version, first
                ),
            }),
            None => {
                seen.insert(heading.version.as_str(), release.no);
            }
        }

        if heading.is_unreleased() && i != 0 {
            diags.push(Diagnostic {
                line: release.no,
                message: "'Unreleased' must be the first release".to_string(),
            });
        }

        if i > 0 {
            let prev = &doc.releases[i - 1].heading;
            if !prev.is_unreleased()
                && !heading.is_unreleased()
                && release::compare_versions(&prev.version, &heading.version).is_lt()
            {
                diags.push(Diagnostic {
                    line: release.no,
                    message: format!(
                        "releases must be listed newest first, but {} comes after {}",
                        heading.version, prev.version
                    ),
                });
            }
        }

        if !heading.is_unreleased() {
            match heading.tail.first() {
                Some(word) if !word.eq_ignore_ascii_case("[yanked]") => {
                    if let Err(message) = release::check_date(word) {
                        diags.push(Diagnostic {
                            line: release.no,
                            message,
                        });
                    }
                }
                _ => diags.push(Diagnostic {
                    line: release.no,
                    message: format!(
                        "release '{}' has no date (expected '## [{}] - YYYY-MM-DD')",
                        heading.version, heading.version
                    ),
                }),
            }
        }

        if config.require_reference_links && !heading.linked {
            if !heading.bracketed {
                diags.push(Diagnostic {
                    line: release.no,
                    message: format!(
                        "release '{}' must be written as a link: '## [{}]'",
                        heading.version, heading.inner
                    ),
                });
            } else if !doc.defs.contains(&md::normalize_label(&heading.inner)) {
                diags.push(Diagnostic {
                    line: release.no,
                    message: format!(
                        "missing link reference definition '[{}]: <url>'",
                        heading.inner
                    ),
                });
            }
        }
    }
}

/// Splits the body of a release into `###` sections, lints and fixes them.
fn process_release(
    release: &Release,
    config: &Configuration,
    diags: &mut Vec<Diagnostic>,
) -> Vec<Line> {
    let mut intro: Vec<Line> = Vec::new();
    let mut sections: Vec<Section> = Vec::new();

    for line in &release.body {
        if let Some((3, name)) = md::heading(line) {
            sections.push(Section {
                no: line.no,
                canonical: release::canonical_section(&name),
                name,
                lines: Vec::new(),
            });
        } else {
            match sections.last_mut() {
                Some(section) => section.lines.push(line.clone()),
                None => intro.push(line.clone()),
            }
        }
    }

    if config.strict_section_names {
        let mut seen = HashSet::new();
        for section in &sections {
            match section.canonical {
                None => {
                    let hint = match release::section_suggestion(&section.name) {
                        Some(suggestion) => format!(" (did you mean '{}'?)", suggestion),
                        None => String::new(),
                    };
                    diags.push(Diagnostic {
                        line: section.no,
                        message: format!(
                            "unknown section '{}', expected one of: {}{}",
                            section.name,
                            release::SECTION_NAMES.join(", "),
                            hint
                        ),
                    });
                }
                Some(i) => {
                    if !seen.insert(i) {
                        diags.push(Diagnostic {
                            line: section.no,
                            message: format!(
                                "duplicate section '{}' in release '{}'",
                                release::SECTION_NAMES[i],
                                release.heading.version
                            ),
                        });
                    }
                }
            }
        }
    }

    for section in &mut sections {
        if let Some(i) = section.canonical {
            section.name = release::SECTION_NAMES[i].to_string();
        }
    }
    if config.remove_empty_sections {
        sections.retain(|s| !s.lines.iter().all(|l| l.is_blank()));
    }
    if config.sort_sections {
        sections.sort_by_key(|s| s.canonical.unwrap_or(usize::MAX));
    }

    let mut result = intro;
    for section in sections {
        result.push(Line::synthetic(format!("### {}", section.name), section.no));
        result.extend(section.lines);
    }
    result
}

/// Trims trailing whitespace, uses `-` list markers, and makes sure that headings are surrounded
/// by exactly one blank line while other runs of blank lines are collapsed to one.
fn normalize(flat: &[Line]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut pending_blank = false;

    for (i, line) in flat.iter().enumerate() {
        if line.in_fence {
            if pending_blank && !out.is_empty() {
                out.push(String::new());
            }
            pending_blank = false;
            out.push(line.text.clone());
            continue;
        }
        if line.is_blank() {
            pending_blank = true;
            continue;
        }

        let is_heading = md::heading(line).is_some();
        if is_heading {
            pending_blank = true;
        }
        if pending_blank && !out.is_empty() {
            out.push(String::new());
        }
        pending_blank = is_heading;

        out.push(normalize_line(line, flat.get(i + 1), is_heading));
    }

    out
}

fn normalize_line(line: &Line, next: Option<&Line>, is_heading: bool) -> String {
    let trimmed = line.text.trim_end();
    let trailing = &line.text[trimmed.len()..];

    let text = replace_list_marker(trimmed);

    // two trailing spaces are a markdown hard line break, keep them when they still mean something
    let hard_break = !is_heading
        && trailing.ends_with("  ")
        && next.is_some_and(|n| !n.is_blank() && md::heading(n).is_none());
    if hard_break {
        format!("{text}  ")
    } else {
        text
    }
}

fn replace_list_marker(s: &str) -> String {
    let indent = s.len() - s.trim_start_matches(' ').len();
    let rest = &s[indent..];
    let mut chars = rest.chars();
    if let (Some(marker @ ('*' | '+')), Some(' ')) = (chars.next(), chars.next()) {
        let is_thematic_break = rest
            .chars()
            .filter(|c| !c.is_whitespace())
            .all(|c| c == marker)
            && rest.chars().filter(|&c| c == marker).count() >= 3;
        if !is_thematic_break {
            return format!("{}- {}", " ".repeat(indent), &rest[2..]);
        }
    }
    s.to_string()
}
