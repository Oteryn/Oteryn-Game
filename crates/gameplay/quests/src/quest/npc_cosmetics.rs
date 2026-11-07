//! S06: the six outfit/addon NPCs with placements in the current World.
//! Crystal 201189f8 data-global/npc/{amber,norma,tom,cledwyn,kesar_the_younger_day,hairycles}.lua.
use std::collections::{BTreeMap, BTreeSet};
mod plan;
pub mod source_program;
mod source_quest_bindings;
pub use plan::{Cost, Grant, Plan, TrackChange, plan};

pub const PROVENANCE: &str = "crystal-201189f-npc-cosmetics-v1";
pub const CITIZEN: &str = "oteryn:content.outfit.citizen_outfits";
pub const DRUID: &str = "oteryn:content.outfit.druid_outfits";
pub const RIFT: &str = "oteryn:content.outfit.rift_warrior_outfits";
pub const ROYAL: &str = "oteryn:content.outfit.royal_bounacean_advisor_outfits";
pub const BACKPACK: &str =
    "oteryn:quest-progress/quest/u7_8/citizen_outfits_rook/addon_backpack_rook";
pub const BACKPACK_TIMER: &str =
    "oteryn:quest-progress/quest/u7_8/citizen_outfits_rook/addon_backpack_rook_timer";
pub const BACKPACK_MISSION: &str =
    "oteryn:quest-progress/quest/u7_8/citizen_outfits_rook/mission_backpack_rook";
pub const HAT: &str = "oteryn:quest-progress/quest/u7_8/citizen_outfits_rook/addon_hat_rook";
pub const HAT_MISSION: &str =
    "oteryn:quest-progress/quest/u7_8/citizen_outfits_rook/mission_hat_rook";
pub const PAWS: &str = "oteryn:quest-progress/quest/u7_8/druid_outfits_rook/druid_body_addon_rook";
pub const OUTFIT_REF: &str = "oteryn:quest-progress/outfit_quest/ref";
pub const LION: &str =
    "oteryn:quest-progress/crystalserver/quest/u12_40/the_order_of_the_lion/kesar_mission";
pub const STAG: &str =
    "oteryn:quest-progress/crystalserver/quest/u15_20/the_order_of_the_stag/questline";
pub const COUNCIL: &str =
    "oteryn:quest-progress/crystalserver/quest/u15_20/the_order_of_the_stag/kings_council";
pub const SHAMAN: &str = "oteryn:content.outfit.shaman_outfits";
pub const APE_CITY: &str = "oteryn:quest-progress/quest/u7_6/the_ape_city/questline";
pub const SHAMAN_CLAIMED: &str = "oteryn:quest-progress/quest/u7_6/the_ape_city/shaman_outfit";
pub const TRACKS: [&str; 12] = [
    APE_CITY,
    SHAMAN_CLAIMED,
    BACKPACK,
    BACKPACK_TIMER,
    BACKPACK_MISSION,
    HAT,
    HAT_MISSION,
    PAWS,
    OUTFIT_REF,
    LION,
    STAG,
    COUNCIL,
];

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub tracks: BTreeMap<String, i64>,
    pub bases: BTreeSet<String>,
    pub addons: BTreeSet<(String, u8)>,
    pub now: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Action {
    AmberStart,
    AmberHandIn,
    AmberFinish,
    NormaStart,
    NormaFinish,
    TomStart,
    TomFinish,
    RiftAddon(u8),
    StagCouncil,
    HairyclesShaman,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    pub topic: u8,
    pub action: Option<Action>,
    pub reply: &'static str,
}

pub fn supported(npc: &str) -> bool {
    if source_program::supported(npc) {
        return true;
    }
    matches!(
        npc,
        "oteryn:npc.amber"
            | "oteryn:npc.norma"
            | "oteryn:npc.tom"
            | "oteryn:npc.cledwyn"
            | "oteryn:npc.kesar_the_younger_day"
            | "oteryn:npc.hairycles"
    )
}

/// The acquisition slice leaves ordinary greetings, shops and other NPC services
/// with their existing owners.
pub fn recognizes(npc: &str, text: &str, topic: u16) -> bool {
    use super::classic_talk::contains as has;
    if source_program::embedded().is_ok_and(|program| program.recognizes(npc, text, topic, has)) {
        return true;
    }
    let keywords: &[&str] = match npc {
        "oteryn:npc.hairycles" => &["outfit", "shamanic"],
        "oteryn:npc.amber" => &["addon", "outfit", "backpack", "minotaur", "leather"],
        "oteryn:npc.norma" => &["addon", "outfit", "hat"],
        "oteryn:npc.tom" => &["addon", "paws"],
        "oteryn:npc.cledwyn" => &["addon", "first", "second"],
        "oteryn:npc.kesar_the_younger_day" => &["help"],
        _ => return false,
    };
    keywords.iter().any(|keyword| has(text, keyword))
        || (topic != 0 && (has(text, "yes") || has(text, "no")))
}

fn value(state: &Snapshot, track: &str) -> i64 {
    state.tracks.get(track).copied().unwrap_or(-1)
}

fn reply(topic: u8, action: Option<Action>, text: &'static str) -> Choice {
    Choice {
        topic,
        action,
        reply: text,
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "six closed donor NPC branches retain source order"
)]
pub fn choose(npc: &str, text: &str, topic: u8, state: &Snapshot) -> Option<Choice> {
    use super::classic_talk::contains as has;
    use Action::*;
    Some(match npc {
        "oteryn:npc.hairycles" => {
            if has(text, "outfit") || has(text, "shamanic") {
                if value(state, APE_CITY) != 18 {
                    reply(
                        0,
                        None,
                        "You not have finished journey for wisdom yet, young human.",
                    )
                } else if value(state, SHAMAN_CLAIMED) == 1 {
                    reply(
                        0,
                        None,
                        "You already are shaman and doctor. Me proud of you.",
                    )
                } else {
                    reply(
                        18,
                        None,
                        "Me truly proud of you, friend. You learn many about plants, charms and ape people. Me want grant you shamanic power now. You ready?",
                    )
                }
            } else if topic == 18
                && has(text, "yes")
                && value(state, APE_CITY) == 18
                && value(state, SHAMAN_CLAIMED) != 1
            {
                reply(
                    0,
                    Some(HairyclesShaman),
                    "Friend of the ape people! Take my gift and become me apprentice! Here is shaman clothing for you!",
                )
            } else if topic == 18 && has(text, "no") {
                reply(0, None, "Come back if change mind.")
            } else {
                return None;
            }
        }
        "oteryn:npc.amber" => {
            let progress = value(state, BACKPACK);
            if has(text, "addon")
                || has(text, "outfit")
                || (progress == 1 && has(text, "leather"))
                || ((progress == 1 || progress == 2) && has(text, "backpack"))
            {
                match progress {
                    i64::MIN..=0 => reply(
                        1,
                        None,
                        "Ah, you noticed my new accessory? Sorry, this one is not for sale. It's handmade from rare minotaur leather.",
                    ),
                    1 => reply(
                        3,
                        None,
                        "Ah, right, almost forgot about the backpack! Have you brought me 100 pieces of minotaur leather as requested?",
                    ),
                    2 if value(state, BACKPACK_TIMER) < state.now => reply(
                        0,
                        Some(AmberFinish),
                        "Just in time! Your backpack is finished. Here you go, I hope you like it.",
                    ),
                    2 => reply(
                        0,
                        None,
                        "Uh... I didn't expect you to return that early. Sorry, but I'm not finished yet with your backpack. I'm doing the best I can, promised.",
                    ),
                    _ => reply(
                        0,
                        None,
                        "Sorry, but I can only make one backpack per person, else I'd have to close my shop and open a leather manufactory.",
                    ),
                }
            } else if topic == 1
                && (has(text, "backpack") || has(text, "minotaur") || has(text, "leather"))
            {
                reply(
                    2,
                    None,
                    "Well, if you really like this backpack, I could make one for you, but minotaur leather is hard to come by these days. Are you willing to put some work into this?",
                )
            } else if has(text, "yes") && topic == 2 && progress < 1 {
                reply(
                    0,
                    Some(AmberStart),
                    "Alright then, if you bring me 100 pieces of fine minotaur leather I will see what I can do for you. You probably have to kill really many minotaurs though... so good luck!",
                )
            } else if has(text, "yes") && topic == 3 && progress == 1 {
                reply(
                    0,
                    Some(AmberHandIn),
                    "Great! Alright, I need a while to finish this backpack for you. Come ask me later, okay?",
                )
            } else if topic == 2 {
                reply(
                    0,
                    None,
                    "Sorry, but I don't run a welfare office, you know... no pain, no gain.",
                )
            } else if topic == 3 {
                reply(
                    0,
                    None,
                    "I know, it's quite some work... don't lose heart, just keep killing minotaurs and you'll eventually get lucky. Would you rather like to buy a normal backpack for 10 gold?",
                )
            } else {
                return None;
            }
        }
        "oteryn:npc.norma" => {
            if has(text, "addon") || has(text, "outfit") || has(text, "hat") {
                if value(state, HAT) == 1 {
                    reply(
                        2,
                        None,
                        "Oh, you're back already? Did you bring a legion helmet, 100 chicken feathers and 50 honeycombs?",
                    )
                } else if value(state, HAT_MISSION) < 1 {
                    reply(
                        1,
                        None,
                        "Pretty, isn't it? I made it myself, but I could teach you how to do that if you like. What do you say?",
                    )
                } else {
                    return None;
                }
            } else if has(text, "yes") && topic == 1 {
                reply(
                    0,
                    Some(NormaStart),
                    "Okay, here we go, listen closely! I need a few things... a basic hat of course, maybe a legion helmet would do. Then about 100 chicken feathers... and 50 honeycombs as glue.That's it, come back to me once you gathered it!!",
                )
            } else if has(text, "yes") && topic == 2 && value(state, HAT) == 1 {
                reply(
                    0,
                    Some(NormaFinish),
                    "Great job! That must have taken a lot of work. Okay, you put it like this... then glue like this... here!",
                )
            } else if topic == 1 {
                reply(
                    0,
                    None,
                    "Aw, I guess you don't like feather hats. No big deal.",
                )
            } else if topic == 2 {
                reply(0, None, "Maybe another time.")
            } else {
                return None;
            }
        }
        "oteryn:npc.tom" => {
            if has(text, "addon") && value(state, PAWS) < 1 {
                reply(
                    0,
                    Some(TomStart),
                    "Would you like to wear bear paws like I do? No problem, just bring me 50 bear paws and 50 wolf paws and I'll fit them on.",
                )
            } else if (has(text, "paws") || has(text, "bear paws")) && value(state, PAWS) == 1 {
                reply(1, None, "Have you brought 50 bear paws and 50 wolf paws?")
            } else if has(text, "yes") && topic == 1 && value(state, PAWS) == 1 {
                reply(
                    0,
                    Some(TomFinish),
                    "Excellent! Like promised, here are your bear paws. ",
                )
            } else {
                return None;
            }
        }
        "oteryn:npc.cledwyn" => {
            if has(text, "addon") {
                if state.bases.contains(RIFT) {
                    reply(
                        3,
                        None,
                        "Ah, very good. Now choose your addon: {first} or {second}.",
                    )
                } else {
                    reply(
                        0,
                        None,
                        "Sorry, friend, but one good turn deserves another. You need to obtain the rift warrior outfit first.",
                    )
                }
            } else if topic == 3
                && state.bases.contains(RIFT)
                && (text.eq_ignore_ascii_case("first") || text.eq_ignore_ascii_case("second"))
            {
                let addon = if text.eq_ignore_ascii_case("first") {
                    1
                } else {
                    2
                };
                if state.addons.contains(&(RIFT.to_owned(), addon)) {
                    reply(
                        0,
                        None,
                        if addon == 1 {
                            "Sorry, friend, you already have the first Rift Warrior addon."
                        } else {
                            "Sorry, friend, you already have the second Rift Warrior addon."
                        },
                    )
                } else {
                    reply(
                        0,
                        Some(RiftAddon(addon)),
                        if addon == 1 {
                            "Ah, excellent. Obtain the first addon for your rift warrior outfit."
                        } else {
                            "Ah, excellent. Obtain the second addon for your rift warrior outfit."
                        },
                    )
                }
            } else {
                return None;
            }
        }
        "oteryn:npc.kesar_the_younger_day"
            if has(text, "help")
                && value(state, LION) >= 5
                && value(state, STAG) >= 13
                && value(state, COUNCIL) < 1 =>
        {
            reply(
                0,
                Some(StagCouncil),
                "You have done Bounac and the Etzels a service that will not be forgotten. From this day on, you have a permanent place at my council.",
            )
        }
        "oteryn:npc.kesar_the_younger_day"
            if has(text, "help") && value(state, LION) >= 5 && value(state, STAG) >= 13 =>
        {
            reply(0, None, "Your place at my council is secure, advisor.")
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srv7_npc_costs_and_backpack_deadline_equal_the_donor_exchange() {
        let mut state = Snapshot {
            now: 1000,
            ..Snapshot::default()
        };
        state.tracks.insert(BACKPACK.into(), 1);
        let delivered = plan(Action::AmberHandIn, &state);
        assert_eq!(
            delivered.as_ref().map(|p| p
                .costs
                .iter()
                .map(|c| (c.item, c.count))
                .collect::<Vec<_>>()),
            Some(vec![(5878, 100)])
        );
        assert_eq!(
            delivered
                .and_then(|p| p.changes.into_iter().find(|c| c.key == BACKPACK_TIMER))
                .map(|c| c.after),
            Some(8200)
        );
        state.tracks.insert(HAT.into(), 1);
        assert_eq!(
            plan(Action::NormaFinish, &state).map(|p| p
                .costs
                .into_iter()
                .map(|c| (c.item, c.count))
                .collect::<Vec<_>>()),
            Some(vec![(3374, 1), (5890, 100), (5902, 50)])
        );
        state.tracks.insert(PAWS.into(), 1);
        assert_eq!(
            plan(Action::TomFinish, &state).map(|p| p
                .costs
                .into_iter()
                .map(|c| (c.item, c.count))
                .collect::<Vec<_>>()),
            Some(vec![(5896, 50), (5897, 50)])
        );
        assert!(plan(Action::RiftAddon(3), &state).is_none());
    }

    #[test]
    fn srv7_npc_norma_hat_and_tom_paws_have_real_confirmation_actions() {
        let state = Snapshot::default();
        assert_eq!(
            choose("oteryn:npc.norma", "yes", 1, &state).and_then(|c| c.action),
            Some(Action::NormaStart)
        );
        assert_eq!(
            choose("oteryn:npc.tom", "addon", 0, &state).and_then(|c| c.action),
            Some(Action::TomStart)
        );
    }

    #[test]
    fn srv7_npc_amber_backpack_waits_strictly_past_two_hour_deadline() {
        let mut state = Snapshot {
            now: 1000,
            ..Snapshot::default()
        };
        state.tracks.insert(BACKPACK.into(), 2);
        state.tracks.insert(BACKPACK_TIMER.into(), 1000);
        assert_eq!(
            choose("oteryn:npc.amber", "backpack", 0, &state).and_then(|c| c.action),
            None
        );
        state.now += 1;
        assert_eq!(
            choose("oteryn:npc.amber", "backpack", 0, &state).and_then(|c| c.action),
            Some(Action::AmberFinish)
        );
    }

    #[test]
    fn srv7_npc_cledwyn_addons_require_base_and_preserve_independent_addon_order() {
        let mut state = Snapshot::default();
        assert_eq!(
            choose("oteryn:npc.cledwyn", "second", 3, &state).and_then(|c| c.action),
            None
        );
        state.bases.insert(RIFT.into());
        assert_eq!(
            choose("oteryn:npc.cledwyn", "second", 3, &state).and_then(|c| c.action),
            Some(Action::RiftAddon(2))
        );
    }

    #[test]
    fn srv7_npc_kesar_council_requires_both_predecessor_quests_and_is_once() {
        let mut state = Snapshot::default();
        state.tracks.insert(LION.into(), 5);
        state.tracks.insert(STAG.into(), 13);
        assert_eq!(
            choose("oteryn:npc.kesar_the_younger_day", "help", 0, &state).and_then(|c| c.action),
            Some(Action::StagCouncil)
        );
        state.tracks.insert(COUNCIL.into(), 1);
        assert_eq!(
            choose("oteryn:npc.kesar_the_younger_day", "help", 0, &state).and_then(|c| c.action),
            None
        );
    }
}

#[cfg(test)]
mod srv7_hairycles_source_tests {
    use super::*;
    const NPC: &str = "oteryn:npc.hairycles";
    const STAGE: &str = "oteryn:quest-progress/quest/u7_6/the_ape_city/questline";
    const CLAIMED: &str = "oteryn:quest-progress/quest/u7_6/the_ape_city/shaman_outfit";
    #[test]
    fn srv7_hairycles_shaman_base_requires_exact_completed_quest_and_unclaimed_flag()
    -> Result<(), &'static str> {
        let mut state = Snapshot::default();
        for value in [-1, 0, 17, 19] {
            state.tracks.insert(STAGE.into(), value);
            assert!(
                choose(NPC, "yes", 18, &state)
                    .and_then(|c| c.action)
                    .is_none()
            );
        }
        state.tracks.insert(STAGE.into(), 18);
        assert!(!state.tracks.contains_key(CLAIMED));
        let absent = choose(NPC, "yes", 18, &state)
            .and_then(|choice| choice.action)
            .ok_or("missing flag must be unclaimed")?;
        let absent = plan(absent, &state).ok_or("missing flag source plan")?;
        assert_eq!(
            absent
                .changes
                .first()
                .ok_or("missing flag transition")?
                .before,
            -1
        );
        state.tracks.insert(CLAIMED.into(), 0);
        assert!(
            recognizes(NPC, "shamanic", 0),
            "new reachable Hairycles owns its actual source keyword"
        );
        let choice = choose(NPC, "outfit", 0, &state).ok_or("source outfit prompt")?;
        assert_eq!(choice.topic, 18);
        let action = choose(NPC, "yes", 18, &state)
            .and_then(|c| c.action)
            .ok_or("source completed quest must grant Shaman base")?;
        let result = plan(action, &state).ok_or("source plan")?;
        let grant = result.grant.ok_or("Shaman base grant")?;
        assert_eq!(grant.outfit, "oteryn:content.outfit.shaman_outfits");
        assert_eq!(grant.addon, 0);
        assert!(result.costs.is_empty());
        assert_eq!(result.changes.len(), 1);
        assert_eq!(result.changes[0].key, CLAIMED);
        assert_eq!(result.changes[0].after, 1);
        assert_eq!(result.changes[0].before, 0);
        state.tracks.insert(CLAIMED.into(), 1);
        assert!(
            choose(NPC, "yes", 18, &state)
                .and_then(|c| c.action)
                .is_none()
        );
        Ok(())
    }
}

#[cfg(test)]
mod srv7_town_source_population_tests {
    #[test]
    fn srv7_current_tibia_town_outfit_sources_have_a_production_npc_rule() {
        let missing: Vec<_> = [
            "hanna", "lubo", "elane", "lynda", "muriel", "sam", "legola", "irmana", "hugo",
            "sandra", "zoltan",
        ]
        .into_iter()
        .filter(|name| !super::supported(&format!("oteryn:npc.{name}")))
        .collect();
        assert!(
            missing.is_empty(),
            "ordinary reachable donor outfit acquisition NPC rules missing: {missing:?}"
        );
    }
}

#[cfg(test)]
mod srv7_foreign_owner_routing_tests {
    use super::*;
    #[test]
    fn srv7_foreign_final_reward_graphs_preserve_retained_quest_conversations() -> Result<(), String>
    {
        let program = source_program::embedded().map_err(|error| format!("{error:?}"))?;
        let keys = [
            "avar_tar",
            "dedoras",
            "the_first_dragon",
            "henricus",
            "hjaern",
            "jack_springer",
            "yalahari",
            "palimuth",
            "navigator",
            "bozo",
        ];
        let unsupported: Vec<_> = keys
            .iter()
            .filter(|key| !supported(&format!("oteryn:npc.{key}")))
            .copied()
            .collect();
        assert!(
            unsupported.is_empty(),
            "reviewed final branches lack composed owner declaration: {unsupported:?}"
        );
        for key in keys {
            let key = format!("oteryn:npc.{key}");
            let npc = program
                .npc(&key)
                .ok_or_else(|| format!("source NPC missing: {key}"))?;
            // Keep the original whole-graph delegation guarantee for even one
            // unqualified foreign expression, instead of a blanket NPC allowlist.
            let mut changed: serde_json::Value = serde_json::from_str(source_program::EMBEDDED)
                .map_err(|error| error.to_string())?;
            let node = changed["npcs"]
                .as_array_mut()
                .ok_or("NPC array")?
                .iter_mut()
                .find(|node| node["npc"].as_str() == Some(key.as_str()))
                .ok_or("actual NPC row")?;
            let mut altered = false;
            for transition in node["transitions"]
                .as_array_mut()
                .ok_or("transition array")?
            {
                for effect in transition["effects"].as_array_mut().ok_or("effect array")? {
                    if effect["kind"] == "set_track"
                        && effect["storage"].as_str().is_some_and(|storage| {
                            !npc.owned_tracks
                                .iter()
                                .any(|owned| owned.as_str() == storage)
                        })
                    {
                        effect["value"] = serde_json::json!({"op":"literal","value":2147483647});
                        altered = true;
                        break;
                    }
                }
                if altered {
                    break;
                }
            }
            assert!(altered, "actual foreign final expression missing for {key}");
            let changed = source_program::Program::parse(
                &serde_json::to_vec(&changed).map_err(|error| error.to_string())?,
            )
            .map_err(|error| format!("bounded altered programme: {error:?}"))?;
            assert!(
                !changed.supports(&key),
                "an unqualified foreign expression gained source routing"
            );
            for transition in &npc.transitions {
                let mut texts = transition.keywords.clone();
                texts.extend(["yes".into(), "no".into(), "outfit".into(), "addon".into()]);
                for topic in [
                    0,
                    transition.topic_before.unwrap_or(0),
                    transition.topic_after,
                ] {
                    for text in &texts {
                        assert!(
                            !changed.recognizes(
                                &key,
                                text,
                                topic,
                                crate::quest::classic_talk::contains
                            ),
                            "uncomposed graph stole retained quest topic: {key}/{}/{topic}/{text}",
                            transition.id
                        );
                    }
                }
            }
        }
        assert!(supported("oteryn:npc.hanna"));
        assert!(recognizes("oteryn:npc.hanna", "hat", 0));
        Ok(())
    }
}
