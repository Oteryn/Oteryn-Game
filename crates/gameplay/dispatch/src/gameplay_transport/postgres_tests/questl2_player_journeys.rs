//! L2 player journeys through source NPC commands, physical doors, claims and custody.
//! Sources: Crystal9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d.
//! Sam quest_system2.lua:97-103,378 sha256:04990d261d85479bf9a75de040f6c158bc6eb557018a7ad359374400c2a49635;
//! Budrik npc/budrik.lua:59-74,101-110 sha256:0bad10f01b039dfb0a8f0460b36a0429c1eb498cb504b378ef4aaba005056aa5;
//! Muriel npc/muriel.lua:61-96,130-139 sha256:7bd9180ba903509cb3a4a6fe01a7b73d9038eff99d15c68bf95fed5ec697d4cc;
//! scripts/quests/others/actions_ectoplasm_container.lua:4-13 sha256:584ec08275530b23bfc4222e6294acfaf84d9a1235d1d0738f77a04306666e28.
//! NPCs are adjacent in the compact fixture; their production cells are separately admitted.
use super::fixture::{Harness, debug, id};
use super::quest_world_callbacks::wire;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};
use oteryn_protocol_oteryn::world_spatial::{
    StepDirection, StepDisposition, decode_step_result, encode_step_intent,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

const SAM: &str = "oteryn:quest-progress/quest/u7_5/sams_old_backpack/sams_old_backpack_npc";
const SAM_DOOR: &str = "oteryn:quest-progress/quest/u7_5/sams_old_backpack/sams_old_backpack_door";
const OUTFOX: &str = "oteryn:quest-progress/quest/u8_1/to_outfox_afox_quest";
const START: &str = "oteryn:quest-progress/quest/u8_1/tibia_tales/default_start";
const BONE: &str = "oteryn:quest-progress/quest/u8_1/tibia_tales/into_the_bone_pit";

#[test]
fn questl2_sam_full_player_journey() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l2-full-sam")
}
#[test]
fn questl2_outfox_full_player_journey() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l2-full-outfox")
}
#[test]
fn questl2_bone_pit_full_player_journey() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l2-full-bone")
}
#[test]
fn questl2_bone_pit_lost_sample_completion() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l2-full-bone-lost")
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
async fn supply(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    item: u32,
    occurrence: u64,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: at(owner, admitted).await?,
            item: format!("oteryn:item.tibia.i{item}"),
            count: 1,
        },
        occurrence,
        true,
    )
    .await
}
async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    direction: StepDirection,
) -> TestResult {
    // Advance the existing server clock; the movement limiter remains enabled.
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(1))
        .ok_or("clock")?;
    let command = admitted.continuity.next_command_id;
    let result = wire(owner, admitted, command, 1, encode_step_intent(direction)).await?;
    assert_eq!(
        decode_step_result(&result).map_err(debug)?,
        StepDisposition::Moved
    );
    Ok(())
}
async fn walk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    x: u16,
    y: u16,
) -> TestResult {
    for _ in 0..24 {
        let p = at(owner, admitted).await?;
        if (p.x, p.y) == (x, y) {
            return Ok(());
        }
        step(
            owner,
            admitted,
            if p.x < x {
                StepDirection::East
            } else if p.x > x {
                StepDirection::West
            } else if p.y < y {
                StepDirection::South
            } else {
                StepDirection::North
            },
        )
        .await?;
    }
    Err("bounded physical approach".into())
}
async fn use_at(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    p: Position,
) -> TestResult<(UseDisposition, u64, Vec<u8>, Vec<u8>)> {
    // Honour the native 200 ms world-action window for a fresh USE.
    tokio::time::sleep(std::time::Duration::from_millis(210)).await;
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
async fn visit_unearned_chest(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    chest: Position,
) -> TestResult {
    // The arena permits this test approach; the MINT guard must independently deny it.
    walk(owner, admitted, chest.x + 1, chest.y - 2).await?;
    walk(owner, admitted, chest.x + 1, chest.y).await?;
    assert_eq!(
        use_at(owner, admitted, chest).await?.0,
        UseDisposition::Rejected
    );
    walk(owner, admitted, chest.x + 1, chest.y - 2).await?;
    walk(owner, admitted, chest.x - 1, chest.y - 2).await
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
    for key in [SAM, SAM_DOOR, OUTFOX, START, BONE] {
        assert_eq!(stage(h, key).await?, None, "no seeded {key}");
    }
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
    if name.ends_with("sam") {
        sam(owner, admitted, h).await
    } else if name.ends_with("outfox") {
        outfox(owner, admitted, h).await
    } else {
        bone(owner, admitted, h, authority, name.ends_with("lost")).await
    }
}

async fn sam(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let [sam, kroox] = super::classic_quests::install_nearby(
        owner,
        admitted,
        ["oteryn:npc.sam", "oteryn:npc.kroox"],
    )
    .await?;
    let door = Position {
        x: 32455,
        y: 31967,
        floor: -14,
    };
    let chest = Position { y: 31968, ..door };
    walk(owner, admitted, door.x, door.y - 1).await?;
    assert_eq!(
        use_at(owner, admitted, door).await?.0,
        UseDisposition::Locked
    );
    visit_unearned_chest(owner, admitted, chest).await?;
    assert_eq!(carried(h, 3397).await?, 0);
    assert_eq!(stage(h, SAM).await?, None);
    assert_eq!(
        grant(owner, admitted, sam, "sam-backpack").await?.0,
        NpcIntentDisposition::Rejected
    );
    supply(owner, admitted, 3244, 4000).await?;
    assert_eq!(
        grant(owner, admitted, sam, "sam-backpack").await?.0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(carried(h, 3244).await?, 0);
    assert_eq!(stage(h, SAM).await?, Some(1));
    visit_unearned_chest(owner, admitted, chest).await?;
    assert_eq!(stage(h, SAM).await?, Some(1));
    assert_eq!(carried(h, 3397).await?, 0);
    assert_eq!(
        grant(owner, admitted, kroox, "sam-kroox").await?.0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(stage(h, SAM).await?, Some(2));
    assert_eq!(stage(h, SAM_DOOR).await?, Some(1));
    let atomic:bool=sqlx::query_scalar("SELECT a.committed_character_revision=b.committed_character_revision FROM game_character_quest_tracks a JOIN game_character_quest_tracks b ON a.character_id=b.character_id WHERE a.track_key=$1 AND b.track_key=$2")
        .bind(SAM).bind(SAM_DOOR).fetch_one(&h.pool).await?;
    assert!(atomic, "Kroox access is one write");
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    walk(owner, admitted, door.x, door.y - 1).await?;
    assert_eq!(
        use_at(owner, admitted, door).await?.0,
        UseDisposition::Committed
    );
    step(owner, admitted, StepDirection::South).await?;
    let (result, command, payload, bytes) = use_at(owner, admitted, chest).await?;
    assert_eq!(result, UseDisposition::Committed);
    assert_eq!(
        stage(h, SAM).await?,
        Some(3),
        "world reward commits progress with armor"
    );
    let atomic: bool = sqlx::query_scalar("SELECT q.xmin=i.xmin FROM game_character_quest_tracks q JOIN game_item_instances i ON i.definition_production_key='oteryn:item.tibia.i3397' AND i.lifecycle=1 WHERE q.track_key=$1")
        .bind(SAM).fetch_one(&h.pool).await?;
    assert!(
        atomic,
        "armor and final quest flag share one PostgreSQL transaction"
    );
    assert_eq!(
        wire(owner, admitted, command, 2, payload).await?,
        bytes,
        "armor command replay"
    );
    assert_eq!(carried(h, 3397).await?, 1);
    assert_eq!(stage(h, SAM).await?, Some(3));
    assert_eq!(
        use_at(owner, admitted, chest).await?.0,
        UseDisposition::Rejected
    );
    assert_eq!(carried(h, 3397).await?, 1);
    Ok(())
}
async fn outfox(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let [budrik] =
        super::classic_quests::install_nearby(owner, admitted, ["oteryn:npc.budrik"]).await?;
    let door = Position {
        x: 32467,
        y: 31969,
        floor: -5,
    };
    let chest = Position { y: 31970, ..door };
    walk(owner, admitted, door.x, door.y - 1).await?;
    assert_eq!(
        use_at(owner, admitted, door).await?.0,
        UseDisposition::Locked
    );
    assert_eq!(
        grant(owner, admitted, budrik, "outfox-start").await?.0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(stage(h, OUTFOX).await?, Some(1));
    assert_eq!(stage(h, START).await?, Some(1));
    let atomic:bool=sqlx::query_scalar("SELECT a.committed_character_revision=b.committed_character_revision FROM game_character_quest_tracks a JOIN game_character_quest_tracks b ON a.character_id=b.character_id WHERE a.track_key=$1 AND b.track_key=$2")
        .bind(OUTFOX).bind(START).fetch_one(&h.pool).await?;
    assert!(atomic, "mission issuance is atomic");
    assert_eq!(
        grant(owner, admitted, budrik, "outfox-helmet").await?.0,
        NpcIntentDisposition::Rejected
    );
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    walk(owner, admitted, door.x, door.y - 1).await?;
    assert_eq!(
        use_at(owner, admitted, door).await?.0,
        UseDisposition::Committed
    );
    step(owner, admitted, StepDirection::South).await?;
    let (result, command, payload, bytes) = use_at(owner, admitted, chest).await?;
    assert_eq!(result, UseDisposition::Committed);
    assert_eq!(carried(h, 139).await?, 1);
    assert_eq!(
        wire(owner, admitted, command, 2, payload).await?,
        bytes,
        "helmet command replay"
    );
    assert_eq!(
        grant(owner, admitted, budrik, "outfox-helmet").await?.0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(carried(h, 139).await?, 0);
    assert_eq!(carried(h, 875).await?, 1);
    assert_eq!(stage(h, OUTFOX).await?, Some(2));
    assert_eq!(
        grant(owner, admitted, budrik, "outfox-helmet").await?.0,
        NpcIntentDisposition::Rejected
    );
    assert_eq!(carried(h, 875).await?, 1);
    Ok(())
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
    let [muriel] =
        super::classic_quests::install_nearby(owner, admitted, ["oteryn:npc.muriel"]).await?;
    assert_eq!(
        grant(owner, admitted, muriel, "bone-pit-issue").await?.0,
        NpcIntentDisposition::Rejected
    );
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
        grant(owner, admitted, muriel, "bone-pit-issue").await?.0,
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
    }
    let before = xp(h).await?;
    let branch = if lost {
        "bone-pit-lost-sample"
    } else {
        "bone-pit-return"
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
