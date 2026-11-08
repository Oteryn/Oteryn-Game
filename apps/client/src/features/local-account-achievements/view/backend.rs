//! The same browser presents own-account and consented Character achievement pages.
use super::*;
use oteryn_client_ui::account_achievements::AchievementsPanel;
#[derive(Clone, Copy)]
pub(super) enum Backend {
    Account,
    Character(u64),
}
impl Backend {
    pub(super) fn valid(self, m: &Mailbox) -> bool {
        match self {
            Self::Account => true,
            Self::Character(g) => {
                !m.cyclopedia_character_shutdown && m.cyclopedia_character.profile_generation == g
            }
        }
    }
    pub(super) fn ready(self, m: &Mailbox) -> bool {
        matches!(self, Self::Account) || !m.cyclopedia_character.pending
    }
    pub(super) fn panel(self, m: &mut Mailbox) -> &mut AchievementsPanel {
        match self {
            Self::Account => &mut m.achievements,
            Self::Character(_) => {
                m.cyclopedia_character.achievements.selected = m.cyclopedia_character.selected;
                &mut m.cyclopedia_character.achievements
            }
        }
    }
    pub(super) fn request(self, m: &mut Mailbox, intent: Intent) -> Option<PlayCommand> {
        match self {
            Self::Account => {
                request(&mut m.achievements, intent).map(PlayCommand::AccountAchievements)
            }
            Self::Character(_) => oteryn_client_ui::cyclopedia_character::achievement_request(
                &mut m.cyclopedia_character,
                intent,
            )
            .map(PlayCommand::Feature),
        }
    }
    pub(super) fn next(self, m: &mut Mailbox) -> Option<PlayCommand> {
        match self {
            Self::Account => {
                catalog_next_request(&mut m.achievements).map(PlayCommand::AccountAchievements)
            }
            Self::Character(_) => oteryn_client_ui::cyclopedia_character::achievement_next_request(
                &mut m.cyclopedia_character,
            )
            .map(PlayCommand::Feature),
        }
    }
    pub(super) fn fail(self, m: &mut Mailbox) {
        if matches!(self, Self::Character(_)) {
            m.cyclopedia_character.pending = false;
            m.cyclopedia_character.achievement_pending = false;
        }
        let p = self.panel(m);
        p.pending = None;
        p.catalog.collecting = false;
        p.error = Some("Session ended");
    }
    pub(super) fn title(self, m: &Mailbox) -> String {
        match self {
            Self::Account => "Account achievements — Ctrl+Shift+A".into(),
            Self::Character(_) => format!(
                "Character achievements — {}",
                m.cyclopedia_character
                    .achievement_profile
                    .as_ref()
                    .map_or("Loading profile", |(_, name)| name)
            ),
        }
    }
}
