//! The three equally likely outcomes of Armenius's final dialogue turn.
//! Source: Crystal 201189f8, data-global/npc/armenius.lua:87–104.

use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
    npc_rules::NpcRulePosition,
};

pub const MISSION: &str = "oteryn:quest-progress/quest/u8_4/blood_brothers/mission03";
pub const RESOLVE: &str = "oteryn:quest-transition/blood-brothers/resolve-armenius";
pub const NPC: &str = "oteryn:npc.armenius";
pub const FINAL_TOPIC: u16 = 5;
pub const RESOLVED_MISSION: i64 = 2;
pub const CREATURE: &str = "oteryn:creature.armenius_creature";
pub const ACHIEVEMENT: &str = "His True Face";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Rats {
        destination: NpcRulePosition,
    },
    Confrontation {
        destination: NpcRulePosition,
        spawn: NpcRulePosition,
    },
    Dismissed,
}

/// Map a server-owned decision to the source's three branches. The caller
/// retains the decision for the occurrence before it applies any effects.
pub fn outcome(draw: u64) -> Outcome {
    match draw % 3 {
        0 => Outcome::Rats {
            destination: NpcRulePosition {
                x: 32759,
                y: 31241,
                floor: -9,
            },
        },
        1 => Outcome::Confrontation {
            destination: NpcRulePosition {
                x: 32856,
                y: 31324,
                floor: -8,
            },
            spawn: NpcRulePosition {
                x: 32857,
                y: 31324,
                floor: -8,
            },
        },
        _ => Outcome::Dismissed,
    }
}

pub(super) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let quest = catalogue
        .track(MISSION)
        .ok_or("Armenius Mission03 track is missing")?
        .quest
        .clone();
    let mut transitions: Vec<_> = catalogue.transitions.into_values().collect();
    transitions.push(QuestTransition {
        key: RESOLVE.into(),
        quest,
        effects: vec![QuestEffect {
            track: MISSION.into(),
            from: QuestComparison::Between(1, RESOLVED_MISSION),
            effect: QuestEffectKind::Set(RESOLVED_MISSION),
        }],
        completes: false,
        experience: None,
        world_change: None,
    });
    QuestStateCatalogue::new(
        catalogue.content_revision,
        catalogue.tracks.into_values().collect(),
        transitions,
    )
    .map_err(|error| format!("{error:?}"))
}

#[cfg(test)]
mod tests {
    use super::{MISSION, Outcome, RESOLVE, RESOLVED_MISSION, outcome};

    #[test]
    fn armenius_resolution_is_registered_with_the_donor_mission_guard() -> Result<(), String> {
        let world = super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|error| format!("{error:?}"))?;
        let transition = world.catalogue().transition(RESOLVE).ok_or("resolution")?;
        assert_eq!(transition.effects.len(), 1);
        let effect = &transition.effects[0];
        assert_eq!(effect.track, MISSION);
        assert_eq!(effect.from, super::QuestComparison::Between(1, 2));
        assert_eq!(effect.effect, super::QuestEffectKind::Set(RESOLVED_MISSION));
        assert!(!transition.completes);
        Ok(())
    }

    #[test]
    fn armenius_final_draw_covers_distinct_source_outcomes() {
        assert!(matches!(outcome(0), Outcome::Rats { .. }));
        assert!(matches!(outcome(1), Outcome::Confrontation { .. }));
        assert_eq!(outcome(2), Outcome::Dismissed);
        assert_eq!(outcome(3), outcome(0));
    }
}
