//! Crystal 201189f8, forgotten_knowledge/creaturescripts_thorn_knight_death.lua:1-23
//! and the_first_dragon/creaturescripts_death_dragon_essence.lua:3-12.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult, fixture::Harness, fixture::debug,
};
use crate::foundation::ExactActorRef;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_protocol_oteryn::world_spatial::StepDirection;
use serde_json::json;

pub(super) const BOWL: &str = "first-dragon-angry-plant-corpse";
pub(super) const EARTH: &str = "forgotten-knowledge-thorn-death-phases";
pub(super) const ENERGY: &str = "forgotten-knowledge-prism-death-relocation";
pub(super) const ENERGY_RECOVERY: &str = "forgotten-knowledge-prism-vacancy-recovery";
pub(super) const ESSENCE: &str = "first-dragon-essence-death-phase";
pub(super) const FALLEN: &str = "first-dragon-fallen-challenger-portal";
pub(super) const LORE: &str = "forgotten-knowledge-lore-keeper-death-phase";
pub(super) const LORE_ROUTE: &str = "forgotten-knowledge-lore-keeper-power-route";
pub(super) const VICTORY: &str = "first-dragon-reward-room-transfer";
const MOUNTED: &str = "oteryn:creature.mounted_thorn_knight";
const MOUNTED_SPAWN: Position = at(32624, 32880);
const SHIELDED_DEATH_TILE: Position = at(32625, 32880);
// Crystal 201189f8, src/game/game.cpp:7417-7423 creates a blood small splash.
// Both pool types are fixed in data/items/appearances.dat and src/utils/utils_definitions.hpp.
const BLOOD: [u16; 2] = [2886, 2889];
// Crystal data/items/items.xml:4091-4093 and data/items/appearances.dat.
const MOVABLE_STATUE: u16 = 2025;
const MOUNTED_ANNOUNCEMENT: &str = "The thorn knight unmounts!";
const SHIELDED: &str = "oteryn:creature.the_shielded_thorn_knight";
const ENRAGED: &str = "oteryn:creature.the_enraged_thorn_knight";
const STEED: &str = "oteryn:creature.thorn_steed";
const DRAGON_ESSENCE: &str = "oteryn:creature.dragon_essence";
const ESSENCE_AREA_MIN: Position = at(33603, 31009);
const ESSENCE_AREA_MAX: Position = at(33631, 31037);
const DRAGON_SPAWN: Position = at(33617, 31023);
const ESSENCE_ANNOUNCEMENT: &str = "BEWARE! THE FIRST DRAGON APROACHES!";
const ESSENCE_WAVE: [Position; 6] = [
    at(33624, 31028),
    at(33609, 31018),
    at(33610, 31018),
    at(33611, 31018),
    at(33612, 31018),
    at(33613, 31018),
];
const ESSENCE_OUTSIDE: [Position; 2] = [at(33632, 31028), at(33632, 31030)];

// Three legal Crystal spectator waves exercise a sweep beyond the old 16-actor cutoff.
fn late_essence_waves() -> impl Iterator<Item = [Position; 6]> {
    [31017, 31018, 31019]
        .into_iter()
        .map(|y| [33609, 33610, 33611, 33612, 33613, 33614].map(|x| at(x, y)))
}
const DRAGON: &str = "oteryn:creature.the_first_dragon";
const CHALLENGER: &str = "oteryn:creature.fallen_challenger";
const SPIRIT: &str = "oteryn:creature.spirit_of_fertility";
const PLANT: &str = "oteryn:creature.angry_plant";
const UNBEATABLE: &str = "oteryn:creature.unbeatable_dragon";
const SOMEWHAT: &str = "oteryn:creature.somewhat_beatable";
const FLAME: &str = "oteryn:item.tibia.i775";
const LEGACY_PORTAL_APPEARANCE: u16 = 1949;
const LEGACY_PORTAL_DELAY_MS: u32 = 30_000;
const FLOWER_BOWL: &str = "oteryn:item.tibia.i2983";
const PLANT_TILE: Position = at(33625, 31022);
const BOWL_APPROACH: Position = at(33625, 31021);
const FINALE_LEVER: Position = at(33583, 30992);
const CHALLENGERS: [Position; 2] = [at(33592, 31013), at(33574, 31013)];
const OTHER_CHALLENGERS: [Position; 3] = [at(33583, 31022), at(33574, 31031), at(33592, 31031)];
const UNBEATABLE_SPAWN: Position = at(33622, 31021);
const STAGING: [Position; 2] = [at(33583, 30993), at(33582, 30993)];
const ARRIVALS: [Position; 2] = [at(33592, 31017), at(33574, 31017)];
const FINALE: Position = at(33621, 31023);
const GLYPH: &str = "oteryn:creature.an_astral_glyph";
const KEEPER: &str = "oteryn:creature.the_last_lore_keeper";
const LLOYD: &str = "oteryn:creature.lloyd";
const PRISM_A: &str = "oteryn:creature.cosmic_energy_prism_a";
const PRISM_B: &str = "oteryn:creature.cosmic_energy_prism_b";
const PRISMS: [(&str, Position); 4] = [
    (PRISM_A, at(32801, 32827)),
    (PRISM_B, at(32798, 32827)),
    ("oteryn:creature.cosmic_energy_prism_c", at(32803, 32826)),
    ("oteryn:creature.cosmic_energy_prism_d", at(32796, 32826)),
];
const LLOYD_SPAWN: Position = at(32799, 32827);
const LLOYD_REBIRTH: Position = at(32799, 32826);
const LLOYD_RETURN: Position = at(32799, 32829);
const LLOYD_RETURN_EFFECT: u16 = 11; // Crystal CONST_ME_TELEPORT.
const PRISM_RECOVERY_EFFECT: u16 = 12; // Crystal src/utils/utils_definitions.hpp:67.
const REWARD_ROOM: Position = Position {
    x: 33617,
    y: 31020,
    floor: -13,
};
const BOUND_POWER: &str = "oteryn:creature.bound_astral_power";
const POWER_ROUTE: [Position; 4] = [
    below(31975, 32856),
    below(31975, 32839),
    below(31995, 32839),
    below(31995, 32856),
];
const POWER_ANNOUNCEMENT: &str =
    "The destruction of the power source gained you more time until the glyph is powered up!";
const ASTRAL_PEER: Position = at(31981, 32850);
const ASTRAL_PEER_AFTER_STEP: Position = at(31982, 32850);
const ASTRAL_ACTORS: [(&str, Position); 5] = [
    ("oteryn:creature.a_shielded_astral_glyph", at(31986, 32840)),
    (BOUND_POWER, POWER_ROUTE[0]),
    ("oteryn:creature.the_astral_source", at(31987, 32839)),
    (
        "oteryn:creature.the_distorted_astral_source",
        below(31986, 32823),
    ),
    (GLYPH, below(31989, 32823)),
];

const fn at(x: u16, y: u16) -> Position {
    Position { x, y, floor: -14 }
}

const fn below(x: u16, y: u16) -> Position {
    Position { x, y, floor: -15 }
}

#[test]
fn angry_plant_corpse_converts_unbeatable_dragon_and_preserves_encounter_membership() -> TestResult
{
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(BOWL, || {
        super::quest_world_callbacks::scenario(BOWL, FINALE_LEVER, false)
    })
}

#[test]
fn thorn_knight_advances_both_phases_and_clears_only_fixed_items_on_the_death_tiles() -> TestResult
{
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(EARTH, || {
        super::quest_world_callbacks::scenario(EARTH, at(32657, 32876), false)
    })
}

#[test]
fn fallen_challenger_corpses_route_two_staged_players_into_the_finale() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(FALLEN, || {
        super::quest_world_callbacks::scenario(FALLEN, FINALE_LEVER, false)
    })
}

#[test]
fn energy_prism_death_returns_lloyd_only_from_the_rebirth_tile() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(ENERGY, || {
        super::quest_world_callbacks::scenario(ENERGY, at(32759, 32867), false)
    })
}

#[test]
fn energy_prism_recovers_before_damage_only_while_the_rebirth_tile_is_empty() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(ENERGY_RECOVERY, || {
        super::quest_world_callbacks::scenario(ENERGY_RECOVERY, at(32759, 32867), false)
    })
}

#[test]
fn lore_keeper_lever_composes_both_floors_and_glyph_death_spawns_its_successor() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(LORE, || {
        super::quest_world_callbacks::scenario(LORE, at(32019, 32843), false)
    })
}

#[test]
fn bound_astral_power_routes_from_top_damage_across_floors_at_six_but_not_seven() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(LORE_ROUTE, || {
        super::quest_world_callbacks::scenario(LORE_ROUTE, at(32019, 32843), false)
    })
}

#[test]
fn first_dragon_death_sends_two_room_spectators_to_the_reward_landing() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(VICTORY, || {
        super::quest_world_callbacks::scenario(VICTORY, at(33583, 30992), false)
    })
}

#[test]
fn later_dragon_essence_death_retires_nearby_essences_and_summons_one_first_dragon() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(ESSENCE, || {
        super::quest_world_callbacks::scenario(ESSENCE, at(33583, 30992), false)
    })
}

pub(super) fn callback(name: &str, lever: Position) -> TestResult<Callback> {
    if name == BOWL {
        let mut monsters = vec![json!({"creature": PLANT, "position": PLANT_TILE})];
        monsters.extend(
            CHALLENGERS
                .into_iter()
                .chain(OTHER_CHALLENGERS)
                .map(|position| json!({"creature": CHALLENGER, "position": position})),
        );
        // Representative distinct coordinates from the donor's randomized bounds.
        monsters.extend(
            [
                UNBEATABLE_SPAWN,
                at(33610, 31016),
                at(33611, 31016),
                at(33612, 31016),
                at(33613, 31016),
            ]
            .into_iter()
            .map(|position| json!({"creature": UNBEATABLE, "position": position})),
        );
        let data = json!({"schema": 1, "callbacks": [{
            "key": format!("oteryn:quest-callback/{BOWL}"),
            "quest": "oteryn:quest.the_first_dragon_quest", "trigger": "use",
            "selector": {"position": lever, "appearance": 8911},
            "reversible": false, "passages": [], "storage": [], "offerings": [],
            "return_on_failure": false,
            "effects": {"encounter": {
                "minimum_level": 0, "shared_staging": true,
                "players": [{"from": STAGING[0], "to": FINALE}],
                "monsters": monsters,
                "room_min": at(33566, 31006), "room_max": at(33626, 31032),
                "death_phases": {
                    "terminal": SOMEWHAT, "replacements": [],
                    "corpse_monster_step": {
                        "defeated": PLANT, "item": FLOWER_BOWL,
                        "creature": UNBEATABLE, "replace_with": SOMEWHAT
                    }
                }
            }},
            "sources": [
                "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_first_dragon/creaturescripts_death_angry_plant.lua:3-7",
                "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_first_dragon/movements_flower_bowl.lua:3-18"
            ]
        }]});
        let [callback] = oteryn_gameplay_quests::quest::world_callbacks::parse(&data.to_string())?
            .try_into()
            .map_err(|_| "one Flower Bowl phase callback")?;
        return Ok(callback);
    }
    if name == FALLEN {
        // Crystal actions_lever.lua:7-27 stages fifteen players at five landings,
        // two of which are outside the source occupancy rectangle.
        let players: Vec<_> = [
            (STAGING[0], ARRIVALS[0]),
            (STAGING[1], ARRIVALS[1]),
            (at(33584, 30993), at(33592, 31035)),
            (at(33582, 30994), at(33574, 31035)),
            (at(33583, 30994), at(33583, 31026)),
            (at(33584, 30994), ARRIVALS[1]),
            (at(33582, 30995), ARRIVALS[0]),
            (at(33583, 30995), at(33592, 31035)),
            (at(33584, 30995), at(33574, 31035)),
            (at(33582, 30996), at(33583, 31026)),
            (at(33583, 30996), ARRIVALS[1]),
            (at(33584, 30996), ARRIVALS[0]),
            (at(33582, 30997), at(33592, 31035)),
            (at(33583, 30997), at(33574, 31035)),
            (at(33584, 30997), at(33583, 31026)),
        ]
        .map(|(from, to)| json!({"from":from,"to":to}))
        .into();
        let data = json!({"schema": 1, "callbacks": [{
            "key": format!("oteryn:quest-callback/{FALLEN}"),
            "quest": "oteryn:quest.the_first_dragon_quest", "trigger": "use",
            "selector": {"position": lever, "appearance": 8911},
            "reversible": false, "passages": [], "storage": [], "offerings": [],
            "return_on_failure": false,
            "effects": {"encounter": {
                "minimum_level": 0, "shared_staging": true,
                "players": players,
                "monsters": [
                    {"creature": CHALLENGER, "position": CHALLENGERS[0]},
                    {"creature": CHALLENGER, "position": CHALLENGERS[1]},
                    {"creature": SPIRIT, "position": at(33625, 31021)}
                ],
                "room_min": at(33566, 31006), "room_max": at(33626, 31032),
                "death_phases": {
                    "terminal": SPIRIT, "replacements": [],
                    "corpse_portal": {
                        "defeated": CHALLENGER, "item": FLAME, "destination": FINALE,
                        "cleanup": {"appearance": LEGACY_PORTAL_APPEARANCE, "delay_ms": LEGACY_PORTAL_DELAY_MS}
                    }
                }
            }},
            "sources": [
                "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_first_dragon/actions_lever.lua:1-51",
                "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_first_dragon/creaturescripts_death_fallen.lua:10-16",
                "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/monster/quests/the_first_dragon/bosses/fallen_challenger.lua:19"
            ]
        }]});
        let [callback] = oteryn_gameplay_quests::quest::world_callbacks::parse(&data.to_string())?
            .try_into()
            .map_err(|_| "one Fallen Challenger phase callback")?;
        return Ok(callback);
    }
    if matches!(name, LORE | LORE_ROUTE) {
        return lore_callback(name, lever);
    }
    if matches!(name, ENERGY | ENERGY_RECOVERY) {
        return energy_callback(name, lever);
    }
    let (quest, source, from, arrival, min, max, monsters, mut death_phases) = if name == EARTH {
        (
            "oteryn:quest.forgotten_knowledge_quest",
            "forgotten_knowledge/creaturescripts_thorn_knight_death.lua:1-23",
            at(32657, 32877),
            at(32624, 32886),
            at(32613, 32869),
            at(32636, 32892),
            vec![json!({"creature":MOUNTED,"position":MOUNTED_SPAWN})],
            json!({"terminal":ENRAGED,"replacements":[
                {"defeated":MOUNTED,"announcement":{"text":MOUNTED_ANNOUNCEMENT},
                 "clear_fixed_items_after_ms":1,
                 "spawn":[{"creature":SHIELDED},{"creature":STEED}]},
                {"defeated":SHIELDED,"spawn":[{"creature":ENRAGED}],"clear_fixed_items_after_ms":1}
            ]}),
        )
    } else {
        (
            "oteryn:quest.the_first_dragon_quest",
            "the_first_dragon/creaturescripts_death_dragon_essence.lua:3-12",
            at(33583, 30993),
            at(33621, 31023),
            at(33603, 31009),
            at(33631, 31037),
            if name == ESSENCE {
                // The donor lever admits the Spirit before the Essence wave exists.
                vec![json!({"creature":SPIRIT,"position":at(33625,31021)})]
            } else {
                // The reward-room case begins with the already admitted wave.
                ESSENCE_WAVE
                    .into_iter()
                    .map(|position| json!({"creature":DRAGON_ESSENCE,"position":position}))
                    .collect()
            },
            json!({"terminal":DRAGON,"replacements":[
                {"defeated":DRAGON_ESSENCE,"retire_siblings":true,
                 "retire_siblings_in":{"minimum":ESSENCE_AREA_MIN,"maximum":ESSENCE_AREA_MAX},
                 "announcement":{"text":ESSENCE_ANNOUNCEMENT,"position":DRAGON_SPAWN},
                 "spawn":[{"creature":DRAGON,"position":DRAGON_SPAWN}]}
            ]}),
        )
    };
    let mut sources = vec![format!(
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/{source}"
    )];
    if name == VICTORY {
        death_phases["terminal_transfer"] = json!({
            "area_min": at(33603,31009), "area_max": at(33631,31037), "destination": REWARD_ROOM
        });
        sources.push("crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_first_dragon/creaturescripts_death_first_dragon.lua:21-30".into());
    }
    let data = json!({"schema":1,"callbacks":[{
        "key": format!("oteryn:quest-callback/{name}"), "quest":quest,"trigger":"use",
        "selector":{"position":lever,"appearance":8911},
        "reversible":false,"passages":[],"storage":[],"offerings":[],"return_on_failure":false,
        "effects":{"encounter":{
            "minimum_level":0,"players":[{"from":from,"to":arrival}],"monsters":monsters,
            "room_min":min,"room_max":max,"shared_staging":true,"death_phases":death_phases
        }},
        "sources":sources
    }]});
    let [callback] = oteryn_gameplay_quests::quest::world_callbacks::parse(&data.to_string())?
        .try_into()
        .map_err(|_| "one phase callback")?;
    Ok(callback)
}

fn lore_callback(name: &str, lever: Position) -> TestResult<Callback> {
    let players: Vec<_> = (32018..=32020)
        .flat_map(|x| (32844..=32848).map(move |y| json!({"from":at(x,y), "to":at(31985,32851)})))
        .collect();
    let monsters =
        ASTRAL_ACTORS.map(|(creature, position)| json!({"creature":creature,"position":position}));
    let mut phases = json!({"terminal":KEEPER,"replacements":[{"defeated":GLYPH,"spawn":[{"creature":KEEPER}]}]});
    if name == LORE_ROUTE {
        let stops: Vec<_> = POWER_ROUTE
            .iter()
            .zip(POWER_ROUTE.iter().cycle().skip(1))
            .map(|(near, spawn_at)| json!({"near":near,"spawn_at":spawn_at}))
            .collect();
        phases["contributor_route"] = json!({"creature":BOUND_POWER,"maximum_distance":6,"stops":stops,"announcement":POWER_ANNOUNCEMENT});
    }
    let data = json!({"schema":1,"callbacks":[{
        "key":format!("oteryn:quest-callback/{name}"),"quest":"oteryn:quest.forgotten_knowledge_quest",
        "trigger":"use","selector":{"position":lever,"appearance":8911},
        "reversible":false,"passages":[],"storage":[],"offerings":[],"return_on_failure":false,
        "effects":{"encounter":{
            "minimum_level":0,"players":players,"monsters":monsters,
            "room_min":at(31968,32821),"room_max":below(32004,32865),"shared_staging":true,
            "death_phases":phases
        }},
        "sources":[
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/forgotten_knowledge/actions_the_last_lore_keeper.lua:1-7,50-74",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/forgotten_knowledge/creaturescripts_astral_glyph_death.lua:2-4",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/forgotten_knowledge/creaturescripts_bound_astral_power.lua:1-6,15-26"
        ]
    }]});
    let [callback] = oteryn_gameplay_quests::quest::world_callbacks::parse(&data.to_string())?
        .try_into()
        .map_err(|_| "one Lore Keeper callback")?;
    Ok(callback)
}

fn energy_callback(name: &str, lever: Position) -> TestResult<Callback> {
    let players: Vec<_> = (32868..=32872)
        .map(|y| json!({"from":at(32759,y), "to":at(32800,32831)}))
        .collect();
    let mut monsters: Vec<_> = PRISMS
        .iter()
        .map(|(creature, position)| json!({"creature":creature,"position":position}))
        .collect();
    monsters.push(json!({"creature":LLOYD,"position":LLOYD_SPAWN}));
    let replacements = PRISMS.map(|(creature, _)| {
        json!({
            "defeated":creature,"spawn":[],
            "relocate":{"creature":LLOYD,"from":LLOYD_REBIRTH,"to":LLOYD_RETURN,"arrival_effect":LLOYD_RETURN_EFFECT}
        })
    });
    let mut phases = json!({"terminal":LLOYD,"replacements":replacements});
    if name == ENERGY_RECOVERY {
        phases["heal_before_damage"] = json!({"creatures":PRISMS.map(|(creature, _)| creature),
            "vacant_cell":LLOYD_REBIRTH,"amount":10000,"effect":PRISM_RECOVERY_EFFECT,"announcement":"*zap!*"});
    }
    let data = json!({"schema":1,"callbacks":[{
        "key":format!("oteryn:quest-callback/{name}"),"quest":"oteryn:quest.forgotten_knowledge_quest",
        "trigger":"use","selector":{"position":lever,"appearance":8911},
        "reversible":false,"passages":[],"storage":[],"offerings":[],"return_on_failure":false,
        "effects":{"encounter":{
            "minimum_level":0,"players":players,"monsters":monsters,
            "room_min":at(32785,32814),"room_max":at(32812,32838),"shared_staging":true,
            "death_phases":phases
        }},
        "sources":[
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/forgotten_knowledge/actions_lloyd.lua:1-12,47-74",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/forgotten_knowledge/creaturescripts_energy_prism_death.lua:2-13",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/forgotten_knowledge/creaturescripts_energy_prism.lua:2-10"
        ]
    }]});
    let [callback] = oteryn_gameplay_quests::quest::world_callbacks::parse(&data.to_string())?
        .try_into()
        .map_err(|_| "one Energy callback")?;
    Ok(callback)
}

pub(super) fn world(
    callback: &Callback,
    lever: Position,
    name: &str,
) -> TestResult<crate::map::boot::BundleWorld> {
    let extra = match name {
        BOWL => vec![BOWL_APPROACH],
        ENERGY | ENERGY_RECOVERY => vec![LLOYD_REBIRTH, LLOYD_RETURN],
        ESSENCE => late_essence_waves()
            .flatten()
            .chain(ESSENCE_OUTSIDE)
            .chain([at(33617, 31023)])
            .collect(),
        FALLEN => vec![
            at(33592, 31014),
            at(33592, 31015),
            at(33592, 31016),
            at(33574, 31014),
            at(33574, 31015),
            at(33574, 31016),
            FINALE,
            at(33622, 31023),
        ],
        LORE_ROUTE => POWER_ROUTE
            .into_iter()
            .chain([ASTRAL_PEER, ASTRAL_PEER_AFTER_STEP])
            .collect(),
        VICTORY => vec![at(33617, 31023), at(33622, 31023), REWARD_ROOM],
        _ => Vec::new(),
    };
    if name == FALLEN {
        return super::quest_world_callbacks::world_with_scene_cells(
            callback,
            lever,
            &CHALLENGERS.map(|at| (at, LEGACY_PORTAL_APPEARANCE)),
            &[],
            &extra,
        );
    }
    if name == EARTH {
        let mut objects = Vec::new();
        for position in [MOUNTED_SPAWN, SHIELDED_DEATH_TILE] {
            objects.extend(
                BLOOD
                    .into_iter()
                    .chain([MOVABLE_STATUE])
                    .map(|appearance| (position, appearance)),
            );
        }
        return super::quest_world_callbacks::world_with_scene_cells(
            callback,
            lever,
            &objects,
            &[],
            &[],
        );
    }
    super::quest_world_callbacks::world_with_cells(callback, lever, &extra)
}

async fn live(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    creature: &str,
) -> TestResult<Vec<ExactActorRef>> {
    let runtime = owner.runtime.lock().await;
    Ok(runtime
        .current_live_creature_census()
        .map_err(debug)?
        .into_iter()
        .filter(|actor| runtime.matches_live_creature_identity(*actor, creature.as_bytes()))
        .collect())
}

async fn flower_bowl(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    use super::quest_world_callbacks::local;
    use crate::gameplay_transport::kill_reward::DrainOutcome;

    let [plant] = live(owner, PLANT)
        .await?
        .try_into()
        .map_err(|_| "one Angry Plant")?;
    assert_eq!(live(owner, CHALLENGER).await?.len(), 5);
    let dragons = live(owner, UNBEATABLE).await?;
    assert_eq!(dragons.len(), 5);
    let dragon = {
        let runtime = owner.runtime.lock().await;
        dragons
            .iter()
            .copied()
            .find(|actor| {
                runtime
                    .read_actor_position(*actor)
                    .is_ok_and(|at| at.position() == local(UNBEATABLE_SPAWN))
            })
            .ok_or("dragon at source spawn")?
    };
    super::engenc2_cults::kill(owner, admitted, plant, PLANT, 101).await?;
    put_dragon_on_approach(owner, dragon).await?;
    super::zamulosh_monster_step::step_monster(owner, h, dragon, PLANT_TILE).await?;
    assert_eq!(live(owner, UNBEATABLE).await?, dragons);
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Drained
    );
    put_dragon_on_approach(owner, dragon).await?;
    super::zamulosh_monster_step::step_monster(owner, h, dragon, PLANT_TILE).await?;
    let remaining = live(owner, UNBEATABLE).await?;
    assert_eq!(remaining.len(), 4);
    assert!(
        dragons
            .iter()
            .all(|actor| remaining.contains(actor) == (*actor != dragon))
    );
    let [next] = live(owner, SOMEWHAT)
        .await?
        .try_into()
        .map_err(|_| "one Somewhat Beatable")?;
    let player = admitted.runtime_actor.ok_or("player")?;
    {
        let runtime = owner.runtime.lock().await;
        assert_eq!(
            runtime.read_actor_position(next).map_err(debug)?.position(),
            local(PLANT_TILE)
        );
        assert!(
            runtime.bound_corpse(plant.corpse_identity()).is_some(),
            "the bowl stays on its tile"
        );
        let states = owner.states.spell_states.lock().await;
        assert!(states.encounter_phases.blocks_channel_switch(
            &runtime,
            player,
            admitted.game_session_id,
            local(FINALE)
        ));
    }
    super::engenc2_cults::kill(owner, admitted, next, SOMEWHAT, 102).await?;
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    assert!(
        !states.encounter_phases.blocks_channel_switch(
            &runtime,
            player,
            admitted.game_session_id,
            local(FINALE)
        ),
        "the replacement's death completes the fixture encounter"
    );
    Ok(())
}

async fn astral_power_route(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    use crate::foundation::{CommandId, CommandRef};
    use oteryn_gameplay_entities::OwnerDamageCommand;
    use oteryn_protocol_oteryn::world_spatial::StepDisposition;

    let peer = super::ferumbras_crystal_reset::admit_peer(owner, h, ASTRAL_PEER).await?;
    let peer_session = owner
        .runtime
        .lock()
        .await
        .positioned_melee_census()
        .map_err(debug)?
        .into_iter()
        .find_map(|(actor, _, session)| (actor == peer).then_some(session).flatten())
        .ok_or("astral peer session")?;
    let [original] = live(owner, BOUND_POWER)
        .await?
        .try_into()
        .map_err(|_| "initial astral power")?;
    let mut defeated = original;
    for (attack, x) in [(110, ASTRAL_PEER.x), (111, ASTRAL_PEER_AFTER_STEP.x)] {
        {
            let mut runtime = owner.runtime.lock().await;
            let peer_position = runtime.read_actor_position(peer).map_err(debug)?.position();
            assert_eq!(
                peer_position,
                super::quest_world_callbacks::local(at(x, 32850))
            );
            let health = runtime.companion_snapshot(defeated).map_err(debug)?.health;
            assert!(health > 2);
            runtime
                .borrow_exact_actor_commit_at(owner.owner_now().get() / 1000)
                .commit_damage_for_bound_attacker(
                    defeated,
                    peer,
                    CommandRef::new(peer_session, CommandId::new(attack).map_err(debug)?),
                    0,
                    OwnerDamageCommand {
                        target: BOUND_POWER.as_bytes(),
                        occurrence: b"astral-peer-hit",
                        binding: b"astral-peer-contributor",
                        damage: health - 1,
                    },
                )
                .map_err(debug)?;
        }
        // The admitted player lands outside every qualifying corner and deals the last hit.
        super::engenc2_cults::kill(owner, admitted, defeated, BOUND_POWER, attack).await?;
        let remaining = live(owner, BOUND_POWER).await?;
        if attack == 110 {
            let [successor] = remaining
                .try_into()
                .map_err(|_| "one successor from the top contributor")?;
            let at = owner
                .runtime
                .lock()
                .await
                .read_actor_position(successor)
                .map_err(debug)?
                .position();
            assert_eq!(at, super::quest_world_callbacks::local(POWER_ROUTE[1]));
            defeated = successor;
            let (outcome, _) = owner
                .ordinary_paced_step(peer, peer_session, StepDirection::East, None, None)
                .await;
            assert_eq!(outcome.disposition, StepDisposition::Moved);
        } else {
            assert!(
                remaining.is_empty(),
                "distance seven does not summon another member"
            );
        }
    }
    let map = owner.bundle_map().ok_or("Astral arena map")?;
    let lines: Vec<_> = map
        .facts
        .movement_speech_since(0)
        .into_iter()
        .filter(|(_, speech)| speech.text == POWER_ANNOUNCEMENT)
        .collect();
    let [(_, speech)] = lines.as_slice() else {
        return Err("one Bound Astral Power announcement".into());
    };
    assert_eq!(speech.speaker, original);
    assert_eq!(
        (speech.position.x, speech.position.y, speech.position.floor),
        (POWER_ROUTE[0].x, POWER_ROUTE[0].y, POWER_ROUTE[0].floor)
    );
    Ok(())
}

async fn prism_recovery(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    peer: ExactActorRef,
) -> TestResult {
    use crate::foundation::{CommandId, CommandRef, OwnerDamageCommand};
    use oteryn_gameplay_domain::domain::condition::ConditionValues;
    use oteryn_gameplay_entities::carrier::runtime_actor_periodic::CreatureFieldEntry;
    use oteryn_gameplay_foundation::foundation::condition::{ConditionDefinition, TickFacts};
    use oteryn_simulation_determinism::DecisionOccurrenceId;

    let [lloyd] = live(owner, LLOYD)
        .await?
        .try_into()
        .map_err(|_| "one Lloyd")?;
    let [prism] = live(owner, PRISM_A)
        .await?
        .try_into()
        .map_err(|_| "one prism A")?;
    let player = admitted.runtime_actor.ok_or("player")?;
    let map = owner.bundle_map().ok_or("Energy room")?;
    let now = owner.owner_now().get();
    let mut runtime = owner.runtime.lock().await;
    for actor in [player, peer] {
        assert_eq!(
            runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            super::quest_world_callbacks::local(at(32800, 32831))
        );
    }
    let hit = |runtime: &mut crate::foundation::ChannelRuntimeV1, sequence| {
        let command = CommandId::new(sequence).map_err(debug)?;
        runtime
            .borrow_exact_actor_commit_at(now / 1000)
            .commit_damage_for_bound_attacker(
                prism,
                player,
                CommandRef::new(admitted.game_session_id, command),
                0,
                OwnerDamageCommand {
                    target: PRISM_A.as_bytes(),
                    occurrence: b"prism-recovery-hit",
                    binding: b"prism-recovery-damage",
                    damage: 100,
                },
            )
            .map_err(debug)
    };
    let first = hit(&mut runtime, 110)?;
    assert_eq!((first.health_before, first.health_after), (1000, 900));
    assert!(!hit(&mut runtime, 110)?.applied);
    let recovery = hit(&mut runtime, 111)?;
    assert_eq!((recovery.health_before, recovery.health_after), (1000, 900));
    assert!(!hit(&mut runtime, 111)?.applied);
    let field = CreatureFieldEntry {
        definition: ConditionDefinition::new(
            "test-prism-vacancy-recovery",
            1,
            ConditionValues::Agony {
                total_min: 100,
                total_max: 100,
                per_tick: 100,
                interval_ms: 1000,
                delayed: false,
            },
        )
        .ok_or("Agony field")?,
        source: None,
        lineage: None,
        occurrence: DecisionOccurrenceId::from_bytes([0xa3; 16]),
    };
    runtime
        .commit_creature_field_entry(prism, now, Some(&field))
        .map_err(debug)?;
    let relocate_lloyd = |runtime: &mut crate::foundation::ChannelRuntimeV1, destination| {
        let before = runtime.read_actor_position(lloyd).map_err(debug)?;
        runtime
            .borrow_movement_position()
            .commit_creature_relocation(
                lloyd,
                before,
                super::quest_world_callbacks::local(destination),
            )
            .map_err(debug)
    };
    let stale = runtime
        .stage_creature_periodic_turn(prism, now, TickFacts::default())
        .map_err(debug)?;
    relocate_lloyd(&mut runtime, LLOYD_REBIRTH)?;
    assert!(matches!(
        runtime.commit_creature_periodic_turn(stale),
        Err(crate::foundation::CarrierError::PlanConflict)
    ));
    assert_eq!(runtime.live_creature_health(prism), Some(900));
    relocate_lloyd(&mut runtime, LLOYD_SPAWN)?;
    let prepared = runtime
        .stage_creature_periodic_turn(prism, now, TickFacts::default())
        .map_err(debug)?;
    let periodic = runtime
        .commit_creature_periodic_turn(prepared)
        .map_err(debug)?;
    assert!(!periodic.ticks.is_empty());
    assert_eq!(periodic.health_after, 900);
    relocate_lloyd(&mut runtime, LLOYD_REBIRTH)?;
    let guarded = hit(&mut runtime, 112)?;
    assert_eq!((guarded.health_before, guarded.health_after), (900, 800));
    let mut states = owner.states.spell_states.lock().await;
    for _ in 0..2 {
        crate::gameplay_transport::encounter_phases::tick(
            &mut runtime,
            &mut states,
            Some(map),
            now,
        );
    }
    let pos = PRISMS[0].1;
    assert_eq!(
        map.facts
            .movement_speech_since(0)
            .iter()
            .filter(|(_, speech)| speech.speaker == prism && speech.text == "*zap!*")
            .count(),
        2
    );
    assert_eq!(
        map.facts
            .movement_effects_since(0)
            .iter()
            .filter(
                |(_, at, effect)| (at.x, at.y, at.floor) == (pos.x, pos.y, pos.floor)
                    && *effect == PRISM_RECOVERY_EFFECT
            )
            .count(),
        2
    );
    Ok(())
}

async fn put_dragon_on_approach(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
) -> TestResult {
    let mut runtime = owner.runtime.lock().await;
    let before = runtime.read_actor_position(actor).map_err(debug)?;
    runtime
        .borrow_movement_position()
        .commit_creature_relocation(
            actor,
            before,
            super::quest_world_callbacks::local(BOWL_APPROACH),
        )
        .map_err(debug)?;
    Ok(())
}

async fn fallen_portals(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    peer: ExactActorRef,
) -> TestResult {
    use super::quest_world_callbacks::local;
    use crate::gameplay_transport::kill_reward::DrainOutcome;
    use crate::map::{
        overlay::TilePos,
        view::{MapFacts, placement_key},
    };
    use oteryn_protocol_oteryn::world_spatial::StepDisposition;

    let primary = admitted.runtime_actor.ok_or("player")?;
    let peer_session = {
        let runtime = owner.runtime.lock().await;
        for (actor, arrival) in [primary, peer].into_iter().zip(ARRIVALS) {
            assert_eq!(
                runtime
                    .read_actor_position(actor)
                    .map_err(debug)?
                    .position(),
                local(arrival)
            );
        }
        runtime
            .positioned_melee_census()
            .map_err(debug)?
            .into_iter()
            .find_map(|(actor, _, session)| (actor == peer).then_some(session).flatten())
            .ok_or("peer session")?
    };

    let map = owner.bundle_map().ok_or("finale map")?;
    let [first, second] = CHALLENGERS.map(|at| {
        placement_key(
            TilePos {
                x: at.x,
                y: at.y,
                floor: at.floor,
            },
            1,
        )
        .ok_or("legacy portal placement")
    });
    let legacy = [first?, second?];
    assert!(legacy.iter().all(|key| !map.facts.base_hidden(*key)));
    let before_deaths = owner.owner_now().get();
    let challengers = live(owner, CHALLENGER).await?;
    assert_eq!(challengers.len(), 2);
    for (index, challenger) in challengers.into_iter().enumerate() {
        super::engenc2_cults::kill(owner, admitted, challenger, CHALLENGER, 101 + index as u64)
            .await?;
    }
    let first_due = map
        .facts
        .next_movement_decay_time()
        .ok_or("portal cleanup deadline")?;
    let delay_us = u64::from(LEGACY_PORTAL_DELAY_MS) * 1_000;
    assert!((before_deaths + delay_us..=owner.owner_now().get() + delay_us).contains(&first_due));
    assert_eq!(map.facts.expire_movement_items(first_due - 1), 0);
    assert!(legacy.iter().all(|key| !map.facts.base_hidden(*key)));
    assert!(map.facts.expire_movement_items(first_due) > 0);
    if let Some(second_due) = map.facts.next_movement_decay_time() {
        assert!(second_due > first_due);
        assert_eq!(map.facts.expire_movement_items(second_due), 1);
    }
    assert!(legacy.iter().all(|key| map.facts.base_hidden(*key)));
    {
        let runtime = owner.runtime.lock().await;
        for at in CHALLENGERS {
            assert_eq!(
                runtime.quest_corpse_portal_at(local(at)),
                None,
                "a projected death needs its committed corpse Item"
            );
        }
    }
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Drained
    );
    {
        let runtime = owner.runtime.lock().await;
        for at in CHALLENGERS {
            assert_eq!(
                runtime.quest_corpse_portal_at(local(at)).map(|(_, to)| to),
                Some(local(FINALE))
            );
        }
    }

    for _ in 0..4 {
        super::questpc7_ferumbras::step(owner, admitted, StepDirection::North).await?;
    }
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(primary)
            .map_err(debug)?
            .position(),
        local(FINALE)
    );

    for _ in 0..4 {
        let (outcome, pace) = owner
            .ordinary_paced_step(peer, peer_session, StepDirection::North, None, None)
            .await;
        assert_eq!(outcome.disposition, StepDisposition::Moved);
        if let Some(pace) = pace {
            tokio::time::sleep(pace).await;
        }
    }
    let mut runtime = owner.runtime.lock().await;
    // Both Crystal source corpses are west of the shared landing.
    for actor in [primary, peer] {
        let landing = runtime.read_actor_position(actor).map_err(debug)?;
        assert_eq!(landing.position(), local(FINALE));
        assert_eq!(
            landing.facing(),
            Some(crate::foundation::MovementFacing::East)
        );
    }
    runtime.retain_live_corpse_items(&[]).map_err(debug)?;
    for at in CHALLENGERS {
        assert_eq!(runtime.quest_corpse_portal_at(local(at)), None);
    }
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: Callback,
    lever: Position,
    name: &str,
) -> TestResult {
    super::questpc7_native_deaths::prepare_native_hunter(owner, admitted).await?;
    let peer = if name == FALLEN {
        Some(super::ferumbras_crystal_reset::admit_peer(owner, h, STAGING[1]).await?)
    } else {
        None
    };
    let recovery_peer = if name == ENERGY_RECOVERY {
        Some(super::ferumbras_crystal_reset::admit_peer(owner, h, at(32759, 32869)).await?)
    } else {
        None
    };
    owner
        .states
        .quest_callbacks
        .fixture
        .set(vec![callback])
        .map_err(|_| "callback already installed")?;
    super::questpc7_ferumbras::step(owner, admitted, StepDirection::East).await?;
    super::questpc7_ferumbras::step(owner, admitted, StepDirection::South).await?;
    assert_eq!(
        super::questpc7_ferumbras::use_at(owner, admitted, lever, 8911).await?,
        UseDisposition::Committed
    );
    if let Some(peer) = peer {
        fallen_portals(owner, admitted, peer).await?;
    } else if let Some(peer) = recovery_peer {
        prism_recovery(owner, admitted, peer).await?;
    } else if name == BOWL {
        flower_bowl(owner, admitted, h).await?;
    } else if name == EARTH {
        use crate::map::view::{MapFacts, placement_key};

        let map = owner.bundle_map().ok_or("Earth room")?;
        let announcements = || {
            map.facts
                .movement_speech_since(0)
                .into_iter()
                .filter(|(_, speech)| speech.text == MOUNTED_ANNOUNCEMENT)
                .map(|(_, speech)| speech)
                .collect::<Vec<_>>()
        };
        assert!(announcements().is_empty());
        let [mounted] = live(owner, MOUNTED)
            .await?
            .try_into()
            .map_err(|_| "one mounted knight")?;
        super::engenc2_cults::kill(owner, admitted, mounted, MOUNTED, 101).await?;
        let [shielded] = live(owner, SHIELDED)
            .await?
            .try_into()
            .map_err(|_| "one shielded knight")?;
        assert_eq!(live(owner, STEED).await?.len(), 1);
        assert!(live(owner, ENRAGED).await?.is_empty());
        let lines = announcements();
        let [line] = lines.as_slice() else {
            return Err("one mounted knight announcement".into());
        };
        assert_eq!(line.speaker, mounted);
        assert_eq!(line.talk_type, 36);
        assert_eq!(
            line.position,
            crate::map::overlay::TilePos {
                x: MOUNTED_SPAWN.x,
                y: MOUNTED_SPAWN.y,
                floor: MOUNTED_SPAWN.floor,
            }
        );
        {
            let mut runtime = owner.runtime.lock().await;
            let expected = runtime.read_actor_position(shielded).map_err(debug)?;
            runtime
                .borrow_movement_position()
                .commit_creature_relocation(
                    shielded,
                    expected,
                    super::quest_world_callbacks::local(SHIELDED_DEATH_TILE),
                )
                .map_err(debug)?;
        }
        super::engenc2_cults::kill(owner, admitted, shielded, SHIELDED, 102).await?;
        assert_eq!(live(owner, ENRAGED).await?.len(), 1);
        assert_eq!(live(owner, STEED).await?.len(), 1);
        assert_eq!(announcements(), lines);
        let keys = [MOUNTED_SPAWN, SHIELDED_DEATH_TILE].map(|position| {
            let tile = crate::map::overlay::TilePos {
                x: position.x,
                y: position.y,
                floor: position.floor,
            };
            [0, 1, 2, 3].map(|ordinal| placement_key(tile, ordinal).expect("test tile"))
        });
        assert!(
            keys.iter()
                .flatten()
                .all(|key| !map.facts.base_hidden(*key))
        );
        assert!(map.facts.next_movement_decay_time().is_some());
        let stop = oteryn_foundation::CancellationToken::new();
        let observe = async {
            loop {
                if keys.iter().all(|[_, first, second, _]| {
                    map.facts.base_hidden(*first) && map.facts.base_hidden(*second)
                }) {
                    stop.cancel();
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        };
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            super::fixture::join_two(owner.run_source_owner_cycles(&stop), observe).await;
        })
        .await?;
        assert!(keys.iter().all(|[ground, _, _, statue]| {
            !map.facts.base_hidden(*ground) && !map.facts.base_hidden(*statue)
        }));
        assert_eq!(map.facts.next_movement_decay_time(), None);
    } else if name == ENERGY {
        let map = owner.bundle_map().ok_or("Energy room")?;
        let return_cell = crate::map::overlay::TilePos {
            x: LLOYD_RETURN.x,
            y: LLOYD_RETURN.y,
            floor: LLOYD_RETURN.floor,
        };
        let return_effects = || {
            map.facts
                .movement_effects_since(0)
                .into_iter()
                .filter(|&(_, position, effect)| {
                    position == return_cell && effect == LLOYD_RETURN_EFFECT
                })
                .count()
        };
        let [lloyd] = live(owner, LLOYD)
            .await?
            .try_into()
            .map_err(|_| "one Lloyd")?;
        let [prism] = live(owner, PRISM_B)
            .await?
            .try_into()
            .map_err(|_| "one prism B")?;
        super::engenc2_cults::kill(owner, admitted, prism, PRISM_B, 101).await?;
        assert_eq!(return_effects(), 0);
        {
            let mut runtime = owner.runtime.lock().await;
            let spawn = runtime.read_actor_position(lloyd).map_err(debug)?;
            assert_eq!(
                spawn.position(),
                super::quest_world_callbacks::local(LLOYD_SPAWN)
            );
            // Arrange the source rebirth position for the second prism death.
            runtime
                .borrow_movement_position()
                .commit_creature_relocation(
                    lloyd,
                    spawn,
                    super::quest_world_callbacks::local(LLOYD_REBIRTH),
                )
                .map_err(debug)?;
        }
        let [prism] = live(owner, PRISM_A)
            .await?
            .try_into()
            .map_err(|_| "one prism A")?;
        super::engenc2_cults::kill(owner, admitted, prism, PRISM_A, 102).await?;
        assert_eq!(return_effects(), 1);
        let runtime = owner.runtime.lock().await;
        let returned = runtime.read_actor_position(lloyd).map_err(debug)?;
        assert_eq!(
            returned.position(),
            super::quest_world_callbacks::local(LLOYD_RETURN)
        );
        assert_eq!(
            returned.facing(),
            Some(crate::foundation::MovementFacing::South)
        );
    } else if matches!(name, LORE | LORE_ROUTE) {
        let runtime = owner.runtime.lock().await;
        let census = runtime.current_live_creature_census().map_err(debug)?;
        for (creature, position) in ASTRAL_ACTORS {
            let matching: Vec<_> = census
                .iter()
                .filter(|actor| {
                    runtime.matches_live_creature_identity(**actor, creature.as_bytes())
                })
                .collect();
            let [actor] = matching.as_slice() else {
                return Err("one actor per Lore role".into());
            };
            assert_eq!(
                runtime
                    .read_actor_position(**actor)
                    .map_err(debug)?
                    .position(),
                super::quest_world_callbacks::local(position)
            );
        }
        drop(runtime);
        if name == LORE_ROUTE {
            astral_power_route(owner, admitted, h).await?;
        }
        let [glyph] = live(owner, GLYPH)
            .await?
            .try_into()
            .map_err(|_| "one astral glyph")?;
        super::engenc2_cults::kill(owner, admitted, glyph, GLYPH, 101).await?;
        let [keeper] = live(owner, KEEPER)
            .await?
            .try_into()
            .map_err(|_| "one Lore Keeper")?;
        let runtime = owner.runtime.lock().await;
        assert_eq!(
            runtime
                .read_actor_position(keeper)
                .map_err(debug)?
                .position(),
            super::quest_world_callbacks::local(below(31989, 32823))
        );
    } else {
        let map = owner.bundle_map().ok_or("First Dragon finale map")?;
        let announcements = || {
            map.facts
                .movement_speech_since(0)
                .into_iter()
                .filter(|(_, speech)| speech.text == ESSENCE_ANNOUNCEMENT)
                .map(|(_, speech)| speech)
                .collect::<Vec<_>>()
        };
        let cohort = live(owner, DRAGON_ESSENCE).await?;
        let (defeated, outside) = if name == ESSENCE {
            assert!(cohort.is_empty());
            let (inside, [outside_death, outside_survivor]) = {
                let mut runtime = owner.runtime.lock().await;
                let health = runtime
                    .companion_policy(DRAGON_ESSENCE)
                    .map_err(debug)?
                    .maximum_health;
                let spawn = |position| {
                    (
                        DRAGON_ESSENCE,
                        health,
                        super::quest_world_callbacks::local(position),
                    )
                };
                let mut inside = Vec::new();
                for wave in late_essence_waves() {
                    inside.extend(
                        runtime
                            .commit_quest_creature_replacement(&[], &wave.map(spawn), || true)
                            .map_err(debug)?,
                    );
                }
                let [outside_death, outside_survivor] = runtime
                    .commit_quest_creature_replacement(&[], &ESSENCE_OUTSIDE.map(spawn), || true)
                    .map_err(debug)?
                    .try_into()
                    .map_err(|_| "two out-of-room Essences")?;
                (inside[0], [outside_death, outside_survivor])
            };
            super::engenc2_cults::kill(owner, admitted, outside_death, DRAGON_ESSENCE, 100).await?;
            assert!(live(owner, DRAGON).await?.is_empty());
            assert!(announcements().is_empty());
            assert_eq!(live(owner, DRAGON_ESSENCE).await?.len(), 19);
            (inside, Some(outside_survivor))
        } else {
            assert_eq!(cohort.len(), 6);
            (cohort[0], None)
        };
        super::engenc2_cults::kill(owner, admitted, defeated, DRAGON_ESSENCE, 101).await?;
        assert_eq!(
            live(owner, DRAGON_ESSENCE).await?,
            outside.into_iter().collect::<Vec<_>>()
        );
        assert_eq!(live(owner, DRAGON).await?.len(), 1);
        let lines = announcements();
        let [line] = lines.as_slice() else {
            return Err("one announcement for the committed Essence transition".into());
        };
        assert_eq!(line.speaker, defeated);
        assert_eq!(line.talk_type, 36);
        assert!(line.captured_name.is_some());
        assert_eq!(
            line.position,
            crate::map::overlay::TilePos {
                x: DRAGON_SPAWN.x,
                y: DRAGON_SPAWN.y,
                floor: DRAGON_SPAWN.floor,
            }
        );
        if name == VICTORY {
            let peer =
                super::ferumbras_crystal_reset::admit_peer(owner, h, at(33622, 31023)).await?;
            let player = admitted.runtime_actor.ok_or("player")?;
            let [dragon] = live(owner, DRAGON)
                .await?
                .try_into()
                .map_err(|_| "one dragon")?;
            super::engenc2_cults::kill(owner, admitted, dragon, DRAGON, 102).await?;
            owner.drain_monster_melee().await;
            let runtime = owner.runtime.lock().await;
            for actor in [player, peer] {
                assert_eq!(
                    runtime
                        .read_actor_position(actor)
                        .map_err(debug)?
                        .position(),
                    super::quest_world_callbacks::local(REWARD_ROOM)
                );
            }
        }
    }
    Ok(())
}
