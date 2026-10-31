//! A genuine transferred Instance session settles a native reward-boss death and
//! uses the same durable collection owner. The physical chest wire traversal is
//! separately exercised by boss_rewards.rs against its qualified bundle fixture.
use super::*;
use crate::durability::reward_claim_mint::boss_reward::{
    BossRewardCollectRequest, BossRewardCollectionStatus, BossRewardSelection,
};
use crate::foundation::runtime_actor_spell_types::{
    OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, record_batch_kills};

const GOLD: &str = "oteryn:item.tibia.i3031";

pub(super) fn source_boss(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult<(String, crate::foundation::CompiledCreaturePolicy)> {
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("active native gameplay")?;
    // The fixture's one test-only classification is compiled before activation.
    // Production reward bosses remain unavailable under current corpse admission.
    let policy = native
        .policies()
        .policies()
        .iter()
        .find(|policy| {
            policy.definition_key == "oteryn:creature.rat" && policy.reward_boss == Some(true)
        })
        .ok_or("compiled native reward-boss fixture")?;
    let row = native
        .reward_table()
        .row(&policy.definition_key)
        .map_err(debug)?;
    let factor = oteryn_gameplay_encounter_rules::boss::reward_chest::contribution_loot_factors(&[
        oteryn_gameplay_encounter_rules::boss::reward_chest::FrozenBossContribution {
            character: [1; 16],
            contribution: oteryn_gameplay_encounter_rules::boss::RewardParticipation {
                damage_dealt: u64::try_from(policy.maximum_health)?,
                damage_received: 0,
                healing_others: 0,
            },
        },
    ])[0];
    assert!(
        row.loot_table
            .entries
            .iter()
            .any(|entry| entry.item.production_key == GOLD
                && entry
                    .probability_ppm
                    .is_some_and(|chance| f64::from(chance) * factor * 0.95 >= 1_000_000.0)),
        "unchanged source gold is guaranteed by solo contribution even at minimum source jitter"
    );
    let boss = row
        .boss
        .as_ref()
        .map_or_else(|| policy.definition_key.clone(), |boss| boss.key.clone());
    Ok((boss, policy.as_ref().clone()))
}

pub(super) async fn settle_and_collect(
    h: &Harness,
    origin: &ComposedFreshAdmission<'_, '_, '_>,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    policy: &crate::foundation::CompiledCreaturePolicy,
    late: bool,
) -> TestResult {
    let fence = admitted.item_fence.ok_or("actual Instance fence")?;
    let RuntimeScopeRefV1::Instance { instance_id, .. } = fence.runtime_scope else {
        return Err("real Instance admission required".into());
    };
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("current native gameplay")?;
    let actor = admitted.runtime_actor.ok_or("transferred actor")?;
    if late {
        let (deadline, now): (i64, i64) = sqlx::query_as(
            "SELECT deadline,floor(extract(epoch FROM clock_timestamp())*1000)::bigint FROM game_boss_room_activities",
        ).fetch_one(&h.pool).await?;
        tokio::time::sleep(std::time::Duration::from_millis(
            u64::try_from(deadline.saturating_sub(now).max(0))? + 5,
        ))
        .await;
    }
    let now = owner.owner_now();
    let batch = {
        let mut runtime = owner.runtime.lock().await;
        assert_eq!(runtime.binding().runtime_scope(), fence.runtime_scope);
        let target = runtime
            .visible_entities()
            .creatures
            .first()
            .ok_or("actual room boss")?
            .actor;
        let batch = OwnerCombatBatch {
            caster: actor,
            attacker: crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                .map_err(debug)?,
            current_lease_generation: fence.character_lease_generation,
            command: CommandRef::new(admitted.game_session_id, CommandId::new(91).map_err(debug)?),
            occurrence: SpellOccurrenceBinding {
                id: "boss-activity:reward-lethal".into(),
                revisions: ["r1", "r1", "r1", "r1", "r1"].map(str::to_owned),
            },
            binding: b"boss-activity-reward-lethal".to_vec(),
            anchor: None,
            now_us: Some(now),
            now_ms: now.get() / 1_000,
            effects: vec![OwnerCombatEffect {
                target,
                sub_ordinal: 0,
                change: OwnerCombatChange::Damage {
                    target_atom: runtime.creature_spell_target_atom(target).map_err(debug)?,
                    magnitude: policy.maximum_health,
                },
            }],
            deferred: None,
        };
        let staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
        let receipt = runtime.commit_spell_batch(staged).map_err(debug)?;
        assert!(receipt.applied);
        assert!(runtime.retains_corpse_projection(target));
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
            now.get() / 1_000,
        );
        assert!(attack.kills().has_session(admitted.game_session_id));
        batch
    };
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Drained
    );
    let retained: (Vec<u8>, i16, Vec<u8>, bool, i64) = sqlx::query_as(
        "SELECT uuid_send(death_id),scope_kind,uuid_send(instance_id),channel_id IS NULL,minted_at FROM game_boss_reward_deaths",
    ).fetch_one(&h.pool).await?;
    let death: [u8; 16] = retained
        .0
        .as_slice()
        .try_into()
        .map_err(|_| "death occurrence")?;
    assert_eq!(
        (retained.1, retained.2.as_slice(), retained.3),
        (2, instance_id.as_slice(), true)
    );
    let outcome: i16 = sqlx::query_scalar("SELECT settlement_outcome FROM game_boss_reward_deaths")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(outcome, if late { 2 } else { 1 });
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_corpse_container_entries")
            .fetch_one(&h.pool)
            .await?,
        0,
        "personal reward bosses have no second ordinary-corpse loot copy"
    );
    if late {
        let terminal: (i16, bool, bool, i64) = sqlx::query_as(
            "SELECT state,defeat_id IS NULL,terminal_at IS NULL,deadline FROM game_boss_room_activities",
        ).fetch_one(&h.pool).await?;
        assert_eq!((terminal.0, terminal.1, terminal.2), (1, true, true));
        assert!(retained.4 > terminal.3);
        let census: (bool, i32) =
            sqlx::query_as("SELECT eligible,item_count FROM game_boss_reward_contributors")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(
            census,
            (true, 0),
            "late reason is distinct from participation eligibility"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_boss_reward_chests")
                .fetch_one(&h.pool)
                .await?,
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_boss_reward_items")
                .fetch_one(&h.pool)
                .await?,
            0
        );
        // A command replay retains the original native death and its terminal
        // declined receipt; it cannot turn a late kill into a new reward roll.
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
                now.get() / 1_000,
            );
        }
        assert_eq!(
            owner.drain_kill_rewards(admitted.game_session_id).await,
            DrainOutcome::Drained
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_boss_reward_deaths")
                .fetch_one(&h.pool)
                .await?,
            1
        );
        let returned = owner
            .return_due_activity(
                admitted,
                crate::durability::activity_handoff::ActivityReturnReason::Timeout,
            )
            .await
            .map_err(|_| "late death must not block native timeout return")?;
        let crate::gameplay_transport::native_activity::NativeActivityOutcome::Committed(commit) =
            returned
        else {
            return Err("late native death timeout return was refused".into());
        };
        let next = commit
            .connections
            .first()
            .ok_or("real timeout successor")?
            .1;
        assert!(matches!(
            next.item_fence.ok_or("returned fence")?.runtime_scope,
            RuntimeScopeRefV1::Channel { .. }
        ));
        assert_ne!(next.game_session_id, admitted.game_session_id);
        origin
            .publish_activity_commit(&commit)
            .map_err(|_| "publish actual timeout return")?;
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_activity_runtime_returns")
                .fetch_one(&h.pool)
                .await?,
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_boss_reward_items")
                .fetch_one(&h.pool)
                .await?,
            0
        );
        return Ok(());
    }
    let terminal: (i16, Vec<u8>, i64, i64) = sqlx::query_as(
        "SELECT state,uuid_send(defeat_id),terminal_at,exit_deadline FROM game_boss_room_activities",
    ).fetch_one(&h.pool).await?;
    assert_eq!(
        (terminal.0, terminal.1.as_slice(), terminal.2),
        (2, death.as_slice(), retained.4)
    );
    assert_eq!(
        terminal.3,
        retained.4 + 1_000,
        "authored after-kill deadline commits with the exact opaque death"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_boss_reward_contributors")
            .fetch_one(&h.pool)
            .await?,
        1
    );
    // Discover through the real admitted command envelope inside the current Instance.
    // Its retained origin Channel is not the authority for this private read.
    let groups = open_reward_groups(owner, admitted).await?;
    assert!(
        groups.windows(death.len()).any(|candidate| candidate == death),
        "command 340 must expose the actual Instance death's personal group"
    );
    let ordinal: i32 = sqlx::query_scalar("SELECT item_ordinal FROM game_boss_reward_items WHERE death_id=encode($1,'hex')::uuid AND definition_production_key=$2 ORDER BY item_ordinal LIMIT 1")
        .bind(death.as_slice()).bind(GOLD).fetch_one(&h.pool).await?;
    let selection = BossRewardSelection::Item {
        death,
        ordinal: u16::try_from(ordinal)?,
    };
    let body = oteryn_protocol_oteryn::boss_rewards::encode_take_request(
        &oteryn_protocol_oteryn::boss_rewards::TakeRequest {
            death,
            ordinal: u16::try_from(ordinal)?,
        },
    )
    .map_err(debug)?;
    let mut intent = oteryn_protocol_oteryn::boss_rewards::COMMAND_TYPE_BOSS_REWARD_TAKE
        .to_be_bytes()
        .to_vec();
    intent.extend_from_slice(&body);
    let request = BossRewardCollectRequest {
        command: CommandRef::new(admitted.game_session_id, CommandId::new(92).map_err(debug)?),
        selection,
        intent,
        source_placement: "oteryn:reward-chest.instance_custody_fixture".into(),
        content_revision: format!(
            "sha256:{}",
            native
                .source_digest()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        ),
        ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION.into(),
        sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
    };
    let content = crate::gameplay_transport::item_ref_admission::NativeItemWeights(native);
    let permit = owner
        .spell_lane_permit()
        .await
        .ok_or("exact Instance lane")?;
    let first = h
        .root
        .collect_boss_rewards(
            &permit,
            owner.character,
            owner.holder,
            fence,
            request.clone(),
            &content,
            false,
        )
        .await?
        .ok_or("actual Instance collection")?;
    assert_eq!(
        (first.status, first.moved, first.remaining),
        (BossRewardCollectionStatus::Collected, 1, 0)
    );
    let replay = h
        .root
        .collect_boss_rewards(
            &permit,
            owner.character,
            owner.holder,
            fence,
            request.clone(),
            &content,
            true,
        )
        .await?
        .ok_or("retained Instance collection")?;
    assert_eq!(replay, first);
    drop(permit);
    let audit: Vec<u8> = sqlx::query_scalar("SELECT a.envelope FROM game_boss_reward_item_collections c JOIN game_item_audit_outbox a USING(event_id)")
        .fetch_one(&h.pool).await?;
    let (envelope, minted) =
        crate::durability::reward_claim_mint_audit::decode_reward_claim_mint_envelope(&audit)
            .map_err(debug)?;
    assert!(envelope.channel_id.is_none());
    assert_eq!(
        envelope.instance_id.as_deref(),
        Some(instance_id.as_slice())
    );
    assert_eq!(
        minted
            .source
            .ok_or("mint source")?
            .boss_reward
            .ok_or("boss provenance")?
            .death,
        death
    );
    let mut borrowed_channel = fence;
    borrowed_channel.runtime_scope =
        RuntimeScopeRefV1::channel(admitted.world_id, admitted.channel_id);
    let origin_lane = origin
        .spell_lane_permit()
        .await
        .ok_or("actual origin Channel lane")?;
    let refused = h
        .root
        .collect_boss_rewards(
            &origin_lane,
            owner.character,
            owner.holder,
            borrowed_channel,
            request,
            &content,
            true,
        )
        .await?
        .ok_or("namespace refusal")?;
    assert_eq!(
        refused.status,
        BossRewardCollectionStatus::Rejected,
        "a current Instance session cannot replay its reward through the origin Channel namespace"
    );
    Ok(())
}

/// Existing command 340 has no physical-map prerequisite. Exercise the real
/// Instance stream and routing, independently of the direct durable custody proof.
async fn open_reward_groups(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult<Vec<u8>> {
    let command_id = admitted.continuity.next_command_id;
    let (mut server, mut client) = tokio::io::duplex(16_384);
    let serving = async {
        let ended = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        ended
    };
    let driving = async {
        loop {
            let frame = read_frame(&mut client).await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        write_frame(
            &mut client,
            &encode_client_command(
                admitted.continuity.connection_generation,
                &ClientCommandValue {
                    command_id,
                    command_type: wire::COMMAND_TYPE_BOSS_REWARD_OPEN,
                    payload: &[],
                },
            ).map_err(debug)?,
        ).await?;
        let payload = loop {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == MessageType::CommandResult {
                let result = decode_command_result(envelope.payload()).map_err(debug)?;
                assert_eq!(result.command_id, command_id);
                assert_eq!(result.status, CommandStatus::Accepted,
                    "current Instance reward discovery must pass the exact scope fence");
                break result.payload.to_vec();
            }
        };
        client.shutdown().await?;
        Ok(payload)
    };
    let (ended, result): (_, TestResult<Vec<u8>>) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        fixture::join_two(serving, driving),
    ).await?;
    result.map_err(|error| format!("Instance command 340 ended {ended:?}: {error}").into())
}
