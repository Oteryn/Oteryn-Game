use super::*;
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::{
    capabilities::SelectedCapabilities,
    connection::{FreshAdmissionAuthority, UseCommand},
    item_view::{ItemKey, ItemViewContinuity, SessionItemView},
};
use oteryn_protocol_oteryn::loop_training::*;
use std::time::Duration;

fn field(out: &mut Vec<u8>, key: u8, mut value: u64) {
    out.push(key);
    while value >= 128 {
        out.push((value as u8 & 127) | 128);
        value >>= 7;
    }
    out.push(value as u8);
}
#[test]
fn loop_training_postgres_exercise_real_wire_atomic_stop_replay_fence_and_reload() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "exercise PG panic")??;
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "loop_training_exercise", true, false)
            .await?;
        eprintln!("POSTGRES_STARTED LOOP-TRAIN-1 exercise");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) =
            super::super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("item index")?;
        let world = super::world::training_world(index, deps.admitted)?;
        let cells = world
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let content_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            active.identity().server_artifact_digest(),
            active.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            world.map_revision_digest(),
            (1, 0, 7),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            content_pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 1,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
        deps.initialize(&owner).await;
        let mut rates = TrainingRates::from_json(include_bytes!(
            "../../../../../../../rulesets/training/provisional-v1.json"
        ))
        .map_err(debug)?;
        rates.enabled = true;
        owner.states.training.config = Some(Arc::new(rates));
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session(admitted.game_session_id)
                .await?;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 18, 21, 54])
                .ok_or("selection")?;
        assert!(admitted.continuity.selected_capabilities.contains(54));
        owner.admit_loop_training(&mut admitted).await;
        assert_eq!(admitted.first_entry, FirstEntryOutcome::Positioned);
        // Existing fixture builders provision custody; production dispatch owns every tested write.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
            "{}{}",
            crate::durability::item_fee_burn::test_support::item(
                70,
                "oteryn:item.tibia.i2854",
                "definition-r1",
                1
            ),
            crate::durability::item_fee_burn::test_support::item(
                72,
                "oteryn:item.tibia.i28552",
                "definition-r1",
                1
            )
        )))
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)").bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(42).as_slice()).bind(id(74).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($5,'hex')::uuid)").bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(72).as_slice()).bind(id(42).as_slice()).bind(id(75).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let dummy = ItemKey::MapBase {
            bundle_digest: world.base().digest(),
            placement_key: crate::map::view::placement_key(
                crate::map::overlay::TilePos {
                    x: 2,
                    y: 0,
                    floor: -7,
                },
                2,
            )
            .ok_or("placement")?,
            reset_epoch: 0,
        };
        let weapon = ItemKey::Instance(id(72));
        let mut items = SessionItemView::resume(ItemViewContinuity::default()).with_map_view();
        let (wh, dh) = items
            .map_view(&[weapon, dummy], |table| {
                Ok((
                    table.handle(&weapon).ok_or(
                        crate::gameplay_transport::item_view::ItemViewError::LimitExceeded,
                    )?,
                    table.handle(&dummy).ok_or(
                        crate::gameplay_transport::item_view::ItemViewError::LimitExceeded,
                    )?,
                ))
            })
            .map_err(debug)?;
        let mut start = vec![8, 1];
        field(&mut start, 16, wh.get());
        field(&mut start, 24, dh.get());
        let command = UseCommand {
            game_session_id: admitted.game_session_id,
            command_id: 1,
            item_fence: Some(fence),
        };
        let mut without = admitted;
        without.continuity.selected_capabilities = SelectedCapabilities::NONE;
        assert_eq!(
            crate::gameplay_transport::loop_training::wire::dispatch(
                &owner,
                without,
                command,
                67,
                &start,
                Some(&items)
            )
            .await
            .ok_or("capability refusal")?
            .disposition,
            4
        );
        let mut stale = admitted;
        let mut stale_fence = fence;
        stale_fence.connection_generation =
            crate::foundation::ConnectionGeneration::new(2).map_err(debug)?;
        stale.item_fence = Some(stale_fence);
        assert_eq!(
            crate::gameplay_transport::loop_training::wire::dispatch(
                &owner,
                stale,
                UseCommand {
                    item_fence: Some(stale_fence),
                    ..command
                },
                67,
                &start,
                Some(&items)
            )
            .await
            .ok_or("stale refusal")?
            .disposition,
            4
        );
        let result = crate::gameplay_transport::loop_training::wire::dispatch(
            &owner,
            admitted,
            command,
            67,
            &start,
            Some(&items),
        )
        .await
        .ok_or("known start")?;
        assert_eq!(result.disposition, 1, "{result:?}");
        assert_eq!(result.charges, 500);
        // An unavailable checkpoint must retain the run before the session is detached.
        let rates = owner.states.training.config.take();
        assert!(
            !owner
                .reconcile_spells_before_control_loss(deps.actor, admitted.game_session_id)
                .await,
            "unknown training save must block control loss"
        );
        owner.states.training.config = rates;
        assert!(
            !owner.training_activity(stale, 1).await,
            "stale connection cannot stop the current trainer"
        );
        assert_eq!(
            crate::gameplay_transport::loop_training::wire::dispatch(
                &owner,
                admitted,
                command,
                67,
                &[8, 2],
                Some(&items)
            )
            .await
            .ok_or("current trainer")?
            .disposition,
            1
        );

        owner.clock_origin -= Duration::from_secs(2);
        let result = crate::gameplay_transport::loop_training::wire::dispatch(
            &owner,
            admitted,
            UseCommand {
                command_id: 2,
                ..command
            },
            67,
            &[8, 3],
            Some(&items),
        )
        .await
        .ok_or("known stop")?;
        assert_eq!(result.disposition, 2);
        assert_eq!(result.charges, 499);
        let stored:(i32,i64,i64)=sqlx::query_as("SELECT (SELECT charges FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid),(SELECT sword_tries FROM game_character_build_state),(SELECT count(*) FROM game_character_exercise_receipts)").bind(id(72).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(stored, (499, 7, 1));
        let occurrence:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(build_occurrence_id) FROM game_character_exercise_receipts ORDER BY committed_character_revision LIMIT 1").fetch_one(&h.pool).await?;
        let request = crate::durability::exercise_training::ExerciseCheckpoint {
            occurrence: crate::durability::character_build::BuildOccurrence::from_bytes(
                occurrence.try_into().map_err(|_| "occurrence bytes")?,
            )
            .map_err(debug)?,
            event_id: id(208),
            weapon: id(72),
            definition: crate::durability::item_timed_state::TimedDefinitionFacts {
                definition: crate::durability::item_mint::TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: "oteryn:item.tibia.i28552".into(),
                    revision_ref: "definition-r1".into(),
                },
                full: crate::domain::timed_item::TimedValues::new(Some(500), None)
                    .map_err(debug)?,
                lit_continuous: false,
            },
            before: crate::durability::exercise_training::TrainingWeapon {
                revision: 0,
                charges: 500,
            },
            spent: 1,
            family: 3,
        };
        let context = crate::durability::loop_training::TrainingContext {
            rates: owner
                .states
                .training
                .config
                .as_ref()
                .ok_or("rates")?
                .clone(),
            formula: native.training_formula().ok_or("formula")?.clone(),
            progression: active.progression().cloned(),
        };
        let stale = crate::durability::character_progression::CurrentCharacterGameplayFence {
            connection_generation: crate::foundation::ConnectionGeneration::new(2)
                .map_err(debug)?,
            ..super::super::fixture::fence(1)?
        };
        let replay = h
            .root
            .commit_exercise_training(&authority, &h.node, stale, context.clone(), request.clone())
            .await
            .map_err(debug)?;
        assert_eq!(replay.weapon.charges, 499);
        assert_eq!(replay.revision.get(), result.revision);
        let mut conflict = request.clone();
        conflict.spent = 2;
        assert!(
            h.root
                .commit_exercise_training(&authority, &h.node, stale, context.clone(), conflict)
                .await
                .is_err()
        );
        let mut fenced = request;
        fenced.occurrence =
            crate::durability::character_build::BuildOccurrence::from_bytes(id(209))
                .map_err(debug)?;
        assert!(
            h.root
                .commit_exercise_training(&authority, &h.node, stale, context, fenced)
                .await
                .is_err()
        );
        let result_bytes = encode_training_result(result).map_err(debug)?;
        assert!(!result_bytes.is_empty());
        // A complete command envelope passes through the production connection as well.
        let bytes = oteryn_protocol_oteryn::encode_client_command(
            1,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id: 1,
                command_type: 67,
                payload: &[8, 2],
            },
        )
        .map_err(debug)?;
        let mut framed = u32::try_from(bytes.len())?.to_be_bytes().to_vec();
        framed.extend(bytes);
        let (_, output) =
            super::super::weather_wire::fixture_access::serve_input(&owner, admitted, &framed)
                .await?;
        let mut reader = output.as_slice();
        let mut results = 0;
        while !reader.is_empty() {
            let frame = crate::gameplay_transport::tcp_tls::read_frame(&mut reader).await?;
            let envelope = oteryn_protocol_oteryn::decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == oteryn_protocol_oteryn::MessageType::CommandResult {
                let response = oteryn_protocol_oteryn::decode_command_result(envelope.payload())
                    .map_err(debug)?;
                assert_eq!(
                    response.status,
                    oteryn_protocol_oteryn::CommandStatus::Accepted
                );
                assert_eq!(response.payload, result_bytes);
                results += 1;
            }
        }
        assert_eq!(
            results, 1,
            "a real command result, not just a join snapshot"
        );
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_exercise_receipts")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(count, 1);

        // Restart the same public dummy after its cooldown, then drive the real owner cadence
        // through every remaining charge, including the atomic last-charge expiry BURN.
        owner.clock_origin -= Duration::from_secs(30);
        let result = crate::gameplay_transport::loop_training::wire::dispatch(
            &owner,
            admitted,
            UseCommand {
                command_id: 3,
                ..command
            },
            67,
            &start,
            Some(&items),
        )
        .await
        .ok_or("restart exercise")?;
        assert_eq!(result.disposition, 1);
        owner.clock_origin -= Duration::from_secs(1000);
        for _ in 0..32 {
            assert!(
                owner
                    .training_before_save(deps.actor, admitted.game_session_id, false)
                    .await
            );
        }
        let spent:(i16,Option<i32>)=sqlx::query_as("SELECT i.lifecycle,t.charges FROM game_item_instances i JOIN game_item_timed_states t USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid").bind(id(72).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(spent.0, 2);
        assert!(
            spent.1.is_some_and(|charges| charges > 0),
            "TIMED0 expiry retains its last nonzero row"
        );
        let total:i64=sqlx::query_scalar("SELECT sum(charges_before-charges_after)::bigint FROM game_character_exercise_receipts").fetch_one(&h.pool).await?;
        assert_eq!(total, 500);

        // Statue activation is the same real dispatch boundary; an undelivered/free account
        // cannot set a skill or end its session.
        let statue = ItemKey::MapBase {
            bundle_digest: world.base().digest(),
            placement_key: crate::map::view::placement_key(
                crate::map::overlay::TilePos {
                    x: 2,
                    y: 0,
                    floor: -7,
                },
                1,
            )
            .ok_or("statue")?,
            reset_epoch: 0,
        };
        let sh = items
            .map_view(&[statue], |table| {
                table
                    .handle(&statue)
                    .ok_or(crate::gameplay_transport::item_view::ItemViewError::LimitExceeded)
            })
            .map_err(debug)?;
        let mut statue_bytes = Vec::new();
        field(&mut statue_bytes, 8, sh.get());
        let refused = crate::gameplay_transport::loop_training::wire::dispatch(
            &owner,
            admitted,
            UseCommand {
                command_id: 4,
                ..command
            },
            66,
            &statue_bytes,
            Some(&items),
        )
        .await
        .ok_or("statue refusal")?;
        assert_eq!(refused.disposition, 4);
        let (consumer, clock, activation, _) =
            crate::gameplay_transport::spell_premium_coordinator::purchase_fixture::current(id(40));
        let undelivered =
            crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
                consumer.clone(),
                Some(clock.clone()),
                None,
                Some(h.root.clone()),
            );
        owner.premium_coordinator = Some(&undelivered);
        assert_eq!(
            crate::gameplay_transport::loop_training::wire::dispatch(
                &owner,
                admitted,
                UseCommand {
                    command_id: 5,
                    ..command
                },
                66,
                &statue_bytes,
                Some(&items)
            )
            .await
            .ok_or("NotActivated refusal")?
            .disposition,
            4
        );
        h.root
            .latch_premium_activation(
                &activation.activation_id,
                activation.switch_over_us,
                activation.switch_over_us + 60_000_000,
            )
            .await
            .map_err(debug)?;
        let premium =
            crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
                consumer,
                Some(clock),
                Some(activation),
                Some(h.root.clone()),
            );
        owner.premium_coordinator = Some(&premium);
        let logged_out = crate::gameplay_transport::loop_training::wire::dispatch(
            &owner,
            admitted,
            UseCommand {
                command_id: 5,
                ..command
            },
            66,
            &statue_bytes,
            Some(&items),
        )
        .await
        .ok_or("statue logout")?;
        assert_eq!(logged_out.disposition, 3);
        assert_eq!(logged_out.pool_seconds, 43200);
        let before = h
            .root
            .read_character_build_state(&authority, fence.character_id)
            .await
            .map_err(debug)?;
        let state:(i16,i16)=sqlx::query_as("SELECT (SELECT session_state FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid),(SELECT offline_skill FROM game_character_build_state)").bind(id(50).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(state, (3, 3));
        // Move only the test's DB clock anchors back; production takes no elapsed-time input.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE game_durability_reconnect_sessions SET training_created_at_us=training_created_at_us-2000000000 WHERE game_session_id=encode($1,'hex')::uuid").bind(id(50).as_slice()).execute(&mut *tx).await?;
        sqlx::query("UPDATE game_character_training_markers SET committed_at_us=committed_at_us-600000000,as_of_us=as_of_us-600000000 WHERE cause='logout'").execute(&mut *tx).await?;
        sqlx::query("UPDATE game_character_build_receipts b SET committed_at=m.committed_at_us/1000 FROM game_character_training_markers m WHERE b.build_occurrence_id=m.build_occurrence_id AND m.cause='logout'").execute(&mut *tx).await?;
        tx.commit().await?;
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&h.pool)
                .await?;
        let mut source = super::super::fresh_fixture::fresh::Source::new(now)?;
        let keys = source
            .rows
            .iter()
            .map(|row| row.key.clone())
            .collect::<Vec<_>>();
        source.rows =
            crate::durability::admission_authority_guards::AdmissionGuardStore::from_root(
                h.root.clone(),
            )
            .load(&keys)
            .await?
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or("current source publications")?;
        // Re-observe and publish the fixture Platform security/signing sources through their
        // existing owner API. A spent-weapon run outlives FND's five-second evidence window.
        let mut refresh = super::super::fresh_fixture::fresh::Source::new(now)?;
        refresh.rows=source.rows.iter().filter(|row|matches!(row.key,crate::foundation::admission_authority_publication::AdmissionAuthorityGuardKeyV1::Account {..}|crate::foundation::admission_authority_publication::AdmissionAuthorityGuardKeyV1::SigningTrust {..})).cloned().collect();
        for row in &mut refresh.rows {
            row.precondition=crate::foundation::admission_authority_publication::AdmissionPublicationPreconditionV1::CompareAndSet {expected_publication_revision:row.publication_revision};
            row.publication_revision += 1;
            row.source.source_revision += 1;
            row.source.decision_identity =
                format!("training-fresh-source-{}", row.source.source_revision);
            row.source.source_observed_at = now;
            if let crate::foundation::admission_authority_publication::AdmissionAuthorityGuardStateV1::Account {security,..}=&mut row.state {
                security.provenance.publication_revision=row.publication_revision;
                security.provenance.source_revision=row.source.source_revision;security.provenance.accepted_source_revision=row.source.source_revision;
                security.provenance.decision_identity=row.source.decision_identity.clone();security.provenance.accepted_decision_identity=row.source.decision_identity.clone();security.provenance.source_observed_at=now;
            }
        }
        let publication=crate::foundation::admission_authority_publication::AdmissionAuthorityPublicationV1::prepare(&refresh,now).map_err(debug)?;
        let guards = crate::durability::admission_authority_guards::AdmissionGuardStore::from_root(
            h.root.clone(),
        );
        guards.publish(&publication).await.map_err(debug)?;
        source.rows = guards
            .load(&keys)
            .await?
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or("refreshed current publications")?;
        source.current.character_lease_generation = 2;
        let next = source.appearance_observer_request(
            &crate::gameplay_transport::canonical_uuid(&id(41)),
            now,
        )?;
        let store =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone());
        assert!(matches!(store.commit(&next).await?,crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1::Committed(_)));
        let next_session = crate::foundation::GameSessionId::decode(&id(110)).map_err(debug)?;
        let current = store.current_session(next_session).await?;
        assert_eq!(current.current_character_lease().generation(), 3);
        let next_fence =
            crate::gameplay_transport::item_fence_of(current).ok_or("successor fence")?;
        let actor = {
            let mut runtime = deps.runtime.lock().await;
            let reservation = runtime.reserve_fresh_session(next_session).map_err(debug)?;
            let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .initialize_pinned_test_position(
                    actor,
                    MovementLocalPosition {
                        x: 1,
                        y: 0,
                        floor: 7,
                    },
                )
                .map_err(debug)?;
            actor
        };
        deps.initialize_player(&owner, actor, next_session).await;
        let mut next_admitted = crate::gameplay_transport::connection::AdmittedSession {
            game_session_id: next_session,
            runtime_actor: Some(actor),
            item_fence: Some(next_fence),
            ..admitted
        };
        owner.admit_loop_training(&mut next_admitted).await;
        assert_eq!(next_admitted.first_entry, FirstEntryOutcome::Positioned);
        let after = h
            .root
            .read_character_build_state(&authority, fence.character_id)
            .await
            .map_err(debug)?;
        let formula = native.training_formula().ok_or("formula")?;
        use crate::domain::character_build::BuildFormula;
        let cumulative = |build: &crate::durability::character_build::DurableBuildState| {
            crate::domain::character_build::cumulative_progress(
                build.skills()[2].0,
                build.skills()[2].1,
                10,
                |level| formula.required(build.vocation(), 3, level),
            )
        };
        assert!(cumulative(&after) >= cumulative(&before) + 150);
        let skill: i16 = sqlx::query_scalar("SELECT offline_skill FROM game_character_build_state")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(skill, -1);
        owner.admit_loop_training(&mut next_admitted).await;
        assert_eq!(
            h.root
                .read_character_build_state(&authority, fence.character_id)
                .await
                .map_err(debug)?,
            after
        );
        drop(owner);
        drop(authority);
        drop(seal);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let reopened = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let build = h
            .root
            .read_character_build_state(&reopened, fence.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(build, after);
        drop(reopened);
        drop(seal);
        h.cleanup().await
    })
}
