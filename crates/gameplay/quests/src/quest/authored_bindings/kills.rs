//! Native counter bindings use the existing death queue and its fenced quest writer.
//! Threshold gate effects reuse questpc3 262618b3's original-snapshot operation.
#[cfg(test)]
use crate::quest::QuestEffectKind;
use serde::{Deserialize, Serialize};

pub const MAX_KILL_BINDINGS: usize = 128;

/// Inclusive victim bounds in native carrier coordinates (positive Tibia floor).
/// Eligibility is frozen from the original lethal facts, never the recipient's later position.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeathArea {
    pub minimum: [i32; 3],
    pub maximum: [i32; 3],
}
impl DeathArea {
    pub(crate) fn valid(self) -> bool {
        (0..3).all(|axis| {
            self.minimum[axis] >= 0
                && self.minimum[axis] <= self.maximum[axis]
                && self.maximum[axis] <= if axis == 2 { 15 } else { 65535 }
        })
    }
    pub fn contains(self, position: [i32; 3]) -> bool {
        (0..3).all(|axis| (self.minimum[axis]..=self.maximum[axis]).contains(&position[axis]))
    }
}

pub use super::super::creature_objectives::KillCounter;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quest::{QuestRefusal, QuestStateCatalogue};
    use std::collections::BTreeMap;

    fn binding() -> Result<super::super::Bindings, String> {
        super::super::Bindings::parse(include_bytes!("fixtures/kill.json"))
    }

    #[test]
    fn threshold_uses_original_count_and_completes_only_at_goal() -> Result<(), String> {
        let bindings = binding()?;
        let base = QuestStateCatalogue::empty("kill-fixture").ok_or("revision")?;
        let catalogue = bindings.attach(&base)?;
        let counter = &bindings.quests[0].stages[1];
        let super::super::Trigger::Kill { counter } = &counter.trigger else {
            return Err("kill stage".into());
        };
        let transition = catalogue.transition(&counter.key).ok_or("transition")?;
        let mut state = BTreeMap::from([(counter.gate.clone(), 4), (counter.track.clone(), 0)]);
        for expected in 1..=3 {
            let changes = catalogue
                .evaluate(transition, &state, 0)
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(
                catalogue.completes_after(transition, &changes),
                expected == 3
            );
            for change in changes {
                state.insert(change.track, change.after);
            }
            assert_eq!(state[&counter.track], expected);
            assert_eq!(state[&counter.gate], if expected == 3 { 5 } else { 4 });
        }
        assert_eq!(
            catalogue.evaluate(transition, &state, 0),
            Err(QuestRefusal::StageMismatch)
        );
        Ok(())
    }

    #[test]
    fn active_counter_is_owned_bounded_and_hash_binds_species() -> Result<(), String> {
        let mut bindings = binding()?;
        let base = QuestStateCatalogue::empty("kill-fixture").ok_or("revision")?;
        let catalogue = bindings.attach(&base)?;
        let super::super::Trigger::Kill { counter } = &bindings.quests[0].stages[1].trigger else {
            return Err("kill".into());
        };
        let state = BTreeMap::from([(counter.gate.clone(), 4)]);
        assert_eq!(
            crate::quest::native_kills::active_for(&catalogue, &state).as_slice(),
            std::slice::from_ref(&counter.key)
        );
        assert!(crate::quest::native_kills::active_for(&catalogue, &BTreeMap::new()).is_empty());
        let quest = bindings.quests[0].quest.clone();
        let before = catalogue.definition_hash(&quest);
        let super::super::Trigger::Kill { counter } = &mut bindings.quests[0].stages[1].trigger
        else {
            return Err("kill".into());
        };
        counter.creature = "oteryn:creature.dragon".into();
        assert_ne!(bindings.attach(&base)?.definition_hash(&quest), before);
        let super::super::Trigger::Kill { counter } = &mut bindings.quests[0].stages[1].trigger
        else {
            return Err("kill".into());
        };
        counter.track = "oteryn:quest-progress/foreign".into();
        assert!(bindings.validate().is_err());
        Ok(())
    }

    #[test]
    fn reserved_key_and_self_referential_effect_refuse() -> Result<(), String> {
        let mut bindings = binding()?;
        let super::super::Trigger::Kill { counter } = &mut bindings.quests[0].stages[1].trigger
        else {
            return Err("kill".into());
        };
        counter.key = crate::quest::native_kills::COUNTERS[0].transition.into();
        assert!(bindings.validate().is_err());
        let bindings = binding()?;
        let base = QuestStateCatalogue::empty("kill-fixture").ok_or("revision")?;
        let catalogue = bindings.attach(&base)?;
        let super::super::Trigger::Kill { counter } = &bindings.quests[0].stages[1].trigger else {
            return Err("kill".into());
        };
        let mut transition = catalogue
            .transition(&counter.key)
            .ok_or("transition")?
            .clone();
        transition.effects[0].effect = QuestEffectKind::SetWhenAtLeast {
            other_effect: 0,
            minimum: 0,
            value: 5,
        };
        assert_eq!(
            catalogue.evaluate(&transition, &BTreeMap::new(), 0),
            Err(QuestRefusal::NotSupported)
        );
        Ok(())
    }
    #[test]
    fn threshold_and_bound_set_have_distinct_definition_encoding_tags() {
        let mut threshold = Vec::new();
        QuestEffectKind::SetWhenAtLeast {
            other_effect: 0,
            minimum: 0,
            value: 0,
        }
        .encode(&mut threshold);
        let mut bound_set = Vec::new();
        QuestEffectKind::SetWithBinding {
            value: 0,
            binding: [0; 32],
        }
        .encode(&mut bound_set);
        assert_ne!(
            threshold[0], bound_set[0],
            "merged effect kinds must retain distinct encoding tags"
        );
    }
}
