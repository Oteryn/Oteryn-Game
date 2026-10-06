//! `OTERYN_LOG` (§1.10 item 5): per-module levels for `info` and `debug` lines.

use crate::line::Level;
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// A malformed `OTERYN_LOG`; the start must fail with its registered code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogSpecError {
    pub entry: String,
}

impl std::fmt::Display for LogSpecError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "OTERYN_LOG entry `{}` is not `level` or `module=level`",
            self.entry
        )
    }
}

impl std::error::Error for LogSpecError {}

/// Levels filter only `info` and `debug` lines; a `warn` or `error` line and every line that
/// carries a code is always written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter {
    default: Level,
    modules: BTreeMap<String, Level>,
}

impl Default for Filter {
    fn default() -> Self {
        Self {
            default: Level::Info,
            modules: BTreeMap::new(),
        }
    }
}

impl Filter {
    /// Parse `info,durability=debug`. An empty spec is the default (`info`).
    pub fn parse(spec: &str) -> Result<Self, LogSpecError> {
        let mut filter = Self::default();
        if spec.is_empty() {
            return Ok(filter);
        }
        for entry in spec.split(',') {
            let bad = || LogSpecError {
                entry: entry.chars().take(64).collect(),
            };
            match entry.split_once('=') {
                None => filter.default = Level::parse(entry).ok_or_else(bad)?,
                Some((module, level)) => {
                    let valid = !module.is_empty()
                        && module.bytes().all(|byte| {
                            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'
                        });
                    if !valid {
                        return Err(bad());
                    }
                    filter
                        .modules
                        .insert(module.to_owned(), Level::parse(level).ok_or_else(bad)?);
                }
            }
        }
        Ok(filter)
    }

    /// Whether a line of `level` from `module` is written. `coded` lines always are.
    #[must_use]
    pub fn allows(&self, level: Level, module: &str, coded: bool) -> bool {
        if coded || level <= Level::Warn {
            return true;
        }
        level <= self.modules.get(module).copied().unwrap_or(self.default)
    }
}

static FILTER: OnceLock<Filter> = OnceLock::new();

/// Read `OTERYN_LOG` once at process start. Returns the error for a malformed value and keeps
/// the default filter, so the caller can write its coded failure line before it exits.
pub fn init_filter(spec: Option<&str>) -> Result<(), LogSpecError> {
    let (filter, result) = match Filter::parse(spec.unwrap_or("")) {
        Ok(filter) => (filter, Ok(())),
        Err(error) => (Filter::default(), Err(error)),
    };
    let _ = FILTER.set(filter);
    result
}

/// The process filter; the default until [`init_filter`] has run.
#[must_use]
pub fn filter() -> &'static Filter {
    FILTER.get_or_init(Filter::default)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn warn_drops_info_but_keeps_coded_and_warn_lines() {
        let filter = Filter::parse("warn").unwrap();
        assert!(!filter.allows(Level::Info, "node", false));
        assert!(!filter.allows(Level::Debug, "node", false));
        assert!(filter.allows(Level::Info, "node", true));
        assert!(filter.allows(Level::Warn, "node", false));
        assert!(filter.allows(Level::Error, "node", false));
    }

    #[test]
    fn modules_override_the_default() {
        let filter = Filter::parse("info,durability=debug,node=warn").unwrap();
        assert!(filter.allows(Level::Debug, "durability", false));
        assert!(!filter.allows(Level::Debug, "gameplay_transport", false));
        assert!(filter.allows(Level::Info, "gameplay_transport", false));
        assert!(!filter.allows(Level::Info, "node", false));
        assert!(
            Filter::parse("")
                .unwrap()
                .allows(Level::Info, "node", false)
        );
    }

    #[test]
    fn a_malformed_spec_is_an_error() {
        for bad in [
            "loud",
            "node=",
            "=info",
            "Node=info",
            "info,,warn",
            "node=loud",
            "a b=info",
        ] {
            assert!(Filter::parse(bad).is_err(), "{bad}");
        }
    }
}
