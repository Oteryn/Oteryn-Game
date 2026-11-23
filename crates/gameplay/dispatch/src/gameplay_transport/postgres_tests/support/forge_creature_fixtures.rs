use super::party_xp_fixtures::Case;

use crate::bestiary_postgres_harness::{Harness, TestResult, debug, id};

use crate::foundation::{ExactActorRef, GameSessionId};

use crate::gameplay_transport::ComposedFreshAdmission;

pub(super) fn select_nominal_sliver_miss(
    runtime: &mut crate::foundation::ChannelRuntimeV1,
    rewards: &crate::content::creature_reward::CreatureRewardTable,
) -> TestResult {
    use oteryn_gameplay_items::loot_plan::{LootPlanDeathKey, plan_creature_loot};
    let row = rewards.row("oteryn:creature.rat").map_err(debug)?;
    let reference = row.loot_table_ref.as_ref().ok_or("rat loot reference")?;
    let mut legacy = row.loot_table.clone();
    let sliver = rewards.forge_sliver().ok_or("sliver item")?.clone();
    legacy.entries.push(crate::combat::LootTableEntry {
        item: sliver.clone(),
        min_count: 3,
        max_count: 3,
        probability_ppm: Some(1_000_000),
        details: Default::default(),
    });
    // Reuse the first carrier slot until the real activation-spawn incarnation
    // has an occurrence whose ordinary 100% sliver entry would miss.
    let position = crate::foundation::MovementLocalPosition { x: 1, y: 11, floor: -7 };
    for _ in 0..2048 {
        let probe = runtime.admit_pinned_test_creature(position).map_err(debug)?;
        runtime.remove_test_actor(probe).map_err(debug)?;
        let seed = LootPlanDeathKey::new(
            *probe.world_id().as_bytes(),
            *probe.channel_id().ok_or("Channel")?.as_bytes(),
            probe.scope_generation().get(),
            probe.actor_local_id(),
            probe.actor_local_generation() + 1,
        );
        let plan = plan_creature_loot(seed, reference, &legacy).map_err(debug)?;
        if !plan.entries.iter().any(|entry| entry.item == sliver) {
            return Ok(());
        }
    }
    Err("no nominal sliver miss found".into())
}

pub(super) async fn prepare(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    players: &[(ExactActorRef, GameSessionId)],
    case: Case,
) -> TestResult {
    {
        let runtime = owner.runtime.lock().await;
        let creature = runtime.visible_entities().creatures[0].actor;
        let snapshot = runtime.companion_snapshot(creature).map_err(debug)?;
        assert_eq!(
            snapshot.forge,
            Some(crate::domain::forge_creature::ForgeCreature::Fiendish)
        );
        let mut states = owner.states.spell_states.lock().await;
        for (index, (actor, session)) in players.iter().enumerate() {
            states
                .get_mut(&runtime, *actor, *session)
                .ok_or("player")?
                .apply_owner_premium_transition(
                    !matches!(case, Case::ForgeAllFree | Case::ForgeSliver)
                        && (!matches!(case, Case::ForgeFree) || index == 0),
                    Some(u64::MAX),
                )
                .map_err(debug)?;
        }
    }
    // A genuine prior ledger entry establishes a nearly full balance, preserving all guards.
    let fence = owner
        .current_quest_fence(players[0].1)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let mut tx = h.pool.begin().await?;
    // Fixture setup uses the existing ledger with the Character root locked. Production
    // settlement proves the full generation fence in party_kill_xp before this ledger call.
    sqlx::query(
        "SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .execute(&mut *tx)
    .await?;
    crate::durability::character_forge_dust::gain_forge_dust_in_transaction(
        &mut tx,
        &fence,
        crate::durability::character_forge_dust::ForgeDustGainCause::CreatureKill {
            occurrence: id(200),
        },
        98,
        crate::durability::character_forge_dust::ForgeDustWriteIds {
            entry_id: id(201),
            transaction_id: id(202),
        },
    )
    .await
    .map_err(debug)?;
    tx.commit().await?;
    if matches!(case, Case::ForgeNoShared | Case::EliteNoShared) {
        let result = crate::gameplay_transport::connection::FreshAdmissionAuthority::chat_intent(
            owner,
            players[0].0,
            players[0].1,
            25,
            oteryn_protocol_oteryn::chat::ChatIntent::Say {
                mode: oteryn_protocol_oteryn::chat::ChatSpeechMode::Say,
                text: "/sharedxp off".into(),
            },
        )
        .await;
        assert_eq!(
            result.disposition,
            oteryn_protocol_oteryn::chat::ChatDisposition::Ok
        );
    }
    Ok(())
}

pub(super) async fn assert_receipts(h: &Harness, case: Case) -> TestResult {
    let awarded_xp: i64 = sqlx::query_scalar(
        "SELECT COALESCE(sum(xp_awarded),0)::BIGINT FROM game_party_kill_xp_members",
    )
    .fetch_one(&h.pool)
    .await?;
    if matches!(case, Case::ForgeReplay) {
        assert_eq!(
            awarded_xp, 42,
            "5 rat XP * fiendish 2x, then 30% two-vocation bonus: ceil(10 * 1.3 / 2) = 7 per member; level 1 low-level bonus: 7 * 2 = 14; Premium stamina: floor(14 * 1.5) = 21 each, 42 total"
        );
    }
    let entries: Vec<(Vec<u8>, i64, i64, i64, i64)> = sqlx::query_as("SELECT uuid_send(character_id),amount,lost_amount,balance_before,balance_after FROM game_character_forge_dust_entries WHERE cause_occurrence_id<>encode($1,'hex')::uuid ORDER BY balance_before LIMIT 3")
        .bind(id(200).as_slice()).fetch_all(&h.pool).await?;
    let expected = match case {
        Case::ForgeAllFree | Case::ForgeSliver => 0,
        Case::ForgeFree | Case::ForgeIdle | Case::ForgeFar => 1,
        Case::ForgeForeign => 3,
        _ => 2,
    };
    eprintln!("CREATURES_FORGE_POSTGRES_CASE_EXECUTED");
    assert_eq!(entries.len(), expected);
    if expected > 0 {
        use sha2::{Digest, Sha256};
        let death_key: Vec<u8> = sqlx::query_scalar("SELECT death_key FROM game_party_kill_xp_receipts")
            .fetch_one(&h.pool).await?;
        assert_eq!(entries.iter().filter(|row| row.3 == 98).count(), 1);
        for (character, requested, lost, before, after) in &entries {
            let digest = Sha256::new()
                .chain_update(b"oteryn:forge-dust-recipient:v1")
                .chain_update(&death_key)
                .chain_update(character)
                .finalize();
            let expected = i64::from(crate::domain::forge_creature::ForgeCreature::Fiendish.dust(digest[0]));
            let credited = expected.min(100 - before);
            assert_eq!((*requested, *lost, *after), (expected, expected - credited, before + credited), "recipient-specific draw and cap");
        }
        if matches!(case, Case::ForgeReplay) {
            assert_ne!(entries[0].1, entries[1].1, "fixture exercises distinct draws");
        }
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_party_kill_xp_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(count, 1, "concurrent replay retains one kill receipt");
    let slivers: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT i.quantity,e.placement_ordinal::bigint FROM game_item_instances i JOIN game_item_corpse_container_entries e ON e.item_instance_id=i.item_instance_id WHERE i.definition_production_key='oteryn:item.tibia.i37109' LIMIT 2"
    ).fetch_all(&h.pool).await?;
    assert_eq!(
        slivers.len(),
        1,
        "one ordinary sliver mint in the existing corpse"
    );
    assert!((3..=7).contains(&slivers[0].0), "pinned fiendish range");
    assert!(slivers[0].1 > 0);
    eprintln!(
        "FORGECREATURE POSTGRES_ASSERTIONS_COMPLETED recipients={}",
        entries.len()
    );
    Ok(())
}

pub(super) async fn ledger(h: &Harness) -> TestResult<Vec<(Vec<u8>, i64, i64, i64, i64)>> {
    Ok(sqlx::query_as("SELECT uuid_send(entry_id),amount,lost_amount,balance_before,balance_after FROM game_character_forge_dust_entries ORDER BY entry_id LIMIT 4").fetch_all(&h.pool).await?)
}

pub(super) async fn assert_no_gain(h: &Harness) -> TestResult {
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_forge_dust_entries WHERE cause_occurrence_id<>encode($1,'hex')::uuid")
        .bind(id(200).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        count, 0,
        "failed fenced transaction cannot leave a dust credit"
    );
    Ok(())
}
