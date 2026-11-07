//! One error code space for Oteryn Game binaries (ARCH-ERROR-CODES-0 §1.4, §1.5, §1.10).
//!
//! The crate has no dependencies. [`error_kinds!`] declares a boundary enum's code kinds from one
//! list, [`Line`] renders the one-line diagnostic format and [`parse_line`] reads it back, and
//! [`install_panic_hook`] is the only function that performs I/O (one line to stderr).

#[cfg(test)]
mod build_source;
mod filter;
mod hook;
mod line;

pub use filter::{Filter, LogSpecError, filter, init_filter};
pub use hook::{PANIC, PanicKind, install_panic_hook, panic_line};
pub use line::{Level, Line, ParseError, ParsedLine, escape_detail, parse_line, unix_ms};

/// The vocabulary categories (`FOUNDATION_ERROR_VOCABULARY.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    InvalidInput,
    UnsupportedRevision,
    AuthenticationFailed,
    SessionRejected,
    StaleGeneration,
    Conflict,
    CapacityExceeded,
    DependencyUnavailable,
    Timeout,
    Cancelled,
    InternalUnavailable,
}

impl Category {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidInput => "INVALID_INPUT",
            Self::UnsupportedRevision => "UNSUPPORTED_REVISION",
            Self::AuthenticationFailed => "AUTHENTICATION_FAILED",
            Self::SessionRejected => "SESSION_REJECTED",
            Self::StaleGeneration => "STALE_GENERATION",
            Self::Conflict => "CONFLICT",
            Self::CapacityExceeded => "CAPACITY_EXCEEDED",
            Self::DependencyUnavailable => "DEPENDENCY_UNAVAILABLE",
            Self::Timeout => "TIMEOUT",
            Self::Cancelled => "CANCELLED",
            Self::InternalUnavailable => "INTERNAL_UNAVAILABLE",
        }
    }
}

/// Whether the same operation may succeed later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progression {
    Retryable,
    Terminal,
    SecurityTerminal,
}

impl Progression {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Retryable => "RETRYABLE",
            Self::Terminal => "TERMINAL",
            Self::SecurityTerminal => "SECURITY_TERMINAL",
        }
    }
}

/// One registered code. `Display` is `E{number:04} {name}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorCode {
    pub number: u32,
    pub name: &'static str,
    category: Category,
    progression: Progression,
}

impl ErrorCode {
    #[must_use]
    pub const fn new(
        number: u32,
        name: &'static str,
        category: Category,
        progression: Progression,
    ) -> Self {
        Self {
            number,
            name,
            category,
            progression,
        }
    }

    #[must_use]
    pub const fn category(&self) -> Category {
        self.category
    }

    #[must_use]
    pub const fn progression(&self) -> Progression {
        self.progression
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "E{:04} {}", self.number, self.name)
    }
}

/// `<version>+<source>`: the build id of a binary, which passes its own `CARGO_PKG_VERSION`.
#[must_use]
pub fn build_id(version: &str) -> String {
    format!("{version}+{}", env!("OTERYN_BUILD_SOURCE"))
}

/// Declares a boundary enum's code kinds from one list: the fieldless kind enum, `ALL` and
/// `code()`, which carries number, name, category and progression, so none can be read from
/// anywhere else and no kind can miss `ALL`.
///
/// ```
/// oteryn_error_codes::error_kinds! {
///     /// Example kinds.
///     pub enum ExampleKind {
///         Broken = (2999, "EXAMPLE_BROKEN", InternalUnavailable, Terminal),
///     }
/// }
/// assert_eq!(ExampleKind::Broken.code().to_string(), "E2999 EXAMPLE_BROKEN");
/// ```
#[macro_export]
macro_rules! error_kinds {
    (
        $(#[$meta:meta])*
        $vis:vis enum $kind:ident {
            $( $(#[$variant_meta:meta])* $variant:ident = ($number:expr, $name:expr, $category:ident, $progression:ident) ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        $vis enum $kind {
            $( $(#[$variant_meta])* $variant ),+
        }

        impl $kind {
            /// Every kind of this boundary.
            pub const ALL: &'static [$kind] = &[ $( $kind::$variant ),+ ];

            /// The registered code of this kind.
            #[must_use]
            pub const fn code(self) -> $crate::ErrorCode {
                match self {
                    $( $kind::$variant => $crate::ErrorCode::new(
                        $number,
                        $name,
                        $crate::Category::$category,
                        $crate::Progression::$progression,
                    ) ),+
                }
            }
        }
    };
}
