//! Retained Sterling placement and real, once-only first-contact command; no later stage is seeded.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

const NPC: &str = "oteryn:npc.sterling";
const TRACK: &str = "oteryn:quest-progress/targuna/stage";

#[test]
fn questpc5_targuna_sterling_first_contact_commits_once_without_reward_or_completion() -> TestResult
{
    super::qengine_rewards::scenario("qengine-rewards-questpc5-targuna-sterling")
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let server: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&harness.pool)
        .await?;
    eprintln!("POSTGRES_CASE_STARTED questpc5_targuna_sterling server_version={server}");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let index: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/npcs/definitions/index.json"),
    )?)?;
    let mut found = None;
    for path in index["shards"].as_array().ok_or("NPC shards")? {
        let doc: serde_json::Value = serde_json::from_slice(&std::fs::read(
            root.join(path.as_str().ok_or("NPC shard")?),
        )?)?;
        for row in doc["records"].as_array().ok_or("NPC records")? {
            if row["declaration"]["identity"]["key"].as_str() == Some(NPC) {
                found = Some(row["declaration"].clone());
            }
        }
    }
    let npc = found.ok_or("Sterling declaration")?;
    let fields = npc["fields"].as_array().ok_or("NPC fields")?;
    let positions = fields
        .iter()
        .find(|field| field["field_path"] == "oteryn:source.npc.bulk.wiki_positions")
        .and_then(|field| field["value"]["value"].as_str())
        .ok_or("Targuna Sterling retained position absent")?;
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(positions)?,
        serde_json::json!([[31928, 31903, 7]])
    );
    let catalogue = NpcDataCatalogue::load(
        root.join("content").as_path(),
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
        "Targuna witness",
        owner.owner_now(),
    );
    owner.states.chat.set_npc_service(admitted.game_session_id);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let npc = {
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
                            x: 31928,
                            y: 31903,
                            floor: -7,
                            direction: MovementFacing::North,
                        }],
                    }],
                },
                |_| true,
                true,
            )
            .map_err(debug)?;
        let actor = runtime
            .visible_entities()
            .npcs
            .into_iter()
            .find(|entry| runtime.npc_key(entry.actor) == Some(NPC))
            .ok_or("Sterling actor")?;
        EntityRef {
            identity: actor.actor.placement_identity(),
            generation: actor.generation,
        }
    };
    let position = owner
        .runtime
        .lock()
        .await
        .visible_entities()
        .npcs
        .into_iter()
        .find(|entry| entry.actor.placement_identity() == npc.identity)
        .ok_or("Sterling actor")?
        .position;
    // NPC placements use native -z; runtime LocalPosition uses legacy z (npc_actor::cell).
    assert_eq!((position.x, position.y, position.floor), (31928, 31903, 7));
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.push(oteryn_protocol_oteryn::npc_service::CAPABILITY_NPC_SERVICE_V1);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("NPC service capability must be offered before selecting it")?;
    assert!(admitted.continuity.selected_capabilities.contains(3));
    let items: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE lifecycle=1")
            .fetch_one(&harness.pool)
            .await?;
    assert_eq!(progress(owner, admitted, authority).await?, -1);
    let callback = oteryn_gameplay_quests::quest::reward_callbacks::load()?
        .into_iter()
        .find(|row| row.key == "oteryn:quest-reward/questpc5-targuna-sterling-accept")
        .ok_or("Targuna callback")?;
    // Keep greeting and acceptance in one connection; disconnect correctly clears focus.
    let (accepted, _, command) =
        super::qengine_rewards::wire_reward(owner, admitted, npc, &callback, false).await?;
    assert_eq!(accepted, NpcIntentDisposition::Ok);
    assert_eq!(progress(owner, admitted, authority).await?, 0);
    wire_talk(owner, admitted, npc, command, "mission").await?;
    // Refuse a new claim in a live conversation, rather than merely without NPC focus.
    super::classic_quests::talk(owner, admitted, npc, command + 1, "hi").await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let refused = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: command + 2,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: npc,
                text: "mission".into(),
            },
        )
        .await;
    assert!(!refused.inventory_changed);
    assert_eq!(progress(owner, admitted, authority).await?, 0);
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-reward/questpc5-targuna-sterling-accept'")
        .fetch_one(&harness.pool).await?;
    assert_eq!(receipts, 1);
    let completed: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_states WHERE quest_key='oteryn:quest.targuna_quest' AND completed_character_revision IS NOT NULL")
        .fetch_one(&harness.pool).await?;
    assert_eq!(completed, 0);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_instances WHERE lifecycle=1")
            .fetch_one(&harness.pool)
            .await?,
        items
    );
    eprintln!("TARGUNA_FIRST_CONTACT_PROVEN stage=0 receipts=1 rewards=0 completes=false");
    Ok(())
}

async fn progress(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult<i64> {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let state = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    Ok(state.tracks().get(TRACK).copied().unwrap_or(-1))
}

async fn wire_talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    id: u64,
    text: &str,
) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let payload = oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
        npc_actor: npc,
        text: text.into(),
    })
    .map_err(debug)?;
    let response = super::quest_world_callbacks::wire(owner, admitted, id, 7, payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&response).map_err(debug)?,
        NpcIntentDisposition::Ok
    );
    Ok(())
}
