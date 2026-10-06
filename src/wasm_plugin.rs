use dprint_core::configuration::ConfigKeyMap;
use dprint_core::configuration::GlobalConfiguration;
use dprint_core::generate_plugin_code;
use dprint_core::plugins::CheckConfigUpdatesMessage;
use dprint_core::plugins::ConfigChange;
use dprint_core::plugins::FileMatchingInfo;
use dprint_core::plugins::FormatError;
use dprint_core::plugins::FormatResult;
use dprint_core::plugins::PluginInfo;
use dprint_core::plugins::PluginResolveConfigurationResult;
use dprint_core::plugins::SyncFormatRequest;
use dprint_core::plugins::SyncHostFormatRequest;
use dprint_core::plugins::SyncPluginHandler;

use crate::configuration::resolve_config;
use crate::configuration::Configuration;
use crate::format_text;

const REPOSITORY: &str = "oriontvv/dprint-plugin-changelog";

struct ChangelogPluginHandler;

impl SyncPluginHandler<Configuration> for ChangelogPluginHandler {
    fn plugin_info(&mut self) -> PluginInfo {
        let version = env!("CARGO_PKG_VERSION").to_string();
        PluginInfo {
            name: env!("CARGO_PKG_NAME").to_string(),
            config_key: "changelog".to_string(),
            help_url: format!("https://github.com/{REPOSITORY}"),
            config_schema_url: format!(
                "https://plugins.dprint.dev/{REPOSITORY}/{version}/schema.json"
            ),
            update_url: Some(format!(
                "https://plugins.dprint.dev/{REPOSITORY}/latest.json"
            )),
            version,
        }
    }

    fn license_text(&mut self) -> String {
        include_str!("../LICENSE").to_string()
    }

    fn resolve_config(
        &mut self,
        config: ConfigKeyMap,
        global_config: &GlobalConfiguration,
    ) -> PluginResolveConfigurationResult<Configuration> {
        let result = resolve_config(config, global_config);
        PluginResolveConfigurationResult {
            file_matching: FileMatchingInfo {
                file_extensions: Vec::new(),
                file_names: vec![
                    "CHANGELOG.md".to_string(),
                    "Changelog.md".to_string(),
                    "changelog.md".to_string(),
                ],
                additive: result.config.additive,
            },
            diagnostics: result.diagnostics,
            config: result.config,
        }
    }

    fn check_config_updates(
        &self,
        _message: CheckConfigUpdatesMessage,
    ) -> Result<Vec<ConfigChange>, FormatError> {
        Ok(Vec::new())
    }

    fn format(
        &mut self,
        request: SyncFormatRequest<Configuration>,
        _format_with_host: impl FnMut(SyncHostFormatRequest) -> FormatResult,
    ) -> FormatResult {
        let text = String::from_utf8(request.file_bytes)?;
        match format_text(request.file_path, &text, request.config) {
            Ok(result) => Ok(result.map(String::into_bytes)),
            Err(err) => Err(FormatError::new(err)),
        }
    }
}

generate_plugin_code!(ChangelogPluginHandler, ChangelogPluginHandler);
