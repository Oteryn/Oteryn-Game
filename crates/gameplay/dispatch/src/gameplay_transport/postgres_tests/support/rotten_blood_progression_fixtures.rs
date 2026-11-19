use super::fixture::debug;

use super::TestResult;

use crate::durability::character_progression::CurrentCharacterGameplayFence;

use crate::gameplay_transport::ComposedFreshAdmission;

pub(super) async fn current(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: crate::foundation::GameSessionId,
) -> TestResult<CurrentCharacterGameplayFence> {
    owner
        .current_quest_fence(session)
        .await
        .map_err(|_| "current Quest fence unavailable")?
        .ok_or_else(|| "current Quest fence missing".into())
}

pub(super) async fn place_npc(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    key: &str,
) -> TestResult<oteryn_protocol_oteryn::world_spatial_entities::EntityRef> {
    use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
    let catalogue = crate::content::NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        crate::content::npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    owner.states.chat.install_npc_talk(std::sync::Arc::new(
        crate::gameplay_transport::npc_talk::NpcTalkBook::from_model(&model),
    ));
    owner
        .states
        .chat
        .admit(session, "Rotten Blood Hero", owner.owner_now());
    owner.states.chat.set_npc_service(session);
    let mut runtime = owner.runtime.lock().await;
    let spot = runtime
        .read_actor_position(actor)
        .map_err(|error| format!("Rotten NPC {key} actor position: {error:?}"))?
        .position();
    runtime
        .place_npcs(
            &NpcPlacementTable {
                npcs: vec![NpcPlacements {
                    key: key.into(),
                    placements: vec![NpcPlacement {
                        floor: -i8::try_from(spot.floor)?,
                        x: u16::try_from(spot.x + 1)?,
                        y: u16::try_from(spot.y)?,
                        direction: MovementFacing::South,
                    }],
                }],
            },
            |_| true,
            true,
        )
        .map_err(|error| {
            format!("Rotten NPC {key} placement beside carrier position {spot:?}: {error:?}")
        })?;
    let placed = runtime
        .visible_entities()
        .npcs
        .first()
        .copied()
        .ok_or("Bloodshade placement")?;
    Ok(oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
        identity: placed.actor.placement_identity(),
        generation: placed.generation,
    })
}

pub(super) async fn talk(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    number: u64,
    text: &str,
) -> TestResult {
    use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
    use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
    let fence = current(owner, session).await?;
    let result = owner
        .npc_talk(
            actor,
            UseCommand {
                game_session_id: session,
                command_id: number,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: npc,
                text: text.into(),
            },
        )
        .await;
    assert!(
        !result.unknown,
        "command {number}, NPC {npc:?}, text {text:?}: NPC outcome must be retained"
    );
    assert_eq!(
        result.disposition,
        NpcIntentDisposition::Ok,
        "command {number}, NPC {npc:?}, text {text:?}"
    );
    Ok(())
}
