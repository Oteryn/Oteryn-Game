//! Local diagnostics choices: the client log level and the diagnostics overlay.
use serde::{Deserialize, Serialize};

/// `log_level` indexes [`DiagnosticsPreferences::LEVELS`]; `OTERYN_LOG` overrides it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DiagnosticsPreferences {
    pub log_level: u8,
    pub overlay: bool,
}
impl DiagnosticsPreferences {
    pub const LEVELS: [&'static str; 3] = ["warn", "info", "debug"];
    /// The tracing level name; an out-of-range stored value reads as `info`.
    pub fn level_name(self) -> &'static str {
        Self::LEVELS
            .get(usize::from(self.log_level))
            .copied()
            .unwrap_or("info")
    }
}
impl Default for DiagnosticsPreferences {
    fn default() -> Self {
        Self {
            log_level: 1,
            overlay: false,
        }
    }
}
