use std::path::PathBuf;
use std::sync::Arc;

use dprint_core::configuration::resolve_global_config;
use dprint_core::configuration::ConfigKeyMap;
use dprint_development::ensure_no_diagnostics;
use dprint_development::run_specs;
use dprint_development::ParseSpecOptions;
use dprint_development::RunSpecsOptions;
use dprint_plugin_changelog::configuration::resolve_config;
use dprint_plugin_changelog::format_text;

fn main() {
    let global_config = resolve_global_config(&mut ConfigKeyMap::new()).config;

    run_specs(
        &PathBuf::from("./tests/specs"),
        &ParseSpecOptions {
            default_file_name: "CHANGELOG.md",
        },
        &RunSpecsOptions {
            fix_failures: false,
            format_twice: true,
        },
        Arc::new(move |file_path, file_text, _range, spec_config| {
            let spec_config: ConfigKeyMap =
                serde_json::from_value(spec_config.clone().into()).unwrap();
            let config_result = resolve_config(spec_config, &global_config);
            ensure_no_diagnostics(&config_result.diagnostics);
            Ok(format_text(file_path, file_text, &config_result.config)?)
        }),
        Arc::new(|_, _, _| panic!("Tracing is not supported.")),
    );
}
