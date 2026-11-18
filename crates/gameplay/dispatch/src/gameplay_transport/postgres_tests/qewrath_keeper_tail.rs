//! Crystal 201189f8 actions_mission03_the_keeper.lua:17–21.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};
use crate::gameplay_transport::{
    connection::{FreshAdmissionAuthority, UseCommand},
    item_view::ItemKey,
};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, authored_bindings::Bindings, world_callbacks::Position,
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent, UseTarget};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{num::NonZeroU64, sync::Arc};

pub(super) const NAME: &str = "qewrath-keeper-tail";
const QUEST: &str = "oteryn:quest.wrath_of_the_emperor_quest";
const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/questline";
const MISSION: &str = "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/mission03";
const KEEPER: &str = "oteryn:creature.the_keeper";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/wrath_of_the_emperor/actions_mission03_the_keeper.lua:17-21";
const BODY: Position = Position { x: 33171, y: 31058, floor: -11 };

#[test]
fn keeper_corpse_plain_use_gives_the_tail_and_advances_mission_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, BODY, false)
    })
}

fn catalogue() -> TestResult<QuestStateCatalogue> {
    let callback = json!({
        "key":"oteryn:quest-reward/wrath/keeper-tail","quest":QUEST,"npc":"",
        "keywords":[],"topic":0,"next_topic":0,"take":[],"outfit":null,"achievement":null,
        "give":{"item":"oteryn:item.tibia.i11367","count":1,"contents":[]},
        "reply":[],"sources":[SOURCE],"creature_target":KEEPER,
        "corpse_target":{"appearance":11360},
        "storage":[
            {"track":QUESTLINE,"minimum":7,"maximum":7,"set":8},
            {"track":MISSION,"minimum":0,"maximum":2,"set":2}
        ]
    });
    let digest = Sha256::digest(serde_json::to_vec(&callback)?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let input = json!({"schema":1,"quests":[{
        "quest":QUEST,"recipe_sha256":digest,"sources":[SOURCE],
        "tracks":[
            {"key":QUESTLINE,"initial":7,"minimum":0,"maximum":34},
            {"key":MISSION,"initial":1,"minimum":0,"maximum":2},
        ],
        "stages":[{"key":"keeper-tail","completes":false,"trigger":{"kind":"carried_use","callback":callback}}],
    }]});
    let empty = QuestStateCatalogue::empty("content-1").ok_or("catalogue")?;
    Ok(Bindings::parse(&serde_json::to_vec(&input)?)?.attach(&empty)?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    owner.quest_catalogue = Some(Arc::new(catalogue()?));
    super::questpc7_native_deaths::prepare_native_hunter(owner, admitted).await?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    let target = {
        let mut runtime = owner.runtime.lock().await;
        let health = runtime.companion_policy(KEEPER).map_err(debug)?.maximum_health;
        let target = runtime.admit_encounter_actor(KEEPER, health, super::quest_world_callbacks::local(BODY)).map_err(debug)?;
        runtime.install_creature_policy(target, KEEPER).map_err(debug)?;
        target
    };
    super::questpc7_native_deaths::lethal_creature(owner, admitted, target, 9150, KEEPER).await?;
    let corpse = {
        let runtime = owner.runtime.lock().await;
        runtime.visible_corpses().into_iter().find(|body| {
            body.item.production_key == "oteryn:item.tibia.i11360"
        }).ok_or("Keeper corpse projected")?
    };
    let intent = UseIntent {
        target: UseTarget::Item(NonZeroU64::MIN),
        use_with: None,
        use_with_position: None,
        use_with_map_item: None,
    };
    for (id, source, expected) in [
        (9160, Some(ItemKey::Entity(corpse.identity)), UseDisposition::Committed),
        (9160, None, UseDisposition::Committed),
        (9161, Some(ItemKey::Entity(corpse.identity)), UseDisposition::Rejected),
    ] {
        let fence = owner.current_quest_fence(session).await.map_err(debug)?.ok_or("fence")?;
        let outcome = owner.use_quest_tool(actor, UseCommand {
            game_session_id: session,
            command_id: id,
            item_fence: Some(fence.item_fence()),
        }, intent.clone(), source, None).await.map_err(debug)?.ok_or("Keeper corpse USE")?;
        assert_eq!(outcome.disposition, expected);
    }
    let rows: (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_quest_corpse_samples WHERE source_item IS NULL AND maximum_age_ms IS NULL),          (SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i11367' AND lifecycle=1),          (SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i11360' AND lifecycle=1)",
    ).fetch_one(&h.pool).await?;
    assert_eq!(rows, (1, 1, 1));
    for (track, expected) in [(QUESTLINE, 8), (MISSION, 2)] {
        let actual: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1").bind(track).fetch_one(&h.pool).await?;
        assert_eq!(actual, expected);
    }
    Ok(())
}
