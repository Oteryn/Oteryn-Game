//! Source selectors and quest-state bindings for the New Frontier arena mechanics.
use super::world_callbacks::{Callback, Position};

pub const CUROS: &str = "oteryn:quest-callback/nfront2-curos-survival";
pub const CUROS_ARENA: Position = Position {
    x: 33154,
    y: 31415,
    floor: -7,
};
pub const CUROS_RADIUS: u16 = 6;
pub const CUROS_SURVIVAL_US: u64 = 120_000_000;

pub(super) fn curos_callback() -> Result<Callback, String> {
    let value = serde_json::json!({"schema":1,"callbacks":[{
        "key":CUROS,
        "quest":"oteryn:quest.the_new_frontier_quest",
        "trigger":"step_in",
        "selector":{"position":{"x":33146,"y":31413,"floor":-6},"ordinal":0},
        "lever":null,"reversible":false,"requires_world_admission":true,"passages":[],
        "teleport":{"x":33145,"y":31419,"floor":-7},
        "storage":[
            {"track":"oteryn:quest-progress/quest/u8_54/the_new_frontier/questline","minimum":17,"maximum":17,"set":18},
            {"track":"oteryn:quest-progress/quest/u8_54/the_new_frontier/mission06","minimum":0,"maximum":5,"set":3}
        ],
        "offerings":[],"return_on_failure":true,
        "failure_message":"You don't have access to this area.",
        "effects":{"teleport_effect":11,"player_speech":"You have braved the tiral of the Mooh'tah master."},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_new_frontier/movement_minotaur_boss.lua:1-41"]
    }]});
    super::world_callbacks::parse(&value.to_string())?
        .pop()
        .ok_or_else(|| "missing Curos survival callback".into())
}

pub const MORTAL_COMBAT: &str = "oteryn:quest-callback/nfront2-mortal-combat";
pub const TIRECZ: &str = "oteryn:creature.tirecz";
pub const TIRECZ_VICTORY: &str = "oteryn:quest-transition/nfront2/tirecz-victory";
pub const QUEST: &str = "oteryn:quest.the_new_frontier_quest";
pub const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/questline";
pub const MISSION09: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/mission09_1";

pub const MORTAL_WAVE_INTERVAL_US: u64 = 90_000_000;
pub const MORTAL_TIMEOUT_US: u64 = 1_800_000_000;
pub const MORTAL_WAVES: [&[&str]; 7] = [
    &[
        "oteryn:creature.baron_brute",
        "oteryn:creature.the_axeorcist",
    ],
    &["oteryn:creature.menace", "oteryn:creature.fatality"],
    &["oteryn:creature.incineron", "oteryn:creature.coldheart"],
    &["oteryn:creature.dreadwing", "oteryn:creature.doomhowl"],
    &["oteryn:creature.haunter", "oteryn:creature.the_dreadorian"],
    &["oteryn:creature.rocko", "oteryn:creature.tremorak"],
    &[TIRECZ],
];
pub const MORTAL_ROOM: [i32; 5] = [33053, 31024, 33073, 31044, -3];
pub const MORTAL_REWARD_EXIT: Position = Position {
    x: 33062,
    y: 31029,
    floor: -7,
};
pub const MORTAL_TIMEOUT_EXIT: Position = Position {
    x: 33049,
    y: 31017,
    floor: -2,
};

pub(super) fn mortal_combat_callback() -> Result<Callback, String> {
    let value = serde_json::json!({"schema":1,"callbacks":[{
        "key":MORTAL_COMBAT,"quest":QUEST,"trigger":"use",
        "selector":{
            "action":30003,
            "native":{"position":{"x":33079,"y":31014,"floor":-2},"ordinal":2,"appearance":2773}
        },
        "lever":null,"reversible":false,"passages":[],"teleport":null,
        "storage":[],"offerings":[],"return_on_failure":false,
        "effects":{"encounter":{
            "minimum_level":0,
            "players":[
                {"from":{"x":33080,"y":31014,"floor":-2},"to":{"x":33059,"y":31032,"floor":-3}},
                {"from":{"x":33081,"y":31014,"floor":-2},"to":{"x":33057,"y":31034,"floor":-3}}
            ],
            "monsters":[],
            "room_min":{"x":33053,"y":31024,"floor":-3},
            "room_max":{"x":33073,"y":31044,"floor":-3},
            "phase":"mortal_combat"
        }},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_new_frontier/action_arena.lua:1-89",
                   "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_new_frontier/creaturescripts_tirecz_kill.lua:19-38"]
    }]});
    super::world_callbacks::parse(&value.to_string())?
        .pop()
        .ok_or_else(|| "missing Mortal Combat callback".into())
}

/// Both arena transitions add to tracks already owned by the imported quest.
pub(super) fn prior_definition(
    catalogue: &super::QuestStateCatalogue,
) -> super::NpcDefinitionPredecessor {
    let transitions = catalogue
        .transitions
        .iter()
        .filter(|(key, transition)| {
            transition.quest == QUEST && !matches!(key.as_str(), CUROS | TIRECZ_VICTORY)
        })
        .map(|(key, transition)| (key.clone(), transition.clone()))
        .collect();
    super::NpcDefinitionPredecessor {
        hash: super::definition_hash(QUEST, &catalogue.tracks, &transitions),
        defaults: catalogue
            .tracks
            .values()
            .filter(|track| track.quest == QUEST)
            .map(|track| (track.key.clone(), track.initial))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nfront2_registrations_load_with_existing_quest_tracks() -> Result<(), String> {
        let loaded = crate::quest::loader::load_embedded_quest_state("content-1")
            .map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();
        let callbacks = crate::quest::world_callbacks::load_for(catalogue)?;
        assert!(callbacks.iter().any(|callback| callback.key == CUROS));
        assert!(
            callbacks
                .iter()
                .any(|callback| callback.key == MORTAL_COMBAT)
        );
        for key in [CUROS, TIRECZ_VICTORY] {
            assert!(catalogue.transition(key).is_some(), "{key}");
        }
        let prior = prior_definition(catalogue);
        assert_ne!(catalogue.definition_hash(QUEST), Some(prior.hash));
        assert_eq!(
            super::super::npc_rules::legacy_defaults(catalogue, QUEST, &prior.hash),
            Some(&prior.defaults),
        );
        Ok(())
    }
}
