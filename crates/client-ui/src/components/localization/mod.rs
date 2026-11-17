//! Typed locale selection; only source templates are translated, never formatted player data.
use std::{
    cell::Cell,
    sync::atomic::{AtomicU8, Ordering},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum Language {
    #[default]
    En,
    De,
    Es,
    Pl,
    Pt,
    Sv,
}
impl Language {
    pub const ALL: [Self; 6] = [Self::En, Self::De, Self::Es, Self::Pl, Self::Pt, Self::Sv];
    pub const fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::De => "de",
            Self::Es => "es",
            Self::Pl => "pl",
            Self::Pt => "pt",
            Self::Sv => "sv",
        }
    }
    pub const fn name(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::De => "Deutsch",
            Self::Es => "Español",
            Self::Pl => "Polski",
            Self::Pt => "Português",
            Self::Sv => "Svenska",
        }
    }
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|language| language.code() == code)
    }
}
static DEFAULT_LANGUAGE: AtomicU8 = AtomicU8::new(0);
thread_local! { static OVERRIDE: Cell<Option<Language>> = const { Cell::new(None) }; }
pub fn current() -> Language {
    OVERRIDE.with(Cell::get).unwrap_or_else(|| {
        Language::ALL
            .get(usize::from(DEFAULT_LANGUAGE.load(Ordering::Relaxed)))
            .copied()
            .unwrap_or_default()
    })
}
pub fn set_default(language: Language) {
    DEFAULT_LANGUAGE.store(language as u8, Ordering::Relaxed);
}
pub fn with_language<T>(language: Language, render: impl FnOnce() -> T) -> T {
    struct Restore(Option<Language>);
    impl Drop for Restore {
        fn drop(&mut self) {
            OVERRIDE.set(self.0);
        }
    }
    let _restore = Restore(OVERRIDE.replace(Some(language)));
    render()
}
pub fn translate(language: Language, source: &'static str) -> &'static str {
    catalog_text(language, source).unwrap_or(source)
}
pub fn tr(source: &'static str) -> &'static str {
    translate(current(), source)
}

include!(concat!(env!("OUT_DIR"), "/localized_messages.rs"));
