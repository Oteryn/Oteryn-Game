//! Crystal 201189f8 quest_system2.lua:119-127 and movements_mission2_all_around_the_world.lua:1-5,73-96.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, unnatural_selection,
    world_callbacks::{self, Callback, Position},
};
use oteryn_protocol_oteryn::{
    world_object::UseDisposition,
    world_spatial::{self, StepDirection, StepDisposition},
};
use serde_json::{Value, json};
use std::sync::Arc;

pub(super) const NAME: &str = "quest-skull-custody";
const MISSION01: &str = "oteryn:quest-progress/quest/u8_54/unnatural_selection/mission01";
const MISSION02: &str = "oteryn:quest-progress/quest/u8_54/unnatural_selection/mission02";
const SKULL: &str = "oteryn:item.tibia.i10159";
pub(super) const FIRST_VIEW: Position = Position {
    x: 33263,
    y: 31834,
    floor: -1,
};

#[test]
fn quest_bone_pile_mints_exact_skull_and_step_requires_its_carried_custody() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, FIRST_VIEW, false)
    })
}

fn rows() -> [Value; 2] {
    [
        json!({
            "key":"oteryn:quest-callback/fixture-skull-pile",
            "quest":unnatural_selection::QUEST, "trigger":"use",
            "selector":{"unique":9259},
            "lever":null,"reversible":false,"passages":[],"teleport":null,"offerings":[],
            "item_on_target":null,"return_on_failure":false,"failure_message":null,
            "storage":[{"track":MISSION01,"minimum":1,"maximum":1,"set":2}],
            "effects":{"item_reward":{"item":SKULL,"count":1}},
            "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/actions/other/others/quest_system2.lua:119-127"]
        }),
        json!({
            "key":"oteryn:quest-callback/fixture-skull-first-view",
            "quest":unnatural_selection::QUEST, "trigger":"step_in",
            "selector":{"position":FIRST_VIEW},
            "lever":null,"reversible":false,"passages":[],"teleport":null,"offerings":[],
            "item_on_target":null,"return_on_failure":false,"failure_message":null,
            "storage":[{"track":MISSION02,"minimum":1,"maximum":1,"set":2}],
            "present":[{"item":SKULL,"count":1}],
            "effects":{"player_speech":"You hold the skull up high. You can resist the urge to have it look through a telescope though."},
            "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/unnatural_selection/movements_mission2_all_around_the_world.lua:1-5","crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/unnatural_selection/movements_mission2_all_around_the_world.lua:73-96"]
        }),
    ]
}

pub(super) fn bone_pile() -> TestResult<Callback> {
    Ok(serde_json::from_value(rows()[0].clone())?)
}

fn catalogue(base: &QuestStateCatalogue) -> TestResult<QuestStateCatalogue> {
    let document = json!({"schema":1,"callbacks":rows()}).to_string();
    Ok(world_callbacks::with_fixture_callbacks(
        base.clone(),
        &document,
    )?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    owner.quest_catalogue = Some(Arc::new(catalogue(
        owner.quest_catalogue.as_deref().ok_or("quest catalogue")?,
    )?));
    super::qengine_outfit_stories::seed_quest_prerequisites(
        h,
        owner,
        admitted,
        &[(MISSION01.into(), 1), (MISSION02.into(), 1)],
        125,
    )
    .await?;
    step(owner, admitted, StepDirection::East).await?;
    assert_eq!(super::m39_bone_journey::stage(h, MISSION02).await?, Some(1));
    step(owner, admitted, StepDirection::West).await?;

    let (result, command, payload, receipt) =
        super::m39_bone_journey::use_at(owner, admitted, FIRST_VIEW).await?;
    assert_eq!(result, UseDisposition::Committed);
    assert_eq!(super::m39_bone_journey::stage(h, MISSION01).await?, Some(2));
    assert_eq!(super::m39_bone_journey::carried(h, 10159).await?, 1);
    assert_eq!(
        super::quest_world_callbacks::wire(owner, admitted, command, 2, payload).await?,
        receipt,
    );
    assert_eq!(super::m39_bone_journey::carried(h, 10159).await?, 1);

    step(owner, admitted, StepDirection::East).await?;
    assert_eq!(super::m39_bone_journey::stage(h, MISSION02).await?, Some(2));
    assert_eq!(super::m39_bone_journey::carried(h, 10159).await?, 1);
    step(owner, admitted, StepDirection::West).await?;
    step(owner, admitted, StepDirection::East).await?;
    assert_eq!(super::m39_bone_journey::stage(h, MISSION02).await?, Some(2));
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
    let command = admitted.continuity.next_command_id;
    let receipt = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        world_spatial::encode_step_intent(direction),
    )
    .await?;
    assert_eq!(
        world_spatial::decode_step_result(&receipt).map_err(debug)?,
        StepDisposition::Moved
    );
    Ok(())
}
