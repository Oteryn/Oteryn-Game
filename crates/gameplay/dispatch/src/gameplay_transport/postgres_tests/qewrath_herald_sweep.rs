//! Crystal 201189f8 movements_herald_red_floors.lua:31–39,59–83.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::Bindings,
    world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{sync::Arc, time::Duration};

pub(super) const NAME: &str = "qewrath-herald-sweep";
pub(super) const FIELD_NAME: &str = "qewrath-herald-sweep-fields";
const QUEST: &str = "oteryn:quest.targuna_quest";
const MISSION: &str =
    "oteryn:quest-progress/crystalserver/quest/u15_24/targuna/burning_heart/mission";
const FIELD_CASTER: &str = "oteryn:creature.fire_elemental";
const BAKED_FIELD: Position = Position {
    y: 32653,
    ..LAST_PLATE
};
const HERALD: &str = "oteryn:creature.herald_of_fire";
const SUMMON: &str = "oteryn:creature.infernoid_blob";
const LAST_PLATE: Position = Position {
    x: 32496,
    y: 32663,
    floor: -15,
};
const FIRST_PLATE: Position = Position {
    y: 32649,
    ..LAST_PLATE
};
const SECOND_PLATE: Position = Position {
    x: 32503,
    y: 32656,
    ..LAST_PLATE
};

#[test]
fn herald_last_plate_resets_the_room_monsters_and_preserves_the_herald() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, LAST_PLATE, false)
    })
}

#[test]
fn herald_last_plate_clears_a_real_native_field_and_the_room_summon() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(FIELD_NAME, || {
        super::quest_world_callbacks::scenario(FIELD_NAME, LAST_PLATE, false)
    })
}

pub(super) fn fixture() -> TestResult<Callback> {
    Ok(serde_json::from_value(json!({
        "key":"oteryn:quest-callback/targuna/herald-room-sweep",
        "quest":QUEST,"trigger":"step_in",
        "selector":{"position":LAST_PLATE,"appearance":53096},
        "reversible":false,"offerings":[],"return_on_failure":false,"storage":[],
        "world_requirements":[
            {"position":FIRST_PLATE,"appearance":53095},
            {"position":SECOND_PLATE,"appearance":53095}
        ],
        "passages":[
            {"position":FIRST_PLATE,"appearance":53095,"replacement":53096},
            {"position":SECOND_PLATE,"appearance":53095,"replacement":53096}
        ],
        "effects":{"sweep_creatures":{
            "minimum":{"x":32484,"y":32644,"floor":-15},
            "maximum":{"x":32508,"y":32668,"floor":-15},
            "except":HERALD
        }},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/targuna/movements_herald_red_floors.lua:31-83"]
    }))?)
}

pub(super) fn world(fields: bool) -> TestResult<crate::map::boot::BundleWorld> {
    let callback = fixture()?;
    if fields {
        super::quest_world_callbacks::world_with_objects(
            &callback,
            LAST_PLATE,
            &[(BAKED_FIELD, 2120)],
            &[2120],
        )
    } else {
        super::quest_world_callbacks::world(&callback, LAST_PLATE)
    }
}

fn field_catalogue() -> TestResult<QuestStateCatalogue> {
    let mut callback = serde_json::to_value(fixture()?)?;
    callback["requires_world_admission"] = true.into();
    callback["storage"] = json!([{
        "track":MISSION,"minimum":0,"maximum":5,"set":0,"preserve":true
    }]);
    callback["effects"]["sweep_fields"] = json!({
        "minimum":{"x":32487,"y":32649,"floor":-15},
        "maximum":{"x":32504,"y":32664,"floor":-15},
        "appearances":[2118,2119,2120]
    });
    let digest = Sha256::digest(serde_json::to_vec(&callback)?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let input = json!({"schema":1,"quests":[{
        "quest":QUEST,"recipe_sha256":digest,"sources":callback["sources"],
        "tracks":[{"key":MISSION,"initial":2,"minimum":0,"maximum":5}],
        "stages":[{"key":"sweep","completes":false,"trigger":{"kind":"step_in","callback":callback}}]
    }]});
    let empty = QuestStateCatalogue::empty("content-1").ok_or("catalogue")?;
    Ok(Bindings::parse(&serde_json::to_vec(&input)?)?.attach(&empty)?)
}

async fn make_field(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult<sqlx::types::Uuid> {
    let actor = admitted.runtime_actor.ok_or("field target")?;
    let caster = {
        let mut runtime = owner.runtime.lock().await;
        let native = owner
            .active_generation
            .and_then(|g| g.native_gameplay())
            .ok_or("native")?;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        let mut position = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        position.y -= 2;
        let health = runtime
            .companion_policy(FIELD_CASTER)
            .map_err(debug)?
            .maximum_health;
        let caster = runtime
            .admit_encounter_actor(FIELD_CASTER, health, position)
            .map_err(debug)?;
        runtime
            .install_creature_policy(caster, FIELD_CASTER)
            .map_err(debug)?;
        let base = crate::gameplay_transport::actor_spell::tests::FACTS;
        owner
            .states
            .spell_states
            .lock()
            .await
            .initialize(
                &runtime,
                actor,
                admitted.game_session_id,
                crate::spell::cast::CharacterCastFacts {
                    max_health: base.max_health.checked_mul(128).ok_or("fixture health")?,
                    ..base
                },
                (0, 0),
                owner.owner_now(),
            )
            .ok_or("player vitals")?;
        caster
    };
    let mut field = None;
    for _ in 0..128 {
        owner.clock_origin -= Duration::from_secs(2);
        owner.drain_monster_melee().await;
        field = sqlx::query_scalar(
            "SELECT i.item_instance_id FROM game_item_instances i
            JOIN game_item_ground_locations g USING(item_instance_id,world_id)
            WHERE i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i2118'
              AND game_quest_dynamic_field_placement(i,g) ORDER BY g.stack_ordinal LIMIT 1",
        )
        .fetch_optional(&h.pool)
        .await?;
        if field.is_some() {
            break;
        }
    }
    owner
        .runtime
        .lock()
        .await
        .remove_encounter_actor(caster)
        .map_err(debug)?;
    field.ok_or_else(|| "source elemental produced no current native fire field".into())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    fields: bool,
) -> TestResult {
    let baked_key = crate::map::view::placement_key(
        TilePos {
            x: BAKED_FIELD.x,
            y: BAKED_FIELD.y,
            floor: BAKED_FIELD.floor,
        },
        1,
    )
    .ok_or("baked field")?;
    let field = if fields {
        assert!(
            !owner
                .bundle_map()
                .ok_or("room map")?
                .facts
                .base_hidden(baked_key)
        );
        owner.quest_catalogue = Some(Arc::new(field_catalogue()?));
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        Some(make_field(owner, admitted, h).await?)
    } else {
        owner
            .states
            .quest_callbacks
            .fixture
            .set(vec![fixture()?])
            .map_err(|_| "arena fixture")?;
        None
    };
    let targets = {
        let mut runtime = owner.runtime.lock().await;
        let native = owner
            .active_generation
            .and_then(|g| g.native_gameplay())
            .ok_or("native")?;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        let mut spawn = |creature: &str, position| {
            let hp = runtime.companion_policy(creature)?.maximum_health;
            runtime.admit_encounter_actor(
                creature,
                hp,
                super::quest_world_callbacks::local(position),
            )
        };
        [
            spawn(
                HERALD,
                Position {
                    y: 32656,
                    ..LAST_PLATE
                },
            )
            .map_err(debug)?,
            spawn(
                SUMMON,
                Position {
                    y: 32655,
                    ..LAST_PLATE
                },
            )
            .map_err(debug)?,
            spawn(
                SUMMON,
                Position {
                    x: 32509,
                    y: 32656,
                    ..LAST_PLATE
                },
            )
            .map_err(debug)?,
        ]
    };
    let command = admitted.continuity.next_command_id;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(StepDirection::East),
    )
    .await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    let runtime = owner.runtime.lock().await;
    assert!(runtime.contains_live_creature(targets[0]));
    assert!(!runtime.contains_live_creature(targets[1]));
    assert!(runtime.contains_live_creature(targets[2]));
    drop(runtime);
    if let Some(field) = field {
        assert!(
            owner
                .bundle_map()
                .ok_or("room map")?
                .facts
                .base_hidden(baked_key)
        );
        let lifecycle: i16 = sqlx::query_scalar(
            "SELECT lifecycle FROM game_item_instances WHERE item_instance_id=$1",
        )
        .bind(field)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(lifecycle, 2);
        let consumed: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions
            WHERE item_instance_id=$1 AND transition_key='oteryn:quest-callback/targuna/herald-room-sweep'")
            .bind(field).fetch_one(&h.pool).await?;
        assert_eq!(consumed, 1);
    }
    Ok(())
}
