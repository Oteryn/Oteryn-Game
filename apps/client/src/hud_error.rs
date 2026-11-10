use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudPart {
    WindowHandle,
    CommonControls,
    PaneFrame,
    PaneText,
    PaneProgress,
    PaneSubclass,
    ProficiencyRoot,
    ProficiencyProgress,
    ProficiencyLabel,
    ProficiencyButton,
    ProficiencySubclass,
    ArcWindow,
    ArcLayer,
    SpellButton,
    ShortcutRoot,
    ShortcutButton,
}

impl Display for HudPart {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::WindowHandle => "window handle",
            Self::CommonControls => "common controls",
            Self::PaneFrame => "pane frame",
            Self::PaneText => "pane text control",
            Self::PaneProgress => "pane progress control",
            Self::PaneSubclass => "pane subclass",
            Self::ProficiencyRoot => "proficiency root",
            Self::ProficiencyProgress => "proficiency progress control",
            Self::ProficiencyLabel => "proficiency label",
            Self::ProficiencyButton => "proficiency button",
            Self::ProficiencySubclass => "proficiency subclass",
            Self::ArcWindow => "arc window",
            Self::ArcLayer => "arc layered-window attributes",
            Self::SpellButton => "spell action button",
            Self::ShortcutRoot => "shortcut root",
            Self::ShortcutButton => "shortcut button",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudInitError {
    pub part: HudPart,
    pub win32_error: u32,
}

impl HudInitError {
    #[must_use]
    pub const fn new(part: HudPart, win32_error: u32) -> Self {
        Self { part, win32_error }
    }
}

impl Display for HudInitError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} failed (Win32 error {})",
            self.part, self.win32_error
        )
    }
}

impl std::error::Error for HudInitError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_names_the_failing_part_and_win32_code() {
        assert_eq!(
            HudInitError::new(HudPart::ArcWindow, 87).to_string(),
            "arc window failed (Win32 error 87)"
        );
    }
}
