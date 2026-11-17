//! Old PL/EN preferences retain their meaning; new files use the selected catalog code.
use super::{ReferencePreferences, SettingsPage};
use oteryn_client_ui::localization::{Language, translate};
impl ReferencePreferences {
    pub fn language(&self) -> Language {
        self.language_code
            .as_deref()
            .and_then(Language::from_code)
            .unwrap_or(if self.english {
                Language::En
            } else {
                Language::Pl
            })
    }
    pub fn set_language(&mut self, language: Language) {
        self.language_code = Some(language.code().into());
        self.english = language == Language::En;
    }
}
impl SettingsPage {
    pub fn localized_label(self, language: Language) -> &'static str {
        translate(language, self.label())
    }
}
