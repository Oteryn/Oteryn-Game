//! Crystal 9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:
//! npc/an_old_dragon_lord.lua:61-74 sha256:85016cc08856f49d94c507f43bc84f54801d3a2844946ae391015a0d8c84a052;
//! scripts/quests/draconia/movement-escape.lua:9-26 sha256:53f3313e99fb541c3e016b5ac09a2e372c0ad8a382786ccd79db6bb40a700312;
//! scripts/quests/draconia/movement-exit_teleport.lua:1-36 sha256:80af40ac459eb655670472a0391db78590d456827917d4a64e714947b64723a4.
//! Paths are below data-global/. The compact fixture places the admitted NPC
//! beside the approach; its production Global placement is independently checked.
use super::fixture::{Harness, debug};
use super::quest_world_callbacks::{local, wire};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Offering, Position};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use oteryn_protocol_oteryn::world_spatial::{
    StepDirection, StepDisposition, decode_step_result, encode_step_intent,
};

#[test]
fn draconia_full_mushroom_fetish_offering_exit_and_replay() -> TestResult {
    scenario(
        "draconia-player-journey",
        Position {
            x: 32818,
            y: 31598,
            floor: -9,
        },
    )
}
#[test]
fn draconia_four_live_levers_control_alternative_exit_without_quest_writes() -> TestResult {
    scenario(
        "draconia-lever-exit",
        Position {
            x: 32803,
            y: 31586,
            floor: -1,
        },
    )
}
fn scenario(name: &'static str, trigger: Position) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run(name, move || {
        super::quest_world_callbacks::scenario(name, trigger, false)
    })?;
    println!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}
async fn active(h: &Harness, item: u32) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
       .bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}
async fn flag(h: &Harness) -> TestResult<Option<i64>> {
    Ok(sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key='oteryn:quest-progress/dragonfetish'").fetch_optional(&h.pool).await?)
}
async fn receipts(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
            .fetch_one(&h.pool)
            .await?,
    )
}
async fn step(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    direction: StepDirection,
) -> TestResult<(u64, Vec<u8>)> {
    let id = admitted.continuity.next_command_id;
    let result = wire(owner, admitted, id, 1, encode_step_intent(direction)).await?;
    assert_eq!(
        decode_step_result(&result).map_err(debug)?,
        StepDisposition::Moved
    );
    Ok((id, result))
}
async fn walk(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    actor: ExactActorRef,
    x: i32,
    y: i32,
) -> TestResult {
    for _ in 0..32 {
        let at = owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        if (at.x, at.y) == (x, y) {
            return Ok(());
        }
        let direction = if at.x < x {
            StepDirection::East
        } else if at.x > x {
            StepDirection::West
        } else if at.y < y {
            StepDirection::South
        } else {
            StepDirection::North
        };
        step(owner, admitted, direction).await?;
    }
    Err("bounded source approach failed".into())
}
async fn control(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    x: u16,
) -> TestResult {
    use crate::map::view::MapFacts;
    use oteryn_protocol_oteryn::world_object::{
        UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
    };
    let map = owner.bundle_map().ok_or("map")?;
    let key = oteryn_world_bundle::bundle::placement_key(-1, x, 31584, 1).ok_or("lever key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    };
    let payload = encode_use_intent(&target).map_err(debug)?;
    let id = admitted.continuity.next_command_id;
    let result = wire(owner, admitted, id, 2, payload.clone()).await?;
    assert_eq!(
        decode_use_result(&result).map_err(debug)?,
        UseDisposition::Committed
    );
    assert_eq!(
        wire(owner, admitted, id, 2, payload).await?,
        result,
        "lever replay is inert"
    );
    Ok(())
}
async fn approach_control(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    actor: ExactActorRef,
    x: u16,
) -> TestResult {
    let at = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    walk(owner, admitted, actor, 32802, at.y).await?;
    walk(owner, admitted, actor, 32802, 31585).await?;
    walk(owner, admitted, actor, i32::from(x.min(32804)), 31585).await
}
async fn enter_lever_exit(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    actor: ExactActorRef,
) -> TestResult<(u64, Vec<u8>)> {
    walk(owner, admitted, actor, 32802, 31585).await?;
    step(owner, admitted, StepDirection::South).await?;
    step(owner, admitted, StepDirection::East).await
}
#[allow(clippy::too_many_arguments)]
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    actor: ExactActorRef,
    callback: &Callback,
    trigger: Position,
    journey: bool,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    println!("POSTGRES_CASE_STARTED draconia server_version={version}");
    let before = receipts(h).await?;
    assert_eq!(flag(h).await?, None, "no seeded mission progress");
    if !journey {
        for x in 32802..=32805 {
            approach_control(owner, admitted, actor, x).await?;
            control(owner, admitted, x).await?;
            enter_lever_exit(owner, admitted, actor).await?;
            assert_eq!(
                owner
                    .runtime
                    .lock()
                    .await
                    .read_actor_position(actor)
                    .map_err(debug)?
                    .position(),
                local(Position {
                    x: 32803,
                    y: 31587,
                    floor: -1
                })
            );
            assert_eq!(receipts(h).await?, before);
            approach_control(owner, admitted, actor, x).await?;
            control(owner, admitted, x).await?;
        }
        let (id, result) = enter_lever_exit(owner, admitted, actor).await?;
        let position = owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?;
        assert_eq!(
            position.position(),
            local(
                callback
                    .remote_teleport
                    .as_ref()
                    .ok_or("remote")?
                    .destination
            )
        );
        assert_eq!(
            wire(
                owner,
                admitted,
                id,
                1,
                encode_step_intent(StepDirection::East)
            )
            .await?,
            result
        );
        assert_eq!(receipts(h).await?, before);
        assert_eq!(flag(h).await?, None);
        return Ok(());
    }
    let [npc] =
        super::classic_quests::install(owner, admitted, ["oteryn:npc.an_old_dragonlord"]).await?;
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 6]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    assert_eq!(
        greet(owner, admitted, npc).await?,
        NpcIntentDisposition::Rejected
    );
    assert_eq!(receipts(h).await?, before);
    assert_eq!(active(h, 3206).await?, 0);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: trigger,
            item: "oteryn:item.tibia.i3723".into(),
            count: 2,
        },
        1000,
        true,
    )
    .await?;
    assert_eq!(greet(owner, admitted, npc).await?, NpcIntentDisposition::Ok);
    assert_eq!(active(h, 3723).await?, 1);
    assert_eq!(active(h, 3206).await?, 1);
    assert_eq!(flag(h).await?, Some(1));
    assert_eq!(greet(owner, admitted, npc).await?, NpcIntentDisposition::Ok);
    assert_eq!(active(h, 3723).await?, 1);
    assert_eq!(active(h, 3206).await?, 1);
    assert_eq!(receipts(h).await?, before + 1);
    step(owner, admitted, StepDirection::West).await?;
    step(owner, admitted, StepDirection::South).await?;
    step(owner, admitted, StepDirection::South).await?;
    drop_fetish(owner, admitted, h, &callback.offerings[0]).await?;
    step(owner, admitted, StepDirection::North).await?;
    step(owner, admitted, StepDirection::North).await?;
    step(owner, admitted, StepDirection::East).await?;
    let (id, result) = step(owner, admitted, StepDirection::East).await?;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        local(callback.teleport.ok_or("exit")?)
    );
    assert_eq!(active(h, 3206).await?, 0);
    assert_eq!(active(h, 3723).await?, 1);
    assert_eq!(flag(h).await?, Some(0));
    assert_eq!(receipts(h).await?, before + 2);
    assert_eq!(
        wire(
            owner,
            admitted,
            id,
            1,
            encode_step_intent(StepDirection::East)
        )
        .await?,
        result
    );
    assert_eq!(receipts(h).await?, before + 2);
    assert_eq!(flag(h).await?, Some(0));
    Ok(())
}
async fn greet(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
) -> TestResult<NpcIntentDisposition> {
    use oteryn_protocol_oteryn::npc_service::{
        COMMAND_TYPE_NPC_TALK_INTENT, NpcTalkIntent, decode_npc_intent_result,
        encode_npc_talk_intent,
    };
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let id = admitted.continuity.next_command_id;
    let payload = encode_npc_talk_intent(&NpcTalkIntent {
        npc_actor: npc,
        text: "hi".into(),
    })
    .map_err(debug)?;
    let result = wire(owner, admitted, id, COMMAND_TYPE_NPC_TALK_INTENT, payload).await?;
    decode_npc_intent_result(&result).map_err(debug)
}

async fn drop_fetish(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    offering: &Offering,
) -> TestResult {
    use crate::durability::item_loop_move::{LoopDestination, LoopGround, LoopMoveRequest};
    let item:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(item_instance_id) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3206' AND lifecycle=1").fetch_one(&h.pool).await?;
    let item: [u8; 16] = item.try_into().map_err(|_| "item identity")?;
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
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let permit = owner.spell_lane_permit().await.ok_or("permit")?;
    let id = 100_000;
    let result = owner
        .root
        .commit_item_loop_move(
            &permit,
            owner.character,
            owner.holder,
            &fence.item_fence(),
            CommandRef::new(admitted.game_session_id, CommandId::new(id).map_err(debug)?),
            &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
            LoopMoveRequest {
                item,
                count: None,
                pickup_cell: None,
                destination: LoopDestination::Ground(LoopGround {
                    spatial_position:
                        crate::durability::map_item_mint_audit::map_item_spatial_position(
                            offering.position.x,
                            offering.position.y,
                            offering.position.floor,
                        ),
                    map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                    content_revision,
                    placement_context: frame,
                }),
            },
        )
        .await
        .map_err(debug)?;
    assert!(result.is_ok(), "earned fetish placement: {result:?}");
    owner.states.loop_ground.invalidate();
    Ok(())
}
