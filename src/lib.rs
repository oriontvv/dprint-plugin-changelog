pub mod configuration;
pub mod formatter;
mod lines;
mod release;

pub use formatter::format_text;
pub use formatter::Diagnostic;
pub use formatter::FormatTextError;

#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
mod wasm_plugin;
