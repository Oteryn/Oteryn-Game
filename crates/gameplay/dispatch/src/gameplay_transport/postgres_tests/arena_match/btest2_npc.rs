use super::super::{Dependencies, TestResult};
use super::{ComposedFreshAdmission, Harness, NativeScenario, debug, id};
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{MovementFacing, NpcPlacement};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

pub(super) async fn exercise(
    mut owner: ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    harness: &Harness,
    route: NativeScenario,
) -> TestResult {
    let buddel = matches!(route, NativeScenario::Buddel);
    let key = if buddel {
        "oteryn:npc.buddel"
    } else {
        "oteryn:npc.iskan"
    };
    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    owner
        .states
        .chat
        .install_npc_talk(Arc::new(NpcTalkBook::from_model(&model)));
    let session = deps.admitted.game_session_id;
    owner
        .states
        .chat
        .admit(session, "Quest witness", owner.owner_now());
    owner.states.chat.set_npc_service(session);
    let npc_actor = {
        let mut runtime = deps.runtime.lock().await;
        let actor = runtime
            .place_dynamic_npc(
                key,
                NpcPlacement {
                    x: 32252,
                    y: 31098,
                    floor: -6,
                    direction: MovementFacing::South,
                },
            )
            .map_err(debug)?;
        let placement = runtime
            .visible_entities()
            .npcs
            .into_iter()
            .find(|n| n.actor == actor)
            .ok_or("placed NPC")?;
        EntityRef {
            identity: actor.placement_identity(),
            generation: placement.generation,
        }
    };
    let tracks = [
        (
            "oteryn:quest-progress/quest/u8_0/barbarian_test/questline",
            8_i64,
        ),
        (
            "oteryn:quest-progress/quest/u8_0/the_ice_islands/questline",
            2,
        ),
        (
            "oteryn:quest-progress/quest/u8_0/the_ice_islands/mission01",
            2,
        ),
    ];
    for (track, value) in tracks {
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,'oteryn:quest.the_ice_islands_quest',$3,2)")
            .bind(id(41).as_slice()).bind(track).bind(value).execute(&harness.pool).await?;
    }
    owner.refresh_quest_session(session, true).await;
    let messages = if buddel {
        ["hi", "okolnir", "yes"]
    } else {
        ["hi", "mission", "passage"]
    };
    let mut lines = Vec::new();
    for (index, text) in messages
        .into_iter()
        .enumerate()
        .chain((!buddel).then_some((3, "yes")))
    {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let result = owner
            .npc_talk(
                deps.actor,
                UseCommand {
                    game_session_id: session,
                    command_id: 44000 + u64::try_from(index)?,
                    item_fence: Some(fence.item_fence()),
                },
                NpcTalkIntent {
                    npc_actor,
                    text: text.into(),
                },
            )
            .await;
        assert_eq!(
            result.disposition,
            NpcIntentDisposition::Ok,
            "{key}: {text}: {:?}",
            result.conversation
        );
        lines.push(
            result
                .conversation
                .map(|reply| reply.lines.join(" "))
                .unwrap_or_default(),
        );
    }
    let actor = deps
        .runtime
        .lock()
        .await
        .read_actor_position(deps.actor)
        .map_err(debug)?
        .position();
    if buddel {
        assert!(lines[1].contains("ring"), "{lines:?}");
        assert_eq!((actor.x, actor.y, actor.floor), (32225, 31381, 7));
        let ring: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3097' AND lifecycle=1")
            .fetch_one(&harness.pool).await?;
        assert_eq!(ring, 0);
    } else {
        assert_eq!((actor.x, actor.y, actor.floor), (32325, 31049, 7));
        for (track, _) in tracks.into_iter().skip(1) {
            let stage: i64 = sqlx::query_scalar(
                "SELECT value FROM game_character_quest_tracks WHERE track_key=$1",
            )
            .bind(track)
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(stage, 3, "{track}");
        }
    }
    Ok(())
}
