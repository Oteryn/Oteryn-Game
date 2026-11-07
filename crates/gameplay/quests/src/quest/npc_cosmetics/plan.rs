//! Source effects for the current World's six permanent-cosmetic NPCs.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TrackChange {
    pub key: String,
    pub quest: String,
    pub before: i64,
    pub after: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cost {
    pub item: u16,
    pub count: u32,
    pub missing: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grant {
    pub key: &'static str,
    pub outfit: &'static str,
    pub addon: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub source_key: &'static str,
    pub changes: Vec<TrackChange>,
    pub costs: Vec<Cost>,
    pub grant: Option<Grant>,
    pub achievement: Option<&'static str>,
    pub magic_effect: Option<u16>,
}

/// All facts are supplied by the durable owner's locked snapshot. Replies and
/// ephemeral NPC topics do not authorize a cost or grant on their own.
#[allow(
    clippy::too_many_lines,
    reason = "closed source effect table has one arm per acquisition action"
)]
pub fn plan(action: Action, state: &Snapshot) -> Option<Plan> {
    use Action::*;
    let mut result = Plan {
        source_key: "",
        changes: vec![],
        costs: vec![],
        grant: None,
        achievement: None,
        magic_effect: None,
    };
    let citizen_quest = "oteryn:quest.citizen_outfits_quest_on_rook";
    let mut change = |key: &str, quest: &str, after: i64| {
        result.changes.push(TrackChange {
            key: key.into(),
            quest: quest.into(),
            before: value(state, key),
            after,
        })
    };
    match action {
        HairyclesShaman if value(state, APE_CITY) == 18 && value(state, SHAMAN_CLAIMED) != 1 => {
            result.source_key = "oteryn:npc-cosmetic/hairycles/shaman";
            change(SHAMAN_CLAIMED, "oteryn:quest.the_ape_city_quest", 1);
            result.grant = Some(Grant {
                key: "oteryn:appearance-grant/npc/hairycles/shaman/base",
                outfit: SHAMAN,
                addon: 0,
            });
        }
        AmberStart if value(state, BACKPACK) < 1 => {
            result.source_key = "oteryn:npc-cosmetic/amber/start";
            change(BACKPACK, citizen_quest, 1);
            change(BACKPACK_MISSION, citizen_quest, 1);
            change(
                OUTFIT_REF,
                "oteryn:quest.outfit_and_addon_quests",
                value(state, OUTFIT_REF).max(0).checked_add(1)?,
            );
        }
        AmberHandIn if value(state, BACKPACK) == 1 => {
            result.source_key = "oteryn:npc-cosmetic/amber/hand-in";
            change(BACKPACK, citizen_quest, 2);
            change(BACKPACK_MISSION, citizen_quest, 2);
            change(BACKPACK_TIMER, citizen_quest, state.now.checked_add(7200)?);
            result.costs.push(Cost { item: 5878, count: 100,
                missing: "Sorry, but that's not enough leather yet to make one of these backpacks. Would you rather like to buy a normal backpack for 10 gold?" });
        }
        AmberFinish if value(state, BACKPACK) == 2 && value(state, BACKPACK_TIMER) < state.now => {
            result.source_key = "oteryn:npc-cosmetic/amber/finish";
            change(BACKPACK, citizen_quest, 3);
            change(BACKPACK_MISSION, citizen_quest, 4);
            change(
                OUTFIT_REF,
                "oteryn:quest.outfit_and_addon_quests",
                value(state, OUTFIT_REF).checked_sub(1)?.min(0),
            );
            result.grant = Some(Grant {
                key: "oteryn:appearance-grant/npc/amber/citizen/addon1",
                outfit: CITIZEN,
                addon: 1,
            });
        }
        NormaStart => {
            result.source_key = "oteryn:npc-cosmetic/norma/start";
            change(HAT, citizen_quest, 1);
            change(HAT_MISSION, citizen_quest, 1);
        }
        NormaFinish if value(state, HAT) == 1 => {
            result.source_key = "oteryn:npc-cosmetic/norma/finish";
            change(HAT, citizen_quest, 2);
            change(HAT_MISSION, citizen_quest, 0);
            result.costs = vec![
                Cost {
                    item: 3374,
                    count: 1,
                    missing: "Sorry, but I can't see a legion helmet.",
                },
                Cost {
                    item: 5890,
                    count: 100,
                    missing: "Sorry, but you don't enough chicken feathers.",
                },
                Cost {
                    item: 5902,
                    count: 50,
                    missing: "Sorry, but you don't have enough honeycombs.",
                },
            ];
            result.grant = Some(Grant {
                key: "oteryn:appearance-grant/npc/norma/citizen/addon2",
                outfit: CITIZEN,
                addon: 2,
            });
        }
        TomStart if value(state, PAWS) < 1 => {
            result.source_key = "oteryn:npc-cosmetic/tom/start";
            change(PAWS, "oteryn:quest.druid_outfits_quest_on_rook", 1);
        }
        TomFinish if value(state, PAWS) == 1 => {
            result.source_key = "oteryn:npc-cosmetic/tom/finish";
            change(PAWS, "oteryn:quest.druid_outfits_quest_on_rook", 2);
            result.costs = vec![
                Cost {
                    item: 5896,
                    count: 50,
                    missing: "You don't have all items.",
                },
                Cost {
                    item: 5897,
                    count: 50,
                    missing: "You don't have all items.",
                },
            ];
            result.grant = Some(Grant {
                key: "oteryn:appearance-grant/npc/tom/druid/addon1",
                outfit: DRUID,
                addon: 1,
            });
        }
        RiftAddon(addon @ 1..=2)
            if state.bases.contains(RIFT) && !state.addons.contains(&(RIFT.into(), addon)) =>
        {
            result.source_key = if addon == 1 {
                "oteryn:npc-cosmetic/cledwyn/addon1"
            } else {
                "oteryn:npc-cosmetic/cledwyn/addon2"
            };
            result.costs.push(Cost { item: 22516, count: 100, missing: "Sorry, friend, but one good turn deserves another. Bring enough silver tokens and it's a deal." });
            result.grant = Some(Grant {
                key: if addon == 1 {
                    "oteryn:appearance-grant/npc/cledwyn/rift_warrior/addon1"
                } else {
                    "oteryn:appearance-grant/npc/cledwyn/rift_warrior/addon2"
                },
                outfit: RIFT,
                addon,
            });
            if state.addons.contains(&(RIFT.into(), 3 - addon)) {
                result.achievement = Some("oteryn:achievement/rift_warrior");
            }
        }
        StagCouncil
            if value(state, LION) >= 5 && value(state, STAG) >= 13 && value(state, COUNCIL) < 1 =>
        {
            result.source_key = "oteryn:npc-cosmetic/kesar/council";
            change(COUNCIL, "oteryn:quest.the_order_of_the_stag_quest", 1);
            result.grant = Some(Grant {
                key: "oteryn:appearance-grant/npc/kesar_the_younger_day/royal_bounacean_advisor/addon2",
                outfit: ROYAL,
                addon: 2,
            });
            result.achievement = Some("oteryn:achievement/king_s_council");
        }
        _ => return None,
    }
    result.changes.sort_by(|a, b| a.key.cmp(&b.key));
    // Amber82, Norma111, Tom248, Kesar125, Hairycles457. Cledwyn has no FX.
    result.magic_effect = match action {
        AmberFinish | NormaFinish | TomFinish | StagCouncil | HairyclesShaman => Some(13),
        _ => None,
    };
    Some(result)
}
