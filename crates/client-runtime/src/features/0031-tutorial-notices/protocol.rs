//! Display projection for validated, server-pushed notices.
pub use oteryn_session::{ServerSaveCountdown, TutorialNotice};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    Tutorial(String),
    ServerSave { minutes: u8 },
}
#[must_use]
pub fn tutorial(value: TutorialNotice) -> Notice {
    Notice::Tutorial(match value {
        TutorialNotice::StatusText(text) => text,
        TutorialNotice::Hint(id) => match id {
            2 => "Tutorial: choose your vocation to begin your adventure.".into(),
            3 => "Knight tutorial: approach a creature and attack with your melee weapon.".into(),
            4 => "Paladin tutorial: equip your distance weapon and attack a creature.".into(),
            5 | 6 => "Mage tutorial: select a creature and use your spells.".into(),
            11 => "Monk tutorial: approach a creature and practice your attacks.".into(),
            _ => format!("Tutorial hint {id}: follow the tutorial path."),
        },
    })
}
#[must_use]
pub fn server_save(value: ServerSaveCountdown) -> Notice {
    Notice::ServerSave {
        minutes: value.minutes_remaining,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn projects_typed_notices_without_losing_status_text() {
        assert_eq!(
            tutorial(TutorialNotice::StatusText("There is no way.".into())),
            Notice::Tutorial("There is no way.".into())
        );
        assert!(
            matches!(tutorial(TutorialNotice::Hint(2)), Notice::Tutorial(text) if text.contains("vocation"))
        );
        assert!(
            matches!(tutorial(TutorialNotice::Hint(255)), Notice::Tutorial(text) if text.contains("255"))
        );
        assert_eq!(
            server_save(ServerSaveCountdown {
                ending_epoch: 9,
                minutes_remaining: 2
            }),
            Notice::ServerSave { minutes: 2 }
        );
    }
}
