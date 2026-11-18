//! Crystal 201189f8 data-global/scripts/quests/wrath_of_the_emperor/movements_prison.lua:3-27.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};
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
use std::sync::Arc;

pub(super) const PREFIX: &str = "qewrath-prison-";
const QUEST: &str = "oteryn:quest.wrath_of_the_emperor_quest";
const RELEASE: &str = "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/prison_release_status";
const CAUGHT: &str = "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/guardcaught_you";
const STAGE: &str = "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/questline";
const CRATE: &str = "oteryn:item.tibia.i11328";
const SOURCE: Position = Position {
    x: 33362,
    y: 31202,
    floor: -8,
};
const LANDING: Position = Position {
    x: 33359,
    y: 31183,
    floor: -8,
};
const DONOR: &str =
    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/wrath_of_the_emperor/movements_prison.lua:3-27";

#[test]
fn prison_release_restores_outfit_and_returns_the_crate_only_during_mission_one() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("qewrath-prison-release", || {
        for name in ["qewrath-prison-with-crate", "qewrath-prison-later"] {
            super::quest_world_callbacks::scenario(name, SOURCE, false)?;
        }
        Ok(())
    })
}

fn callback(with_crate: bool) -> serde_json::Value {
    let stage = if with_crate { 2 } else { 3 };
    let mut row = json!({
        "key": format!("oteryn:quest-callback/wrath/prison-release-{stage}"), "quest": QUEST,
        "trigger": "step_in", "selector": {"position": SOURCE, "ordinal": 0},
        "reversible": false, "passages": [], "offerings": [],
        "teleport": LANDING, "return_on_failure": false, "requires_world_admission": true,
        "storage": [
            {"track":RELEASE,"minimum":1,"maximum":1,"set":0},
            {"track":CAUGHT,"minimum":-1,"maximum":1,"set":-1},
            {"track":STAGE,"minimum":stage,"maximum":stage,"set":stage,"preserve":true},
        ],
        "effects": {"restore_disguise": true, "teleport_effect": 11,
            "magic": {"position": SOURCE, "effect": 11}},
        "sources": [DONOR],
    });
    if with_crate {
        row["effects"]["item_reward"] = json!({"item": CRATE, "count": 1, "contents": []});
        row["effects"]["item_reward_per_use"] = true.into();
    }
    row
}

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    let row: Callback = serde_json::from_value(callback(true))?;
    super::quest_world_callbacks::world(&row, SOURCE)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    with_crate: bool,
) -> TestResult {
    let stages = [true, false].map(|with_crate| {
        json!({
            "key": if with_crate { "prison-with-crate" } else { "prison-later" },
            "completes": false, "trigger": {"kind":"step_in","callback":callback(with_crate)},
        })
    });
    let digest = Sha256::digest(serde_json::to_vec(&stages)?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let fixture = json!({"schema":1,"quests":[{
        "quest":QUEST,"recipe_sha256":digest,"sources":[DONOR],
        "tracks":[
            {"key":RELEASE,"initial":1,"minimum":0,"maximum":1},
            {"key":CAUGHT,"initial":1,"minimum":-1,"maximum":1},
            {"key":STAGE,"initial":if with_crate {2} else {3},"minimum":2,"maximum":3},
        ],"stages":stages,
    }]});
    let bindings = Bindings::parse(&serde_json::to_vec(&fixture)?)?;
    let empty = QuestStateCatalogue::empty("content-1").ok_or("quest catalogue")?;
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&empty)?));
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    {
        let member = owner
            .active_generation
            .and_then(|g| g.native_gameplay())
            .and_then(|n| n.spell_appearances())
            .and_then(|a| a.for_look_type(1593))
            .ok_or("test outfit")?;
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        let state = states
            .get_mut(&runtime, actor, admitted.game_session_id)
            .ok_or("player")?;
        crate::spell::actor_conditions::apply_appearance(
            state,
            member.selection(),
            member.look_type(),
            600000,
            owner.owner_now().get() / 1000,
        )
        .map_err(debug)?;
        state.advance_batch_revision().map_err(debug)?;
    }
    let command = admitted.continuity.next_command_id;
    let payload = encode_step_intent(StepDirection::East);
    let first = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        payload.clone(),
    )
    .await?;
    assert_eq!(
        decode_step_result(&first).map_err(debug)?,
        StepDisposition::Moved
    );
    {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        assert_eq!(
            runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            super::quest_world_callbacks::local(LANDING)
        );
        let state = states
            .get(&runtime, actor, admitted.game_session_id)
            .ok_or("player")?;
        assert!(
            state
                .owned_conditions()
                .displayed_temporary_appearance_at(owner.owner_now().get())
                .is_none()
        );
    }
    let replay = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        payload,
    )
    .await?;
    assert_eq!(replay, first);
    for (track, expected) in [(RELEASE, 0), (CAUGHT, -1)] {
        let value: i64 =
            sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
                .bind(track)
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(value, expected);
    }
    let quantity: i64 = sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
        .bind(CRATE).fetch_one(&h.pool).await?;
    assert_eq!(quantity, i64::from(with_crate));
    Ok(())
}
