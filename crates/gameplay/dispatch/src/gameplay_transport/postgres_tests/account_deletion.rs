//! Real Character producer -> periodic connection authority -> durable release -> quarantine.
use super::fixture::{Harness, debug, id};
use super::kill_reward_e2e::scenario_thread::run_async as run;
use super::{Dependencies, TestResult, prepare_current_session, reset, track};
use crate::durability::character_command::account_deletion::{Config, Intent};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::native_admission_source::{
    DescriptorRegistration, FreshStoreProvenance, NativeSourceOperation, NativeSourceSubject,
    SourceObservation,
};
use crate::foundation::GameSessionState;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::native_admission_source::{
    account_closure::{LifecycleObservation, LifecycleRequest},
    character_commands::{Decision, Input},
};

#[test]
fn account_deletion_dispatch_kicks_retires_and_quarantines() -> TestResult {
    run(async |admin| {
        let started = std::time::Instant::now();
        reset();
        let h = Harness::create_with_admission_seed(admin, "account_closure_dispatch", true, false)
            .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let d = Dependencies::new()?;
        let owner = d.owner(&h, &authority);
        d.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(d.actor)
            .map_err(debug)?;
        track(&owner, &d)?;
        let account = crate::gameplay_transport::canonical_uuid(
            &d.admitted
                .controller
                .ok_or("controller missing")?
                .account_id,
        );
        let now: i64 = sqlx::query_scalar("SELECT game_account_closure_now()")
            .fetch_one(&h.pool)
            .await?;
        let provenance = FreshStoreProvenance {
            namespace: "closure-dispatch".into(),
            authorization: "test-owner".into(),
            source_authority: "platform".into(),
            initialized_at: 10,
        };
        let descriptor = DescriptorRegistration {
            revision: 1,
            facts: vec![1],
            installed_at: 10,
        };
        assert!(
            h.root
                .record_native_source_descriptor_issuance(
                    "platform",
                    descriptor.clone(),
                    Some(provenance.clone())
                )
                .await
                .map_err(debug)?
        );
        h.root
            .initialize_native_admission_source(&h.node, provenance, descriptor)
            .await
            .map_err(debug)?;
        let facts = format!(
            r#"{{"version":1,"operation":"ReadAccountSecurityV1","result":"observed","source_authority":"platform","source_revision":"1","decision_identity":"1","source_observed_at":"{now}","clock_uncertainty_seconds":"0","account_id":"{account}","purpose":"platform_security","scope":"fresh_admission","allowed":false,"minimum_valid_generation":"2"}}"#
        );
        h.root
            .accept_native_source_observation(
                &h.node,
                SourceObservation {
                    source_authority: "platform".into(),
                    operation: NativeSourceOperation::ReadAccountSecurityV1,
                    subject: NativeSourceSubject::account_security(&account).map_err(debug)?,
                    source_revision: 1,
                    decision_identity: "1".into(),
                    observed_at: now,
                    semantic_facts: facts.into_bytes(),
                },
            )
            .await
            .map_err(debug)?;
        let command = Intent {
            contract_version: 2,
            command: "CloseAccount".into(),
            variant: "PLATFORM_ACCOUNT_CLOSE".into(),
            issuer_authority: "OTERYN_PLATFORM_CHARACTER_AUTHORITY".into(),
            issuer_decision_id: crate::gameplay_transport::canonical_uuid(&id(180)),
            source_revision: "1".into(),
            operation_id: crate::gameplay_transport::canonical_uuid(&id(181)),
            account_id: account.clone(),
            termination_id: crate::gameplay_transport::canonical_uuid(&id(182)),
            lifecycle_generation: "8".into(),
            expected_recovery_generation: "1".into(),
            close_operation_id: None,
            decision: None,
            scheduled_for: (now - 1).to_string(),
            issued_at_source: (now - 1).to_string(),
            expires_at_source: (now + 120).to_string(),
            audience: "OTERYN_GAME_CHARACTER_AUTHORITY".into(),
        };
        let raw = serde_json::to_vec(&command)?;
        let config = Config::default();
        assert!(config.enabled);
        let request = LifecycleRequest::from_intent("oteryn:character-authority:primary", &raw, 1)?;
        let observation = LifecycleObservation {
            contract_version: 2,
            operation: "ReadAccountClosureLifecycleV1".into(),
            source_authority: "OTERYN_PLATFORM_CHARACTER_AUTHORITY".into(),
            account_id: account,
            termination_id: command.termination_id.clone(),
            lifecycle_generation: "8".into(),
            source_revision: "1".into(),
            phase: "CLOSE".into(),
            financial_obligations_clear: true,
            recovery_generation: "1".into(),
            observed_at: now.to_string(),
            valid_until: (now + 5).to_string(),
        };
        h.root
            .observe_account_closure_lifecycle(
                &authority,
                &h.node,
                &request,
                &serde_json::to_vec(&observation)?,
            )
            .await
            .map_err(debug)?;
        let decide = async || {
            h.root
                .decide_character_command_configured(
                    &authority,
                    &h.node,
                    "oteryn:character-authority:primary",
                    (&command.command, &command.operation_id),
                    Input::Intent(&raw),
                    &config,
                )
                .await
        };
        assert_eq!(decide().await, Decision::Transient);
        assert_eq!(
            decide().await,
            Decision::Transient,
            "live actor cannot produce a closure receipt"
        );
        let mut late_world = h.pool.begin().await?;
        let refused=sqlx::query("UPDATE game_character_roots SET world_id=encode($1,'hex')::uuid WHERE account_id=$2::uuid").bind(id(250).as_slice()).bind(&command.account_id).execute(&mut *late_world).await.err().ok_or("closing must fence same-account world transfer")?;
        assert_eq!(
            refused
                .as_database_error()
                .and_then(|e| e.code())
                .as_deref(),
            Some("55000")
        );
        late_world.rollback().await?;
        assert!(owner.account_closure_check(d.admitted).await);
        let current = FreshAdmissionStore::from_root(h.root.clone())
            .current_session(d.admitted.game_session_id)
            .await
            .map_err(debug)?;
        assert_eq!(current.session_state(), GameSessionState::Terminal);
        assert!(
            !owner
                .runtime
                .lock()
                .await
                .contains_player_session(d.admitted.game_session_id)
                .map_err(debug)?
        );
        let offline: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_closure_offline")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(offline, 1);
        assert_eq!(
            decide().await,
            Decision::Transient,
            "one character step per transaction"
        );
        let snapshot = h
            .root
            .next_account_characters_snapshot(&authority)
            .await
            .map_err(debug)?
            .ok_or("LCFA snapshot missing")?;
        assert!(snapshot.characters.is_empty());
        let refused =
            sqlx::query("UPDATE game_character_roots SET character_revision=character_revision+1")
                .execute(&h.pool)
                .await;
        assert!(
            refused.is_err(),
            "quarantine must fence a late durable writer"
        );

        let new_root = h.independent_root()?;
        assert!(new_root.maintain_ready_once().await?);
        let new_authority = new_root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut cancel = command.clone();
        cancel.command = "ResolveAccountClosure".into();
        cancel.variant = "PLATFORM_ACCOUNT_CLOSE_RESOLUTION".into();
        cancel.operation_id = crate::gameplay_transport::canonical_uuid(&id(183));
        cancel.source_revision = "2".into();
        cancel.lifecycle_generation = "9".into();
        cancel.close_operation_id = Some(command.operation_id.clone());
        cancel.decision = Some("CANCEL".into());
        let mut current_lifecycle = observation.clone();
        current_lifecycle.lifecycle_generation = "9".into();
        current_lifecycle.source_revision = "2".into();
        current_lifecycle.phase = "CANCEL".into();
        new_root
            .observe_account_closure_lifecycle(
                &new_authority,
                &h.node,
                &request,
                &serde_json::to_vec(&current_lifecycle)?,
            )
            .await
            .map_err(debug)?;
        let cancel_raw = serde_json::to_vec(&cancel)?;
        assert_eq!(
            new_root
                .decide_character_command_configured(
                    &new_authority,
                    &h.node,
                    "oteryn:character-authority:primary",
                    (&cancel.command, &cancel.operation_id),
                    Input::Intent(&cancel_raw),
                    &config
                )
                .await,
            Decision::Transient
        );
        // A partial CANCEL must not publish a highscore revision whose body
        // would change without another revision when the account reopens.
        sqlx::query("SELECT game_public_highscores_touch(encode($1,'hex')::uuid,1::smallint)")
            .bind(d.admitted.world_id.as_bytes().as_slice())
            .execute(&h.pool)
            .await?;
        assert!(
            new_root
                .next_public_highscores(&new_authority, &[])
                .await
                .map_err(debug)?
                .is_none()
        );
        let restored = new_root
            .decide_character_command_configured(
                &new_authority,
                &h.node,
                "oteryn:character-authority:primary",
                (&cancel.command, &cancel.operation_id),
                Input::Intent(&cancel_raw),
                &config,
            )
            .await;
        assert!(
            matches!(restored,Decision::Publish{ref result_code,..} if result_code=="CHAR_CMD_OK"),
            "{restored:?}"
        );
        assert!(
            new_root
                .next_public_highscores(&new_authority, &[])
                .await
                .map_err(debug)?
                .is_some()
        );
        let snapshot = new_root
            .next_account_characters_snapshot(&new_authority)
            .await
            .map_err(debug)?
            .ok_or("restored LCFA missing")?;
        assert_eq!(snapshot.characters.len(), 1);
        let current = FreshAdmissionStore::from_root(new_root.clone())
            .current_session(d.admitted.game_session_id)
            .await
            .map_err(debug)?;
        assert_eq!(current.session_state(), GameSessionState::Terminal);
        assert!(
            !owner
                .runtime
                .lock()
                .await
                .contains_player_session(d.admitted.game_session_id)
                .map_err(debug)?
        );
        // Reconfirm completed cancellation after a real recovery. Old restoration
        // steps must not run again against an account whose admission reopened.
        drop(new_authority);
        drop(new_root);
        drop(owner);
        drop(authority);
        drop(seal);
        let transition = h
            .recovery
            .begin_recovery(1, id(181), now as u64)
            .map_err(debug)?;
        h.root
            .reconcile_character_recovery(&transition)
            .await
            .map_err(debug)?;
        drop(transition);
        let recovered_seal = h.recovery.seal_current().map_err(debug)?;
        let recovered = h
            .root
            .open_character_authority(&recovered_seal)
            .await
            .map_err(debug)?;
        let mut reissued = cancel.clone();
        reissued.operation_id = "0192a000-0000-7000-8000-0000000000b2".into();
        reissued.source_revision = "101".into();
        reissued.expected_recovery_generation = "2".into();
        let reissued_raw = serde_json::to_vec(&reissued)?;
        let request =
            LifecycleRequest::from_intent("oteryn:character-authority:primary", &reissued_raw, 2)?;
        let refreshed_now: i64 = sqlx::query_scalar("SELECT game_account_closure_now()")
            .fetch_one(&h.pool)
            .await?;
        current_lifecycle.observed_at = refreshed_now.to_string();
        current_lifecycle.valid_until = (refreshed_now + 5).to_string();
        current_lifecycle.source_revision = "102".into();
        current_lifecycle.recovery_generation = "2".into();
        h.root
            .observe_account_closure_lifecycle(
                &recovered,
                &h.node,
                &request,
                &serde_json::to_vec(&current_lifecycle)?,
            )
            .await
            .map_err(debug)?;
        let outcome = h
            .root
            .decide_character_command_configured(
                &recovered,
                &h.node,
                "oteryn:character-authority:primary",
                (&reissued.command, &reissued.operation_id),
                Input::Intent(&reissued_raw),
                &config,
            )
            .await;
        assert!(
            matches!(outcome,Decision::Publish{ref result_code,..} if result_code=="CHAR_CMD_OK"),
            "{outcome:?}"
        );
        // A second real termination proves the nonempty terminal Close path.
        // The unapproved physical-erasure policy must still hold FINALIZE.
        let next_now: i64 = sqlx::query_scalar("SELECT game_account_closure_now()")
            .fetch_one(&h.pool)
            .await?;
        let fresh_facts = format!(
            r#"{{"version":1,"operation":"ReadAccountSecurityV1","result":"observed","source_authority":"platform","source_revision":"2","decision_identity":"2","source_observed_at":"{next_now}","clock_uncertainty_seconds":"0","account_id":"{}","purpose":"platform_security","scope":"fresh_admission","allowed":false,"minimum_valid_generation":"2"}}"#,
            command.account_id
        );
        h.root
            .accept_native_source_observation(
                &h.node,
                SourceObservation {
                    source_authority: "platform".into(),
                    operation: NativeSourceOperation::ReadAccountSecurityV1,
                    subject: NativeSourceSubject::account_security(&command.account_id)
                        .map_err(debug)?,
                    source_revision: 2,
                    decision_identity: "2".into(),
                    observed_at: next_now,
                    semantic_facts: fresh_facts.into_bytes(),
                },
            )
            .await
            .map_err(debug)?;
        let mut full = command.clone();
        full.operation_id = crate::gameplay_transport::canonical_uuid(&id(244));
        full.termination_id = crate::gameplay_transport::canonical_uuid(&id(245));
        full.issuer_decision_id = crate::gameplay_transport::canonical_uuid(&id(246));
        full.lifecycle_generation = "10".into();
        full.expected_recovery_generation = "2".into();
        full.source_revision = "200".into();
        full.scheduled_for = next_now.to_string();
        full.issued_at_source = next_now.to_string();
        full.expires_at_source = (next_now + 300).to_string();
        current_lifecycle.termination_id = full.termination_id.clone();
        current_lifecycle.lifecycle_generation = "10".into();
        current_lifecycle.source_revision = "200".into();
        current_lifecycle.phase = "CLOSE".into();
        current_lifecycle.observed_at = next_now.to_string();
        current_lifecycle.valid_until = (next_now + 5).to_string();
        let full_raw = serde_json::to_vec(&full)?;
        let request =
            LifecycleRequest::from_intent("oteryn:character-authority:primary", &full_raw, 2)?;
        h.root
            .observe_account_closure_lifecycle(
                &recovered,
                &h.node,
                &request,
                &serde_json::to_vec(&current_lifecycle)?,
            )
            .await
            .map_err(debug)?;
        let submit = async |intent: &Intent| -> TestResult<Decision> {
            let raw = serde_json::to_vec(intent)?;
            Ok(h.root
                .decide_character_command_configured(
                    &recovered,
                    &h.node,
                    "oteryn:character-authority:primary",
                    (&intent.command, &intent.operation_id),
                    Input::Intent(&raw),
                    &config,
                )
                .await)
        };
        assert_eq!(submit(&full).await?, Decision::Transient);
        assert_eq!(submit(&full).await?, Decision::Transient);
        let committed = submit(&full).await?;
        let Decision::Publish {
            receipt: nonempty_proof,
            result_code,
        } = committed
        else {
            return Err(format!("nonempty close: {committed:?}").into());
        };
        assert_eq!(result_code, "CHAR_CMD_OK");
        let proof: serde_json::Value = serde_json::from_slice(&nonempty_proof)?;
        assert_eq!(proof["result_data"]["detached_characters"], "1");
        assert_eq!(proof["result_data"]["closure_state"], "CLOSED_DETACHED");
        let empty = h
            .root
            .next_account_characters_snapshot(&recovered)
            .await
            .map_err(debug)?
            .ok_or("closed portfolio missing")?;
        assert!(empty.characters.is_empty());
        let mut finalize = cancel.clone();
        finalize.operation_id = crate::gameplay_transport::canonical_uuid(&id(247));
        finalize.termination_id = full.termination_id.clone();
        finalize.close_operation_id = Some(full.operation_id.clone());
        finalize.lifecycle_generation = "11".into();
        finalize.expected_recovery_generation = "2".into();
        finalize.source_revision = "201".into();
        finalize.decision = Some("FINALIZE".into());
        finalize.scheduled_for = full.scheduled_for.clone();
        finalize.issued_at_source = full.issued_at_source.clone();
        finalize.expires_at_source = full.expires_at_source.clone();
        current_lifecycle.lifecycle_generation = "11".into();
        current_lifecycle.source_revision = "201".into();
        current_lifecycle.phase = "FINALIZE".into();
        h.root
            .observe_account_closure_lifecycle(
                &recovered,
                &h.node,
                &request,
                &serde_json::to_vec(&current_lifecycle)?,
            )
            .await
            .map_err(debug)?;
        assert_eq!(submit(&finalize).await?, Decision::Transient);
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_account_closure_members WHERE step=3"
            )
            .fetch_one(&h.pool)
            .await?,
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_name_reservations")
                .fetch_one(&h.pool)
                .await?,
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_account_closures WHERE state='FINALIZING'"
            )
            .fetch_one(&h.pool)
            .await?,
            1
        );
        drop(recovered);
        drop(recovered_seal);
        eprintln!(
            "GPROD_DELETION_DISPATCH_PG_EXECUTED real_caller=account_closure_check world_transfer_fence=proved durable_terminal=3 actor_absent=proved one_character_step=proved partial_restart_cancel=proved highscore_restore_publication=proved no_session_resurrection=proved recovery_cancel_without_repeating_restore=proved nonempty_close_proof=proved finalize_policy_pending=proved elapsed={:?}",
            started.elapsed()
        );
        h.cleanup().await
    })
}

#[test]
fn account_deletion_dispatch_existing_market_credit_drains_while_closing() -> TestResult {
    market_drain_case(true)
}

#[test]
fn account_deletion_dispatch_expired_market_escrow_drains_while_closing() -> TestResult {
    market_drain_case(false)
}

fn market_drain_case(held: bool) -> TestResult {
    run(async move |admin| {
        let started = std::time::Instant::now();
        let h =
            Harness::create_with_admission_seed(admin, "closure_market_drain", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        sqlx::query("INSERT INTO game_market_books VALUES(encode($1,'hex')::uuid,'oteryn:item.tibia.i3031')").bind(id(42).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_account_bank_balances(account_id,world_id,balance) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,0) ON CONFLICT DO NOTHING").bind(id(40).as_slice()).bind(id(42).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_market_offers(offer_id,world_id,definition_key,definition_revision,side,account_id,character_id,piece_price,amount,remaining,created_at_us,expires_at_us,state,ended_as,escrow_gold,held_credit_gold,ended_at_us,end_sequence) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'oteryn:item.tibia.i3031','definition-r1',1,encode($3,'hex')::uuid,encode($4,'hex')::uuid,100,1,1,1,2592000000001,CASE WHEN $5 THEN 5 ELSE 1 END,CASE WHEN $5 THEN 4 ELSE NULL END,CASE WHEN $5 THEN 0 ELSE 100 END,CASE WHEN $5 THEN 100 ELSE 0 END,CASE WHEN $5 THEN 2 ELSE NULL END,CASE WHEN $5 THEN nextval('game_market_end_sequence') ELSE NULL END)")
            .bind(id(190).as_slice()).bind(id(42).as_slice()).bind(id(40).as_slice()).bind(id(41).as_slice()).bind(held).execute(&h.pool).await?;
        // Isolate the durable closing boundary reached by the real producer in
        // the adjacent dispatch case; no session/custody proof is fabricated.
        sqlx::query("INSERT INTO game_account_closures(account_id,termination_id,close_operation_id,lifecycle_generation,close_generation,state,recovery_generation,closed_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,1,'CLOSING',1,game_account_closure_now())")
            .bind(id(40).as_slice()).bind(id(191).as_slice()).bind(id(192).as_slice()).execute(&h.pool).await?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        h.root
            .advance_market(&authority, &h.node, world)
            .await
            .map_err(debug)?;
        h.root
            .advance_market(&authority, &h.node, world)
            .await
            .map_err(debug)?;
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT balance::bigint FROM game_account_bank_balances")
                .fetch_one(&h.pool)
                .await?,
            100
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT held_credit_gold::bigint FROM game_market_offers")
                .fetch_one(&h.pool)
                .await?,
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, i16>("SELECT state FROM game_market_offers")
                .fetch_one(&h.pool)
                .await?,
            4
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_market_settlements WHERE kind IN(4,6)"
            )
            .fetch_one(&h.pool)
            .await?,
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_market_value_lines")
                .fetch_one(&h.pool)
                .await?,
            1
        );
        let mut denied = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *denied)
            .await?;
        let error = sqlx::query("UPDATE game_account_bank_balances SET balance=99")
            .execute(&mut *denied)
            .await
            .err()
            .ok_or("unrelated bank work must stay fenced")?;
        assert_eq!(
            error.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("55000")
        );
        denied.rollback().await?;
        eprintln!(
            "GPROD_DELETION_MARKET_PG_EXECUTED existing_owner=advance_market held_credit={held} credited_once=100 unrelated_bank_work=refused elapsed={:?}",
            started.elapsed()
        );
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
