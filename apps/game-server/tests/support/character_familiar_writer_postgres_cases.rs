//! Candidate FAMILIAR-1 actual writer and restored-state admission cases on PostgreSQL17.6.
#![allow(clippy::panic)]
use crate::bestiary_postgres_harness::{
    CHARACTER, Harness, SESSION, TestResult, configured_admin, debug, fence, id, runtime,
};
use crate::domain::CharacterId;
use crate::durability::character_familiar::{
    DurableFamiliarProjection, DurableFamiliarState, FamiliarStateOccurrence, FamiliarStateOutcome,
    FamiliarStateRequest,
};
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::character_stance::{StanceChangeOccurrence, StanceChangeRequest};
use crate::durability::{DurabilityError, DurabilityRoot};
use crate::foundation::ConnectionGeneration;
fn run<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult,
{
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, tag, true).await?;
        let result = body(&h).await;
        h.cleanup().await?;
        result
    })
}
fn state() -> DurableFamiliarState {
    DurableFamiliarState {
        selected_look: 991,
        granted_looks: vec![991],
        saved_expiry_unix: 1_800_001_800,
        last_logout_unix: 0,
        lifecycle_epoch: 1,
        familiar_definition: Some("creature:knight/familiar".into()),
        familiar_revision: Some("r20".into()),
        profile_revision: "spell-p2-r20".into(),
        cooldowns: Vec::new(),
    }
}
fn save(
    tag: u8,
    before: DurableFamiliarState,
    after: DurableFamiliarState,
) -> TestResult<FamiliarStateRequest> {
    Ok(FamiliarStateRequest {
        occurrence: FamiliarStateOccurrence::from_bytes(id(tag)).map_err(debug)?,
        before,
        after,
        content_revision: "content-1".into(),
        policy_revision: "policy-1".into(),
        policy_digest: [1; 32],
    })
}
async fn snapshot(h: &Harness) -> TestResult<(String, String, i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT r.character_revision::text,p.character_revision::text,p.level,p.total_experience,(SELECT count(*) FROM game_character_familiar_receipts) FROM game_character_roots r JOIN game_character_progression_state p USING(character_id) WHERE r.character_id=encode($1,'hex')::uuid")
        .bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?)
}
#[test]
fn familiar_writer_commits_before_runtime_and_replays_with_stale_session() -> TestResult {
    run("familiar_writer", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let unix = h
            .root
            .read_familiar_unix_time(&authority)
            .await
            .map_err(debug)?;
        let db_unix: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&h.pool)
                .await?;
        assert!(unix.seconds() <= db_unix);
        assert!(db_unix.saturating_sub(unix.seconds()) < 60);
        let charid = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
        assert_eq!(
            h.root
                .read_character_familiar_state(&authority, charid)
                .await
                .map_err(debug)?,
            DurableFamiliarProjection::default()
        );
        let first = save(110, DurableFamiliarState::default(), state())?;
        let result = h
            .root
            .commit_character_familiar_state(&authority, &h.node, fence(1)?, first.clone())
            .await
            .map_err(debug)?;
        let FamiliarStateOutcome::Committed(receipt) = result else {
            return Err(format!("first:{result:?}").into());
        };
        assert_eq!(receipt.committed_character_revision().get(), 2);
        assert_eq!(receipt.after(), &state());
        assert!(receipt.matches_request(&fence(1)?, &first));
        assert_eq!(snapshot(h).await?, ("2".into(), "2".into(), 50, 1000, 1));
        sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation=2 WHERE game_session_id=encode($1,'hex')::uuid").bind(id(SESSION).as_slice()).execute(&h.pool).await?;
        let replay = h
            .root
            .commit_character_familiar_state(&authority, &h.node, fence(1)?, first.clone())
            .await
            .map_err(debug)?;
        assert_eq!(
            replay,
            FamiliarStateOutcome::AlreadyCommitted(receipt.clone())
        );
        assert_eq!(
            h.root
                .reconcile_character_familiar_state(&authority, first.occurrence)
                .await
                .map_err(debug)?,
            Some(receipt)
        );
        let mut logout = state();
        logout.last_logout_unix = 1_800_000_010;
        let stale = h
            .root
            .commit_character_familiar_state(
                &authority,
                &h.node,
                fence(2)?,
                save(111, state(), logout.clone())?,
            )
            .await;
        assert!(
            matches!(stale, Err(CharacterProgressionError::AuthorityRejected)),
            "{stale:?}"
        );
        assert_eq!(snapshot(h).await?, ("2".into(), "2".into(), 50, 1000, 1));
        sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation=1 WHERE game_session_id=encode($1,'hex')::uuid").bind(id(SESSION).as_slice()).execute(&h.pool).await?;
        // Interleaved standard-stance write proves the shared mixed-kind chain remains intact.
        h.root
            .commit_character_stance(
                &authority,
                &h.node,
                fence(2)?,
                StanceChangeRequest {
                    occurrence: StanceChangeOccurrence::from_bytes(id(112)).map_err(debug)?,
                    before: None,
                    after: Some("protector".into()),
                    content_revision: "content-1".into(),
                    policy_revision: "policy-1".into(),
                    policy_digest: [2; 32],
                },
            )
            .await
            .map_err(debug)?;
        h.root
            .commit_character_familiar_state(
                &authority,
                &h.node,
                fence(3)?,
                save(111, state(), logout.clone())?,
            )
            .await
            .map_err(debug)?;
        assert_eq!(snapshot(h).await?, ("4".into(), "4".into(), 50, 1000, 2));
        let restarted = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(restarted.maintain_ready_once().await?);
        let reopened = restarted
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let loaded = restarted
            .read_character_familiar_state(&reopened, charid)
            .await
            .map_err(debug)?;
        assert_eq!(loaded.state(), &logout);
        assert_eq!(
            loaded.committed_character_revision().map(|v| v.get()),
            Some(4)
        );
        assert_eq!(
            restarted
                .read_character_stance(&reopened, charid)
                .await
                .map_err(debug)?
                .key(),
            Some("protector")
        );
        // Familiar death closes the Unix lifetime but does not touch the standard stance.
        let mut dead = logout.clone();
        dead.saved_expiry_unix = 1_800_000_020;
        restarted
            .commit_character_familiar_state(
                &reopened,
                &h.node,
                fence(4)?,
                save(113, logout, dead.clone())?,
            )
            .await
            .map_err(debug)?;
        assert_eq!(
            restarted
                .read_character_familiar_state(&reopened, charid)
                .await
                .map_err(debug)?
                .state(),
            &dead
        );
        assert_eq!(
            restarted
                .read_character_stance(&reopened, charid)
                .await
                .map_err(debug)?
                .key(),
            Some("protector")
        );
        assert_eq!(snapshot(h).await?, ("5".into(), "5".into(), 50, 1000, 3));
        Ok(())
    })
}
#[test]
fn familiar_writer_rejects_changed_binding_wrong_before_and_stale_generation() -> TestResult {
    run("familiar_reject", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let before = snapshot(h).await?;
        let mut stale = fence(1)?;
        stale.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        let result = h
            .root
            .commit_character_familiar_state(
                &authority,
                &h.node,
                stale,
                save(120, DurableFamiliarState::default(), state())?,
            )
            .await;
        assert!(
            matches!(result, Err(CharacterProgressionError::AuthorityRejected)),
            "{result:?}"
        );
        assert_eq!(snapshot(h).await?, before);
        let mut bad = state();
        bad.saved_expiry_unix += 1;
        let result = h
            .root
            .commit_character_familiar_state(
                &authority,
                &h.node,
                fence(1)?,
                save(120, state(), bad)?,
            )
            .await;
        assert!(
            matches!(
                result,
                Err(CharacterProgressionError::FamiliarStateMismatch)
            ),
            "{result:?}"
        );
        assert_eq!(snapshot(h).await?, before);
        let first = save(120, DurableFamiliarState::default(), state())?;
        h.root
            .commit_character_familiar_state(&authority, &h.node, fence(1)?, first.clone())
            .await
            .map_err(debug)?;
        let before = snapshot(h).await?;
        for field in [
            "expiry",
            "logout",
            "look",
            "epoch",
            "definition",
            "profile",
            "policy",
            "digest",
            "revision",
        ] {
            let mut changed = first.clone();
            let mut expected = fence(1)?;
            match field {
                "expiry" => changed.after.saved_expiry_unix += 1,
                "logout" => changed.after.last_logout_unix = 1,
                "look" => changed.after.selected_look = 993,
                "epoch" => changed.after.lifecycle_epoch = 2,
                "definition" => {
                    changed.after.familiar_definition = Some("creature:other/familiar".into())
                }
                "profile" => changed.after.profile_revision = "spell-p2-r21".into(),
                "policy" => changed.policy_revision = "policy-2".into(),
                "digest" => changed.policy_digest = [2; 32],
                "revision" => expected = fence(2)?,
                _ => return Err("unknown field".into()),
            }
            let result = h
                .root
                .commit_character_familiar_state(&authority, &h.node, expected, changed)
                .await;
            assert!(
                matches!(
                    result,
                    Err(CharacterProgressionError::ConflictingOccurrence)
                ),
                "{field}:{result:?}"
            );
            assert_eq!(snapshot(h).await?, before);
        }
        let same = save(121, state(), state())?;
        let result = h
            .root
            .commit_character_familiar_state(&authority, &h.node, fence(2)?, same)
            .await;
        assert!(
            matches!(result, Err(CharacterProgressionError::InvalidInput)),
            "{result:?}"
        );
        assert_eq!(snapshot(h).await?, before);
        Ok(())
    })
}
#[test]
fn familiar_admission_rejects_semantically_corrupt_snapshot_with_preserved_receipt_hash()
-> TestResult {
    run("familiar_restored", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        h.root
            .commit_character_familiar_state(
                &authority,
                &h.node,
                fence(1)?,
                save(130, DurableFamiliarState::default(), state())?,
            )
            .await
            .map_err(debug)?;
        // Controlled restore simulation: administrator bypasses triggers; retained command hash
        // stays valid while the projection's semantic data differs from the actual receipt.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE game_character_familiar_state SET state=jsonb_set(state,'{saved_expiry_unix}','1800009999'::jsonb)").execute(&mut *tx).await?;
        tx.commit().await?;
        let result = h.root.open_character_authority(&seal).await.err();
        assert!(
            matches!(
                result,
                Some(
                    crate::durability::character_authority::CharacterAuthorityError::Unavailable(
                        DurabilityError::InvalidStoredState
                    )
                )
            ),
            "{result:?}"
        );
        Ok(())
    })
}

#[test]
fn familiar_offline_cooldown_extends_legacy_chain_without_reencoding_old_receipts() -> TestResult {
    run("familiar_cooldown", async |h| {
        use crate::durability::character_familiar::FamiliarCooldownSnapshot;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let legacy = save(116, DurableFamiliarState::default(), state())?;
        let first = h
            .root
            .commit_character_familiar_state(&authority, &h.node, fence(1)?, legacy.clone())
            .await
            .map_err(debug)?;
        let FamiliarStateOutcome::Committed(first_receipt) = first else {
            return Err(format!("legacy familiar did not commit: {first:?}").into());
        };
        let old_binding:Vec<u8>=sqlx::query_scalar("SELECT command_binding FROM game_character_familiar_receipts WHERE familiar_occurrence_id=encode($1,'hex')::uuid")
            .bind(legacy.occurrence.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        let mut logout = state();
        logout.last_logout_unix = 1_800_000_010;
        logout.cooldowns = vec![FamiliarCooldownSnapshot {
            reference_spell_id: 194,
            remaining_micros: 123_456_789,
        }];
        let next = save(117, state(), logout.clone())?;
        let committed = h
            .root
            .commit_character_familiar_state(&authority, &h.node, fence(2)?, next.clone())
            .await
            .map_err(debug)?;
        let FamiliarStateOutcome::Committed(receipt) = committed else {
            return Err("cooldown did not commit".into());
        };
        assert_eq!(receipt.after(), &logout);
        let before_replay = snapshot(h).await?;
        assert_eq!(before_replay, ("3".into(), "3".into(), 50, 1000, 2));
        let replay = h
            .root
            .commit_character_familiar_state(&authority, &h.node, fence(1)?, legacy.clone())
            .await
            .map_err(debug)?;
        assert_eq!(
            replay,
            FamiliarStateOutcome::AlreadyCommitted(first_receipt)
        );
        assert_eq!(snapshot(h).await?, before_replay);
        let preserved:Vec<u8>=sqlx::query_scalar("SELECT command_binding FROM game_character_familiar_receipts WHERE familiar_occurrence_id=encode($1,'hex')::uuid")
            .bind(legacy.occurrence.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(preserved, old_binding);
        let invalid:bool=sqlx::query_scalar("SELECT game_character_familiar_state_valid($1::jsonb || '{\"cooldowns\":[{\"reference_spell_id\":0,\"remaining_micros\":1}]}'::jsonb)")
            .bind(serde_json::to_string(&logout)?).fetch_one(&h.pool).await?;
        assert!(!invalid);
        let root = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(root.maintain_ready_once().await?);
        let restored = root.open_character_authority(&seal).await.map_err(debug)?;
        assert_eq!(
            root.read_character_familiar_state(
                &restored,
                CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?
            )
            .await
            .map_err(debug)?
            .state(),
            &logout
        );
        Ok(())
    })
}

#[derive(Clone)]
struct UnusedTrainingFormula;
impl crate::durability::character_build::BuildFormula for UnusedTrainingFormula {
    fn required(&self, _: &str, _: usize, _: u16) -> Option<u64> {
        panic!("live-only payment must not call durable training formula")
    }
    fn digest(&self) -> [u8; 32] {
        panic!("live-only payment must not call durable training formula")
    }
    fn content_revision(&self) -> &str {
        panic!("live-only payment must not call durable training formula")
    }
}
#[test]
fn familiar_and_complete_cost_commit_in_one_real_transaction_and_replay_as_history() -> TestResult {
    run("familiar_cast_commit", async |h| {
        use crate::durability::item_mint::TypedDefinitionRef;
        use crate::durability::spell_items_abi::{CastCostBinding, SpellItemTransactionRequest};
        use crate::foundation::{CommandId, CommandRef, GameSessionId};
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let request = save(118, DurableFamiliarState::default(), state())?;
        // Explicit cost-writer input fixture, not a runtime payment capability. The actual
        // Channel compositor separately derives and checks this projection from its held owner.
        let mut cost = SpellItemTransactionRequest {
            command: CommandRef::new(
                GameSessionId::decode(&id(SESSION)).map_err(debug)?,
                CommandId::new(1).map_err(debug)?,
            ),
            spell: TypedDefinitionRef {
                family: "spell".into(),
                production_key: "candidate:spell/knight_familiar".into(),
                revision_ref: "spell-p2-r20".into(),
            },
            catalog_digest: [1; 32],
            transaction_id: id(119),
            event_id: id(120),
            cost: CastCostBinding {
                vitals_revision_before: 1,
                vitals_revision_after: 2,
                mana_before: 200,
                mana_after: 160,
                soul_before: 100,
                soul_after: 100,
                cooldowns_before: Vec::new(),
                cooldowns_after: vec![("spell:knight_familiar".into(), 100_000)],
                caster_digest_before: [11; 32],
                caster_digest_after: [12; 32],
            },
            caster_origin: None,
            operations: Vec::new(),
            companion: None,
            direct_companion: None,
        };
        // Failure after staging familiar rows must roll back their Character successor too.
        assert!(
            h.root
                .commit_familiar_spell::<UnusedTrainingFormula>(
                    &authority,
                    &h.node,
                    fence(1)?,
                    request.clone(),
                    cost.clone(),
                    None
                )
                .await
                .is_err()
        );
        assert_eq!(snapshot(h).await?, ("1".into(), "1".into(), 50, 1000, 0));
        cost.spell.family = "Spell".into();
        // Expired/missing current source eligibility permits only historical completion.
        // An absent original cannot create even an otherwise-valid familiar/cost pair.
        assert!(
            h.root
                .reconcile_familiar_spell::<UnusedTrainingFormula>(
                    &authority,
                    &h.node,
                    fence(1)?,
                    request.clone(),
                    cost.clone(),
                    None,
                )
                .await
                .is_err()
        );
        assert_eq!(snapshot(h).await?, ("1".into(), "1".into(), 50, 1000, 0));
        let absent_costs: i64 = sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(absent_costs, 0);
        let result = h
            .root
            .commit_familiar_spell::<UnusedTrainingFormula>(
                &authority,
                &h.node,
                fence(1)?,
                request.clone(),
                cost.clone(),
                None,
            )
            .await
            .map_err(debug)?;
        let familiar = result
            .familiar()
            .ok_or("missing genuine familiar receipt")?;
        assert!(familiar.matches_request(&fence(1)?, &request));
        assert_eq!(
            result
                .cost()
                .ok_or("missing genuine cost receipt")?
                .transaction_id,
            cost.transaction_id
        );
        assert!(result.training().is_none());
        let linked:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_familiar_receipts f JOIN game_spell_item_receipts c ON f.created_xact_id=c.created_xact_id WHERE f.familiar_occurrence_id=encode($1,'hex')::uuid AND c.transaction_id=encode($2,'hex')::uuid)")
            .bind(request.occurrence.as_bytes().as_slice()).bind(cost.transaction_id.as_slice()).fetch_one(&h.pool).await?;
        assert!(linked);
        let count:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_spell_item_receipts),(SELECT count(*) FROM game_spell_item_audit_outbox),(SELECT count(*) FROM game_character_build_receipts)")
            .fetch_one(&h.pool).await?;
        assert_eq!(count, (1, 1, 0));
        let replay = h
            .root
            .commit_familiar_spell::<UnusedTrainingFormula>(
                &authority,
                &h.node,
                fence(1)?,
                request.clone(),
                cost.clone(),
                None,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(
            replay,
            crate::durability::character_familiar::FamiliarSpellCommit::Reconciled { .. }
        ));
        assert_eq!(snapshot(h).await?, ("2".into(), "2".into(), 50, 1000, 1));
        let history_only = h
            .root
            .reconcile_familiar_spell::<UnusedTrainingFormula>(
                &authority,
                &h.node,
                fence(1)?,
                request.clone(),
                cost.clone(),
                None,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(
            history_only,
            crate::durability::character_familiar::FamiliarSpellCommit::Reconciled { .. }
        ));
        assert_eq!(snapshot(h).await?, ("2".into(), "2".into(), 50, 1000, 1));
        cost.cost.mana_after = 159;
        assert!(
            h.root
                .reconcile_familiar_spell::<UnusedTrainingFormula>(
                    &authority,
                    &h.node,
                    fence(1)?,
                    request,
                    cost,
                    None
                )
                .await
                .is_err()
        );
        Ok(())
    })
}

/// Explicit bounded source vector: Canary vocation None has magic multiplier4, req(1)=1600,
/// req(2)=6400; unchanged skill10 families need only their authored next-level requirement.
/// This fixture implements durability's pure formula ABI, without depending on Spell.
#[derive(Clone)]
struct BoundTrainingFormula;
impl crate::durability::character_build::BuildFormula for BoundTrainingFormula {
    fn required(&self, vocation: &str, family: usize, level: u16) -> Option<u64> {
        if vocation != "none" {
            return None;
        }
        match (family, level) {
            (0, 1) => Some(1600),
            (0, 2) => Some(6400),
            (1..=7, 11) => Some([50, 50, 50, 50, 30, 100, 20][family - 1]),
            _ => None,
        }
    }
    fn digest(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        Sha256::digest(include_bytes!(
            "../../../../tools/content-schema/native-gameplay/build-training.json"
        ))
        .into()
    }
    fn content_revision(&self) -> &str {
        "content-1"
    }
}
#[test]
fn familiar_cost_and_real_training_receipt_share_commit_and_legacy_null_proof_refuses_join()
-> TestResult {
    run("familiar_training_commit", async |h| {
        use crate::durability::character_build::{
            BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState,
        };
        use crate::durability::item_mint::TypedDefinitionRef;
        use crate::durability::spell_items_abi::{CastCostBinding, SpellItemTransactionRequest};
        use crate::foundation::{CommandId, CommandRef, GameSessionId};
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let familiar = save(121, DurableFamiliarState::default(), state())?;
        let training = BuildChangeRequest {
            occurrence: BuildOccurrence::from_bytes(id(122)).map_err(debug)?,
            cause: BuildCause::Training,
            before: DurableBuildState::default(),
            after: DurableBuildState::new("none", (1, 0), [(10, 0); 7]).map_err(debug)?,
            pruned_stance: None,
        };
        // Input binds a source payment of1000 plus600 already accumulated live; this case
        // verifies only the genuine shared durable boundary, not a fabricated runtime witness.
        let mut cost = SpellItemTransactionRequest {
            command: CommandRef::new(
                GameSessionId::decode(&id(SESSION)).map_err(debug)?,
                CommandId::new(1).map_err(debug)?,
            ),
            spell: TypedDefinitionRef {
                family: "spell".into(),
                production_key: "candidate:spell/knight_familiar".into(),
                revision_ref: "spell-p2-r20".into(),
            },
            catalog_digest: [1; 32],
            transaction_id: id(123),
            event_id: id(124),
            cost: CastCostBinding {
                vitals_revision_before: 1,
                vitals_revision_after: 2,
                mana_before: 2000,
                mana_after: 1000,
                soul_before: 100,
                soul_after: 100,
                cooldowns_before: Vec::new(),
                cooldowns_after: vec![("spell:knight_familiar".into(), 100_000)],
                caster_digest_before: [11; 32],
                caster_digest_after: [12; 32],
            },
            caster_origin: None,
            operations: Vec::new(),
            companion: None,
            direct_companion: None,
        };
        assert!(
            h.root
                .commit_familiar_spell(
                    &authority,
                    &h.node,
                    fence(1)?,
                    familiar.clone(),
                    cost.clone(),
                    Some((training.clone(), BoundTrainingFormula))
                )
                .await
                .is_err()
        );
        assert_eq!(snapshot(h).await?, ("1".into(), "1".into(), 50, 1000, 0));
        assert_eq!(
            h.root
                .read_character_build_state(
                    &authority,
                    CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?
                )
                .await
                .map_err(debug)?,
            DurableBuildState::default()
        );
        cost.spell.family = "Spell".into();
        let committed = h
            .root
            .commit_familiar_spell(
                &authority,
                &h.node,
                fence(1)?,
                familiar.clone(),
                cost.clone(),
                Some((training.clone(), BoundTrainingFormula)),
            )
            .await
            .map_err(debug)?;
        let build = committed
            .training()
            .ok_or("missing actual post-COMMIT build proof")?;
        assert_eq!(build.after, training.after);
        assert_eq!(build.original_character_revision.get(), 2);
        assert_eq!(build.committed_character_revision.get(), 3);
        let same:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_familiar_receipts f JOIN game_spell_item_receipts c ON f.created_xact_id=c.created_xact_id JOIN game_character_build_receipts b ON b.created_xact_id=f.created_xact_id WHERE f.familiar_occurrence_id=encode($1,'hex')::uuid AND c.transaction_id=encode($2,'hex')::uuid AND b.build_occurrence_id=encode($3,'hex')::uuid)")
            .bind(familiar.occurrence.as_bytes().as_slice()).bind(cost.transaction_id.as_slice()).bind(training.occurrence.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert!(same);
        assert_eq!(snapshot(h).await?, ("3".into(), "3".into(), 50, 1000, 1));
        let replay = h
            .root
            .commit_familiar_spell(
                &authority,
                &h.node,
                fence(1)?,
                familiar.clone(),
                cost.clone(),
                Some((training.clone(), BoundTrainingFormula)),
            )
            .await
            .map_err(debug)?;
        assert_eq!(replay.training(), Some(build));
        assert_eq!(snapshot(h).await?, ("3".into(), "3".into(), 50, 1000, 1));
        let counts:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_character_build_receipts),(SELECT count(*) FROM game_spell_item_receipts)").fetch_one(&h.pool).await?;
        assert_eq!(counts, (1, 1));
        // Restore a historical pre0039 receipt: NULL source-xact remains valid history,
        // but must never qualify a common Familiar/cost/training install proof.
        let mut restore = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *restore)
            .await?;
        sqlx::query("UPDATE game_character_build_receipts SET created_xact_id=NULL WHERE build_occurrence_id=encode($1,'hex')::uuid")
            .bind(training.occurrence.as_bytes().as_slice()).execute(&mut *restore).await?;
        restore.commit().await?;
        assert!(
            h.root
                .commit_familiar_spell(
                    &authority,
                    &h.node,
                    fence(1)?,
                    familiar,
                    cost,
                    Some((training, BoundTrainingFormula))
                )
                .await
                .is_err()
        );
        Ok(())
    })
}

#[test]
fn ordinary_group_is_explicit_fenced_bootstrap_and_unknown_flags_do_not_become_false() -> TestResult
{
    run("familiar_ordinary_group", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert!(
            h.root
                .read_familiar_group(&authority, &h.node, fence(1)?)
                .await
                .map_err(debug)?
                .is_none()
        );
        let group = h
            .root
            .initialize_familiar_group(&authority, &h.node, fence(1)?)
            .await
            .map_err(debug)?;
        assert_eq!(group.group_id(), 1);
        assert_eq!(group.account_type(), 1);
        assert!(!group.account_at_least_god());
        assert_eq!(group.flag("canedithouses"), Some(false));
        assert_eq!(group.flag("cansummonall"), Some(false));
        assert_eq!(group.flag("hasinfinitemana"), Some(false));
        assert_eq!(group.flag("unrecognizedSourceFlag"), None);
        assert_eq!(
            h.root
                .initialize_familiar_group(&authority, &h.node, fence(1)?)
                .await
                .map_err(debug)?,
            group
        );
        assert_eq!(snapshot(h).await?, ("1".into(), "1".into(), 50, 1000, 0));
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_source_group_receipts")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(count, 1);
        assert!(
            sqlx::query("UPDATE game_character_source_group SET account_type=6")
                .execute(&h.pool)
                .await
                .is_err()
        );
        sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation=2 WHERE game_session_id=encode($1,'hex')::uuid")
            .bind(id(SESSION).as_slice()).execute(&h.pool).await?;
        assert!(
            h.root
                .read_familiar_group(&authority, &h.node, fence(1)?)
                .await
                .is_err()
        );
        let mut fresh = fence(1)?;
        fresh.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        assert_eq!(
            h.root
                .read_familiar_group(&authority, &h.node, fresh)
                .await
                .map_err(debug)?,
            Some(group)
        );
        Ok(())
    })
}
#[test]
fn restored_group_with_self_consistent_privilege_hash_fails_independent_admission() -> TestResult {
    run("familiar_group_restore", async |h| {
        use sha2::{Digest, Sha256};
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        h.root
            .initialize_familiar_group(&authority, &h.node, fence(1)?)
            .await
            .map_err(debug)?;
        let text: String = sqlx::query_scalar(
            "SELECT source_document::text FROM game_character_source_group_receipts",
        )
        .fetch_one(&h.pool)
        .await?;
        let mut changed: serde_json::Value = serde_json::from_str(&text)?;
        changed["enabled_flags"] = serde_json::json!(["canedithouses"]);
        let binding: [u8; 32] = Sha256::new()
            .chain_update(b"oteryn:game-source-group:v1")
            .chain_update(id(CHARACTER))
            .chain_update(id(SESSION))
            .chain_update(serde_json::to_vec(&changed)?)
            .finalize()
            .into();
        let mut restore = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *restore)
            .await?;
        sqlx::query("UPDATE game_character_source_group_receipts SET source_document=$1::jsonb,semantic_binding=$2")
            .bind(serde_json::to_string(&changed)?).bind(binding.as_slice()).execute(&mut *restore).await?;
        restore.commit().await?;
        // A self-consistent replacement hash cannot turn the imported source
        // group into a privilege grant, including through an existing capability.
        let current_read = h
            .root
            .read_familiar_group(&authority, &h.node, fence(1)?)
            .await
            .err();
        assert!(
            matches!(
                current_read,
                Some(CharacterProgressionError::Unavailable(
                    DurabilityError::InvalidStoredState
                ))
            ),
            "{current_read:?}"
        );
        let root = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(root.maintain_ready_once().await?);
        let refused = root.open_character_authority(&seal).await.err();
        assert!(
            matches!(
                refused,
                Some(
                    crate::durability::character_authority::CharacterAuthorityError::Unavailable(
                        DurabilityError::InvalidStoredState
                    )
                )
            ),
            "{refused:?}"
        );
        let preserved: String = sqlx::query_scalar(
            "SELECT source_document::text FROM game_character_source_group_receipts",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&preserved)?,
            changed
        );
        Ok(())
    })
}
