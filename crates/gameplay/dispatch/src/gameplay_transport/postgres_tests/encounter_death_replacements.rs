//! Crystal 201189f8, forgotten_knowledge/creaturescripts_thorn_knight_death.lua:12-23
//! and the_first_dragon/creaturescripts_death_dragon_essence.lua:3-12.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::debug};
use crate::foundation::ExactActorRef;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_protocol_oteryn::world_spatial::StepDirection;
use serde_json::json;

pub(super) const EARTH: &str = "forgotten-knowledge-thorn-death-phases";
pub(super) const ESSENCE: &str = "first-dragon-essence-death-phase";
pub(super) const LORE: &str = "forgotten-knowledge-lore-keeper-death-phase";
const MOUNTED: &str = "oteryn:creature.mounted_thorn_knight";
const SHIELDED: &str = "oteryn:creature.the_shielded_thorn_knight";
const ENRAGED: &str = "oteryn:creature.the_enraged_thorn_knight";
const STEED: &str = "oteryn:creature.thorn_steed";
const DRAGON_ESSENCE: &str = "oteryn:creature.dragon_essence";
const DRAGON: &str = "oteryn:creature.the_first_dragon";
const GLYPH: &str = "oteryn:creature.an_astral_glyph";
const KEEPER: &str = "oteryn:creature.the_last_lore_keeper";
const ASTRAL_ACTORS: [(&str, Position); 5] = [
    ("oteryn:creature.a_shielded_astral_glyph", at(31986, 32840)),
    ("oteryn:creature.bound_astral_power", below(31975, 32856)),
    ("oteryn:creature.the_astral_source", at(31987, 32839)),
    ("oteryn:creature.the_distorted_astral_source", below(31986, 32823)),
    (GLYPH, below(31989, 32823)),
];

const fn at(x: u16, y: u16) -> Position { Position {x,y,floor:-14} }

const fn below(x: u16, y: u16) -> Position { Position {x,y,floor:-15} }

#[test]
fn thorn_knight_deaths_spawn_the_shielded_knight_and_steed_then_enraged_knight() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(EARTH, || {
        super::quest_world_callbacks::scenario(EARTH, at(32657,32876), false)
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
fn dragon_essence_death_retires_the_five_siblings_and_summons_one_first_dragon() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(ESSENCE, || {
        super::quest_world_callbacks::scenario(ESSENCE, at(33583,30992), false)
    })
}

pub(super) fn callback(name: &str, lever: Position) -> TestResult<Callback> {
    if name == LORE {
        return lore_callback(lever);
    }
    let (quest, source, from, arrival, min, max, monsters, death_phases) = if name == EARTH {
        (
            "oteryn:quest.forgotten_knowledge_quest",
            "forgotten_knowledge/creaturescripts_thorn_knight_death.lua:12-23",
            at(32657,32877), at(32624,32886), at(32613,32869), at(32636,32892),
            vec![json!({"creature":MOUNTED,"position":at(32624,32880)})],
            json!({"terminal":ENRAGED,"replacements":[
                {"defeated":MOUNTED,"spawn":[{"creature":SHIELDED},{"creature":STEED}]},
                {"defeated":SHIELDED,"spawn":[{"creature":ENRAGED}]}
            ]}),
        )
    } else {
        (
            "oteryn:quest.the_first_dragon_quest",
            "the_first_dragon/creaturescripts_death_dragon_essence.lua:3-12",
            at(33583,30993), at(33621,31023), at(33603,31009), at(33631,31037),
            // Six legal points in the random spawn rectangle. qspawn supplies
            // the random wave after the five Somewhat Beatable deaths in content.
            (33609..=33614).map(|x| json!({"creature":DRAGON_ESSENCE,"position":at(x,31018)})).collect(),
            json!({"terminal":DRAGON,"replacements":[
                {"defeated":DRAGON_ESSENCE,"retire_siblings":true,
                 "spawn":[{"creature":DRAGON,"position":at(33617,31023)}]}
            ]}),
        )
    };
    let source = format!("crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/{source}");
    let data = json!({"schema":1,"callbacks":[{
        "key": format!("oteryn:quest-callback/{name}"), "quest":quest,"trigger":"use",
        "selector":{"position":lever,"appearance":8911},
        "reversible":false,"passages":[],"storage":[],"offerings":[],"return_on_failure":false,
        "effects":{"encounter":{
            "minimum_level":0,"players":[{"from":from,"to":arrival}],"monsters":monsters,
            "room_min":min,"room_max":max,"shared_staging":true,"death_phases":death_phases
        }},
        "sources":[source]
    }]});
    let [callback] = oteryn_gameplay_quests::quest::world_callbacks::parse(&data.to_string())?
        .try_into().map_err(|_| "one phase callback")?;
    Ok(callback)
}

fn lore_callback(lever: Position) -> TestResult<Callback> {
    let players: Vec<_> = (32018..=32020)
        .flat_map(|x| (32844..=32848).map(move |y| json!({"from":at(x,y), "to":at(31985,32851)})))
        .collect();
    let monsters = ASTRAL_ACTORS.map(|(creature, position)| json!({"creature":creature,"position":position}));
    let data = json!({"schema":1,"callbacks":[{
        "key":format!("oteryn:quest-callback/{LORE}"),"quest":"oteryn:quest.forgotten_knowledge_quest",
        "trigger":"use","selector":{"position":lever,"appearance":8911},
        "reversible":false,"passages":[],"storage":[],"offerings":[],"return_on_failure":false,
        "effects":{"encounter":{
            "minimum_level":0,"players":players,"monsters":monsters,
            "room_min":at(31968,32821),"room_max":below(32004,32865),"shared_staging":true,
            "death_phases":{"terminal":KEEPER,"replacements":[{"defeated":GLYPH,"spawn":[{"creature":KEEPER}]}]}
        }},
        "sources":[
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/forgotten_knowledge/actions_the_last_lore_keeper.lua:1-7,50-74",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/forgotten_knowledge/creaturescripts_astral_glyph_death.lua:2-4"
        ]
    }]});
    let [callback] = oteryn_gameplay_quests::quest::world_callbacks::parse(&data.to_string())?
        .try_into().map_err(|_| "one Lore Keeper callback")?;
    Ok(callback)
}

pub(super) fn world(callback: &Callback, lever: Position, name: &str) -> TestResult<crate::map::boot::BundleWorld> {
    let extra = if name == ESSENCE { vec![at(33617,31023)] } else { Vec::new() };
    super::quest_world_callbacks::world_with_cells(callback, lever, &extra)
}

async fn live(owner: &ComposedFreshAdmission<'_, '_, '_>, creature: &str) -> TestResult<Vec<ExactActorRef>> {
    let runtime = owner.runtime.lock().await;
    Ok(runtime.current_live_creature_census().map_err(debug)?.into_iter()
        .filter(|actor| runtime.matches_live_creature_identity(*actor, creature.as_bytes())).collect())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    callback: Callback,
    lever: Position,
    name: &str,
) -> TestResult {
    super::questpc7_native_deaths::prepare_native_hunter(owner, admitted).await?;
    owner.states.quest_callbacks.fixture.set(vec![callback]).map_err(|_| "callback already installed")?;
    super::questpc7_ferumbras::step(owner, admitted, StepDirection::East).await?;
    super::questpc7_ferumbras::step(owner, admitted, StepDirection::South).await?;
    assert_eq!(super::questpc7_ferumbras::use_at(owner, admitted, lever, 8911).await?, UseDisposition::Committed);
    if name == EARTH {
        let [mounted] = live(owner, MOUNTED).await?.try_into().map_err(|_| "one mounted knight")?;
        super::engenc2_cults::kill(owner, admitted, mounted, MOUNTED, 101).await?;
        let [shielded] = live(owner, SHIELDED).await?.try_into().map_err(|_| "one shielded knight")?;
        assert_eq!(live(owner, STEED).await?.len(), 1);
        assert!(live(owner, ENRAGED).await?.is_empty());
        super::engenc2_cults::kill(owner, admitted, shielded, SHIELDED, 102).await?;
        assert_eq!(live(owner, ENRAGED).await?.len(), 1);
        assert_eq!(live(owner, STEED).await?.len(), 1);
    } else if name == LORE {
        let runtime = owner.runtime.lock().await;
        let census = runtime.current_live_creature_census().map_err(debug)?;
        for (creature, position) in ASTRAL_ACTORS {
            let matching: Vec<_> = census.iter().filter(|actor| {
                runtime.matches_live_creature_identity(**actor, creature.as_bytes())
            }).collect();
            let [actor] = matching.as_slice() else { return Err("one actor per Lore role".into()); };
            assert_eq!(runtime.read_actor_position(**actor).map_err(debug)?.position(), super::quest_world_callbacks::local(position));
        }
        drop(runtime);
        let [glyph] = live(owner, GLYPH).await?.try_into().map_err(|_| "one astral glyph")?;
        super::engenc2_cults::kill(owner, admitted, glyph, GLYPH, 101).await?;
        let [keeper] = live(owner, KEEPER).await?.try_into().map_err(|_| "one Lore Keeper")?;
        let runtime = owner.runtime.lock().await;
        assert_eq!(runtime.read_actor_position(keeper).map_err(debug)?.position(), super::quest_world_callbacks::local(below(31989,32823)));
    } else {
        let cohort = live(owner, DRAGON_ESSENCE).await?;
        assert_eq!(cohort.len(), 6);
        super::engenc2_cults::kill(owner, admitted, cohort[0], DRAGON_ESSENCE, 101).await?;
        assert!(live(owner, DRAGON_ESSENCE).await?.is_empty());
        assert_eq!(live(owner, DRAGON).await?.len(), 1);
    }
    Ok(())
}
