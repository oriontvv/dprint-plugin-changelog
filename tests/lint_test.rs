use std::path::Path;

use dprint_plugin_changelog::configuration::Configuration;
use dprint_plugin_changelog::format_text;
use dprint_plugin_changelog::Diagnostic;

fn lint_with(text: &str, config: &Configuration) -> Vec<Diagnostic> {
    match format_text(Path::new("CHANGELOG.md"), text, config) {
        Ok(_) => Vec::new(),
        Err(err) => err.diagnostics,
    }
}

fn lint(text: &str) -> Vec<Diagnostic> {
    lint_with(text, &Configuration::default())
}

fn messages(diags: &[Diagnostic]) -> Vec<String> {
    diags
        .iter()
        .map(|d| format!("{}: {}", d.line, d.message))
        .collect()
}

#[test]
fn valid_changelog_has_no_diagnostics() {
    let text =
        "# Changelog\n\n## [Unreleased]\n\n## [1.0.0] - 2024-01-01\n\n### Added\n\n- Thing.\n";
    assert_eq!(lint(text), Vec::new());
}

#[test]
fn reports_invalid_date() {
    let text = "## [1.0.0] - 2024-13-01\n\n### Added\n\n- Thing.\n";
    assert_eq!(
        messages(&lint(text)),
        vec!["1: '2024-13-01' is not a valid calendar date"]
    );
}

#[test]
fn reports_non_iso_date() {
    let text = "## [1.0.0] - 01.02.2024\n\n### Added\n\n- Thing.\n";
    assert_eq!(
        messages(&lint(text)),
        vec!["1: '01.02.2024' is not an ISO 8601 date (expected YYYY-MM-DD)"]
    );
}

#[test]
fn rejects_feb_29_in_non_leap_year() {
    let text = "## [1.0.0] - 2023-02-29\n\n### Added\n\n- Thing.\n";
    assert_eq!(lint(text).len(), 1);
    let text = "## [1.0.0] - 2024-02-29\n\n### Added\n\n- Thing.\n";
    assert_eq!(lint(text).len(), 0);
}

#[test]
fn reports_missing_date() {
    let text = "## [1.0.0]\n\n### Added\n\n- Thing.\n";
    assert_eq!(
        messages(&lint(text)),
        vec!["1: release '1.0.0' has no date (expected '## [1.0.0] - YYYY-MM-DD')"]
    );
}

#[test]
fn reports_duplicate_versions() {
    let text = "## [1.0.0] - 2024-01-02\n\n- a\n\n## [1.0.0] - 2024-01-01\n\n- b\n";
    assert_eq!(
        messages(&lint(text)),
        vec!["5: duplicate release '1.0.0' (first defined on line 1)"]
    );
}

#[test]
fn reports_wrong_version_order() {
    let text = "## [1.0.0] - 2024-01-01\n\n### Added\n\n- a\n\n## [1.1.0] - 2024-02-01\n\n### Added\n\n- b\n";
    assert_eq!(
        messages(&lint(text)),
        vec!["7: releases must be listed newest first, but 1.1.0 comes after 1.0.0"]
    );
}

#[test]
fn compares_versions_numerically() {
    let text = "## [1.10.0] - 2024-02-01\n\n### Added\n\n- a\n\n## [1.9.0] - 2024-01-01\n\n### Added\n\n- b\n";
    assert_eq!(lint(text), Vec::new());
}

#[test]
fn reports_unreleased_not_first() {
    let text = "## [1.0.0] - 2024-01-01\n\n### Added\n\n- a\n\n## [Unreleased]\n\n";
    assert_eq!(
        messages(&lint(text)),
        vec!["7: 'Unreleased' must be the first release"]
    );
}

#[test]
fn reports_invalid_version() {
    let text = "## [1.0] - 2024-01-01\n\n### Added\n\n- a\n";
    let diags = lint(text);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 1);
    assert!(diags[0].message.contains("not a valid semantic version"));
}

#[test]
fn reports_unknown_section_with_suggestion() {
    let text = "## [1.0.0] - 2024-01-01\n\n### Bugfixes\n\n- a\n";
    let diags = lint(text);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 3);
    assert!(diags[0].message.contains("unknown section 'Bugfixes'"));
    assert!(diags[0].message.contains("did you mean 'Fixed'?"));
}

#[test]
fn allows_unknown_sections_when_not_strict() {
    let config = Configuration {
        strict_section_names: false,
        ..Configuration::default()
    };
    let text = "## [1.0.0] - 2024-01-01\n\n### Notes\n\n- a\n";
    assert_eq!(lint_with(text, &config), Vec::new());
}

#[test]
fn reports_duplicate_sections() {
    let text = "## [1.0.0] - 2024-01-01\n\n### Added\n\n- a\n\n### Added\n\n- b\n";
    assert_eq!(
        messages(&lint(text)),
        vec!["7: duplicate section 'Added' in release '1.0.0'"]
    );
}

#[test]
fn reports_release_heading_at_wrong_level() {
    let text = "# Changelog\n\n### [1.0.0] - 2024-01-01\n\n- a\n";
    let diags = lint(text);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 3);
    assert!(diags[0].message.contains("must be a level 2 heading"));
}

#[test]
fn reports_missing_unreleased_only_when_required() {
    let text = "## [1.0.0] - 2024-01-01\n\n### Added\n\n- a\n";
    assert_eq!(lint(text), Vec::new());
    let config = Configuration {
        require_unreleased: true,
        ..Configuration::default()
    };
    assert_eq!(
        messages(&lint_with(text, &config)),
        vec!["1: missing '## [Unreleased]' section"]
    );
}

#[test]
fn reports_missing_reference_links_when_required() {
    let config = Configuration {
        require_reference_links: true,
        ..Configuration::default()
    };
    let text = "## [Unreleased]\n\n## [1.0.0] - 2024-01-01\n\n### Added\n\n- a\n\n[unreleased]: https://example.com/compare/v1.0.0...HEAD\n";
    assert_eq!(
        messages(&lint_with(text, &config)),
        vec!["3: missing link reference definition '[1.0.0]: <url>'"]
    );
}

#[test]
fn reference_labels_are_case_insensitive() {
    let config = Configuration {
        require_reference_links: true,
        ..Configuration::default()
    };
    let text = "## [Unreleased]\n\n[unreleased]: https://example.com/compare/v1.0.0...HEAD\n";
    assert_eq!(lint_with(text, &config), Vec::new());
}

#[test]
fn reports_unbracketed_release_when_links_required() {
    let config = Configuration {
        require_reference_links: true,
        ..Configuration::default()
    };
    let text = "## 1.0.0 - 2024-01-01\n\n### Added\n\n- a\n";
    assert_eq!(
        messages(&lint_with(text, &config)),
        vec!["1: release '1.0.0' must be written as a link: '## [1.0.0]'"]
    );
}

#[test]
fn ignores_headings_inside_code_fences() {
    let text =
        "## [1.0.0] - 2024-01-01\n\n### Added\n\n```\n## [0.0.0] - garbage\n### Bugfixes\n```\n";
    assert_eq!(lint(text), Vec::new());
}

#[test]
fn reports_unclosed_code_fence() {
    let text = "## [1.0.0] - 2024-01-01\n\n```\ncode\n";
    assert_eq!(messages(&lint(text)), vec!["3: code fence is never closed"]);
}

#[test]
fn reports_diagnostics_sorted_by_line() {
    let text = "## [1.0.0] - 2024-13-01\n\n### Bugfixes\n\n- a\n\n## [1.0.0] - 2024-01-01\n";
    let lines: Vec<usize> = lint(text).iter().map(|d| d.line).collect();
    let mut sorted = lines.clone();
    sorted.sort();
    assert_eq!(lines, sorted);
    assert_eq!(lines.len(), 3);
}

#[test]
fn formats_crlf_with_auto_newlines() {
    let text = "## [1.0.0] - 2024-01-01\r\n### Added\r\n- a\r\n";
    let result = format_text(Path::new("CHANGELOG.md"), text, &Configuration::default()).unwrap();
    assert_eq!(
        result.unwrap(),
        "## [1.0.0] - 2024-01-01\r\n\r\n### Added\r\n\r\n- a\r\n"
    );
}

#[test]
fn converts_newlines_when_configured() {
    use dprint_core::configuration::NewLineKind;
    let config = Configuration {
        new_line_kind: NewLineKind::LineFeed,
        ..Configuration::default()
    };
    let text = "## [1.0.0] - 2024-01-01\r\n\r\n### Added\r\n\r\n- a\r\n";
    let result = format_text(Path::new("CHANGELOG.md"), text, &config).unwrap();
    assert_eq!(
        result.unwrap(),
        "## [1.0.0] - 2024-01-01\n\n### Added\n\n- a\n"
    );
}

#[test]
fn returns_none_when_already_formatted() {
    let text = "## [1.0.0] - 2024-01-01\n\n### Added\n\n- a\n";
    assert_eq!(
        format_text(Path::new("CHANGELOG.md"), text, &Configuration::default()).unwrap(),
        None
    );
}

#[test]
fn keeps_bom() {
    let text = "\u{feff}## [1.0.0] - 2024-01-01\n### Added\n- a\n";
    let result = format_text(Path::new("CHANGELOG.md"), text, &Configuration::default())
        .unwrap()
        .unwrap();
    assert!(result.starts_with('\u{feff}'));
}

#[test]
fn formats_empty_file_without_changes() {
    assert_eq!(
        format_text(Path::new("CHANGELOG.md"), "", &Configuration::default()).unwrap(),
        None
    );
}

#[test]
fn keeps_non_version_second_level_headings() {
    let text = "# Changelog\n\n## About\n\nText.\n\n## [1.0.0] - 2024-01-01\n\n### Added\n\n- a\n";
    assert_eq!(
        format_text(Path::new("CHANGELOG.md"), text, &Configuration::default()).unwrap(),
        None
    );
}
