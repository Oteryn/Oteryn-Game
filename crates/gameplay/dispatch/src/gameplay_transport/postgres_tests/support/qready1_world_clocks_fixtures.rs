use oteryn_gameplay_world::map::view::MapFacts;
use super::fixture::Harness;

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use oteryn_gameplay_quests::quest::world_callbacks::Callback;

use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

async fn use_command(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    target: &WorldObjectTarget,
    id: u64,
) -> TestResult<UseDisposition> {
    // The wire helper respects the real 200 ms action cooldown and retains replay state.
    super::quest_callbacks_fixtures::wire_use_targets_retained(owner, admitted, &[(id, target)])
        .await?
        .first()
        .map(|(_, disposition)| *disposition)
        .ok_or_else(|| "USE response".into())
}

async fn snapshot(h: &Harness, callback: &Callback) -> TestResult<(i64, i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),-1),(SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$2),(SELECT count(*) FROM game_reward_claim_mint_receipts),(SELECT coalesce(sum(quantity),0)::bigint FROM game_quest_callback_consumptions WHERE transition_key=$2)")
        .bind(&callback.state_ops[0].track).bind(&callback.key).fetch_one(&h.pool).await?)
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: &Callback,
) -> TestResult {
    let epoch = 1_700_000_000;
    h.root.set_test_quest_clock(epoch)?;
    let wait = match callback.state_ops[0].from {
        oteryn_gameplay_quests::quest::reward_callbacks::StateCondition::ElapsedAtLeast {
            seconds,
        } => i64::from(seconds),
        _ => return Err("production clock predicate".into()),
    };
    let captain = callback.key == "oteryn:quest-callback/captain-iglues-left-chest";
    if captain {
        assert_eq!(wait, 86_400);
    }
    let mint_lines = i64::from(callback.effects.item_reward.is_some());
    let position = callback.selector.position.ok_or("source position")?;
    let key = oteryn_world_bundle::bundle::placement_key(position.floor, position.x, position.y, 1)
        .ok_or("source key")?;
    let mut placement = owner
        .bundle_map()
        .ok_or("map")?
        .overlay
        .base()
        .digest()
        .to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let mut target = WorldObjectTarget {
        placement,
        expected_revision: 0,
    };
    let fence = admitted.item_fence.ok_or("item fence")?;
    // Start the Captain case one second before a different collector's stock renews.
    // This character has no quest receipt and would pass a personal clock alone.
    if captain {
        sqlx::query("INSERT INTO game_captain_iglue_chest_stock(world_id, channel_id, collected_at) VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3)")
            .bind(admitted.world_id.as_bytes().as_slice())
            .bind(admitted.channel_id.as_bytes().as_slice())
            .bind(epoch - wait + 1)
            .execute(&h.pool)
            .await?;
        assert_eq!(use_command(owner, admitted, &target, 1).await?, UseDisposition::Rejected);
        assert_eq!(snapshot(h, callback).await?, (-1, 0, 0, 0));
        h.root.advance_test_quest_clock(1)?;
    }
    let epoch = epoch + i64::from(captain);
    let offset = u64::from(callback.offerings.is_empty() && !captain);
    if !callback.offerings.is_empty() {
        assert_eq!(
            use_command(owner, admitted, &target, 1).await?,
            UseDisposition::Rejected
        );
        assert_eq!(snapshot(h, callback).await?, (-1, 0, 0, 0));
        for (i, input) in callback.offerings.iter().enumerate() {
            super::quest_world_callbacks_fixtures::input_item(owner, fence, input, 101 + i as u64, false)
                .await?;
        }
    }
    assert_eq!(
        use_command(owner, admitted, &target, 2 - offset).await?,
        UseDisposition::Committed
    );
    let first_target = target.clone();
    target.expected_revision = owner.bundle_map().ok_or("map")?.facts.object_revision(key);
    let before = snapshot(h, callback).await?;
    assert_eq!(
        before,
        (
            epoch,
            1,
            mint_lines,
            if callback.offerings.is_empty() { 0 } else { 6 }
        )
    );
    assert_eq!(
        use_command(owner, admitted, &target, 3 - offset).await?,
        UseDisposition::Rejected
    );
    assert_eq!(snapshot(h, callback).await?, before);
    h.root.advance_test_quest_clock(wait - 1)?;
    assert_eq!(
        use_command(owner, admitted, &target, 4 - offset).await?,
        UseDisposition::Rejected
    );
    assert_eq!(snapshot(h, callback).await?, before);
    h.root.advance_test_quest_clock(1)?;
    for (i, input) in callback.offerings.iter().enumerate() {
        super::quest_world_callbacks_fixtures::input_item(owner, fence, input, 201 + i as u64, false)
            .await?;
    }
    assert_eq!(
        use_command(owner, admitted, &target, 5 - offset).await?,
        UseDisposition::Committed
    );
    let after = snapshot(h, callback).await?;
    assert_eq!(
        after,
        (
            epoch + wait,
            2,
            mint_lines * 2,
            if callback.offerings.is_empty() { 0 } else { 12 }
        )
    );
    // A new transport retains its sequence, not its retired response cache.
    // SQL replay after reconnect is covered by qready1_world_clock_rewards.
    let replay = use_command(owner, admitted, &first_target, 2 - offset).await;
    assert!(
        replay
            .as_ref()
            .err()
            .is_some_and(|error| error.to_string().contains("CommandOutcomeExpired")),
        "expired historical wire command must close the transport"
    );
    assert_eq!(
        snapshot(h, callback).await?,
        after,
        "historical wire replay cannot mint or consume twice"
    );
    if captain {
        let collected_at: i64 = sqlx::query_scalar(
            "SELECT collected_at FROM game_captain_iglue_chest_stock
             WHERE world_id = encode($1,'hex')::uuid AND channel_id = encode($2,'hex')::uuid",
        )
        .bind(admitted.world_id.as_bytes().as_slice())
        .bind(admitted.channel_id.as_bytes().as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(collected_at, epoch + wait);
        let rewards: Vec<(String, i64)> = sqlx::query_as(
            "SELECT i.definition_production_key,sum(i.quantity)::bigint
             FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id)
             WHERE i.lifecycle=1 AND i.definition_production_key IN ('oteryn:item.tibia.i3506','oteryn:item.tibia.i3579')
             GROUP BY 1 ORDER BY 1",
        ).fetch_all(&h.pool).await?;
        assert_eq!(rewards, vec![("oteryn:item.tibia.i3506".into(), 2), ("oteryn:item.tibia.i3579".into(), 24)]);
        let texts: Vec<String> = sqlx::query_scalar(
            "SELECT t.text FROM game_item_texts t JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i3506'",
        ).fetch_all(&h.pool).await?;
        assert_eq!(texts, ["Treasure of Captain Iglue"; 2]);
    } else if callback.effects.item_reward.is_some() {
        let reward:(String,i64)=sqlx::query_as("SELECT min(i.definition_production_key),sum(m.quantity)::bigint FROM game_reward_claim_mint_receipts m JOIN game_item_instances i USING(item_instance_id)").fetch_one(&h.pool).await?;
        assert_eq!(reward, ("oteryn:item.tibia.i5953".into(), 2));
    }
    Ok(())
}
