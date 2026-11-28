use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use super::{TestResult, quest_world_callbacks_fixtures as quest_world_callbacks};

use oteryn_gameplay_quests::quest::world_callbacks::Position;

use super::fixture::{Harness, debug, id};

use super::questpc7_ferumbras_fixtures::use_at;
use super::questpc8_authored_programmes_fixtures::step;

use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};

use oteryn_protocol_oteryn::world_object::UseDisposition;

use oteryn_protocol_oteryn::world_spatial::StepDirection;

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    owner.premium_coordinator = None;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    native
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    owner
        .runtime
        .lock()
        .await
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
    assert!(
        owner
            .quest_guardian_for_test("oteryn:quest-death/authored-smith-guardian")
            .await
            .is_none()
    );
    step(owner, admitted, 1, StepDirection::East).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let guardian = owner
        .quest_guardian_for_test("oteryn:quest-death/authored-smith-guardian")
        .await
        .ok_or("the native objective must create its guardian")?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(
        owner
            .quest_guardian_for_test("oteryn:quest-death/authored-smith-guardian")
            .await,
        Some(guardian),
        "refresh cannot duplicate the guardian"
    );
    assert_eq!(progress(h, "stage").await?, 1);
    let position = Position {
        x: 32776,
        y: 32253,
        floor: -8,
    };
    assert_eq!(
        use_at(owner, admitted, position, 2472).await?,
        UseDisposition::Rejected
    );
    super::grizzly_npc_fixtures::questpc8_campaign::lethal(
        owner,
        admitted,
        guardian,
        "oteryn:creature.giant_spider",
        2,
        1,
    )
    .await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(progress(h, "kills").await?, 1);
    assert!(
        owner
            .quest_guardian_for_test("oteryn:quest-death/authored-smith-guardian")
            .await
            .is_none()
    );
    let input = oteryn_gameplay_quests::quest::world_callbacks::Offering {
        position: Position {
            x: 32776,
            y: 32254,
            floor: -8,
        },
        item: "oteryn:item.tibia.i20357".into(),
        count: 1,
    };
    for n in 100..120 {
        quest_world_callbacks::input_item(owner, fence.item_fence(), &input, n, true).await?;
    }
    let occupied: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        occupied, 20,
        "the first native reward is tested with a full backpack"
    );
    assert_eq!(
        use_at(owner, admitted, position, 2472).await?,
        UseDisposition::Rejected,
        "a full backpack cannot admit a source box reward"
    );
    assert_eq!(quantity(h, "oteryn:item.tibia.i3208").await?, 0);
    assert_eq!(quantity(h, "oteryn:item.tibia.i3034").await?, 0);
    assert_eq!(progress(h, "stage").await?, 1);
    let gold_before = quantity(h, "oteryn:item.tibia.i3031").await?;
    for (index, x) in [32778, 32776, 32774].into_iter().enumerate() {
        let approach = x.clamp(32775, 32777);
        super::questpc7_ferumbras_fixtures::walk(owner, admitted, Position { x: approach, y: 32254, floor: -8 }).await?;
        let site = Position { x, y: 32253, floor: -8 };
        assert_eq!(use_at(owner, admitted, site, 2472).await?, UseDisposition::Rejected);
        assert_eq!(progress(h, "stage").await?, 1);
        drop_filler(owner, admitted, h, fence.item_fence(), Position { x: approach, y: 32254, floor: -8 }, 999000 + u64::try_from(index)?).await?;
        let command = admitted.continuity.next_command_id;
        let payload = super::questpc7_hoist_story_fixtures::target(owner, site, 2472)?;
        assert_eq!(use_at(owner, admitted, site, 2472).await?, UseDisposition::Committed);
        let replay = quest_world_callbacks::wire(owner, admitted, command, 2, payload).await?;
        assert_eq!(oteryn_protocol_oteryn::world_object::decode_use_result(&replay).map_err(debug)?, UseDisposition::Committed);
    }
    assert_eq!(progress(h, "stage").await?, 3);
    assert_eq!(quantity(h, "oteryn:item.tibia.i3208").await?, 1);
    assert_eq!(quantity(h, "oteryn:item.tibia.i3034").await?, 1);
    assert_eq!(
        quantity(h, "oteryn:item.tibia.i3031").await? - gold_before,
        100
    );
    let entries: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(entries, 20);
    super::questpc7_ferumbras_fixtures::walk(owner, admitted, Position { x: 32776, y: 32254, floor: -8 }).await?;
    assert_ne!(use_at(owner, admitted, position, 2472).await?, UseDisposition::Committed);
    assert_eq!(quantity(h, "oteryn:item.tibia.i3208").await?, 1);
    assert_eq!(
        quantity(h, "oteryn:item.tibia.i3031").await? - gold_before,
        100
    );
    let completed: bool = sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.giant_smithhammer_quest'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert!(completed);
    Ok(())
}

async fn quantity(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1").bind(key).fetch_one(&h.pool).await?)
}

async fn progress(h: &Harness, suffix: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),-1)").bind(id(41).as_slice()).bind(format!("oteryn:quest-progress/chosen/smith-cache/{suffix}")).fetch_one(&h.pool).await?)
}

async fn drop_filler(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    position: Position,
    sequence: u64,
) -> TestResult {
    use crate::durability::item_loop_move::{LoopDestination, LoopGround, LoopMoveRequest};
    let bytes: Vec<u8> = sqlx::query_scalar("SELECT decode(replace(e.item_instance_id::text,'-',''),'hex') FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.definition_production_key='oteryn:item.tibia.i20357' ORDER BY e.placement_ordinal LIMIT 1").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    let item: [u8; 16] = bytes.try_into().map_err(|_| "item")?;
    let map = owner.bundle_map().ok_or("map")?;
    let (digest, frame) = {
        let runtime = owner.runtime.lock().await;
        (
            runtime.content_pin().server_artifact_digest(),
            runtime.content_pin().frame_binding_digest(),
        )
    };
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let permit = owner.spell_lane_permit().await.ok_or("permit")?;
    let moved = owner
        .root
        .commit_item_loop_move(
            &permit,
            owner.character,
            owner.holder,
            &fence,
            crate::foundation::CommandRef::new(
                admitted.game_session_id,
                crate::foundation::CommandId::new(sequence).map_err(debug)?,
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
                    content_revision: format!(
                        "sha256:{}",
                        digest
                            .iter()
                            .map(|b| format!("{b:02x}"))
                            .collect::<String>()
                    ),
                    placement_context: frame.to_vec(),
                }),
            },
        )
        .await
        .map_err(debug)?;
    assert!(
        moved.is_ok(),
        "free one entry through the existing item owner: {moved:?}"
    );
    Ok(())
}
