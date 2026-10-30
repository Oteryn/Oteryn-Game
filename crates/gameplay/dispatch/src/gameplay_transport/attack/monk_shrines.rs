//! Read bonuses from the admitted quest copy under the Channel owner lock.

use super::super::quest_admission::QuestSession;
use super::*;
use std::collections::HashMap;
use std::sync::Mutex;

pub(super) type QuestSessions = Mutex<HashMap<GameSessionId, QuestSession>>;

pub(super) fn counts(sessions: &QuestSessions, session: GameSessionId) -> (u8, u8) {
    let Ok(sessions) = sessions.lock() else {
        return (0, 0);
    };
    let Some(copy) = sessions.get(&session).and_then(|s| s.copy.as_ref()) else {
        return (0, 0);
    };
    let tracks = copy.tracks();
    (
        oteryn_gameplay_quests::quest::monk_shrines::completed_count(tracks) as u8,
        oteryn_gameplay_quests::quest::monk_shrines::offensive_count(tracks),
    )
}

pub(super) fn is_monk(vocation: crate::spell::Vocation) -> bool {
    matches!(
        vocation,
        crate::spell::Vocation::Monk | crate::spell::Vocation::ExaltedMonk
    )
}

pub(super) fn taken(
    sessions: &QuestSessions,
    session: GameSessionId,
    vocation: crate::spell::Vocation,
    damage: i64,
) -> Option<i64> {
    if !is_monk(vocation) {
        return Some(damage);
    }
    oteryn_gameplay_combat_rules::monk_shrines::MonkShrineBonuses::checked_in()?.taken(
        damage,
        counts(sessions, session).0,
        is_monk(vocation),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::creature_bite::CreatureBiteVitals;
    use crate::domain::{CharacterId, CharacterRevision};
    use crate::durability::quest_state::{CommittedQuestTransition, QuestCause, QuestStateCopy};
    use crate::gameplay_transport::actor_spell::{ChannelSpellStates, tests as fixture};
    use crate::gameplay_transport::attack::player_block::PlayerBiteVitals;
    use oteryn_gameplay_quests::quest::{QuestTrackChange, monk_shrines};

    #[test]
    fn monk_shrine_melee_adapter_reads_committed_flags_and_checks_vocation() -> Result<(), String> {
        let (mut runtime, actor, session) = fixture::runtime_with_player(179);
        let position = |x| MovementLocalPosition { x, y: 10, floor: 7 };
        runtime
            .initialize_movement_test_position(actor, position(10))
            .map_err(|e| format!("{e:?}"))?;
        let creature = runtime
            .admit_test_creature(position(11))
            .map_err(|e| format!("{e:?}"))?;
        let mut copy = QuestStateCopy::default();
        let character_id =
            CharacterId::from_bytes(*session.as_bytes()).map_err(|e| format!("{e:?}"))?;
        let receipt = CommittedQuestTransition {
            character_id,
            cause: QuestCause::Use(CommandRef::new(
                session,
                CommandId::new(1).map_err(|e| format!("{e:?}"))?,
            )),
            transition_key: "oteryn:quest-callback/monk-eternity".into(),
            quest_key: "oteryn:quest.the_way_of_the_monk_quest".into(),
            original_character_revision: CharacterRevision::new(1).map_err(|e| format!("{e:?}"))?,
            committed_character_revision: CharacterRevision::new(2)
                .map_err(|e| format!("{e:?}"))?,
            pinned_content_revision: "content-1".into(),
            definition_hash: [7; 32],
            completes: false,
            changes: monk_shrines::NAMES
                .iter()
                .map(|name| QuestTrackChange {
                    track: format!("{}{name}", monk_shrines::FLAG_PREFIX),
                    before: 0,
                    after: 1,
                })
                .collect(),
            experience: None,
        };
        assert!(copy.apply(&receipt));
        let sessions = Mutex::new(HashMap::from([(
            session,
            QuestSession {
                copy: Some(copy),
                experience_deferred: false,
                version: 1,
                retry_at: None,
            },
        )]));
        assert_eq!(counts(&sessions, session), (10, 5));
        for (index, (vocation, expected)) in [
            (crate::spell::Vocation::Monk, 200),
            (crate::spell::Vocation::ExaltedMonk, 200),
            (crate::spell::Vocation::Knight, 250),
        ]
        .into_iter()
        .enumerate()
        {
            let mut states = ChannelSpellStates::default();
            states
                .initialize(
                    &runtime,
                    actor,
                    session,
                    crate::spell::cast::CharacterCastFacts {
                        vocation,
                        max_health: 1000,
                        ..fixture::FACTS
                    },
                    (0, 0),
                    SemanticTimeMicros::from_micros(0),
                )
                .ok_or("actor initialization")?;
            let mut attack = ChannelAttackStates::default();
            let mut trained = Vec::new();
            let mut vitals = PlayerBiteVitals {
                quest_sessions: Some(&sessions),
                charm: None,
                states: &mut states,
                attack: &mut attack,
                formula: None,
                trained: &mut trained,
            };
            assert_eq!(
                vitals.melee_damage_taken(&runtime, actor, session, 250),
                expected
            );
            use crate::ability::creature_bite::{
                CreatureBiteDefinition, CreatureBiteLedger, ReentryProtection, commit_ai_bite,
            };
            let hit = commit_ai_bite(
                &mut CreatureBiteLedger::default(),
                &mut runtime,
                &mut vitals,
                crate::ability::AiAbilityAdapter::bite(
                    creature,
                    index as u64 + 1,
                    actor,
                    session,
                    0,
                    0,
                ),
                CreatureBiteDefinition::new(2_000_000, 250).ok_or("bite definition")?,
                RevisionSet::new(
                    "ruleset:ai-v1",
                    "content:1",
                    "world:ai-v1",
                    "formula:bite-v1",
                    "simulation:v1",
                )
                .map_err(|e| format!("{e:?}"))?,
                ReentryProtection {
                    protected_until: None,
                },
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
                    index as u64 * 2_000_000,
                ),
            )
            .map_err(|e| format!("{e:?}"))?;
            assert_eq!(hit.damage.applied, expected);
            assert_eq!(hit.damage.health_after, 1000 - expected);
        }
        Ok(())
    }
}
