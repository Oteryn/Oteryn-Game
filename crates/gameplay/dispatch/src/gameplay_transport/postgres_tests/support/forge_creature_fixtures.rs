use super::party_xp_fixtures::Case;

use crate::bestiary_postgres_harness::{Harness, TestResult, debug, id};

use crate::foundation::{ExactActorRef, GameSessionId};

use crate::gameplay_transport::ComposedFreshAdmission;

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
                    !matches!(case, Case::ForgeAllFree)
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
    let entries: Vec<(i64, i64, i64, i64)> = sqlx::query_as("SELECT amount,lost_amount,balance_before,balance_after FROM game_character_forge_dust_entries WHERE cause_occurrence_id<>encode($1,'hex')::uuid ORDER BY balance_before LIMIT 3")
        .bind(id(200).as_slice()).fetch_all(&h.pool).await?;
    let expected = match case {
        Case::ForgeAllFree => 0,
        Case::ForgeFree | Case::ForgeIdle | Case::ForgeFar => 1,
        Case::ForgeForeign => 3,
        _ => 2,
    };
    assert_eq!(entries.len(), expected);
    if expected > 0 {
        let capped = entries
            .iter()
            .find(|row| row.2 == 98)
            .ok_or("capped recipient")?;
        assert!((15..=45).contains(&capped.0));
        assert_eq!((capped.1, capped.3), (capped.0 - 2, 100));
        if expected >= 2 {
            let full: Vec<_> = entries.iter().filter(|row| row.2 == 0).collect();
            assert_eq!(full.len(), expected - 1);
            for row in full {
                assert_eq!((row.0, row.1, row.3), (capped.0, 0, capped.0));
            }
        }
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_party_kill_xp_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(count, 1, "concurrent replay retains one kill receipt");
    assert_committed_sliver_in_corpse(h).await?;
    eprintln!(
        "FORGECREATURE POSTGRES_ASSERTIONS_COMPLETED recipients={}",
        entries.len()
    );
    Ok(())
}

pub(super) async fn assert_committed_sliver_in_corpse(h: &Harness) -> TestResult {
    // materialized_at belongs to the corpse's Ground receipt. A child MINT
    // commits its own receipt and corpse entry in the same transaction.
    let slivers: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT i.quantity,e.placement_ordinal::bigint
           FROM game_item_mint_receipts r
           JOIN game_item_instances i
             ON i.item_instance_id=r.item_instance_id AND i.minted_transaction_id=r.transaction_id
           JOIN game_item_corpse_container_entries e
             ON e.item_instance_id=i.item_instance_id
            AND e.parent_item_instance_id=r.destination_parent_item_instance_id
            AND e.placement_ordinal=r.destination_ordinal
            AND e.placed_transaction_id=r.transaction_id
           JOIN game_item_mint_receipts corpse ON corpse.item_instance_id=e.parent_item_instance_id
          WHERE r.loot_purpose_key='oteryn:item.tibia.i37109'
            AND i.definition_production_key=r.loot_purpose_key AND i.lifecycle=1
            AND corpse.loot_purpose_key='CORPSE_MATERIALIZATION'
            AND corpse.materialized_at IS NOT NULL
          LIMIT 2",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(slivers.len(), 1, "one committed sliver MINT in the corpse");
    assert!((3..=7).contains(&slivers[0].0), "pinned fiendish range");
    assert!(slivers[0].1 > 0);
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

pub(super) fn rat_loot_that_misses() -> TestResult<Vec<u8>> {
    let mut loot: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../../../../tools/content-schema/native-gameplay/loot-tables.json"
    ))?;
    let rat = loot["tables"].as_array_mut().ok_or("loot tables")?.iter_mut()
        .find(|table| table["identity"]["key"] == "oteryn:loot.creature.rat")
        .ok_or("Rat loot table")?;
    for entry in rat["entries"].as_array_mut().ok_or("Rat entries")? {
        entry["probability_ppm"] = 0.into();
    }
    Ok(serde_json::to_vec(&loot)?)
}

pub(super) async fn assert_duplex_expiry_retry(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    pending: &crate::gameplay_transport::kill_reward::PendingKillSettlement,
) -> TestResult {
    use crate::gameplay_transport::kill_reward::DrainOutcome;
    let principal = pending.facts.principal;
    let character = crate::domain::CharacterId::from_bytes(*principal.character.as_bytes()).map_err(debug)?;
    let mut tx = h.pool.begin().await?;
    // Seed only the existing effect and clock, as in the potion durations fixture.
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    sqlx::raw_sql(
        "CREATE TABLE fiendish_clock(at timestamptz NOT NULL);
         GRANT SELECT ON fiendish_clock TO oteryn_game_runtime;
         INSERT INTO fiendish_clock VALUES(clock_timestamp());
         CREATE OR REPLACE FUNCTION game_daily_effect_now() RETURNS timestamptz
           LANGUAGE sql VOLATILE AS $$ SELECT at FROM public.fiendish_clock $$",
    ).execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO game_character_concoction_effects(transaction_id,character_id,concoction,started_at,expires_at)
         SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,'wealth_duplex',at,at+interval '1 hour'
         FROM fiendish_clock",
    )
    .bind(id(202).as_slice()).bind(principal.character.as_bytes().as_slice())
    .execute(&mut *tx).await?;
    tx.commit().await?;
    sqlx::raw_sql(
        "CREATE FUNCTION fiendish_task_credit_fault() RETURNS trigger LANGUAGE plpgsql
         AS $$ BEGIN RAISE EXCEPTION 'fixture defers Task Board credit'; END $$;
         CREATE TRIGGER fiendish_task_credit_fault BEFORE INSERT ON game_task_board_receipts
         FOR EACH ROW EXECUTE FUNCTION fiendish_task_credit_fault()",
    ).execute(&h.pool).await?;

    assert!(h.root.wealth_duplex_active(owner.character, character).await?);
    assert_eq!(owner.drain_kill_rewards(principal.session).await, DrainOutcome::Deferred);
    let sliver_cause = "SELECT draw_ordinal FROM game_item_mint_reservations WHERE loot_purpose_key='oteryn:item.tibia.i37109' ORDER BY draw_ordinal LIMIT 2";
    let first: Vec<i64> = sqlx::query_scalar(sliver_cause).fetch_all(&h.pool).await?;
    assert_eq!(first.len(), 1);
    assert_committed_sliver_in_corpse(h).await?;

    sqlx::raw_sql(
        "UPDATE fiendish_clock SET at=at+interval '2 hours';
         DROP TRIGGER fiendish_task_credit_fault ON game_task_board_receipts",
    ).execute(&h.pool).await?;
    assert!(!h.root.wealth_duplex_active(owner.character, character).await?);
    assert_eq!(owner.drain_kill_rewards(principal.session).await, DrainOutcome::Drained);
    let retried: Vec<i64> = sqlx::query_scalar(sliver_cause).fetch_all(&h.pool).await?;
    assert_eq!(retried, first, "retry reuses the committed sliver MINT cause");
    assert_committed_sliver_in_corpse(h).await?;
    let credits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_task_board_receipts WHERE cause_kind='creature_death'",
    ).fetch_one(&h.pool).await?;
    assert!(credits > 0);
    Ok(())
}
