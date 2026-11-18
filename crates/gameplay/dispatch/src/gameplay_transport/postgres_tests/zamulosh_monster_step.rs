//! Crystal 201189f8, ferumbras_ascension/movements_lovely_monsters.lua:1-28.
use super::{ComposedFreshAdmission, TestResult, fixture::Harness, fixture::debug};
use crate::foundation::ExactActorRef;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};

pub(super) const CASE: &str = "ferumbras-zamulosh-lovely-monster-step";
pub(super) const BLOSSOM: &str = "first-dragon-fertility-blossom-step";
const FRAZZLEMAW: &str = "oteryn:creature.lovely_frazzlemaw";
const DEER: &str = "oteryn:creature.lovely_deer";
const SPIRIT: &str = "oteryn:creature.spirit_of_fertility";
const PLANT: &str = "oteryn:creature.angry_plant";
const MATCHED_CUE: &str = "That seemed correct!";
const THUNDER: u16 = 73; // Crystal src/utils/utils_definitions.hpp:128.
// Mount the source UID on a tile in the finale fixture. Crystal selects its
// appearance and UID, while the map asset determines the coordinates.
const FLOWER: Position = Position {
    x: 33625,
    y: 31022,
    floor: -14,
};
// Crystal config selects AID 34315 and checks its original creature. The mounted
// World places that AID beside its source spectator centre (33618, 32666, 12).
const PORTAL: Position = Position {
    x: 33619,
    y: 32666,
    floor: -12,
};

#[test]
fn zamulosh_monster_step_retires_only_the_creature_paired_with_the_portal() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(CASE, || {
        super::quest_world_callbacks::scenario(CASE, PORTAL, false)
    })
}

#[test]
fn spirit_of_fertility_replaces_itself_and_consumes_only_the_source_dry_flower() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(BLOSSOM, || {
        super::quest_world_callbacks::scenario(BLOSSOM, FLOWER, false)
    })
}

pub(super) fn callback(name: &str) -> TestResult<Callback> {
    // Crystal 201189f8, the_first_dragon/movements_heaven_blossom.lua:3-20.
    let row = if name == BLOSSOM {
        serde_json::json!({
            "key":"oteryn:quest-callback/first-dragon/fertility-blossom",
            "quest":"oteryn:quest.the_first_dragon_quest",
            "trigger":"step_in","actor_kind":"monster",
            "selector":{"position":FLOWER,"unique":1066,"appearance":3657},
            "reversible":false,"passages":[],"storage":[],"offerings":[],"return_on_failure":false,
            "effects":{"monster_step":{"creature":SPIRIT,"replace_with":PLANT,"consume_source":true}},
            "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_first_dragon/movements_heaven_blossom.lua:3-20"]
        })
    } else {
        serde_json::json!({
            "key":"oteryn:quest-callback/ferumbras/zamulosh/lovely-frazzlemaw",
            "quest":"oteryn:quest.ferumbras_ascension_quest",
            "trigger":"step_in","actor_kind":"monster",
            "selector":{"action":34315},
            "reversible":false,"passages":[],"storage":[],"offerings":[],"return_on_failure":false,
            "effects":{"monster_step":{"creature":FRAZZLEMAW,
                "retirement_cue":{"text":MATCHED_CUE,"effect":THUNDER}}},
            "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/ferumbras_ascension/movements_lovely_monsters.lua:1-49"]
        })
    };
    let data = serde_json::json!({"schema":1,"callbacks":[row]});
    let [callback] = oteryn_gameplay_quests::quest::world_callbacks::parse(&data.to_string())?
        .try_into()
        .map_err(|_| "one Zamulosh callback")?;
    Ok(callback)
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    callback: Callback,
) -> TestResult {
    use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
    let position = callback.selector.position.unwrap_or(PORTAL);
    owner
        .states
        .quest_callbacks
        .fixture
        .set(vec![callback])
        .map_err(|_| "callback installed")?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native source")?;
    native
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    let map = owner.bundle_map().ok_or("map")?;
    let candidates = if position == FLOWER {
        vec![(DEER, position, true), (SPIRIT, position, false)]
    } else {
        let adjacent = Position {
            x: position.x - 1,
            ..position
        };
        vec![
            (FRAZZLEMAW, adjacent, true),
            (DEER, position, true),
            (FRAZZLEMAW, position, false),
        ]
    };
    for (creature, destination, survives) in candidates {
        let approach = super::quest_world_callbacks::local(Position {
            y: destination.y - 1,
            ..destination
        });
        let actor = {
            let mut runtime = owner.runtime.lock().await;
            let policy = runtime.companion_policy(creature).map_err(debug)?;
            let actor = runtime
                .admit_encounter_actor(creature, policy.maximum_health, approach)
                .map_err(debug)?;
            runtime
                .install_creature_policy(actor, creature)
                .map_err(debug)?;
            actor
        };
        step_monster(owner, h, actor, destination).await?;
        let mut runtime = owner.runtime.lock().await;
        assert_eq!(runtime.contains_live_creature(actor), survives);
        if position == PORTAL {
            let speech: Vec<_> = map
                .facts
                .movement_speech_since(0)
                .into_iter()
                .filter(|(_, cue)| cue.speaker == actor)
                .collect();
            if survives {
                assert!(speech.is_empty());
            } else {
                let [(_, cue)] = speech.as_slice() else {
                    return Err("expected one paired-creature cue".into());
                };
                assert_eq!(cue.text, MATCHED_CUE);
                assert_eq!(cue.talk_type, 36);
                assert_eq!(
                    (cue.position.x, cue.position.y, cue.position.floor),
                    (position.x, position.y, position.floor)
                );
                assert!(cue.captured_name.is_some());
            }
        }
        if survives {
            runtime.remove_encounter_actor(actor).map_err(debug)?;
        }
    }
    if position == PORTAL {
        assert_eq!(
            map.facts
                .movement_effects_since(0)
                .into_iter()
                .filter(|(_, _, effect)| *effect == THUNDER)
                .map(|(_, at, _)| (at.x, at.y, at.floor))
                .collect::<Vec<_>>(),
            [(PORTAL.x, PORTAL.y, PORTAL.floor)]
        );
    }
    if position == FLOWER {
        use crate::map::view::MapFacts;
        let map = owner.bundle_map().ok_or("map")?;
        let tile = crate::map::overlay::TilePos {
            x: FLOWER.x,
            y: FLOWER.y,
            floor: FLOWER.floor,
        };
        let key = crate::map::view::placement_key(tile, 1).ok_or("dry flower key")?;
        assert!(map.facts.base_hidden(key));
        let runtime = owner.runtime.lock().await;
        let plants: Vec<_> = runtime
            .current_live_creature_census()
            .map_err(debug)?
            .into_iter()
            .filter(|actor| runtime.matches_live_creature_identity(*actor, PLANT.as_bytes()))
            .collect();
        let [plant] = plants.try_into().map_err(|_| "one angry plant")?;
        assert_eq!(
            runtime
                .read_actor_position(plant)
                .map_err(debug)?
                .position(),
            super::quest_world_callbacks::local(FLOWER)
        );
    }
    Ok(())
}

pub(super) async fn step_monster(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    actor: ExactActorRef,
    position: Position,
) -> TestResult {
    use crate::durability::spell_item_transaction::assert_spell_item_scope_in_transaction;
    use crate::movement::{CardinalStep, monster_map::CurrentMonsterMap};
    let permit = owner.spell_lane_permit().await.ok_or("creature map lane")?;
    let mut runtime = owner.runtime.lock().await;
    let before = runtime.read_actor_position(actor).map_err(debug)?;
    let room = owner
        .effective_rooms()
        .gate_monster_ai()
        .ok_or("creature room")?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native source")?;
    let objects = owner.door.lock().await;
    let mut tx = h.pool.begin().await?;
    let authority = assert_spell_item_scope_in_transaction(
        &mut tx,
        &permit,
        owner.root,
        owner.character,
        owner.holder,
        runtime.binding().runtime_scope(),
        runtime.binding().scope_generation().get(),
    )
    .await
    .map_err(debug)?;
    let mut map = CurrentMonsterMap::new(&mut tx, &authority, &runtime, room, &objects, native);
    let to = super::quest_world_callbacks::local(position);
    assert_eq!(map.read(&runtime, to).await, Some(true));
    let moved = map
        .view()
        .step(
            &mut runtime,
            room.movement_cells(),
            actor,
            before,
            CardinalStep::South,
        )
        .map_err(debug)?;
    assert_eq!(moved.position(), to);
    drop(map);
    tx.rollback().await?;
    let states = owner.states.spell_states.lock().await;
    crate::gameplay_transport::paced_movement::monster_trap::apply_monster_trap_steps(
        &mut runtime,
        &states,
        owner.bundle_map().ok_or("map")?,
        owner.owner_now().get(),
        owner.callback_content(),
    );
    Ok(())
}
