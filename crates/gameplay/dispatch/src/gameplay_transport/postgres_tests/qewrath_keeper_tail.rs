//! Crystal 201189f8 actions_mission03_the_keeper.lua:19–23.
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
pub(super) const BROWSE_NAME: &str = "qewrath-keeper-tail-browse";
const QUEST: &str = "oteryn:quest.wrath_of_the_emperor_quest";
const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/questline";
const MISSION: &str = "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/mission03";
const KEEPER: &str = "oteryn:creature.the_keeper";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/wrath_of_the_emperor/actions_mission03_the_keeper.lua:19-23";
const BODY: Position = Position {
    x: 33171,
    y: 31058,
    floor: -11,
};

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

#[test]
fn keeper_corpse_browse_field_use_gives_the_same_tail_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(BROWSE_NAME, || {
        super::quest_world_callbacks::scenario(BROWSE_NAME, BODY, false)
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
    browse: bool,
) -> TestResult {
    owner.quest_catalogue = Some(Arc::new(catalogue()?));
    super::questpc7_native_deaths::prepare_native_hunter(owner, admitted).await?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    let target = {
        let mut runtime = owner.runtime.lock().await;
        let health = runtime
            .companion_policy(KEEPER)
            .map_err(debug)?
            .maximum_health;
        let target = runtime
            .admit_encounter_actor(KEEPER, health, super::quest_world_callbacks::local(BODY))
            .map_err(debug)?;
        runtime
            .install_creature_policy(target, KEEPER)
            .map_err(debug)?;
        target
    };
    super::questpc7_native_deaths::lethal_creature(owner, admitted, target, 9150, KEEPER).await?;
    let corpse = {
        let runtime = owner.runtime.lock().await;
        runtime
            .visible_corpses()
            .into_iter()
            .find(|body| body.item.production_key == "oteryn:item.tibia.i11360")
            .ok_or("Keeper corpse projected")?
    };
    if browse {
        let reference = owner
            .active_generation
            .and_then(|generation| generation.native_gameplay())
            .and_then(|native| native.item_index())
            .and_then(|index| {
                index.definition_ref(
                    &corpse.item.family,
                    &corpse.item.production_key,
                    &corpse.item.revision_ref,
                )
            })
            .ok_or("Keeper corpse item definition")?;
        use_browsed_corpse(owner, *admitted, reference).await?;
    } else {
        use_map_corpse(owner, actor, session, corpse.identity).await?;
    }
    let rows: (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_quest_corpse_samples WHERE source_item IS NULL AND maximum_age_ms IS NULL),          (SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i11367' AND lifecycle=1),          (SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i11360' AND lifecycle=1)",
    ).fetch_one(&h.pool).await?;
    assert_eq!(rows, (1, 1, 1));
    for (track, expected) in [(QUESTLINE, 8), (MISSION, 2)] {
        let actual: i64 =
            sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
                .bind(track)
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(actual, expected);
    }
    Ok(())
}

async fn use_map_corpse(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    corpse: [u8; 16],
) -> TestResult {
    let intent = UseIntent {
        target: UseTarget::Item(NonZeroU64::MIN),
        use_with: None,
        use_with_position: None,
        use_with_map_item: None,
    };
    for (id, source, expected) in [
        (
            9160,
            Some(ItemKey::Entity(corpse)),
            UseDisposition::Committed,
        ),
        (9160, None, UseDisposition::Committed),
        (
            9161,
            Some(ItemKey::Entity(corpse)),
            UseDisposition::Rejected,
        ),
    ] {
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(debug)?
            .ok_or("fence")?;
        let outcome = owner
            .use_quest_tool(
                actor,
                UseCommand {
                    game_session_id: session,
                    command_id: id,
                    item_fence: Some(fence.item_fence()),
                },
                intent.clone(),
                source,
                None,
            )
            .await
            .map_err(debug)?
            .ok_or("Keeper corpse USE")?;
        assert_eq!(outcome.disposition, expected);
    }
    Ok(())
}

async fn use_browsed_corpse(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    definition: std::num::NonZeroU32,
) -> TestResult {
    use super::browse_field::wire;
    use crate::gameplay_transport::{
        capabilities::SelectedCapabilities,
        connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted},
    };
    use oteryn_protocol_oteryn::{
        browse_field::{BrowseFieldIntent, CAPABILITY_BROWSE_FIELD_V1},
        item_view::WorldTilePosition,
        world_object::{COMMAND_TYPE_USE_INTENT, decode_use_result, encode_item_use_intent},
    };
    use tokio::io::AsyncWriteExt;

    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        owner.offered_capabilities(),
        &[4, 6, 12, 14, 15, 18, CAPABILITY_BROWSE_FIELD_V1],
    )
    .ok_or("browse and USE capabilities")?;
    let command = admitted.continuity.next_command_id;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let client_task = async move {
        wire::snapshot(&mut client).await?;
        let position = WorldTilePosition {
            x: i32::from(BODY.x),
            y: i32::from(BODY.y),
            floor: i16::from(BODY.floor),
        };
        wire::browse(&mut client, command, BrowseFieldIntent::Open { position }).await?;
        let views = wire::views(&mut client).await?;
        let handle = views
            .views
            .iter()
            .find(|view| view.browse.is_some_and(|page| page.position == position))
            .and_then(|view| {
                view.entries
                    .iter()
                    .find(|entry| entry.item_definition_ref == definition)
            })
            .ok_or("browsed Keeper corpse")?
            .handle;
        let payload = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(handle),
            use_with: None,
            use_with_position: None,
            use_with_map_item: None,
        })
        .map_err(debug)?;
        for (id, expected) in [
            (command + 1, UseDisposition::Committed),
            (command + 2, UseDisposition::Rejected),
        ] {
            wire::send(&mut client, id, COMMAND_TYPE_USE_INTENT, &payload).await?;
            assert_eq!(
                decode_use_result(&wire::result(&mut client, id).await?).map_err(debug)?,
                expected
            );
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (end, result) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(
            serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS),
            client_task
        )
    })
    .await?;
    result?;
    assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)), "{end:?}");
    Ok(())
}
