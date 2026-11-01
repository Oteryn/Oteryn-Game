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
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::connection::{
    ConnectionEnd, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
};
use oteryn_gameplay_progression::atelier::{AtelierAction, AtelierState, FragmentKind, GemQuality};
use oteryn_gameplay_progression::wheel::WheelDomain;
use oteryn_gameplay_progression::wheel_gem_data::WheelGemData;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[path = "atelier/fixture.rs"]
pub(super) mod fixture;
#[path = "atelier/paid.rs"]
mod paid;
#[path = "atelier/wire_paid.rs"]
mod wire_paid;

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
    eprintln!("ATELIER POSTGRES_CASE_COMPLETED");
    Ok(())
}

async fn stored(h: &Harness) -> TestResult<AtelierState> {
    let json: serde_json::Value = sqlx::query_scalar("SELECT state_json FROM game_character_atelier_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    Ok(serde_json::from_value(json)?)
}

async fn send_atelier(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    kind: u32,
    payload: &[u8],
) -> TestResult<Vec<u8>> {
    let command_id = admitted.continuity.next_command_id;
    let frame = oteryn_protocol_oteryn::encode_client_command(
        admitted.continuity.connection_generation,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id,
            command_type: kind,
            payload,
        },
    )
    .map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client
        .write_all(&(frame.len() as u32).to_be_bytes())
        .await?;
    client.write_all(&frame).await?;
    client.shutdown().await?;
    let end = Box::pin(serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS)).await;
    // The client reads to EOF, which arrives only once the server half is closed.
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    if let ConnectionEnd::AdmittedThenDisconnected(current)
    | ConnectionEnd::AdmittedThenClosed(current, _) = &end
    {
        *admitted = *current;
    }
    // A playable session streams its snapshot first; the command answers with its
    // CommandResult and never with a ProtocolError.
    let mut cursor = 0;
    while cursor < output.len() {
        let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
        let envelope =
            decode_wire_envelope(&output[cursor + 4..cursor + 4 + length]).map_err(debug)?;
        cursor += 4 + length;
        assert_ne!(
            envelope.message_type(),
            MessageType::ProtocolError,
            "command {kind}: {:?}",
            oteryn_protocol_oteryn::decode_protocol_error(envelope.payload())
        );
        if envelope.message_type() == MessageType::CommandResult {
            let result =
                oteryn_protocol_oteryn::decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(result.command_id, command_id);
            assert_eq!(
                result.status,
                oteryn_protocol_oteryn::CommandStatus::Accepted
            );
            return Ok(result.payload.to_vec());
        }
    }
    Err(format!("Atelier command {kind} result missing: {end:?}").into())
}

// These existing custody/rule tests advance the canonical owner clock between
// distinct fresh actions. Immediate replays keep their original timing.
fn advance_ex_action(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult {
    owner.clock_origin = owner.clock_origin
        .checked_sub(std::time::Duration::from_millis(490))
        .ok_or("Atelier fixture clock")?;
    Ok(())
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
        // Capability 148 is offered before Character eligibility. Opening the panel
        // must be a normal refusal, not Unavailable (which closes the connection).
        let mut query_admitted = dependencies.admitted;
        query_admitted.item_fence = Some(owner.current_quest_fence(session).await
            .map_err(|_| "query fence")?.ok_or("query fence")?.item_fence());
        query_admitted.continuity.selected_capabilities =
            crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                owner.offered_capabilities(),
                &[oteryn_protocol_oteryn::atelier::CAPABILITY_WHEEL_ALLOCATION_V1],
            ).ok_or("Wheel query capability")?;
        let crate::gameplay_transport::atelier::AtelierWireReply::Payload(query) = owner.atelier_wire(
            query_admitted, oteryn_protocol_oteryn::atelier::COMMAND_TYPE_ATELIER_QUERY, &[],
        ).await else { return Err("ineligible Atelier query must not close play".into()); };
        let query = oteryn_protocol_oteryn::atelier::decode_reply(&query).map_err(debug)?;
        assert!(query.refused);
        assert_eq!(query.revision, 0);
        assert!(query.gems.is_empty() && query.grades.is_empty());
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_atelier_state")
            .fetch_one(&h.pool).await?, 0, "query does not own initialization");
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
            resources: None,
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
        // The wire commands go through the composed connection: a query, then an unlock and a
        // re-lock of the gem locked above, each committed with the current Atelier revision.
        // The wire path needs what a real admission leaves: a positioned actor (an unplayable
        // session is only held), the durable session's item fence and the negotiated Wheel
        // allocation capability; without them the server refuses the command.
        {
            let mut runtime = dependencies.runtime.lock().await;
            runtime
                .initialize_first_entry_position(dependencies.actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    dependencies.actor,
                    session,
                    crate::foundation::CharacterLease::new(
                        crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                            .map_err(debug)?,
                        fence.character_lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
        }
        let (current, _) =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(session)
                .await?;
        let mut admitted = dependencies.admitted;
        admitted.first_entry = crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        admitted.continuity.selected_capabilities =
            crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                owner.offered_capabilities(),
                &[oteryn_protocol_oteryn::atelier::CAPABILITY_WHEEL_ALLOCATION_V1],
            )
            .ok_or("Wheel allocation capability")?;
        let query = send_atelier(
            &owner,
            &mut admitted,
            oteryn_protocol_oteryn::atelier::COMMAND_TYPE_ATELIER_QUERY,
            &[],
        )
        .await?;
        assert!(!query.is_empty());
        for (occurrence, locked) in [(230, false), (231, true)] {
            advance_ex_action(&mut owner)?;
            let lock = oteryn_protocol_oteryn::atelier::encode_set_lock(
                &oteryn_protocol_oteryn::atelier::SetLockRequest {
                    occurrence: id(occurrence),
                    expected_revision: stored(&h).await?.revision,
                    gem,
                    locked,
                },
            )
            .map_err(debug)?;
            let reply = send_atelier(
                &owner,
                &mut admitted,
                oteryn_protocol_oteryn::atelier::COMMAND_TYPE_ATELIER_SET_LOCK,
                &lock,
            )
            .await?;
            assert!(!reply.is_empty());
            assert_eq!(
                stored(&h).await?.gems[&u128::from_be_bytes(gem)].locked,
                locked
            );
        }
        migration_reapplication(&h).await?;
        paid::exercise(&mut owner, &h).await?;
        migration_reapplication(&h).await?;
        drop(restarted);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

// Reapply on initialized and then paid history: proof arms must not accumulate,
// immutable receipts must survive, and historical imbuement fees stay NOT VALID.
async fn migration_reapplication(h: &Harness) -> TestResult {
    let state = stored(h).await?;
    let definitions = |pool: sqlx::PgPool| async move {
        sqlx::query_scalar::<_, String>(
            "SELECT pg_get_functiondef(p.oid) FROM pg_proc p
             WHERE p.oid IN ('game_item_fee_burn_consistency_guard()'::regprocedure,
               'game_item_instance_change_proven()'::regprocedure,
               'game_item_container_entry_removal_proven()'::regprocedure,
               'game_item_placement_proven()'::regprocedure,
               'game_item_mint_consistency_guard()'::regprocedure,
               'game_atelier_item_admit()'::regprocedure,
               'game_atelier_assets_guard()'::regprocedure)
             ORDER BY p.proname LIMIT 7",
        )
        .fetch_all(&pool)
        .await
    };
    let receipts = |pool: sqlx::PgPool| async move {
        sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT to_jsonb(r) FROM game_character_atelier_receipts r
             WHERE character_id=encode($1,'hex')::uuid ORDER BY after_revision LIMIT 100",
        )
        .bind(id(41).as_slice())
        .fetch_all(&pool)
        .await
    };
    let history = receipts(h.pool.clone()).await?;
    assert!(
        history.len() < 100,
        "fixture history must fit the bounded read"
    );
    let before = definitions(h.pool.clone()).await?;
    // Include both quality-yield guards advanced by 0624 in the idempotence proof.
    assert_eq!(before.len(), 7);
    let mut tx = h.pool.begin().await?;
    sqlx::raw_sql(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../apps/game-server/migrations/0314_character_gem_atelier_paid.sql"
    )))
    .execute(&mut *tx)
    .await?;
    sqlx::raw_sql(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../apps/game-server/migrations/0624_character_atelier_crystal_fragment_yields.sql"
    )))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    assert_eq!(definitions(h.pool.clone()).await?, before);
    assert_eq!(stored(h).await?, state);
    assert_eq!(receipts(h.pool.clone()).await?, history);
    let validated: bool = sqlx::query_scalar(
        "SELECT convalidated FROM pg_constraint
         WHERE conrelid='game_item_fee_burns'::regclass
           AND conname='game_item_fee_burns_purchase_cause'",
    )
    .fetch_one(&h.pool)
    .await?;
    assert!(!validated, "historical imbuement fee receipts remain valid");
    Ok(())
}
