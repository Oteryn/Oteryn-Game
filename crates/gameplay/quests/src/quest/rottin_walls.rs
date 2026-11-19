//! Crystal hammer.lua:13-20,64-101. Bind the six broken walls on the retained map.
use super::world_callbacks::{
    Callback, CallbackEffectSet, ItemOnTarget, NativeSelector, Position, Selector, Storage, Trigger,
};
use std::sync::LazyLock;

pub const PREFIX: &str = "oteryn:quest-callback/rottin-repair-";
pub const QUEST: &str = "oteryn:quest.rottin_wood_and_the_married_men_quest";

/// Admit only the published hammer bindings to the carried-tool route.
pub fn accepts(callback: &Callback) -> bool {
    static BINDINGS: LazyLock<[Callback; 6]> = LazyLock::new(callbacks);
    callback.key.starts_with(PREFIX) && BINDINGS.contains(callback)
}

pub(crate) fn callbacks() -> [Callback; 6] {
    [
        (42501, 32647, 32216, -7, 2, 12183, 6474),
        (42502, 32660, 32213, -7, 2, 12183, 6474),
        (42503, 32644, 32183, -6, 2, 12185, 6473),
        (42504, 32660, 32201, -7, 2, 12184, 6473),
        (42505, 32652, 32200, -5, 2, 12185, 6473),
        (42506, 32679, 32210, -6, 4, 12185, 6473),
    ]
    .map(|(action, x, y, floor, ordinal, appearance, repaired)| {
        let position = Position { x, y, floor };
        let mut effects = CallbackEffectSet::default();
        effects.transform = Some(repaired);
        effects.reset_after_ms = Some(120_000);
        Callback {
            key: format!("{PREFIX}{action}"),
            quest: QUEST.into(),
            trigger: Trigger::Use,
            selector: Selector {
                native: Some(NativeSelector { position, ordinal, appearance }),
                position: Some(position),
                action: Some(action),
                appearance: Some(appearance),
                position_aliases: Vec::new(),
                ordinal: None,
                unique: None,
            },
            use_range: 1,
            requires_world_admission: true,
            storage: vec![Storage {
                track: super::classic_talk::ROTTIN_WALLCOUNT.into(),
                minimum: 0,
                maximum: 5,
                set: 1,
                increment: true,
                preserve: false,
                at_least: false,
                read_only: false,
                set_if: None,
                add: None,
            }],
            item_on_target: Some(ItemOnTarget {
                item: "oteryn:item.tibia.i3460".into(),
                alternatives: Vec::new(),
                count: 1,
                replacement: None,
                target_replacement: None,
                target_remove: false,
                reusable: true,
                on_step: false,
                selected_instance: true,
                ingredients: Vec::new(),
            }),
            effects,
            success_message: Some("You fixed this broken wall.".into()),
            failure_message: Some("You already fixed many broken walls today.".into()),
            sources: vec!["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/actions/tools/hammer.lua:13-20,64-101".into()],
            actor_kind: Default::default(),
            activation: None,
            admission: None,
            achievements: Vec::new(),
            character_requirements: None,
            completes: false,
            disguise: None,
            experience: None,
            facing: None,
            failure_teleport: None,
            item_reward: None,
            lever: None,
            minimum_level: None,
            monk_reward: None,
            native_rule: None,
            offerings: Vec::new(),
            opening_only: false,
            passages: Vec::new(),
            premium_required: false,
            present: Vec::new(),
            remote_teleport: None,
            required_completed_quests: Vec::new(),
            required_travel: Vec::new(),
            requirements: None,
            requires_premium: false,
            return_on_failure: false,
            reversible: false,
            reward_bundle: false,
            state_operations: Vec::new(),
            state_ops: Vec::new(),
            teleport: None,
            track_initials: Default::default(),
            world_change: None,
            world_requirements: Vec::new(),
        }
    })
}

/// Retain both shipped definitions for characters who already started Disrepair.
pub(super) fn retain_predecessor(catalogue: &mut super::QuestStateCatalogue) -> Result<(), String> {
    use super::classic_talk::ROTTIN_WALLCOUNT;

    let legacy_track = |key: &str| {
        !matches!(
            key,
            "oteryn:quest-progress/crystalserver/kv/handyman-task"
                | "oteryn:quest-progress/crystalserver/kv/lucky-loop-task"
                | "oteryn:quest-progress/crystalserver/kv/nest-timer"
                | "oteryn:quest-progress/crystalserver/kv/rottinwood-corpsecount"
                | "oteryn:quest-progress/crystalserver/kv/rottinwood-trap-count"
                | "oteryn:quest-progress/rottin-wood/net-replacements"
        )
    };
    for before_hammer in [true, false] {
        let tracks: Vec<_> = catalogue
            .tracks
            .values()
            .filter(|track| track.quest == QUEST && legacy_track(&track.key))
            .cloned()
            .map(|mut track| {
                if before_hammer && track.key == ROTTIN_WALLCOUNT {
                    track.max = 0;
                }
                track
            })
            .collect();
        let defaults = tracks
            .iter()
            .map(|track| (track.key.clone(), track.initial))
            .collect();
        let transitions = catalogue
            .transitions
            .values()
            .filter(|transition| {
                transition.quest == QUEST
                    && !transition
                        .key
                        .starts_with("oteryn:quest-reward/rottin-wood/")
                    && (!before_hammer || !transition.key.starts_with(PREFIX))
                    && transition
                        .effects
                        .iter()
                        .all(|effect| legacy_track(&effect.track))
            })
            .cloned()
            .collect();
        let predecessor =
            super::QuestStateCatalogue::new(catalogue.content_revision(), tracks, transitions)
                .map_err(|error| format!("{error:?}"))?;
        let hash = predecessor
            .definition_hash(QUEST)
            .ok_or("Rottin predecessor definition")?;
        catalogue
            .predecessors
            .entry(QUEST.into())
            .or_default()
            .push(super::NpcDefinitionPredecessor { hash, defaults });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rottin_wall_bindings_are_registered_and_bounded() -> Result<(), String> {
        let expected = callbacks();
        super::super::world_callbacks::validate_callbacks(&expected)?;
        let registered = super::super::world_callbacks::load()?;
        assert!(
            expected
                .iter()
                .all(|callback| registered.contains(callback))
        );
        let loaded = super::super::loader::load_embedded_quest_state("rottin-wall-test")
            .map_err(|err| format!("{err:?}"))?;
        let catalogue = loaded.catalogue();
        assert_eq!(
            catalogue
                .track(super::super::classic_talk::ROTTIN_WALLCOUNT)
                .ok_or("wall counter")?
                .max,
            6
        );
        assert!(
            expected
                .iter()
                .all(|callback| catalogue.transition(&callback.key).is_some())
        );
        let reset = catalogue
            .transition("oteryn:quest-transition/classic/rottin-wood/start-disrepair")
            .ok_or("wall counter reset")?;
        let validate = |transition| {
            super::super::admission_binding::validate_resolved_world(
                catalogue,
                transition,
                false,
                &registered,
            )
        };
        validate(reset)?;
        // Captured by running the existing NPC case with the base catalogue.
        let prior = [
            213, 240, 38, 86, 164, 115, 74, 12, 93, 92, 21, 227, 246, 158, 57, 59, 158, 103, 133,
            26, 221, 105, 16, 177, 56, 211, 145, 223, 48, 215, 141, 56,
        ];
        assert_eq!(
            super::super::npc_rules::legacy_defaults(catalogue, QUEST, &prior)
                .and_then(|defaults| defaults.get(super::super::classic_talk::ROTTIN_WALLCOUNT)),
            Some(&0),
        );
        let mut alias = reset.clone();
        alias.key.push_str("-alias");
        assert!(validate(&alias).is_err());
        Ok(())
    }
}
