//! GEM2: real composed admission -> Character sequencer -> PostgreSQL -> restart/replay.
#![allow(clippy::expect_used)]
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_atelier::{AtelierMutation, AtelierOutcome, AtelierRequest};
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use crate::durability::character_wheel::WheelRuleset;
use crate::durability::npc_purchase::PurchaseOutcome;
use crate::foundation::ConnectionGeneration;
use oteryn_gameplay_progression::atelier::{AtelierAction, AtelierState, FragmentKind, GemQuality};
use oteryn_gameplay_progression::wheel::WheelDomain;
use oteryn_gameplay_progression::wheel_gem_data::WheelGemData;
use std::sync::Arc;

#[path = "atelier/fixture.rs"]
mod fixture;

#[test]
fn atelier_postgres_runtime_grants_grade_json_reload_and_receipt_constraints() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "atelier PostgreSQL thread panic")??;
    Ok(())
}

async fn stored(h: &Harness) -> TestResult<AtelierState> {
    let json: serde_json::Value = sqlx::query_scalar("SELECT state_json FROM game_character_atelier_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    Ok(serde_json::from_value(json)?)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "atelier_dispatch", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let gate =
            crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
                Default::default(),
                None,
                None,
                Some(h.root.clone()),
            );
        let data = WheelGemData::embedded().map_err(debug)?;
        let ruleset =
            Arc::new(WheelRuleset::from_catalogue(data.revision(), data.wheel()).map_err(debug)?);
        let mut owner = dependencies.owner(&h, &authority);
        owner.wheel_ruleset = Some(Arc::clone(&ruleset));
        owner.premium_coordinator = Some(&gate);
        let session = dependencies.admitted.game_session_id;
        // A real first admission below eligibility writes nothing.
        owner.admit_wheel_session(session).await;
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_atelier_state")
                .fetch_one(&h.pool)
                .await?,
            0
        );
        fixture::eligible(&h.pool).await?;
        let request = fixture::prepare(&h).await?;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?;
        let purchase = owner
            .revision_sequencer
            .acquire(fence.character_id)
            .await
            .commit_npc_purchase(&h.root, &authority, &h.node, fence, request, &gate)
            .await
            .map_err(debug)?;
        assert!(
            matches!(purchase, PurchaseOutcome::Committed(_)),
            "{purchase:?}"
        );
        // Reuse the admission path after eligibility changes; no synthetic Atelier balance.
        owner.admit_wheel_session(session).await;
        let initial = stored(&h).await?;
        assert_eq!(initial.revision, 1);
        assert_eq!(initial.gems.len(), 8);
        assert_eq!(initial.gold, 0);
        assert!(initial.unrevealed.is_empty() && initial.fragments.is_empty());
        for domain in [
            WheelDomain::Green,
            WheelDomain::Red,
            WheelDomain::Purple,
            WheelDomain::Blue,
        ] {
            for quality in [GemQuality::Lesser, GemQuality::Regular] {
                let gems: Vec<_> = initial
                    .gems
                    .values()
                    .filter(|gem| gem.domain == domain && gem.quality == quality)
                    .collect();
                assert_eq!(gems.len(), 1);
                assert_eq!(
                    gems[0].mods.len(),
                    if quality == GemQuality::Lesser { 1 } else { 2 }
                );
                assert!(!gems[0].locked && !gems[0].vessel);
                assert_eq!(gems[0].ruleset_revision, data.revision());
                if quality == GemQuality::Regular {
                    assert_ne!(gems[0].mods[0], gems[0].mods[1]);
                }
            }
        }
        owner.admit_wheel_session(session).await;
        assert_eq!(stored(&h).await?, initial);
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_character_atelier_receipts WHERE kind='GEM_INIT'"
            )
            .fetch_one(&h.pool)
            .await?,
            1
        );
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?;
        let mut stale = fence;
        stale.connection_generation =
            ConnectionGeneration::new(fence.connection_generation.get() + 1).map_err(debug)?;
        let gem = initial
            .gems
            .keys()
            .next()
            .ok_or("initial gem")?
            .to_be_bytes();
        let request = AtelierRequest {
            occurrence: id(170),
            expected_atelier_revision: 1,
            mutation: AtelierMutation::SetLock { gem, locked: true },
        };
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        assert!(
            slot.atelier(
                &h.root,
                &authority,
                &h.node,
                stale,
                request.clone(),
                Arc::clone(&ruleset),
                &gate
            )
            .await
            .is_err()
        );
        assert_eq!(stored(&h).await?, initial);
        let outcome = slot
            .atelier(
                &h.root,
                &authority,
                &h.node,
                fence,
                request.clone(),
                Arc::clone(&ruleset),
                &gate,
            )
            .await
            .map_err(debug)?;
        assert!(
            matches!(outcome, AtelierOutcome::Committed { .. }),
            "{outcome:?}"
        );
        let locked = stored(&h).await?;
        assert!(locked.gems[&u128::from_be_bytes(gem)].locked);
        // Stale authority can read an exact receipt, but cannot write a fresh occurrence.
        assert!(matches!(
            slot.atelier(
                &h.root,
                &authority,
                &h.node,
                stale,
                request.clone(),
                Arc::clone(&ruleset),
                &gate
            )
            .await
            .map_err(debug)?,
            AtelierOutcome::AlreadyCommitted { .. }
        ));
        let mut conflicting = request.clone();
        conflicting.mutation = AtelierMutation::SetLock { gem, locked: false };
        assert!(matches!(
            slot.atelier(
                &h.root,
                &authority,
                &h.node,
                fence,
                conflicting,
                Arc::clone(&ruleset),
                &gate
            )
            .await,
            Err(CharacterProgressionError::ConflictingOccurrence)
        ));
        drop(slot);
        // A new runtime and sequencer load the permanent receipt; initialization never replaces state.
        let mut restarted = dependencies.owner(&h, &authority);
        restarted.wheel_ruleset = Some(Arc::clone(&ruleset));
        restarted.premium_coordinator = Some(&gate);
        restarted.admit_wheel_session(session).await;
        assert_eq!(stored(&h).await?, locked);
        let sequencer = CharacterRevisionSequencer::new();
        let mut slot = sequencer.acquire(fence.character_id).await;
        let replay = slot
            .atelier(
                &h.root,
                &authority,
                &h.node,
                stale,
                request,
                Arc::clone(&ruleset),
                &gate,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(replay,AtelierOutcome::AlreadyCommitted {state,..} if state==locked));
        drop(slot);
        // Runtime grants support the actual writer and readback; arbitrary state updates fail at commit.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE game_character_atelier_state SET gold=1 WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).execute(&mut *tx).await?;
        assert_eq!(
            tx.commit()
                .await
                .expect_err("custody counters cannot be forged")
                .as_database_error()
                .and_then(|e| e.code())
                .as_deref(),
            Some("23514")
        );
        assert_eq!(stored(&h).await?, locked);
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        let error = sqlx::query(
            "DELETE FROM game_character_atelier_receipts WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(41).as_slice())
        .execute(&mut *tx)
        .await
        .expect_err("append-only receipts");
        assert_eq!(
            error.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("42501")
        );
        tx.rollback().await?;
        // Keep the grade-map JSONB encoding regression independently of the pending custody writer.
        let mut graded = locked.clone();
        graded.gold = 2_000_000;
        graded.fragments.insert(FragmentKind::Basic, 5);
        let mod_id = graded.gems.values().next().ok_or("gem")?.mods[0];
        graded
            .commit(
                graded.revision,
                AtelierAction::GradeUp {
                    kind: FragmentKind::Basic,
                    mod_id,
                },
            )
            .map_err(debug)?;
        let json = serde_json::to_value(&graded)?;
        let roundtrip: serde_json::Value = sqlx::query_scalar("SELECT $1::jsonb")
            .bind(json)
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(serde_json::from_value::<AtelierState>(roundtrip)?, graded);
        // Retain the original negative-before-revision regression (-1 -> 0).
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        let error = sqlx::query("INSERT INTO game_character_atelier_receipts(character_id,occurrence,request_sha256,before_revision,after_revision,outcome_json,kind,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision) SELECT character_id,encode($2,'hex')::uuid,request_sha256,-1,0,outcome_json,'SET_LOCK',original_character_revision+100,committed_character_revision+100,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision FROM game_character_atelier_receipts WHERE character_id=encode($1,'hex')::uuid AND kind='GEM_INIT'")
            .bind(id(41).as_slice()).bind(id(171).as_slice()).execute(&mut *tx).await.expect_err("negative before revision");
        assert_eq!(
            error.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("23514")
        );
        tx.rollback().await?;
        drop(restarted);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
