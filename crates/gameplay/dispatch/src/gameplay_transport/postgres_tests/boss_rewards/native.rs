//! Real activated reward-boss policy, owner-issued lethal receipt, durable drain and replay.
use super::*;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::runtime_actor_spell_types::{
    OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
};
use crate::foundation::{CharacterLease, CommandId, CommandRef, MovementLocalPosition};
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, record_batch_kills};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::boss_rewards::{
    BossRewardStatus, BrowseRequest, COMMAND_TYPE_BOSS_REWARD_BROWSE, MAX_PAGE_ROWS,
    decode_browse_reply, encode_browse_request,
};

pub(super) async fn native_death(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    fence: CurrentCharacterItemFence,
    creature: &str,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("active native gameplay")?;
    // Crystal data-global/monster/raids/grand_mother_foulscale.lua:5-25,40-46,78-97:
    // the production case keeps source boss classification, HP, XP, corpse and loot.
    // The original rat case retains its explicitly test-only classification.
    let policy = native
        .policies()
        .policies()
        .iter()
        .find(|policy| {
            policy.definition_key == creature && policy.reward_boss == Some(true)
        })
        .ok_or("compiled native reward-boss fixture")?;
    native
        .reward_table()
        .row(&policy.definition_key)
        .map_err(debug)?;
    let actor = admitted.runtime_actor.ok_or("player actor")?;
    let now = owner.owner_now();
    let batch = {
        let mut runtime = owner.runtime.lock().await;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                admitted.game_session_id,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        let pin = runtime.content_pin().clone();
        // The physical fixture authors its static reward chest on x=6. Spawn
        // the native reward fixture on the existing open Ground cell instead.
        runtime
            .realize_native_qualification_spawn(
                pin,
                &policy.definition_key,
                MovementLocalPosition {
                    x: 3,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        let boss = runtime
            .visible_entities()
            .creatures
            .first()
            .ok_or("source-qualified boss")?
            .actor;
        let batch = OwnerCombatBatch {
            caster: actor,
            attacker: crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                .map_err(debug)?,
            current_lease_generation: fence.character_lease_generation,
            command: CommandRef::new(admitted.game_session_id, CommandId::new(91).map_err(debug)?),
            occurrence: SpellOccurrenceBinding {
                id: "boss-rewards:native-lethal".into(),
                revisions: ["r1", "r1", "r1", "r1", "r1"].map(str::to_owned),
            },
            binding: b"boss-rewards-native-lethal".to_vec(),
            anchor: None,
            now_us: Some(now),
            now_ms: now.get() / 1000,
            effects: vec![OwnerCombatEffect {
                target: boss,
                sub_ordinal: 0,
                change: OwnerCombatChange::Damage {
                    target_atom: runtime.creature_spell_target_atom(boss).map_err(debug)?,
                    magnitude: policy.maximum_health,
                },
            }],
            deferred: None,
        };
        let staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
        let receipt = runtime.commit_spell_batch(staged).map_err(debug)?;
        assert!(receipt.applied);
        assert!(receipt.effects.iter().any(|effect| {
            effect.target == boss && effect.health.is_some_and(|health| health.health_after == 0)
        }));
        assert!(runtime.live_creature_health(boss).is_none());
        let attack = owner.states.attack.lock().await;
        record_batch_kills(
            &mut runtime,
            KillSink {
                prey: None,
                activity: &[],
                queue: attack.kills(),
                rewards: native.reward_table(),
                charms: None,
            },
            &receipt,
            now.get() / 1000,
        );
        // The combat commit installs lethal vitals. record_batch_kills invokes
        // project_fixed_one_creature_death and owns the retained projection.
        // This is the native death proof, independent of any later Item MINT.
        assert!(runtime.retains_corpse_projection(boss));
        assert!(
            attack.kills().has_session(admitted.game_session_id),
            "real lethal capture enters the production queue"
        );
        batch
    };
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Drained
    );
    let before: (Vec<u8>, i64, Vec<u8>) =
        sqlx::query_as("SELECT uuid_send(death_id),minted_at,intent FROM game_boss_reward_deaths")
            .fetch_one(&h.pool)
            .await?;
    let count: (i64, i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_boss_reward_deaths),(SELECT count(*) FROM game_boss_reward_contributors),(SELECT count(*) FROM game_character_boss_reward_chests),(SELECT count(*) FROM game_boss_reward_items)")
        .fetch_one(&h.pool).await?;
    assert_eq!((count.0, count.1, count.2), (1, 1, 1));
    assert!(
        count.3 > 0,
        "real source loot reaches immutable personal entitlements"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_corpse_container_entries")
            .fetch_one(&h.pool)
            .await?,
        0,
        "personal reward loot is not also minted inside the ordinary corpse"
    );
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    {
        let mut runtime = owner.runtime.lock().await;
        let staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
        let receipt = runtime.commit_spell_batch(staged).map_err(debug)?;
        assert!(!receipt.applied);
        let attack = owner.states.attack.lock().await;
        record_batch_kills(
            &mut runtime,
            KillSink {
                prey: None,
                activity: &[],
                queue: attack.kills(),
                rewards: native.reward_table(),
                charms: None,
            },
            &receipt,
            now.get() / 1000,
        );
        assert!(attack.kills().has_session(admitted.game_session_id));
    }
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Drained,
        "a later native replay retains the first mint timestamp instead of conflicting forever"
    );
    let after: (Vec<u8>, i64, Vec<u8>) =
        sqlx::query_as("SELECT uuid_send(death_id),minted_at,intent FROM game_boss_reward_deaths")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(before, after);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_boss_reward_items")
            .fetch_one(&h.pool)
            .await?,
        count.3
    );
    let death: [u8; 16] = before.0.try_into().map_err(|_| "death UUID")?;
    let browse = encode_browse_request(&BrowseRequest {
        death,
        after_ordinal: 0,
    })
    .map_err(debug)?;
    let (_, bytes) = exchange(owner, admitted, 1, COMMAND_TYPE_BOSS_REWARD_BROWSE, &browse).await?;
    let page = decode_browse_reply(&bytes).map_err(debug)?;
    assert_eq!(page.status, BossRewardStatus::Ok);
    assert_eq!(
        page.rows.len(),
        usize::try_from(count.3)?.min(MAX_PAGE_ROWS)
    );
    let first = page.rows.first().ok_or("native personal reward row")?;
    let take = encode_take_request(&TakeRequest { death, ordinal: first.ordinal }).map_err(debug)?;
    let (_, bytes) = exchange(owner, admitted, 2, COMMAND_TYPE_BOSS_REWARD_TAKE, &take).await?;
    let collected = decode_collect_reply(&bytes).map_err(debug)?;
    assert_eq!(collected.status, BossRewardStatus::Ok);
    assert_eq!(collected.moved, 1);
    let custody: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_boss_reward_items r JOIN game_boss_reward_item_collections c ON c.transaction_id=r.collected_transaction_id AND c.death_id=r.death_id AND c.item_ordinal=r.item_ordinal JOIN game_item_instances i ON i.item_instance_id=c.item_instance_id JOIN game_item_container_entries e ON e.item_instance_id=i.item_instance_id AND e.parent_item_instance_id=c.parent_item_instance_id JOIN game_item_container_slots s ON s.item_instance_id=e.parent_item_instance_id AND s.character_id=r.character_id WHERE r.death_id=encode($1,'hex')::uuid AND r.item_ordinal=$2 AND i.lifecycle=1",
    ).bind(death.as_slice()).bind(i32::from(first.ordinal)).fetch_one(&h.pool).await?;
    assert_eq!(custody, 1, "the native entitlement reaches live backpack Item custody");
    let (_, replay) = exchange(owner, admitted, 2, COMMAND_TYPE_BOSS_REWARD_TAKE, &take).await?;
    assert_eq!(bytes, replay, "physical collection command replay preserves its receipt");
    Ok(())
}
