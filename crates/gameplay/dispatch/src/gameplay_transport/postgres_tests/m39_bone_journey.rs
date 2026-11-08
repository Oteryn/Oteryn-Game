//! M39: both sample outcomes use native commands, carried custody and fenced XP.
//! Reuse L2 1819e54c7; Muriel9f5a72c npc/muriel.lua:61-96,130-139,
//! sha256:7bd9180ba903509cb3a4a6fe01a7b73d9038eff99d15c68bf95fed5ec697d4cc.
use super::fixture::{Harness, debug, id};
use super::quest_world_callbacks::wire;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
const BONE: &str = "oteryn:quest-progress/quest/u8_1/tibia_tales/into_the_bone_pit";
#[path = "m39_bone_journey/absence.rs"]
mod absence;
#[test]
fn m39_bone_pit_full_player_journey() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-m39-bone")
}
#[test]
fn m39_bone_pit_lost_sample_journey() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-m39-bone-lost")
}
fn callback(key: &str) -> TestResult<RewardCallback> {
    oteryn_gameplay_quests::quest::reward_callbacks::load()?
        .into_iter()
        .find(|c| c.key == format!("oteryn:quest-reward/{key}"))
        .ok_or_else(|| format!("source callback {key}").into())
}
async fn stage(h: &Harness, track: &str) -> TestResult<Option<i64>> {
    Ok(
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(track)
            .fetch_optional(&h.pool)
            .await?,
    )
}
async fn carried(h: &Harness, item: u32) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.lifecycle=1 AND i.definition_production_key=$3")
        .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}
async fn xp(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT total_experience::bigint FROM game_character_progression_state")
            .fetch_one(&h.pool)
            .await?,
    )
}
async fn grant(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    key: &str,
) -> TestResult<(NpcIntentDisposition, u64)> {
    let (disposition, _, last) =
        super::qengine_rewards::wire_reward(owner, admitted, npc, &callback(key)?, false).await?;
    Ok((disposition, last))
}
async fn use_at(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    p: Position,
) -> TestResult<(UseDisposition, u64, Vec<u8>, Vec<u8>)> {
    // Honour the native 200 ms world-action window for a fresh USE.
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let map = owner.bundle_map().ok_or("map")?;
    let key =
        oteryn_world_bundle::bundle::placement_key(p.floor, p.x, p.y, 1).ok_or("placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
    .map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let result = wire(owner, admitted, command, 2, payload.clone()).await?;
    Ok((
        decode_use_result(&result).map_err(debug)?,
        command,
        payload,
        result,
    ))
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    name: &str,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    println!("POSTGRES_PLAYER_JOURNEY {name} server_version={version}");
    use sha2::Digest;
    let checksum: Vec<u8> =
        sqlx::query_scalar("SELECT checksum FROM _sqlx_migrations WHERE version=2020 AND success")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        checksum.as_slice(),
        sha2::Sha384::digest(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../apps/game-server/migrations/2020_quest_reward_quantity_variable.sql"
        )))
        .as_slice(),
        "the quantity guard repair actually ran"
    );
    assert_eq!(stage(h, BONE).await?, None, "no seeded mission progress");
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 4, 6, 11, 15, 18]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    bone(owner, admitted, h, authority, name.ends_with("lost")).await
}
async fn bone(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    lost: bool,
) -> TestResult {
    use crate::durability::character_progression::{
        ExperienceAwardRequest, ExperienceRewardOccurrence,
    };
    let [muriel] = super::classic_quests::install(owner, admitted, ["oteryn:npc.muriel"]).await?;
    // The normal successful-conversation helper requires every offer to pass.
    // Here the source offer itself must refuse the under-level player.
    for (word, expected) in [
        ("hi", NpcIntentDisposition::Ok),
        ("mission", NpcIntentDisposition::Rejected),
    ] {
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let outcome = owner
            .npc_talk(
                admitted.runtime_actor.ok_or("actor")?,
                crate::gameplay_transport::connection::UseCommand {
                    game_session_id: admitted.game_session_id,
                    // These read-only service calls do not consume wire continuity.
                    command_id: 10_000 + u64::from(word == "mission"),
                    item_fence: Some(fence.item_fence()),
                },
                oteryn_protocol_oteryn::npc_service::NpcTalkIntent {
                    npc_actor: muriel,
                    text: word.into(),
                },
            )
            .await;
        assert_eq!(outcome.disposition, expected, "under-level {word}");
        assert!(!outcome.unknown);
        if word == "mission" {
            assert!(
                outcome
                    .conversation
                    .ok_or("source refusal")?
                    .lines
                    .iter()
                    .any(|line| line.contains("more experienced"))
            );
        }
    }
    assert_eq!(stage(h, BONE).await?, None);
    assert_eq!(carried(h, 4852).await?, 0);
    let binding = owner
        .progression_binding(admitted.game_session_id)
        .ok_or("progression")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    // Earn the normal level prerequisite; no mission progress is seeded.
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let outcome = slot
        .commit_experience(
            owner.root,
            authority,
            &h.node,
            fence,
            ExperienceAwardRequest {
                occurrence: ExperienceRewardOccurrence::from_bytes(id(97)).map_err(debug)?,
                amount: oteryn_simulation_determinism::ExactI64::new(601_800),
                context: binding.context.clone(),
                policy_revision: binding.policy_revision.clone(),
                reward_revision: binding.reward_revision.clone(),
                policy: binding.policy.clone(),
            },
            None,
        )
        .await
        .map_err(debug)?;
    drop(slot);
    owner
        .states
        .level_refresh
        .mark(crate::gameplay_transport::level_up_live::leveled_character(
            &outcome,
        ));
    owner
        .refresh_level_up_vitals(
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
        )
        .await;
    let level: i64 =
        sqlx::query_scalar("SELECT level::bigint FROM game_character_progression_state")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(level, 35);
    assert_eq!(
        grant(owner, admitted, muriel, "l2-bone-start").await?.0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(stage(h, BONE).await?, Some(1));
    assert_eq!(carried(h, 4852).await?, 1);
    let instance:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(item_instance_id) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i4852'").fetch_one(&h.pool).await?;
    let target = Position {
        x: 33294,
        y: 31677,
        floor: -15,
    };
    let (result, command, payload, bytes) = use_at(owner, admitted, target).await?;
    assert_eq!(result, UseDisposition::Committed);
    assert_eq!(
        wire(owner, admitted, command, 2, payload).await?,
        bytes,
        "collection replay"
    );
    assert_eq!(stage(h, BONE).await?, Some(2));
    assert_eq!(carried(h, 4852).await?, 0);
    assert_eq!(carried(h, 131).await?, 1);
    let after:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(item_instance_id) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i131'").fetch_one(&h.pool).await?;
    assert_eq!(
        instance, after,
        "transform the issued instance exactly once"
    );
    let key = oteryn_world_bundle::bundle::placement_key(-15, 33294, 31677, 1).ok_or("target")?;
    assert!(
        owner.bundle_map().ok_or("map")?.facts.base_hidden(key),
        "collected source target is removed"
    );
    if lost {
        drop_sample(owner, admitted, h).await?;
        assert_eq!(carried(h, 131).await?, 0);
    } else {
        absence::reject_stale_advisory_loss(owner, admitted, h).await?;
    }
    let before = xp(h).await?;
    let branch = if lost {
        "bone-pit-lost-sample"
    } else {
        "l2-bone-return"
    };
    let (result, last) = grant(owner, admitted, muriel, branch).await?;
    assert_eq!(result, NpcIntentDisposition::Ok);
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(stage(h, BONE).await?, Some(3));
    assert_eq!(carried(h, 131).await?, 0);
    assert_eq!(carried(h, 6299).await?, i64::from(!lost));
    // Owner-authorized local rule: pinned Muriel omits XP; only the actual sample earns1,000.
    assert_eq!(xp(h).await?, before + if lost { 0 } else { 1000 });
    let replay = oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(
        &oteryn_protocol_oteryn::npc_service::NpcTalkIntent {
            npc_actor: muriel,
            text: "mission".into(),
        },
    )
    .map_err(debug)?;
    let response = wire(owner, admitted, last, 7, replay).await?;
    assert_eq!(
        oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&response).map_err(debug)?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(xp(h).await?, before + if lost { 0 } else { 1000 });
    assert_eq!(carried(h, 6299).await?, i64::from(!lost));
    Ok(())
}
async fn drop_sample(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    use crate::durability::item_loop_move::{LoopDestination, LoopGround, LoopMoveRequest};
    let item:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(item_instance_id) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i131'").fetch_one(&h.pool).await?;
    let item: [u8; 16] = item.try_into().map_err(|_| "sample identity")?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let map = owner.bundle_map().ok_or("map")?;
    let (digest, frame) = {
        let runtime = owner.runtime.lock().await;
        (
            runtime.content_pin().server_artifact_digest(),
            runtime.content_pin().frame_binding_digest().to_vec(),
        )
    };
    let content_revision = format!(
        "sha256:{}",
        digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let permit = owner.spell_lane_permit().await.ok_or("permit")?;
    let position = at(owner, admitted).await?;
    let result = owner
        .root
        .commit_item_loop_move(
            &permit,
            owner.character,
            owner.holder,
            &fence.item_fence(),
            CommandRef::new(
                admitted.game_session_id,
                CommandId::new(100_000).map_err(debug)?,
            ),
            &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
            LoopMoveRequest {
                item,
                count: None,
                pickup_cell: None,
                destination: LoopDestination::Ground(LoopGround {
                    spatial_position:
                        crate::durability::map_item_mint_audit::map_item_spatial_position(
                            position.x,
                            position.y,
                            position.floor,
                        ),
                    map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                    content_revision,
                    placement_context: frame,
                }),
            },
        )
        .await
        .map_err(debug)?;
    assert!(result.is_ok(), "normal sample custody drop: {result:?}");
    owner.states.loop_ground.invalidate();
    Ok(())
}

async fn at(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<Position> {
    let position = owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    Ok(Position {
        x: u16::try_from(position.x)?,
        y: u16::try_from(position.y)?,
        floor: -i8::try_from(position.floor)?,
    })
}
