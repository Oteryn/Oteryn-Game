//! Fixed contribution recipes selected by NPC dialogue and committed by the Quest owner.
//! Coal12600 and medicine12517 are retained Crystal item identities. The corpse
//! route conservatively accepts only aged Swamp Troll4178; its donor monster definition
//! names that corpse. Dialogue wording and donation sizes are authored policy.

pub const COAL_ONE: &str = "oteryn:quest-transition/world-contribution/coal/1";
pub const COAL_HUNDRED: &str = "oteryn:quest-transition/world-contribution/coal/100";
pub const MEDICINE_ONE: &str = "oteryn:quest-transition/world-contribution/medicine/1";
pub const CORPSE_ONE: &str = "oteryn:quest-transition/world-contribution/corpses/1";
pub const CORPSE_USE: &str = "oteryn:quest-callback/world-contribution/corpses";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Coal,
    Medicine,
    Corpses,
    Votes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Recipe {
    pub kind: Kind,
    pub amount: u32,
    pub item_key: Option<&'static str>,
    pub points_track: Option<&'static str>,
}

pub fn recipe(key: &str) -> Option<Recipe> {
    if super::rathleton::ballot_for_transition(key).is_some() {
        return Some(Recipe {
            kind: Kind::Votes,
            amount: 1,
            item_key: None,
            points_track: Some(super::rathleton::POINTS_TRACK),
        });
    }
    let (kind, amount, item) = match key {
        COAL_ONE => (Kind::Coal, 1, "oteryn:item.tibia.i12600"),
        COAL_HUNDRED => (Kind::Coal, 100, "oteryn:item.tibia.i12600"),
        MEDICINE_ONE => (Kind::Medicine, 1, "oteryn:item.tibia.i12517"),
        CORPSE_ONE | CORPSE_USE => (Kind::Corpses, 1, "oteryn:item.tibia.i4178"),
        _ => return None,
    };
    Some(Recipe {
        kind,
        amount,
        item_key: Some(item),
        points_track: None,
    })
}

/// Authored fallback: explicitly use the donor's dirty lake water to deposit one
/// owned Swamp Troll corpse. Yawno accepts the same deposit while the lake is clean.
/// This does not infer a fishing-rod offering: custody consumption is exclusively
/// the fixed recipe above, so the world callback must not consume a second input.
pub(crate) fn corpse_callback() -> super::world_callbacks::Callback {
    use super::world_callbacks::{Callback, Selector, Storage, Trigger};
    Callback {
        key: CORPSE_USE.into(),
        quest: "oteryn:quest.authored.twisted_waters_world_change".into(),
        trigger: Trigger::Use,
        selector: Selector { position:None, unique:None, action:None, appearance:Some(12561) },
        lever:None,
        reversible:false,
        passages:Vec::new(),
        teleport:None,
        storage:vec![Storage {track:"oteryn:quest-progress/world-contribution/corpse_deposit_used".into(), minimum:-1, maximum:1, set:1, preserve:false}],
        minimum_level:None,
        experience:None,
        monk_reward:None,
        offerings:Vec::new(),
        item_on_target:None,
        return_on_failure:true,
        failure_message:Some("You need an owned Swamp Troll corpse to deposit here.".into()),
        success_message:Some("The lake receives your corpse contribution.".into()),
        native_rule:None,
        effects: Default::default(),
        sources:vec![
            "crystal:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/scripts/actions/other/fishing.lua:43-49".into(),
            "https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/scripts/actions/other/fishing.lua#L43-L49".into(),
            "policy:WCONTRIB-owner-fallback-explicit-water-use-deposits-owned-corpse".into(),
        ],
    }
}

pub struct Dialogue {
    pub topic: u16,
    pub transition: Option<&'static str>,
    pub reply: &'static str,
}

/// Named NPC identity and a private topic jointly bind every confirmation.
/// Ordinary Ottokar medicine/exchange keywords continue through their existing route.
pub fn dialogue(npc: &str, message: &str, topic: u16) -> Option<Dialogue> {
    if let Some(choice) = super::horse_station::dialogue(npc, message, topic) {
        return Some(choice);
    }
    if let Some(choice) = super::rathleton::dialogue(npc, message, topic) {
        return Some(choice);
    }
    let text = message.trim().to_ascii_lowercase();
    let (first, second) = match npc {
        "oteryn:npc.junkar_mines" | "oteryn:npc.junkar_robsons" | "oteryn:npc.junkar_thais" => {
            (1, 2)
        }
        "oteryn:npc.ottokar" => (3, 3),
        "oteryn:npc.yawno" => (5, 5),
        _ => return None,
    };
    let offer = match (first, text.as_str()) {
        (1, "coal" | "donate coal") => {
            Some((first, "Contribute one coal to the shared steamship supply?"))
        }
        (1, "coal 100" | "donate coal 100") => Some((
            second,
            "Contribute 100 coal to the shared steamship supply?",
        )),
        (3, "epidemic" | "donate medicine") => Some((
            first,
            "Donate one medicine pouch to the shared epidemic response?",
        )),
        (5, "corpse" | "donate corpse") => Some((
            first,
            "Deposit one aged Swamp Troll corpse in the shared lake?",
        )),
        _ => None,
    };
    if let Some((topic, reply)) = offer {
        return Some(Dialogue {
            topic,
            transition: None,
            reply,
        });
    }
    if topic != first && topic != second {
        return None;
    }
    if text == "no" {
        return Some(Dialogue {
            topic: 0,
            transition: None,
            reply: "Then keep it.",
        });
    }
    if text != "yes" {
        return None;
    }
    let transition = match topic {
        1 => COAL_ONE,
        2 => COAL_HUNDRED,
        3 => MEDICINE_ONE,
        5 => CORPSE_ONE,
        _ => return None,
    };
    Some(Dialogue {
        topic: 0,
        transition: Some(transition),
        reply: "Your contribution is recorded for the next server save.",
    })
}

pub(super) fn extend(
    catalogue: super::QuestStateCatalogue,
) -> Result<super::QuestStateCatalogue, String> {
    use super::{QuestComparison, QuestEffect, QuestEffectKind, QuestTrack, QuestTransition};
    let mut tracks: Vec<_> = catalogue.tracks.values().cloned().collect();
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    for (quest, suffix, keys) in [
        (
            "oteryn:quest.steamship_world_change",
            "coal_donated",
            &[COAL_ONE, COAL_HUNDRED][..],
        ),
        (
            "oteryn:quest.swamp_fever_world_change",
            "medicine_donated",
            &[MEDICINE_ONE][..],
        ),
        (
            "oteryn:quest.authored.twisted_waters_world_change",
            "corpses_donated",
            &[CORPSE_ONE][..],
        ),
    ] {
        let track = format!("oteryn:quest-progress/world-contribution/{suffix}");
        tracks.push(QuestTrack {
            key: track.clone(),
            quest: quest.into(),
            initial: 0,
            min: 0,
            max: i64::MAX,
        });
        for key in keys {
            let amount = recipe(key).ok_or("missing contribution recipe")?.amount;
            transitions.push(QuestTransition {
                key: (*key).into(),
                quest: quest.into(),
                effects: vec![QuestEffect {
                    track: track.clone(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Add(i64::from(amount)),
                }],
                completes: false,
                experience: None,
            });
        }
    }
    super::QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_contribution_recipes_bind_resource_amount_and_custody() {
        assert_eq!(
            recipe(COAL_HUNDRED).map(|r| (r.kind, r.amount, r.item_key)),
            Some((Kind::Coal, 100, Some("oteryn:item.tibia.i12600")))
        );
        assert_eq!(
            recipe(MEDICINE_ONE).map(|r| (r.kind, r.amount, r.item_key)),
            Some((Kind::Medicine, 1, Some("oteryn:item.tibia.i12517")))
        );
        assert_eq!(
            recipe(CORPSE_ONE).map(|r| (r.kind, r.amount)),
            Some((Kind::Corpses, 1))
        );
        assert!(recipe("oteryn:quest-transition/world-contribution/coal/101").is_none());
    }

    #[test]
    fn world_contribution_dialogue_requires_explicit_donation_and_confirmation() {
        assert!(dialogue("oteryn:npc.ottokar", "medicine", 0).is_none());
        assert_eq!(
            dialogue("oteryn:npc.ottokar", "donate medicine", 0).map(|c| c.topic),
            Some(3)
        );
        assert_eq!(
            dialogue("oteryn:npc.junkar_mines", "coal 100", 0).map(|c| c.topic),
            Some(2)
        );
        assert_eq!(
            dialogue("oteryn:npc.junkar_mines", "yes", 2).and_then(|c| c.transition),
            Some(COAL_HUNDRED)
        );
        assert!(dialogue("oteryn:npc.junkar_mines", "yes", 3).is_none());
        assert!(dialogue("oteryn:npc.ottokar", "yes", 0).is_none());
    }

    #[test]
    fn world_contribution_catalogue_mounts_every_recipe_as_a_repeatable_transition()
    -> Result<(), String> {
        let lowered = super::super::loader::load_embedded_quest_state("world-contribution-test")
            .map_err(|error| format!("{error:?}"))?;
        let catalogue = lowered.catalogue();
        for key in [COAL_ONE, COAL_HUNDRED, MEDICINE_ONE, CORPSE_ONE] {
            let rule = recipe(key).ok_or("recipe")?;
            let transition = catalogue.transition(key).ok_or("mounted transition")?;
            assert!(!transition.completes);
            assert_eq!(transition.effects.len(), 1);
            assert_eq!(
                transition.effects[0].effect,
                super::super::QuestEffectKind::Add(i64::from(rule.amount))
            );
            assert_eq!(
                transition.effects[0].from,
                super::super::QuestComparison::Any
            );
        }
        Ok(())
    }

    #[test]
    fn world_contribution_corpse_use_mounts_water_selector_and_single_custody_debit()
    -> Result<(), String> {
        let callbacks = super::super::world_callbacks::load()?;
        let callback = callbacks
            .iter()
            .find(|callback| callback.key == CORPSE_USE)
            .ok_or("corpse use callback")?;
        assert_eq!(callback.selector.appearance, Some(12561));
        assert!(callback.item_on_target.is_none() && callback.offerings.is_empty());
        assert_eq!(
            recipe(&callback.key).map(|recipe| (recipe.kind, recipe.amount)),
            Some((Kind::Corpses, 1))
        );
        let lowered = super::super::loader::load_embedded_quest_state("corpse-use-test")
            .map_err(|error| format!("{error:?}"))?;
        assert!(lowered.catalogue().transition(CORPSE_USE).is_some());
        Ok(())
    }
}
