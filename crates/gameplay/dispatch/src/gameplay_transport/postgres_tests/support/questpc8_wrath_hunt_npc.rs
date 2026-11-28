//! Source placements for the imperial hunt and the palace-to-dragon handoff.

use super::fixture::debug;

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};

use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};

use crate::gameplay_transport::npc_talk::NpcTalkBook;

use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

pub(super) async fn install(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<[EntityRef; 3]> {
    const ZLAK: &str = "oteryn:npc.zlak";
    const ZIZZLE: &str = "oteryn:npc.zizzle";
    use oteryn_gameplay_quests::quest::wrath_sleeping_dragon::NPC as DRAGON;
    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    owner.states.chat.install_npc_talk(std::sync::Arc::new(
        NpcTalkBook::from_model_with_native_quests(&model, &[DRAGON, ZIZZLE, ZLAK]),
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
                npcs: [
                    (DRAGON, 33242, 31213, -10),
                    (ZIZZLE, 33105, 31120, -12),
                    (ZLAK, 33082, 31220, -7),
                ]
                .into_iter()
                .map(|(key, x, y, floor)| NpcPlacements {
                    key: key.into(),
                    placements: vec![NpcPlacement {
                        x,
                        y,
                        floor,
                        direction: MovementFacing::North,
                    }],
                })
                .collect(),
            },
            |_| true,
            true,
        )
        .map_err(debug)?;
    let entities = runtime.visible_entities();
    let lookup = |key| {
        entities
            .npcs
            .iter()
            .find_map(|npc| {
                (runtime.npc_key(npc.actor) == Some(key)).then_some(EntityRef {
                    identity: npc.actor.placement_identity(),
                    generation: npc.generation,
                })
            })
            .ok_or("missing Wrath contact")
    };
    Ok([lookup(ZLAK)?, lookup(ZIZZLE)?, lookup(DRAGON)?])
}
