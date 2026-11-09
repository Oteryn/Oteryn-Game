//! Source world-object lifecycle bindings. OTS_HYPOTHESIS_ONLY, retained donor corpus.
use super::world_callbacks::{
    Callback, CallbackEffectSet, NativeSelector, Position, Selector, Storage, Trigger,
};

pub const UNEASY_CRYSTAL: &str = "oteryn:quest-callback/uneasy-alliance-crystal";

pub(crate) fn uneasy_crystal() -> Callback {
    let mut effects = CallbackEffectSet::default();
    effects.transform = Some(10187);
    effects.reset_after_ms = Some(10_000);
    Callback { track_initials: Default::default(), admission: None,
        world_change: None,
        actor_kind: Default::default(),
        opening_only: false,
        requires_world_admission: false,
        key: UNEASY_CRYSTAL.into(),
        quest: "oteryn:quest.an_uneasy_alliance_quest".into(),
        trigger: Trigger::Use,
        selector: Selector {
            // The retained B3 counterpart has no legacy action attribute.
            native: Some(NativeSelector {
                position: Position { x: 33155, y: 31297, floor: -3 },
                ordinal: 2,
                appearance: 10186,
            }),
            position: None,
            unique: None,
            action: Some(40030),
            appearance: Some(10186),
        },
        lever: None,
        reversible: false,
        passages: Vec::new(),
        teleport: None,
        facing: None,
        state_ops: Vec::new(),
        world_requirements: Vec::new(),
        storage: vec![Storage {
            track: "oteryn:quest-progress/quest/u8_54/an_uneasy_alliance/questline".into(),
            minimum: 3,
            maximum: 3,
            set: 4,
            preserve: false,
            increment: false,
            at_least: false,
        }],
        minimum_level: None,
        experience: None,
        monk_reward: None,
        offerings: Vec::new(),
        item_on_target: None,
        return_on_failure: false,
        failure_message: None,
        success_message: None,
        native_rule: None,
        effects,
        sources: vec![
            "crystalserver:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/scripts/quests/an_uneasy_alliance/actions_crystal_ball.lua:1-24".into(),
        ],
    }
}

/// Source readiness is re-proven by the Channel owner immediately before projection.
pub fn source_is_ready(callback: &Callback, appearance: u16) -> bool {
    callback.key != UNEASY_CRYSTAL || appearance == 10186
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uneasy_native_counterpart_matches_the_retained_bare_map_object() -> Result<(), String> {
        let callback = uneasy_crystal();
        let native = callback
            .selector
            .native
            .as_ref()
            .ok_or("missing native counterpart")?;
        assert_eq!(
            (
                native.position.x,
                native.position.y,
                native.position.floor,
                native.ordinal,
                native.appearance
            ),
            (33155, 31297, -3, 2, 10186)
        );
        assert_eq!(callback.selector.action, Some(40030));
        Ok(())
    }
    #[test]
    fn uneasy_busy_crystal_cannot_advance_another_players_quest() {
        let callback = uneasy_crystal();
        assert!(source_is_ready(&callback, 10186));
        assert!(!source_is_ready(&callback, 10187));
        assert!(!source_is_ready(&callback, 10185));
        let mut unrelated = callback;
        unrelated.key = "oteryn:quest-callback/other-source".into();
        assert!(source_is_ready(&unrelated, 10187));
    }
}
