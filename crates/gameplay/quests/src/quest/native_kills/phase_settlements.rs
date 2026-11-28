//! Source progress for terminal deaths proved by the shared encounter owner.
use super::{Counter, QuestComparison, QuestEffect, QuestEffectKind};

// Crystal 201189f, creaturescripts_bosses_mission_cults.lua:4,48-52:
// https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/cults_of_tibia/creaturescripts_bosses_mission_cults.lua
pub(super) const COUNTERS: &[Counter] = &[Counter {
    authored_stage: false,
    transition: "oteryn:quest-transition/native-kills/cults-essence",
    creature: "oteryn:creature.essence_of_malice",
    quest: "oteryn:quest.cults_of_tibia_quest",
    track: "oteryn:quest-progress/quest/u11_40/cults_of_tibia/humans/mission",
    gate: "oteryn:quest-progress/quest/u11_40/cults_of_tibia/humans/mission",
    maximum: 2,
    death_area: None,
    world_change: None,
}];

pub(super) fn gate_bounds(counter: &Counter) -> Option<(i64, i64)> {
    COUNTERS
        .iter()
        .any(|entry| entry.transition == counter.transition)
        .then_some((0, counter.maximum - 1))
}

pub(super) fn effects(counter: &Counter) -> Option<Vec<QuestEffect>> {
    let (minimum, maximum) = gate_bounds(counter)?;
    Some(vec![QuestEffect {
        track: counter.track.into(),
        from: QuestComparison::Between(minimum, maximum),
        effect: QuestEffectKind::Set(counter.maximum),
    }])
}

#[cfg(test)]
mod tests {
    #[test]
    fn cults_terminal_death_sets_humans_mission_to_two() {
        let counter = super::super::counter("oteryn:quest-transition/native-kills/cults-essence");
        assert!(
            counter.is_some(),
            "Cults terminal settlement is not registered"
        );
        if let Some(counter) = counter {
            let transition = counter.transition();
            assert_eq!(transition.effects.len(), 1);
            assert_eq!(transition.effects[0].effect, super::QuestEffectKind::Set(2));
            assert_eq!(counter.gate_bounds(), (0, 1));
            let mut tracks = std::collections::BTreeMap::new();
            assert!(super::super::active_encounter(&tracks).contains(&counter.transition));
            assert!(!super::super::active(&tracks).contains(&counter.transition));
            tracks.insert(counter.track.into(), 1);
            assert!(super::super::active_encounter(&tracks).contains(&counter.transition));
            assert!(!super::super::active(&tracks).contains(&counter.transition));
            tracks.insert(counter.track.into(), 2);
            assert!(!super::super::active_encounter(&tracks).contains(&counter.transition));
        }
    }
}

#[cfg(test)]
#[test]
fn review18_phase_settlements_registered_for_loader_and_progress() -> Result<(), String> {
    for counter in COUNTERS {
        assert!(!super::counters().any(|entry| entry.transition == counter.transition));
        assert!(super::counter(counter.transition).is_some());
        assert!(!super::active(&std::collections::BTreeMap::new()).contains(&counter.transition));
        let original = counter.transition();
        crate::quest::admission_binding::validate(&original, false)?;
        let mut changed = original;
        changed.effects[0].effect = crate::quest::QuestEffectKind::Add(2);
        assert!(crate::quest::admission_binding::validate(&changed, false).is_err());
        assert!(super::registered_counters()?.any(|entry| entry.transition == counter.transition));
    }
    Ok(())
}
