//! Source World NPC placement for the already-qualified Sleeping Dragon route.
use super::fixture::{debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use crate::map::overlay::TilePos;
use oteryn_gameplay_quests::quest::wrath_sleeping_dragon as quest;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

fn placement() -> TestResult<NpcPlacement> {
    let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../"));
    let index: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/world/npc-placements/index.json"),
    )?)?;
    for shard in index["shards"].as_array().ok_or("shards")? {
        let data: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join(shard.as_str().ok_or("shard")?))?)?;
        for record in data["records"].as_array().ok_or("records")? {
            let row = &record["declaration"];
            if row["npc"] == quest::NPC {
                assert_eq!(
                    row["cell"],
                    serde_json::json!({"x":33242,"y":31213,"floor":10})
                );
                assert_eq!(row["direction"], "north");
                return Ok(NpcPlacement {
                    x: 33242,
                    y: 31213,
                    floor: -10,
                    direction: MovementFacing::North,
                });
            }
        }
    }
    Err(
        "WRATH_SOURCE_DRAGON_PLACEMENT_RED: source NPC is absent from native placement index"
            .into(),
    )
}
pub(super) fn require_bindings() -> TestResult {
    eprintln!(
        "POSTGRES_CASE_STARTED questpc8_wrath_source_dragon after PostgreSQL fixture initialization"
    );
    placement()?;
    Ok(())
}
pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, _) =
        super::death_ml_loss_composed::production_generation_with_pin(world_id, "content-1")?;
    let native = controller
        .active()
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    super::questpc8_native_slice::world(
        native,
        &[(33237, 31208, 33247, 31220, -10)],
        &[],
        TilePos {
            x: 33242,
            y: 31214,
            floor: -10,
        },
    )
}
pub(super) async fn install(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<EntityRef> {
    let source = placement()?;
    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    owner.states.chat.install_npc_talk(std::sync::Arc::new(
        NpcTalkBook::from_model_with_native_quests(&model, &[quest::NPC]),
    ));
    owner
        .states
        .chat
        .admit(admitted.game_session_id, "Dream walker", owner.owner_now());
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
                    key: quest::NPC.into(),
                    placements: vec![source],
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
            (runtime.npc_key(npc.actor) == Some(quest::NPC)).then_some(EntityRef {
                identity: npc.actor.placement_identity(),
                generation: npc.generation,
            })
        })
        .ok_or_else(|| "source Sleeping Dragon".into())
}
