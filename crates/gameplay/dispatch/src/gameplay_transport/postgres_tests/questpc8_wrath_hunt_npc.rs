//! Real source Zlak placement, independent of the player's approach direction.
use super::fixture::debug;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

pub(super) async fn install(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<EntityRef> {
    const NPC: &str = "oteryn:npc.zlak";
    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    owner.states.chat.install_npc_talk(std::sync::Arc::new(
        NpcTalkBook::from_model_with_native_quests(&model, &[NPC]),
    ));
    owner.states.chat.admit(
        admitted.game_session_id,
        "Imperial hunter",
        owner.owner_now(),
    );
    owner.states.chat.set_npc_service(admitted.game_session_id);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let mut runtime = owner.runtime.lock().await;
    runtime
        .bind_attacker_lease(
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    runtime
        .place_npcs(
            &NpcPlacementTable {
                npcs: vec![NpcPlacements {
                    key: NPC.into(),
                    placements: vec![NpcPlacement {
                        x: 33082,
                        y: 31220,
                        floor: -7,
                        direction: MovementFacing::North,
                    }],
                }],
            },
            |_| true,
            true,
        )
        .map_err(debug)?;
    runtime
        .visible_entities()
        .npcs
        .into_iter()
        .find_map(|npc| {
            (runtime.npc_key(npc.actor) == Some(NPC)).then_some(EntityRef {
                identity: npc.actor.placement_identity(),
                generation: npc.generation,
            })
        })
        .ok_or_else(|| "source Zlak placement".into())
}
