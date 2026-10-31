//! Two independent durable owners race for one live personal entitlement.
use super::*;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::reward_claim_mint::boss_reward::{
    BossRewardCollectRequest, BossRewardCollectionStatus, BossRewardSelection,
};
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{CommandId, CommandRef, GameSessionState, RuntimeScopeRefV1};

type CustodyCounts = (i64, i64, i64, i64, i32, i64);
type CustodyAudit = (Vec<u8>, Vec<u8>, Vec<u8>);

pub(super) async fn collect_on_two_roots(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    fence: CurrentCharacterItemFence,
    death: [u8; 16],
) -> TestResult {
    // Follow the existing item_transfer two-root concurrency fixture. Each
    // DB-only caller owns its fresh scoped lane and an independently opened
    // authority; neither uses a stale lease or a second native actor owner.
    let second_root = h.independent_root()?;
    assert!(second_root.maintain_ready_once().await?);
    let seal = h.recovery.seal_current().map_err(debug)?;
    let first = h.root.open_character_authority(&seal).await.map_err(debug)?;
    let second = second_root.open_character_authority(&seal).await.map_err(debug)?;
    for root in [&h.root, &second_root] {
        let current = FreshAdmissionStore::from_root(root.clone())
            .current_session_at(fence.game_session_id)
            .await?
            .0;
        assert_eq!(current.session_state(), GameSessionState::Active);
        assert_eq!(item_fence_of(current), Some(fence));
    }
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("concurrent collection's current native content")?;
    let content = crate::gameplay_transport::item_ref_admission::NativeItemWeights(native);
    let first_page = h.root
        .read_boss_reward_items(&first, &h.node, fence, death, 0, native.source_digest())
        .await?
        .ok_or("first root's live reward positive control")?;
    let second_page = second_root
        .read_boss_reward_items(&second, &h.node, fence, death, 0, native.source_digest())
        .await?
        .ok_or("second root's live reward positive control")?;
    assert_eq!(first_page, second_page);
    assert_eq!(first_page.status, BossRewardCollectionStatus::Collected);
    assert_eq!(first_page.items.len(), 1);
    assert_eq!((first_page.items[0].ordinal, first_page.items[0].quantity), (1, 1));
    let now: i64 = sqlx::query_scalar(
        "SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint",
    ).fetch_one(&h.pool).await?;
    assert!(first_page.expires_at_unix_ms > u64::try_from(now)?);

    let payload = encode_collect_all_request(&CollectAllRequest { death: Some(death) })
        .map_err(debug)?;
    let mut intent = COMMAND_TYPE_BOSS_REWARD_COLLECT_ALL.to_be_bytes().to_vec();
    intent.extend_from_slice(&payload);
    let left_request = BossRewardCollectRequest {
        command: CommandRef::new(fence.game_session_id, CommandId::new(1).map_err(debug)?),
        selection: BossRewardSelection::Group { death },
        intent,
        // This case exercises the durable owner boundary. Physical reach and
        // capability negotiation remain covered by the mounted wire cases.
        source_placement: "oteryn:reward-chest.concurrent_collection".into(),
        content_revision: format!("sha256:{}", native.source_digest().iter()
            .map(|byte| format!("{byte:02x}")).collect::<String>()),
        ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION.into(),
        sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
    };
    let mut right_request = left_request.clone();
    right_request.command = CommandRef::new(fence.game_session_id, CommandId::new(2).map_err(debug)?);
    let left_permit = SpellLanePermit::of_fresh_scope(fence.runtime_scope).await.map_err(debug)?;
    let right_permit = SpellLanePermit::of_fresh_scope(fence.runtime_scope).await.map_err(debug)?;
    let before: (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_item_instances),(SELECT count(*) FROM game_item_audit_outbox),(SELECT count(*) FROM game_item_container_entries)",
    ).fetch_one(&h.pool).await?;
    assert_eq!(before.2, 0, "the positive collection has a genuinely empty backpack");

    // These are two collection futures, not the server/client pair used by the
    // wire harness. The barrier admits both before either starts its DB call.
    let start = tokio::sync::Barrier::new(2);
    let left = async {
        start.wait().await;
        h.root.collect_boss_rewards(
            &left_permit, &first, &h.node, fence, left_request.clone(), &content, false,
        ).await
    };
    let right = async {
        start.wait().await;
        second_root.collect_boss_rewards(
            &right_permit, &second, &h.node, fence, right_request.clone(), &content, false,
        ).await
    };
    let (left, right) = tokio::time::timeout(
        std::time::Duration::from_secs(30), fixture::join_two(left, right),
    ).await.map_err(|_| "two-root collection did not settle within the existing PG bound")?;
    let left = left.map_err(debug)?.ok_or("left collection result")?;
    let right = right.map_err(debug)?.ok_or("right collection result")?;
    let winner = match (left.status, right.status) {
        (BossRewardCollectionStatus::Collected, BossRewardCollectionStatus::Empty) => 1_u64,
        (BossRewardCollectionStatus::Empty, BossRewardCollectionStatus::Collected) => 2_u64,
        other => return Err(format!("expected exactly one collector, got {other:?}").into()),
    };
    assert_eq!(u32::from(left.moved) + u32::from(right.moved), 1);
    assert_eq!((left.remaining, right.remaining), (0, 0));

    let counts: CustodyCounts = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_boss_reward_item_collections),(SELECT count(*) FROM game_boss_reward_collection_receipts),(SELECT count(*) FROM game_item_instances),(SELECT count(*) FROM game_item_audit_outbox),(SELECT item_count FROM game_character_boss_reward_chests),(SELECT count(*) FROM game_item_container_entries)",
    ).fetch_one(&h.pool).await?;
    assert_eq!(counts, (1, 2, before.0 + 1, before.1 + 1, 0, 1));
    let consistent: bool = sqlx::query_scalar(
        r#"SELECT count(*)=1 AND bool_and(
            c.item_ordinal=1 AND c.command_id=$2::text::numeric
            AND e.collected_transaction_id=c.transaction_id AND NOT e.expired
            AND i.quantity=1 AND i.lifecycle=1 AND i.minted_transaction_id=c.transaction_id
            AND i.world_id=c.world_id AND i.definition_family=e.definition_family
            AND i.definition_production_key=e.definition_production_key
            AND i.definition_revision_ref=e.definition_revision_ref
            AND p.character_id=c.character_id AND p.world_id=c.world_id
            AND p.parent_item_instance_id=c.parent_item_instance_id
            AND p.placement_ordinal=c.placement_ordinal AND p.placed_transaction_id=c.transaction_id
            AND a.transaction_id=c.transaction_id AND a.item_instance_id=c.item_instance_id)
        FROM game_boss_reward_item_collections c
        JOIN game_boss_reward_items e USING(character_id,world_id,death_id,item_ordinal)
        JOIN game_item_instances i ON i.item_instance_id=c.item_instance_id
        JOIN game_item_container_entries p ON p.item_instance_id=c.item_instance_id
        JOIN game_item_audit_outbox a ON a.event_id=c.event_id
        WHERE c.death_id=encode($1,'hex')::uuid"#,
    ).bind(death.as_slice()).bind(winner.to_string()).fetch_one(&h.pool).await?;
    assert!(consistent, "one ordinal must have one matching live custody and audit chain");
    let (item, parent, bytes): CustodyAudit = sqlx::query_as(
        "SELECT uuid_send(c.item_instance_id),uuid_send(c.parent_item_instance_id),a.envelope FROM game_boss_reward_item_collections c JOIN game_item_audit_outbox a USING(event_id)",
    ).fetch_one(&h.pool).await?;
    let (envelope, mint) = crate::durability::reward_claim_mint_audit::decode_reward_claim_mint_envelope(&bytes)
        .map_err(debug)?;
    let RuntimeScopeRefV1::Channel { channel_id, .. } = fence.runtime_scope else {
        return Err("two-root fixture must retain its actual Channel scope".into());
    };
    assert_eq!(envelope.channel_id.as_deref(), Some(channel_id.as_bytes().as_slice()));
    assert!(envelope.instance_id.is_none());
    let after = mint.after.ok_or("reward MINT state")?;
    assert_eq!(after.item_instance_id, item);
    assert_eq!(after.quantity, 1);
    let destination = mint.destination.ok_or("reward MINT destination")?;
    assert_eq!(destination.character_id.as_slice(), fence.character_id.as_bytes().as_slice());
    assert_eq!(destination.expected_game_session_id.as_slice(), fence.game_session_id.as_bytes().as_slice());
    assert_eq!(destination.expected_session_generation, fence.connection_generation.get());
    assert_eq!(destination.expected_character_lease_generation, fence.character_lease_generation);
    assert_eq!(destination.container_entry.ok_or("direct backpack entry")?.parent_item_instance_id, parent);
    let source = mint.source.ok_or("reward MINT source")?;
    assert_eq!(source.command_ref.ok_or("winning collection command")?.command_id, winner);
    assert!(source.reward_claim.is_none());
    let entitlement = source.boss_reward.ok_or("personal entitlement audit")?;
    assert_eq!(entitlement.death, death);
    assert_eq!(entitlement.item_ordinal, 1);

    let retained = snapshot(h).await?;
    // Read each command from the other root. The winner must retain success even
    // though no pending entitlement remains; the loser retains its Empty reply.
    let replay_left = second_root.collect_boss_rewards(
        &right_permit, &second, &h.node, fence, left_request, &content, false,
    ).await?.ok_or("left receipt from the other root")?;
    let replay_right = h.root.collect_boss_rewards(
        &left_permit, &first, &h.node, fence, right_request, &content, false,
    ).await?.ok_or("right receipt from the other root")?;
    assert_eq!(replay_left, left);
    assert_eq!(replay_right, right);
    assert_eq!(snapshot(h).await?, retained, "replay cannot alter custody, entitlements or either receipt");
    eprintln!("BOSS_REWARDS_CONCURRENT_COLLECTION POSTGRES_CASE_COMPLETED");
    Ok(())
}

async fn snapshot(h: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar(r#"SELECT jsonb_build_object(
        'chests',(SELECT jsonb_agg(to_jsonb(c) ORDER BY death_id) FROM game_character_boss_reward_chests c),
        'items',(SELECT jsonb_agg(to_jsonb(i) ORDER BY death_id,item_ordinal) FROM game_boss_reward_items i),
        'collections',(SELECT jsonb_agg(to_jsonb(c) ORDER BY transaction_id) FROM game_boss_reward_item_collections c),
        'receipts',(SELECT jsonb_agg(to_jsonb(r) ORDER BY command_id) FROM game_boss_reward_collection_receipts r),
        'instances',(SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i),
        'entries',(SELECT jsonb_agg(to_jsonb(e) ORDER BY item_instance_id) FROM game_item_container_entries e),
        'audits',(SELECT jsonb_agg(to_jsonb(a) ORDER BY event_id) FROM game_item_audit_outbox a)
    )::text"#).fetch_one(&h.pool).await?)
}
