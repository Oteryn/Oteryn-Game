//! The three equally likely outcomes of Armenius's final dialogue turn.
//! Source: Crystal 201189f8, data-global/npc/armenius.lua:87–104.

use super::npc_rules::NpcRulePosition;

pub const MISSION: &str = "oteryn:quest-progress/quest/u8_4/blood_brothers/mission03";
pub const NPC: &str = "oteryn:npc.armenius";
pub const FINAL_TOPIC: u16 = 5;
pub const RESOLVED_MISSION: i64 = 2;
pub const CREATURE: &str = "oteryn:creature.armenius";
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

#[cfg(test)]
mod tests {
    use super::{Outcome, outcome};

    #[test]
    fn armenius_final_draw_covers_distinct_source_outcomes() {
        assert!(matches!(outcome(0), Outcome::Rats { .. }));
        assert!(matches!(outcome(1), Outcome::Confrontation { .. }));
        assert_eq!(outcome(2), Outcome::Dismissed);
        assert_eq!(outcome(3), outcome(0));
    }
}
