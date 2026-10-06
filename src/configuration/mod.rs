use dprint_core::configuration::get_unknown_property_diagnostics;
use dprint_core::configuration::get_value;
use dprint_core::configuration::ConfigKeyMap;
use dprint_core::configuration::ConfigurationDiagnostic;
use dprint_core::configuration::GlobalConfiguration;
use dprint_core::configuration::NewLineKind;
use dprint_core::configuration::ResolveConfigurationResult;
use serde::Serialize;

/// Resolved plugin configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Configuration {
    /// Newline style of the output.
    pub new_line_kind: NewLineKind,
    /// Reorder `###` sections of each release into the order
    /// Added, Changed, Deprecated, Removed, Fixed, Security.
    pub sort_sections: bool,
    /// Remove `###` sections that have no content.
    pub remove_empty_sections: bool,
    /// Lint: require a `## [Unreleased]` section at the top.
    pub require_unreleased: bool,
    /// Lint: only allow the six section names defined by Keep a Changelog.
    pub strict_section_names: bool,
    /// Lint: require a `[version]: url` reference definition for every release
    /// heading that is not an inline link.
    pub require_reference_links: bool,
    /// Run in addition to the plugin that claims `CHANGELOG.md` (for example the markdown plugin).
    pub additive: bool,
}

impl Default for Configuration {
    fn default() -> Self {
        Configuration {
            new_line_kind: NewLineKind::Auto,
            sort_sections: false,
            remove_empty_sections: true,
            require_unreleased: false,
            strict_section_names: true,
            require_reference_links: false,
            additive: false,
        }
    }
}

/// Resolves the plugin configuration from the raw config map and the global configuration.
pub fn resolve_config(
    config: ConfigKeyMap,
    global_config: &GlobalConfiguration,
) -> ResolveConfigurationResult<Configuration> {
    let mut config = config;
    let mut diagnostics: Vec<ConfigurationDiagnostic> = Vec::new();
    let defaults = Configuration::default();

    let resolved = Configuration {
        new_line_kind: get_value(
            &mut config,
            "newLineKind",
            global_config
                .new_line_kind
                .unwrap_or(defaults.new_line_kind),
            &mut diagnostics,
        ),
        sort_sections: get_value(
            &mut config,
            "sortSections",
            defaults.sort_sections,
            &mut diagnostics,
        ),
        remove_empty_sections: get_value(
            &mut config,
            "removeEmptySections",
            defaults.remove_empty_sections,
            &mut diagnostics,
        ),
        require_unreleased: get_value(
            &mut config,
            "requireUnreleased",
            defaults.require_unreleased,
            &mut diagnostics,
        ),
        strict_section_names: get_value(
            &mut config,
            "strictSectionNames",
            defaults.strict_section_names,
            &mut diagnostics,
        ),
        require_reference_links: get_value(
            &mut config,
            "requireReferenceLinks",
            defaults.require_reference_links,
            &mut diagnostics,
        ),
        additive: get_value(&mut config, "additive", defaults.additive, &mut diagnostics),
    };

    diagnostics.extend(get_unknown_property_diagnostics(config));

    ResolveConfigurationResult {
        config: resolved,
        diagnostics,
    }
}
