//! Noodles's reusable collar and escort transitions use the shared Quest and World owners.
use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};

pub const QUEST: &str = "oteryn:quest.authored.noodles_is_gone_mini_world_change";
pub const COLLAR_REWARD: &str = "oteryn:quest-reward/noodles-leashes";
pub const COLLAR_ITEM: &str = "oteryn:item.tibia.i14156";
pub const ESCORT: &str = "oteryn:quest-transition/world-action/noodles/escort";
pub const RETURN: &str = "oteryn:quest-transition/world-action/noodles/return";
pub const ESCORT_TRACK: &str = "oteryn:quest-progress/noodles/escort";
pub const RETURN_TRACK: &str = "oteryn:quest-progress/noodles/returned";
pub const COLLAR_TRACK: &str = "oteryn:quest-progress/noodles/collar_earned";
pub const NOODLES: &str = "oteryn:npc.noodles";
pub const KING: &str = "oteryn:npc.king_tibianus";
pub const DOG_LADY: &str = "oteryn:npc.dog_lady";
/// Native donor world-npc.xml:762-763, Crystal9f5a72c.
pub const PALACE: (u16, u16, i8) = (32315, 32178, -6);
pub const DOG_LADY_POSITION: (u16, u16, i8) = (32311, 32174, -6);
/// Retained external quest guide's thirteen documented roaming locations.
/// https://github.com/Lavovskii/grimorio/blob/dbeee0138a54b30a74dd690c4a7e6fab514c8db0/quests/data/quests/noodles-is-gone-mini-world-change.json
pub const ROAMING: [(u16, u16, i8); 13] = [
    (32313, 32171, -7),
    (32314, 32184, -7),
    (32284, 32165, -7),
    (32224, 32116, -7),
    (32237, 32070, -7),
    (32221, 32030, -7),
    (32362, 32326, -7),
    (32287, 32402, -7),
    (32402, 32386, -7),
    (32429, 32391, -7),
    (32361, 32360, -7),
    (32458, 32326, -7),
    (32487, 32241, -7),
];

/// The donor King dialogue returns the dog after the personal escort was started.
pub fn returns_to_king(npc: &str, text: &str, escort: i64) -> bool {
    npc == KING && text.trim().eq_ignore_ascii_case("noodles") && escort == 1
}

pub fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let mut tracks: Vec<_> = catalogue.tracks.values().cloned().collect();
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    for key in [ESCORT_TRACK, RETURN_TRACK] {
        tracks.push(QuestTrack {
            key: key.into(),
            quest: QUEST.into(),
            initial: -1,
            min: -1,
            max: 1,
        });
    }
    for (key, track) in [(ESCORT, ESCORT_TRACK), (RETURN, RETURN_TRACK)] {
        transitions.push(QuestTransition {
            key: key.into(),
            quest: QUEST.into(),
            effects: {
                let mut effects = vec![QuestEffect {
                    track: track.into(),
                    from: QuestComparison::Between(-1, 1),
                    effect: QuestEffectKind::Set(1),
                }];
                if key == RETURN {
                    effects.push(QuestEffect {
                        track: ESCORT_TRACK.into(),
                        from: QuestComparison::Between(-1, 1),
                        effect: QuestEffectKind::Set(0),
                    });
                }
                effects
            },
            completes: false,
            experience: None,
        });
    }
    QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|error| format!("{error:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn noodles_callbacks_use_existing_quest_tracks_and_one_reusable_collar() -> Result<(), String> {
        let base =
            QuestStateCatalogue::new("r1", Vec::new(), Vec::new()).map_err(|e| format!("{e:?}"))?;
        let catalogue = extend(base)?;
        assert!(catalogue.transition(ESCORT).is_some());
        assert!(catalogue.transition(RETURN).is_some());
        assert!(returns_to_king(KING, " Noodles ", 1));
        assert!(!returns_to_king(KING, "noodles", 0));
        assert!(!returns_to_king(NOODLES, "noodles", 1));
        assert!(!returns_to_king(KING, "leashes", 1));
        assert_eq!(ROAMING.len(), 13);
        let rewards = super::super::reward_callbacks::load()?;
        let reward = rewards
            .iter()
            .find(|r| r.key == COLLAR_REWARD)
            .ok_or("collar reward callback")?;
        assert_eq!(reward.npc, KING);
        assert_eq!(
            reward.give.as_ref().map(|r| (r.item.as_str(), r.count)),
            Some((COLLAR_ITEM, 1))
        );
        assert!(reward.take.is_empty());
        Ok(())
    }
}
