//! Selected Spirit story: all three discovered wills are retained; no outfit grant.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;

#[test]
fn l6_spirit_three_retained_wills_unlock_only_confession() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    eprintln!("POSTGRES_CASE_STARTED l6_spirit");
    super::qengine_rewards::scenario("qengine-rewards-questpc4-spirit")
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("POSTGRES_DATABASE_READY l6_spirit version={version}");
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    let accepted = callbacks
        .iter()
        .find(|callback| callback.key == "oteryn:quest-reward/l6-spirit-accept")
        .ok_or("missing selected Spirit acceptance callback")?;
    assert_eq!(accepted.quest, "oteryn:quest.the_spirit_will_get_you_quest");
    assert!(accepted.requires_premium);
    let final_callback = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-reward/l6-spirit-confession")
        .ok_or("confession")?;
    assert!(final_callback.completes);
    assert!(final_callback.take.is_empty());
    assert!(final_callback.outfit.is_none());
    let [npc] = super::classic_quests::install(owner, admitted, [accepted.npc.as_str()]).await?;
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 6, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    let initial = counts(h).await?;
    // Move one cell to the leftmost native box, before accepting its request.
    step(
        owner,
        admitted,
        oteryn_protocol_oteryn::world_spatial::StepDirection::West,
    )
    .await?;
    assert_eq!(
        use_source(owner, admitted, 33079, 31158, 1, 0).await?,
        oteryn_protocol_oteryn::world_object::UseDisposition::Rejected
    );
    assert_eq!(counts(h).await?, initial);
    let (outcome, _, _) =
        super::qengine_rewards::wire_reward(owner, admitted, npc, accepted, false).await?;
    assert_eq!(
        outcome,
        oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Ok
    );
    assert_eq!(
        super::classic_quests::progress(
            owner,
            admitted,
            authority,
            "oteryn:quest-progress/l6/spirit/stage"
        )
        .await?,
        1
    );
    let accepted_counts = counts(h).await?;
    let (outcome, _, _) =
        super::qengine_rewards::wire_reward(owner, admitted, npc, final_callback, false).await?;
    assert_eq!(
        outcome,
        oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Rejected
    );
    assert_eq!(counts(h).await?, accepted_counts);
    let premium = owner.premium_coordinator.take();
    assert_eq!(
        use_source(owner, admitted, 33079, 31158, 1, 0).await?,
        oteryn_protocol_oteryn::world_object::UseDisposition::Rejected
    );
    assert_eq!(
        counts(h).await?,
        accepted_counts,
        "fresh paper grants require current premium authority"
    );
    owner.premium_coordinator = premium;
    let slots: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries")
        .fetch_one(&h.pool)
        .await?;
    for n in slots..20 {
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?;
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &oteryn_gameplay_quests::quest::world_callbacks::Offering {
                position: oteryn_gameplay_quests::quest::world_callbacks::Position {
                    x: 33080,
                    y: 31159,
                    floor: -8,
                },
                item: "oteryn:item.tibia.i2901".into(),
                count: 1,
            },
            100 + u64::try_from(n)?,
            true,
        )
        .await?;
    }
    let full = counts(h).await?;
    assert_eq!(
        use_source(owner, admitted, 33079, 31158, 1, 0).await?,
        oteryn_protocol_oteryn::world_object::UseDisposition::Rejected
    );
    assert_eq!(
        counts(h).await?,
        full,
        "no-room must roll back source flag, paper MINT and audit"
    );
    let fillers:Vec<sqlx::types::Uuid>=sqlx::query_scalar("SELECT item_instance_id FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i2901' ORDER BY item_instance_id LIMIT 3").fetch_all(&h.pool).await?;
    assert_eq!(fillers.len(), 3);
    for (i, item) in fillers.iter().enumerate() {
        move_will(
            owner,
            admitted,
            *item.as_bytes(),
            false,
            9100 + u64::try_from(i)?,
        )
        .await?;
    }
    assert_eq!(
        use_source(owner, admitted, 33079, 31158, 1, 0).await?,
        oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    );
    let first = counts(h).await?;
    // A second object in the same pile must share the once flag.
    assert_eq!(
        use_source(owner, admitted, 33079, 31158, 2, 0).await?,
        oteryn_protocol_oteryn::world_object::UseDisposition::Rejected
    );
    assert_eq!(counts(h).await?, first);
    step(
        owner,
        admitted,
        oteryn_protocol_oteryn::world_spatial::StepDirection::East,
    )
    .await?;
    step(
        owner,
        admitted,
        oteryn_protocol_oteryn::world_spatial::StepDirection::East,
    )
    .await?;
    assert_eq!(
        use_source(owner, admitted, 33083, 31158, 1, 0).await?,
        oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    );
    let two = counts(h).await?;
    let (outcome, _, _) =
        super::qengine_rewards::wire_reward(owner, admitted, npc, final_callback, false).await?;
    assert_eq!(
        outcome,
        oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Rejected
    );
    assert_eq!(counts(h).await?, two);
    assert_eq!(
        use_source(owner, admitted, 33083, 31159, 1, 0).await?,
        oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    );
    let wills: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i11544'").fetch_one(&h.pool).await?;
    assert_eq!(wills, 3);
    let receipts_before: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_receipts")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(receipts_before, 3);
    let premium = owner.premium_coordinator.take();
    let no_premium = counts(h).await?;
    let (outcome, _, _) =
        super::qengine_rewards::wire_reward(owner, admitted, npc, final_callback, false).await?;
    assert_eq!(
        outcome,
        oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Rejected
    );
    assert_eq!(
        counts(h).await?,
        no_premium,
        "confession requires current premium authority"
    );
    owner.premium_coordinator = premium;
    // All discovery flags alone are insufficient if one will leaves carried custody.
    let will: sqlx::types::Uuid=sqlx::query_scalar("SELECT item_instance_id FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i11544' ORDER BY item_instance_id LIMIT 1").fetch_one(&h.pool).await?;
    move_will(owner, admitted, *will.as_bytes(), false, 9000).await?;
    let dropped = counts(h).await?;
    let (outcome, _, _) =
        super::qengine_rewards::wire_reward(owner, admitted, npc, final_callback, false).await?;
    assert_eq!(
        outcome,
        oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Rejected
    );
    assert_eq!(counts(h).await?, dropped);
    move_will(owner, admitted, *will.as_bytes(), true, 9001).await?;
    let (outcome, _, command) =
        super::qengine_rewards::wire_reward(owner, admitted, npc, final_callback, false).await?;
    assert_eq!(
        outcome,
        oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Ok
    );
    let completed: bool=sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE quest_key='oteryn:quest.the_spirit_will_get_you_quest'").fetch_one(&h.pool).await?;
    assert!(completed);
    assert_eq!(
        super::classic_quests::progress(
            owner,
            admitted,
            authority,
            "oteryn:quest-progress/l6/spirit/stage"
        )
        .await?,
        2
    );
    let retained:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i11544'").fetch_one(&h.pool).await?;
    assert_eq!(retained, 3);
    let outfit_grants: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlock_receipts")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(outfit_grants, 0);
    let consumes: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(consumes, 0);
    let same_tx:bool=sqlx::query_scalar("SELECT s.xmin=r.xmin AND r.xmin=q.xmin FROM game_quest_reward_sources s JOIN game_character_quest_receipts r USING(character_id,cause_id,cause_ordinal,transition_key) JOIN game_character_quest_states q ON q.character_id=s.character_id AND q.quest_key=r.quest_key WHERE s.transition_key='oteryn:quest-reward/l6-spirit-confession'").fetch_one(&h.pool).await?;
    assert!(same_tx);
    let final_counts = counts(h).await?;
    let lines = super::classic_quests::talk(owner, admitted, npc, 9200, "confession").await?;
    assert!(
        lines
            .iter()
            .any(|line| line.contains("You know my confession now"))
    );
    assert_eq!(
        counts(h).await?,
        final_counts,
        "earned conversation is read-only"
    );

    // The durable receipt still wins after the final stage has advanced.
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let weights = crate::gameplay_transport::item_ref_admission::NativeItemWeights(native)
        .with_wheel(owner.premium_coordinator);
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let replay = slot
        .commit_quest_reward(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            crate::durability::quest_state::QuestTransitionRequest {
                transition_key: final_callback.key.clone(),
                cause: crate::durability::quest_state::QuestCause::Command(
                    crate::foundation::CommandRef::new(
                        admitted.game_session_id,
                        crate::foundation::CommandId::new(command).map_err(debug)?,
                    ),
                ),
            },
            owner.quest_catalogue.as_ref().ok_or("catalogue")?.clone(),
            crate::durability::reward_claim_mint::QuestRewardAdmission {
                actor_conditions: None,
                carried_use: None,

                native,
                weights: &weights,
                fee_payment: None,
            },
            &owner.premium_coordinator,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(
        replay,
        crate::durability::quest_state::QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    drop(slot);
    assert_eq!(counts(h).await?, final_counts);
    let (outcome, _, _) =
        super::qengine_rewards::wire_reward(owner, admitted, npc, final_callback, false).await?;
    assert_eq!(
        outcome,
        oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Rejected
    );
    assert_eq!(counts(h).await?, final_counts);
    Ok(())
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_crop};
    native_crop(
        native,
        NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z08-x129-y121.b3"
            )),
            region: (8, 129, 121),
            bounds: (33077, 31157, 33085, 31162),
            start: (33081, 31159),
            expected: vec![
                ExpectedTile {
                    position: (33079, 31158),
                    ids: vec![11002, 2473, 2473, 2473],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (33083, 31158),
                    ids: vec![11000, 2469, 2469],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (33083, 31159),
                    ids: vec![10999, 2472],
                    counted_entry: None,
                },
            ],
        },
    )
}

async fn counts(h: &Harness) -> TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('instances',(SELECT count(*) FROM game_item_instances),'mints',(SELECT count(*) FROM game_reward_claim_mint_receipts),'consumes',(SELECT count(*) FROM game_quest_callback_consumptions),'audits',(SELECT count(*) FROM game_item_audit_outbox),'quest_receipts',(SELECT count(*) FROM game_character_quest_receipts),'sources',(SELECT count(*) FROM game_quest_reward_sources),'roots',(SELECT jsonb_agg(to_jsonb(r)) FROM game_character_roots r),'tracks',(SELECT jsonb_agg(to_jsonb(t)) FROM game_character_quest_tracks t))").fetch_one(&h.pool).await?)
}
async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    direction: oteryn_protocol_oteryn::world_spatial::StepDirection,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::*;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        admitted.continuity.next_command_id,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(direction),
    )
    .await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    Ok(())
}
async fn use_source(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    x: u16,
    y: u16,
    ordinal: u8,
    revision: u64,
) -> TestResult<oteryn_protocol_oteryn::world_object::UseDisposition> {
    use oteryn_protocol_oteryn::world_object::*;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let mut placement = owner
        .bundle_map()
        .ok_or("map")?
        .overlay
        .base()
        .digest()
        .to_vec();
    placement.extend_from_slice(
        &oteryn_world_bundle::bundle::placement_key(-8, x, y, ordinal)
            .ok_or("source key")?
            .to_be_bytes(),
    );
    let payload = encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: revision,
    })
    .map_err(debug)?;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        admitted.continuity.next_command_id,
        2,
        payload,
    )
    .await?;
    decode_use_result(&reply).map_err(|e| format!("{e:?}").into())
}
async fn move_will(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    item: [u8; 16],
    pickup: bool,
    command: u64,
) -> TestResult {
    use crate::durability::item_loop_move::{LoopDestination, LoopGround, LoopMoveRequest};
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let runtime = owner.runtime.lock().await;
    let at = runtime
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    let cell = crate::durability::map_item_mint_audit::map_item_spatial_position(
        u16::try_from(at.x)?,
        u16::try_from(at.y)?,
        i8::try_from(-at.floor)?,
    );
    let ground = LoopGround {
        spatial_position: cell.clone(),
        map_revision: crate::map::overlay::map_revision(
            owner.bundle_map().ok_or("map")?.overlay.base(),
        ),
        content_revision: format!(
            "sha256:{}",
            runtime
                .content_pin()
                .server_artifact_digest()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        ),
        placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
    };
    drop(runtime);
    let request = LoopMoveRequest {
        item,
        count: None,
        pickup_cell: pickup.then_some(cell),
        destination: if pickup {
            LoopDestination::MainBackpack
        } else {
            LoopDestination::Ground(ground)
        },
    };
    let permit = owner.spell_lane_permit().await.ok_or("lane")?;
    let result = owner
        .root
        .commit_item_loop_move(
            &permit,
            owner.character,
            owner.holder,
            &fence.item_fence(),
            crate::foundation::CommandRef::new(
                admitted.game_session_id,
                crate::foundation::CommandId::new(command).map_err(debug)?,
            ),
            &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
            request,
        )
        .await
        .map_err(debug)?;
    assert!(result.is_ok(), "retained will custody: {result:?}");
    Ok(())
}
