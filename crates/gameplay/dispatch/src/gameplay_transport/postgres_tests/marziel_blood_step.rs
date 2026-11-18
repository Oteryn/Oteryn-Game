//! Crystal 201189f8 movements_marziel_blood.lua:3-43.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::{self, Callback, Position};
use oteryn_protocol_oteryn::world_spatial::{self, StepDirection};
use serde_json::{Value, json};

pub(super) const NAME: &str = "marziel-blood-step";
const QUEST: &str = "oteryn:quest.blood_brothers_quest";
const MISSION: &str = "oteryn:quest-progress/quest/u8_4/blood_brothers/mission09";
const BRIDE: &str = "oteryn:creature.vampire_bride";
const TRIGGER: Position = Position {
    x: 32940,
    y: 31458,
    floor: -2,
};
const STATUE: Position = Position {
    x: 32940,
    y: 31457,
    floor: -2,
};

#[test]
fn marziel_step_requires_female_character_and_exact_blood_before_spawning() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        for name in ["marziel-blood-step-male", "marziel-blood-step-female"] {
            super::quest_world_callbacks::scenario(name, TRIGGER, false)?;
        }
        Ok(())
    })
}

fn binding() -> Value {
    json!({
        "key":"oteryn:quest-callback/fixture-marziels-blood",
        "quest":QUEST,"trigger":"step_in",
        "selector":{"position":TRIGGER,"appearance":8695,"native":{"position":TRIGGER,"ordinal":1,"appearance":8695}},
        "lever":null,"reversible":false,"teleport":null,"offerings":[],
        "passages":[{"position":STATUE,"appearance":8325,"replacement":8326}],
        "item_on_target":null,"return_on_failure":false,"failure_message":null,
        "requires_world_admission":true,
        "character_requirements":{"sex":"female"},
        "world_requirements":[{"position":STATUE,"appearance":8325}],
        "storage":[{"track":MISSION,"minimum":-1,"maximum":1,"set":1,"preserve":true}],
        "effects":{"player_speech":"AAAAH... THE SCENT OF A WOMAN... GIVE ME MORE...",
            "first_carried_input_only":true,
            "carried_inputs":[{"item":"oteryn:item.tibia.i2874","count":1,"fluid_subtype":5}],
            "stationary_encounter":[
                {"creature":BRIDE,"position":{"x":32938,"y":31458,"floor":-2}},
                {"creature":BRIDE,"position":{"x":32942,"y":31458,"floor":-2}}
            ]},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/blood_brothers_quest/movements_marziel_blood.lua:3-43"]
    })
}

pub(super) fn callback() -> TestResult<Callback> {
    Ok(serde_json::from_value(binding())?)
}

pub(super) fn world(row: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    super::quest_world_callbacks::world_with_objects(row, TRIGGER, &[(STATUE, 8325)], &[8326])
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    female: bool,
) -> TestResult {
    let base = owner.quest_catalogue.as_deref().ok_or("quest catalogue")?;
    let fixture = json!({"schema":1,"callbacks":[binding()]}).to_string();
    owner.quest_catalogue = Some(std::sync::Arc::new(
        world_callbacks::with_fixture_callbacks(base.clone(), &fixture)?,
    ));
    super::qengine_outfit_stories::seed_quest_prerequisites(
        h,
        owner,
        admitted,
        &[(MISSION.into(), 1)],
        120,
    )
    .await?;
    // Crystal's breadth-first lookup checks the newest pouch first in the main bag's
    // display order. The third step must leave the older blood vial untouched.
    for (index, subtypes) in [&[1][..], &[5, 1], &[5]].into_iter().enumerate() {
        for (offset, &subtype) in subtypes.iter().enumerate() {
            super::m42_hot_cuisine::typed_fluids(
                owner,
                admitted,
                h,
                authority,
                (2874, subtype, 1),
                u64::try_from(2 * index + offset)?,
            )
            .await?;
        }
        let newest_vial: String = sqlx::query_scalar(
            "SELECT n.item_instance_id::text FROM game_item_nested_entries n
             JOIN game_item_container_entries pouch ON pouch.item_instance_id=n.parent_item_instance_id
             JOIN game_item_fluid_states f ON f.item_instance_id=n.item_instance_id
             ORDER BY pouch.placement_ordinal DESC LIMIT 1",
        )
        .fetch_one(&h.pool)
        .await?;
        step(owner, admitted, StepDirection::East).await?;
        let should_spawn = female && index == 2;
        let runtime = owner.runtime.lock().await;
        let count = runtime
            .visible_entities()
            .creatures
            .into_iter()
            .filter(|row| runtime.matches_live_creature_identity(row.actor, BRIDE.as_bytes()))
            .count();
        assert_eq!(count, if should_spawn { 2 } else { 0 });
        drop(runtime);
        let consumed: Vec<String> = sqlx::query_scalar("SELECT item_instance_id::text FROM game_quest_callback_consumptions WHERE transition_key='oteryn:quest-callback/fixture-marziels-blood'")
            .fetch_all(&h.pool).await?;
        assert_eq!(consumed.as_slice(), if should_spawn { std::slice::from_ref(&newest_vial) } else { &[] });
        if index < 2 {
            step(owner, admitted, StepDirection::West).await?;
        }
    }
    Ok(())
}

async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    direction: StepDirection,
) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(5))
        .ok_or("clock")?;
    let id = admitted.continuity.next_command_id;
    let result = super::quest_world_callbacks::wire(
        owner,
        admitted,
        id,
        world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        world_spatial::encode_step_intent(direction),
    )
    .await?;
    assert_eq!(
        world_spatial::decode_step_result(&result).map_err(super::fixture::debug)?,
        world_spatial::StepDisposition::Moved
    );
    Ok(())
}
