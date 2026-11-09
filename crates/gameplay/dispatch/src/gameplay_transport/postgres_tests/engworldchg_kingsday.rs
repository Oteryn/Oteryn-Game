//! Kingsday's source NPC, native STEP and achievement share the production quest owner.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_protocol_oteryn::npc_service::NpcTalkIntent;
use oteryn_protocol_oteryn::world_spatial::StepDirection;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

const KING: &str = "oteryn:npc.king_tibianus";
const QUEST: &str = "oteryn:quest.authored.kingsday_mini_world_change";
const PHASE: &str = "oteryn:quest-progress/world-change-journal/kingsday/phase";

async fn install(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<EntityRef> {
    let catalogue = crate::content::NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        crate::content::npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    let programs =
        oteryn_gameplay_quests::quest::npc_rules::NpcRuleCatalog::runtime().map_err(debug)?;
    let native_npcs = programs
        .programs()
        .iter()
        .flat_map(|program| program.document.rules.iter().map(|rule| rule.npc.as_str()))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    owner
        .states
        .chat
        .install_npc_talk(Arc::new(NpcTalkBook::from_model_with_native_quests(
            &model,
            &native_npcs,
        )));
    owner
        .states
        .chat
        .admit(admitted.game_session_id, "Royal visitor", owner.owner_now());
    owner.states.chat.set_npc_service(admitted.game_session_id);
    let placements: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../content/world/npc-placements/placements-00000-00055.json"
    ))?;
    let row = placements["records"]
        .as_array()
        .ok_or("placements")?
        .iter()
        .map(|row| &row["declaration"])
        .find(|row| row["npc"] == KING)
        .ok_or("native royal placement")?;
    assert_eq!(
        row["cell"],
        serde_json::json!({"x":32311,"y":32172,"floor":6})
    );
    assert_eq!(row["direction"], "north");
    let mut runtime = owner.runtime.lock().await;
    let placed = runtime
        .place_npcs(
            &NpcPlacementTable {
                npcs: vec![NpcPlacements {
                    key: KING.into(),
                    placements: vec![NpcPlacement {
                        x: 32311,
                        y: 32172,
                        floor: -6,
                        direction: MovementFacing::North,
                    }],
                }],
            },
            |key| model.npcs().keys().any(|reference| reference.key == key),
            true,
        )
        .map_err(debug)?;
    let actor = *placed.actors.first().ok_or("royal actor")?;
    let npc = runtime
        .visible_entities()
        .npcs
        .into_iter()
        .find(|npc| npc.actor == actor)
        .ok_or("royal generation")?;
    Ok(EntityRef {
        identity: actor.placement_identity(),
        generation: npc.generation,
    })
}

async fn talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    npc: EntityRef,
    command_id: u64,
    text: &str,
) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let outcome = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: npc,
                text: text.into(),
            },
        )
        .await;
    assert!(
        !outcome.unknown,
        "native royal dialogue must have a definitive outcome"
    );
    Ok(())
}

async fn achievements(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/loyal_subject'")
        .fetch_one(&h.pool).await?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let npc = install(owner, admitted).await?;
    owner.premium_coordinator = None;
    let mut supported = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    supported.push(oteryn_protocol_oteryn::npc_service::CAPABILITY_NPC_SERVICE_V1);
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &supported,
        )
        .ok_or("NPC capability selection")?;
    assert!(
        admitted
            .continuity
            .selected_capabilities
            .contains(oteryn_protocol_oteryn::npc_service::CAPABILITY_NPC_SERVICE_V1)
    );
    talk(owner, admitted, npc, 30000, "hi").await?;
    talk(owner, admitted, npc, 30001, "hail the king").await?;
    assert_eq!(super::engworldchg_journals::value(h, PHASE).await?, 0);
    assert_eq!(
        achievements(h).await?,
        0,
        "a greeting cannot skip the chosen royal visit"
    );
    super::engworldchg_journals::step(owner, admitted, 1, StepDirection::West).await?;
    assert_eq!(super::engworldchg_journals::value(h, PHASE).await?, 1);
    super::engworldchg_journals::step(owner, admitted, 2, StepDirection::East).await?;
    owner.open_npc_session(admitted.game_session_id).await;
    talk(owner, admitted, npc, 30002, "hi").await?;
    talk(owner, admitted, npc, 30003, "hail the king").await?;
    assert_eq!(super::engworldchg_journals::value(h, PHASE).await?, 3);
    assert_eq!(achievements(h).await?, 1);
    talk(owner, admitted, npc, 30003, "hail the king").await?;
    talk(owner, admitted, npc, 30004, "hail the king").await?;
    assert_eq!(achievements(h).await?, 1);
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1 AND completes",
    )
    .bind(QUEST)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(receipts, 1);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let loaded = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(loaded.tracks().get(PHASE), Some(&3));
    Ok(())
}
