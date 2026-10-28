//! FORGECREATURE: existing real attacks, multi-recipient settlement, replay and capped dust.
use super::party_xp::{self, Case};
use crate::bestiary_postgres_harness::{Harness, TestResult, debug, id};
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::gameplay_transport::ComposedFreshAdmission;

#[test]
fn forge_creature_postgres_real_dispatch_party_cap_and_replay() -> TestResult {
    party_xp::run(Case::ForgeReplay)
}
#[test]
fn forge_creature_postgres_real_dispatch_premium_gate_and_replay() -> TestResult {
    party_xp::run(Case::ForgeFree)
}
#[test]
fn forge_creature_postgres_real_dispatch_idle_party_member_gets_no_dust() -> TestResult {
    party_xp::run(Case::ForgeIdle)
}
#[test]
fn forge_creature_postgres_real_dispatch_free_players_still_loot_slivers() -> TestResult {
    party_xp::run(Case::ForgeAllFree)
}
#[test]
fn forge_creature_postgres_real_dispatch_shared_xp_disabled_still_awards_dust() -> TestResult {
    party_xp::run(Case::ForgeNoShared)
}
#[test]
fn forge_creature_postgres_real_dispatch_stale_lease_awards_nothing() -> TestResult {
    party_xp::run(Case::ForgeStale)
}
#[test]
fn forge_creature_postgres_real_dispatch_member_failure_rolls_back_dust_and_xp() -> TestResult {
    party_xp::run(Case::ForgeRollback)
}
#[test]
fn forge_creature_postgres_real_dispatch_party_range_excludes_distant_member() -> TestResult {
    party_xp::run(Case::ForgeFar)
}
#[test]
fn forge_creature_postgres_real_dispatch_foreign_contributor_gets_full_dust() -> TestResult {
    party_xp::run(Case::ForgeForeign)
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
    if matches!(case, Case::ForgeNoShared) {
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
            awarded_xp, 20,
            "5 rat XP * fiendish 2x, then 30% two-vocation bonus: ceil(10 * 1.3 / 2) = 7 per member; Premium stamina: floor(7 * 1.5) = 10 each, 20 total"
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
