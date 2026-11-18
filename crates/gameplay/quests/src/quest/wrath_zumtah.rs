//! Crystal 201189f8 data-global/npc/zumtah.lua:48-127.
use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
    wrath_sleeping_dragon::QUEST,
};

pub const NPC: &str = "oteryn:npc.zumtah";
pub const RELEASE: &str =
    "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/prison_release_status";
pub const STATUS: &str = "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/zumtah_status";
pub const FIRST: &str = "oteryn:quest-transition/wrath-of-the-emperor/zumtah-first-release";
pub const RETURN: &str = "oteryn:quest-transition/wrath-of-the-emperor/zumtah-repeat-release";
pub const LOOK_TYPE: u32 = 348;

pub struct Reply {
    pub topic: u16,
    pub line: &'static str,
    pub transition: Option<&'static str>,
}

pub fn respond(known: bool, topic: u16, message: &str) -> Option<Reply> {
    let message = message.to_ascii_lowercase();
    let has = |word| {
        message
            .split(|c: char| !c.is_alphanumeric())
            .any(|part| part == word)
    };
    let (topic, line, transition) = if has("exit") {
        if known {
            (
                0,
                "I'll turn you into a beetle again. Your crate will be waiting at the exit.",
                Some(RETURN),
            )
        } else {
            match topic {
                0 => (1, "You want out? Surely you're giving up already.", None),
                3 => (
                    4,
                    "I have no idea what you mean. Are you still trying to get away?",
                    None,
                ),
                6 => (
                    7,
                    "You still want to leave? Why can't you enjoy it here?",
                    None,
                ),
                10 => (
                    11,
                    "Have I ever left this place in all those years? I can't remember.",
                    None,
                ),
                11 => (12, "What was the world outside like? I've forgotten.", None),
                12 => (
                    13,
                    "I never stopped to think what this place does to people. I'm sorry.",
                    None,
                ),
                13 => (
                    14,
                    "I can make you small enough to escape. Ask me for the exit once more.",
                    None,
                ),
                14 => (
                    0,
                    "Go through the hole as a beetle. Your crate will be waiting outside.",
                    Some(FIRST),
                ),
                _ => return None,
            }
        }
    } else if has("no") {
        match topic {
            1 => (2, "You can't leave. Do you see a way out?", None),
            4 => (5, "Enough. Do you mean to keep asking?", None),
            7 => (
                8,
                "Still no exit! Are you trying to get on my nerves?",
                None,
            ),
            _ => return None,
        }
    } else if has("yes") {
        match topic {
            2 => (3, "Do you mean that tiny hole among the bones?", None),
            5 => (6, "You are certainly persistent.", None),
            8 => (
                9,
                "Guess how many years I've been here. 89, 164, or 278?",
                None,
            ),
            _ => return None,
        }
    } else if topic == 9 && has("278") {
        (
            10,
            "Correct. I've lost count of the souls I've seen here.",
            None,
        )
    } else if topic == 9 && (has("89") || has("164")) {
        (0, "Wrong. You'll have to start again.", None)
    } else {
        return None;
    };
    Some(Reply {
        topic,
        line,
        transition,
    })
}

pub(super) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    for key in [STATUS, RELEASE] {
        if catalogue
            .track(key)
            .is_none_or(|track| track.quest != QUEST)
        {
            return Err(format!("Zumtah source track {key}"));
        }
    }
    let mut transitions: Vec<_> = catalogue.transitions.into_values().collect();
    for (key, status) in [(FIRST, 0), (RETURN, 1)] {
        transitions.push(QuestTransition {
            key: key.into(),
            quest: QUEST.into(),
            effects: vec![
                QuestEffect {
                    track: STATUS.into(),
                    from: QuestComparison::Eq(status),
                    effect: QuestEffectKind::Set(1),
                },
                QuestEffect {
                    track: RELEASE.into(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Set(1),
                },
            ],
            completes: false,
            experience: None,
            world_change: None,
        });
    }
    QuestStateCatalogue::new(
        catalogue.content_revision,
        catalogue.tracks.into_values().collect(),
        transitions,
    )
    .map_err(|error| format!("{error:?}"))
}
