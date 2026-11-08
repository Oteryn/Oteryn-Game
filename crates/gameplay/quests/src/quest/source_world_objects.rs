//! Source world-object lifecycle bindings. OTS_HYPOTHESIS_ONLY, retained donor corpus.
use super::world_callbacks::{Callback, CallbackEffectSet, Selector, Storage, Trigger};

pub(crate) fn uneasy_crystal() -> Callback {
    let mut effects = CallbackEffectSet::default();
    effects.transform = Some(10187);
    effects.reset_after_ms = Some(10_000);
    Callback { admission: None, requires_premium: false,
        world_change: None,
        actor_kind: Default::default(),
        opening_only: false,
        requires_world_admission: false,
        key: "oteryn:quest-callback/uneasy-alliance-crystal".into(),
        quest: "oteryn:quest.an_uneasy_alliance_quest".into(),
        trigger: Trigger::Use,
        selector: Selector {
            native: None,
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
