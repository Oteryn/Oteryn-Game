//! Character-owned cast inputs for first admission (SPELL-D4, A13 §4.1).
//!
//! These are reads of the existing durable Character owners, not an initializer or a client
//! profile. The root revision brackets the separate owner reads: every Character mutation
//! serializes on that root and advances its revision, so a changed bracket refuses a mixed
//! snapshot. This proves data consistency only. The caller must independently refresh the
//! admitted GameSession and its Character guard before installing a runtime actor's state.

use oteryn_protocol_oteryn::actor_spell::{MAX_SOUL, MAX_VITAL_POOL};

use crate::domain::{CharacterId, CharacterRevision};
use crate::durability::DurabilityRoot;
use crate::durability::character_authority::{
    CharacterAuthorityRecord, ReconciledCharacterAuthority,
};
use crate::durability::character_build::{DurableBuildState, NO_VOCATION};
use crate::durability::character_progression::CharacterProgressionState;
use crate::spell::Vocation;
use crate::spell::cast::CharacterCastFacts;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastFactsLoad {
    Ready {
        facts: CharacterCastFacts,
        character_revision: CharacterRevision,
    },
    /// No class was chosen. Never substitute a class or initialize a durable build row here.
    NoVocation {
        facts: CharacterCastFacts,
        character_revision: CharacterRevision,
    },
    /// Classed Character missing progression, corrupt content, failed reads, or a changing root.
    Unavailable,
}

pub async fn load_character_cast_facts(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    character_id: CharacterId,
) -> CastFactsLoad {
    let Ok(before) = root.read_current_character(authority, character_id).await else {
        return CastFactsLoad::Unavailable;
    };
    let Ok(build) = root
        .read_character_build_state(authority, character_id)
        .await
    else {
        return CastFactsLoad::Unavailable;
    };
    let Ok(progression) = root
        .read_character_progression(authority, character_id)
        .await
    else {
        return CastFactsLoad::Unavailable;
    };
    let Ok(after) = root.read_current_character(authority, character_id).await else {
        return CastFactsLoad::Unavailable;
    };
    facts_from_owned_state(character_id, &before, &after, &build, progression.as_ref())
}

fn facts_from_owned_state(
    character_id: CharacterId,
    before: &CharacterAuthorityRecord,
    after: &CharacterAuthorityRecord,
    build: &DurableBuildState,
    progression: Option<&CharacterProgressionState>,
) -> CastFactsLoad {
    // An XP row can legitimately lag the root: stance/build/death receipts also advance it.
    // It may never belong to another Character, precede level 1, or exceed the root revision.
    if before.character_id != character_id
        || after.character_id != character_id
        || before.account_id != after.account_id
        || before.world_id != after.world_id
        || before.revision != after.revision
        || progression.is_some_and(|xp| {
            xp.character_id != character_id
                || xp.character_revision > after.revision
                || xp.level == 0
                || xp.total_experience.get() < 0
        })
    {
        return CastFactsLoad::Unavailable;
    }
    if build.vocation() == NO_VOCATION {
        return vocationless_vitals::load(build, progression, after.revision);
    }
    let Some(progression) = progression else {
        return CastFactsLoad::Unavailable;
    };
    let Some(vocation) = Vocation::from_key(build.vocation()) else {
        return CastFactsLoad::Unavailable;
    };
    let Some((max_health, max_mana, max_soul)) = maxima(build.vocation(), progression.level) else {
        return CastFactsLoad::Unavailable;
    };
    CastFactsLoad::Ready {
        facts: CharacterCastFacts {
            vocation,
            level: progression.level,
            magic_level: u32::from(build.magic().0),
            max_health,
            max_mana,
            max_soul,
        },
        character_revision: after.revision,
    }
}

pub use oteryn_gameplay_characters::vitals::maxima as maxima_with_bounds;

fn maxima(vocation: &str, level: u32) -> Option<(u32, u32, u32)> {
    maxima_with_bounds(vocation, level, MAX_VITAL_POOL, MAX_SOUL)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    mod vocationless_vitals_tests;
    use super::*;
    use crate::domain::progression::ProgressionRevisionContext;
    use crate::domain::{AccountId, WorldId};
    use oteryn_simulation_determinism::ExactI64;

    fn uuid(tag: u8) -> [u8; 16] {
        let mut value = [tag; 16];
        value[6] = 0x70;
        value[8] = 0x80;
        value
    }

    fn root() -> CharacterAuthorityRecord {
        CharacterAuthorityRecord {
            account_id: AccountId::from_bytes(uuid(1)).expect("account"),
            character_id: CharacterId::from_bytes(uuid(2)).expect("character"),
            world_id: WorldId::from_bytes(uuid(3)).expect("world"),
            revision: CharacterRevision::new(5).expect("revision"),
            event_id: uuid(4),
            transaction_id: uuid(5),
            payload: vec![],
        }
    }

    fn progression(root: &CharacterAuthorityRecord) -> CharacterProgressionState {
        CharacterProgressionState {
            character_id: root.character_id,
            character_revision: CharacterRevision::new(3).expect("older XP revision"),
            level: 20,
            total_experience: ExactI64::new(98_800),
            context: ProgressionRevisionContext {
                profile: "profile".into(),
                ruleset: "ruleset".into(),
                content: "content".into(),
                simulation: "simulation".into(),
                evidence: "evidence".into(),
                declaration: "declaration".into(),
            },
            policy_revision: "policy".into(),
            reward_revision: "reward".into(),
        }
    }

    fn build(vocation: &str) -> DurableBuildState {
        DurableBuildState::new(vocation, (42, 17), [(10, 0); 7]).expect("durable build")
    }

    #[test]
    fn uses_actual_vocation_magic_and_progression_without_requiring_equal_row_revisions() {
        let root = root();
        let loaded = facts_from_owned_state(
            root.character_id,
            &root,
            &root,
            &build("druid"),
            Some(&progression(&root)),
        );
        assert_eq!(
            loaded,
            CastFactsLoad::Ready {
                facts: CharacterCastFacts {
                    vocation: Vocation::Druid,
                    level: 20,
                    magic_level: 42,
                    max_health: 245,
                    max_mana: 450,
                    max_soul: 100,
                },
                character_revision: root.revision,
            }
        );
    }

    #[test]
    fn no_vocation_is_explicit_and_unknown_vocation_fails_closed() {
        let root = root();
        let xp = progression(&root);
        assert_eq!(
            facts_from_owned_state(
                root.character_id,
                &root,
                &root,
                &DurableBuildState::default(),
                Some(&xp)
            ),
            CastFactsLoad::NoVocation {
                facts: vocationless_vitals::facts(&DurableBuildState::default(), 20)
                    .expect("none vitals"),
                character_revision: root.revision
            }
        );
        assert_eq!(
            facts_from_owned_state(
                root.character_id,
                &root,
                &root,
                &build("unregistered"),
                Some(&xp)
            ),
            CastFactsLoad::Unavailable
        );
    }

    #[test]
    fn no_vocation_can_enter_without_progression_but_a_class_requires_owned_progression() {
        let before = root();
        let no_vocation = DurableBuildState::default();
        assert_eq!(
            facts_from_owned_state(before.character_id, &before, &before, &no_vocation, None),
            CastFactsLoad::NoVocation {
                facts: vocationless_vitals::facts(&DurableBuildState::default(), 1)
                    .expect("none vitals"),
                character_revision: before.revision,
            }
        );
        assert_eq!(
            facts_from_owned_state(before.character_id, &before, &before, &build("druid"), None),
            CastFactsLoad::Unavailable
        );
    }

    #[test]
    fn absent_progression_does_not_bypass_no_vocation_root_fences_or_foreign_data() {
        let before = root();
        let build = DurableBuildState::default();
        for operator in 0..4 {
            let mut after = root();
            match operator {
                0 => after.character_id = CharacterId::from_bytes(uuid(9)).expect("foreign"),
                1 => after.account_id = AccountId::from_bytes(uuid(9)).expect("foreign"),
                2 => after.world_id = WorldId::from_bytes(uuid(9)).expect("foreign"),
                _ => after.revision = CharacterRevision::new(6).expect("advanced"),
            }
            assert_eq!(
                facts_from_owned_state(before.character_id, &before, &after, &build, None),
                CastFactsLoad::Unavailable
            );
        }
        let mut foreign = progression(&before);
        foreign.character_id = CharacterId::from_bytes(uuid(9)).expect("foreign");
        assert_eq!(
            facts_from_owned_state(
                before.character_id,
                &before,
                &before,
                &build,
                Some(&foreign)
            ),
            CastFactsLoad::Unavailable
        );
    }

    #[test]
    fn rejects_each_changed_root_identity_or_revision_and_foreign_or_future_progression() {
        let before = root();
        let xp = progression(&before);
        let build = build("druid");
        for operator in 0..4 {
            let mut after = root();
            match operator {
                0 => after.character_id = CharacterId::from_bytes(uuid(9)).expect("foreign"),
                1 => after.account_id = AccountId::from_bytes(uuid(9)).expect("foreign"),
                2 => after.world_id = WorldId::from_bytes(uuid(9)).expect("foreign"),
                _ => after.revision = CharacterRevision::new(6).expect("advanced"),
            }
            assert_eq!(
                facts_from_owned_state(before.character_id, &before, &after, &build, Some(&xp)),
                CastFactsLoad::Unavailable
            );
        }
        for operator in 0..4 {
            let mut invalid = xp.clone();
            match operator {
                0 => invalid.character_id = CharacterId::from_bytes(uuid(9)).expect("foreign"),
                1 => invalid.character_revision = CharacterRevision::new(6).expect("future"),
                2 => invalid.level = 0,
                _ => invalid.total_experience = ExactI64::new(-1),
            }
            assert_eq!(
                facts_from_owned_state(
                    before.character_id,
                    &before,
                    &before,
                    &build,
                    Some(&invalid)
                ),
                CastFactsLoad::Unavailable
            );
        }
    }
}

mod vocationless_vitals;
