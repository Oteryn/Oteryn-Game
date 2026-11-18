//! Crystal 201189f8, ferumbras_ascension/movements_lovely_monsters.lua:1-28.
use super::{ComposedFreshAdmission, TestResult, fixture::Harness, fixture::debug};
use crate::foundation::ExactActorRef;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};

pub(super) const CASE: &str = "ferumbras-zamulosh-lovely-monster-step";
const FRAZZLEMAW: &str = "oteryn:creature.lovely_frazzlemaw";
const DEER: &str = "oteryn:creature.lovely_deer";
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

pub(super) fn callback() -> TestResult<Callback> {
    let data = serde_json::json!({"schema":1,"callbacks":[{
        "key":"oteryn:quest-callback/ferumbras/zamulosh/lovely-frazzlemaw",
        "quest":"oteryn:quest.ferumbras_ascension_quest",
        "trigger":"step_in","actor_kind":"monster",
        "selector":{"position":PORTAL,"action":34315},
        "reversible":false,"passages":[],"storage":[],"offerings":[],"return_on_failure":false,
        "effects":{"monster_step":{"creature":FRAZZLEMAW}},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/ferumbras_ascension/movements_lovely_monsters.lua:1-28"]
    }]});
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
    let approach = super::quest_world_callbacks::local(Position {
        y: PORTAL.y - 1,
        ..PORTAL
    });
    for (creature, survives) in [(DEER, true), (FRAZZLEMAW, false)] {
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
        step_monster(owner, h, actor).await?;
        let mut runtime = owner.runtime.lock().await;
        assert_eq!(runtime.contains_live_creature(actor), survives);
        if survives {
            runtime.remove_encounter_actor(actor).map_err(debug)?;
        }
    }
    Ok(())
}

async fn step_monster(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    actor: ExactActorRef,
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
    let to = super::quest_world_callbacks::local(PORTAL);
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
