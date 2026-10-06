# dprint-plugin-changelog

[![Actions Status](https://github.com/oriontvv/dprint-plugin-changelog/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/oriontvv/dprint-plugin-changelog/actions/workflows/ci.yml) [![Coverage badge](https://raw.githubusercontent.com/oriontvv/dprint-plugin-changelog/coverage/htmlcov/badges/flat.svg)](https://htmlpreview.github.io/?https://github.com/oriontvv/dprint-plugin-changelog/coverage/htmlcov/index.html) [![dependency status](https://deps.rs/repo/github/oriontvv/dprint-plugin-changelog/status.svg)](https://deps.rs/repo/github/oriontvv/dprint-plugin-changelog) [![Crates.io](https://img.shields.io/crates/v/dprint-plugin-changelog.svg)](https://crates.io/crates/dprint-plugin-changelog)

[dprint](https://dprint.dev) plugin that lints and formats `CHANGELOG.md` files written in the
[Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/) style.

dprint has no channel for lint diagnostics, so the plugin does two things:

- **Formats** whatever can be fixed mechanically.
- **Fails** the file with `line N: message` errors for violations that cannot be fixed safely.

## Usage

From a release (replace `0.1.0` with the version you want):

```sh
dprint add oriontvv/dprint-plugin-changelog
```

```jsonc
{
  "plugins": ["https://plugins.dprint.dev/oriontvv/dprint-plugin-changelog-0.1.0.wasm"]
}
```

Or build the Wasm plugin yourself and reference the file in `dprint.json`:

```sh
cargo build --release --target wasm32-unknown-unknown --features wasm
```

```jsonc
{
  "plugins": ["./target/wasm32-unknown-unknown/release/dprint_plugin_changelog.wasm"],
  "changelog": {
    "sortSections": true
  }
}
```

The plugin claims files named `CHANGELOG.md` (also `Changelog.md`, `changelog.md`). To keep the Markdown
plugin running on them as well, set `"additive": true`. Use `associations` to match other files.

## What gets fixed

- Blank lines around headings, repeated blank lines collapsed, trailing whitespace removed, final newline.
- `*` and `+` list markers become `-`.
- Release headings are normalized to `## [1.0.0] - 2024-01-01` (and `[YANKED]` upper-cased).
- Section names are normalized to `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`.
- Empty `###` sections are removed (`removeEmptySections`). An empty `## [Unreleased]` is kept.
- Sections are sorted in the order above (`sortSections`, off by default).
- Line endings follow `newLineKind`.

Fenced code blocks are never modified. The text of the introduction, notes and list items is left as written.

## What fails

| Problem | Example message |
| --- | --- |
| Invalid semantic version | `'1.0' is not a valid semantic version` |
| Missing / non-ISO / impossible date | `'2024-13-01' is not a valid calendar date` |
| Duplicate release | `duplicate release '1.0.0' (first defined on line 1)` |
| Releases not newest first (semver precedence) | `releases must be listed newest first, ...` |
| `Unreleased` not first | `'Unreleased' must be the first release` |
| Unknown section name (`strictSectionNames`) | `unknown section 'Bugfixes' ... (did you mean 'Fixed'?)` |
| Duplicate section in a release | `duplicate section 'Added' in release '1.0.0'` |
| Release heading not at level 2 | `release heading ... must be a level 2 heading (##)` |
| Missing `[Unreleased]` (`requireUnreleased`) | `missing '## [Unreleased]' section` |
| Missing `[x]: url` definition (`requireReferenceLinks`) | `missing link reference definition '[1.0.0]: <url>'` |
| Unclosed code fence | `code fence is never closed` |

## Configuration

| Option | Default | Description |
| --- | --- | --- |
| `newLineKind` | `auto` | `auto`, `lf`, `crlf` or `system`. |
| `sortSections` | `false` | Reorder `###` sections. |
| `removeEmptySections` | `true` | Drop `###` sections without content. |
| `requireUnreleased` | `false` | Fail without an `## [Unreleased]` section. |
| `strictSectionNames` | `true` | Fail on sections outside the six standard names. |
| `requireReferenceLinks` | `false` | Fail when a release has no link reference definition. |
| `additive` | `false` | Run alongside the Markdown plugin instead of claiming the file. |

## Development

```sh
cargo test
```

Format specs live in `tests/specs/*.txt` (`!! message !!`, `[expect]`), lint rules are tested in
`tests/lint_test.rs`.

## Releasing

1. Set `version` in `Cargo.toml` and commit.
2. Push a tag equal to that version, without a `v` prefix: `git tag 0.1.0 && git push origin 0.1.0`.


## License

Apache-2.0
