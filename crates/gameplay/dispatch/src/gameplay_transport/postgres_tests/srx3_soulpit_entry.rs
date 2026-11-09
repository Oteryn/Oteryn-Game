//! W07: the shared admission owner must debit the core before terminalizing sources.
use super::TestResult;
use super::fixture::{Harness, debug, id, run};
use crate::durability::activity_handoff::{ActivityEnterOutcome, ActivityHandoffOwner};
use crate::durability::soulpit::{SoulpitEntryRequest, SoulpitParticipantFact};
use crate::foundation::{CommandId, CommandRef};

#[test]
fn srx3_soulpit_shared_entry_commits_core_and_successor_atomically_postgres() -> TestResult {
    exercise_shared_soulpit_entry(false, false)
}

#[test]
fn srx3_soulpit_last_native_exit_terminalizes_encounter_postgres() -> TestResult {
    exercise_shared_soulpit_entry(true, false)
}

#[test]
fn srx3_soulpit_reserved_activity_key_requires_core_context_postgres() -> TestResult {
    exercise_shared_soulpit_entry(false, true)
}

fn exercise_shared_soulpit_entry(exit_while_live: bool, without_core_context: bool) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("srx3-soulpit-entry", move || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "srx3_soulpit_entry", true, false)
                .await?;
            let result: TestResult = async {
            let (controller, pin) =
                crate::content::native_gameplay::test_support::activated_with_loot_tables(
                    crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
                    None,
                );
            let native = controller
                .active()
                .and_then(crate::content::ActiveGeneration::native_gameplay)
                .ok_or("native content")?;
            // Native catalogue qualification is deliberately before the short-lived
            // ready/session proofs; production refreshes them continuously.
            let (_evidence, mut activity) = super::activity_runtime::prepare_activity_fixture(&h)
                .await
                .map_err(|e| format!("prepare shared activity: {e}"))?;
            let source = activity.participants[0].source;
            let core_key = "oteryn:item.tibia.i47398";
            assert!(native.item_policy(core_key, "definition-r1").is_some());
            // The exact same control issuer advances the fixture's activation from its
            // generic content to the unchanged production Soul Core content.
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,2,1,$3,$4,$5)")
                .bind(id(42).as_slice()).bind(id(43).as_slice())
                .bind(pin.identity().server_artifact_digest().as_slice())
                .bind(pin.identity().client_artifact_digest().as_slice())
                .bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
                .bind(source.character_id.as_bytes().as_slice()).execute(&h.pool).await?;
            super::item_look::seed_main_backpack(&h, &[(core_key, 2)])
                .await
                .map_err(|e| format!("seed Soul Core: {e}"))?;
            activity.activity_key = crate::durability::soulpit::SOULPIT_ACTIVITY_KEY.into();
            activity.time_limit_ms = 600_000;
            activity.participants[0].source_position.cell =
                crate::durability::character_last_position::LastPositionCell {
                    x: 32375,
                    y: 31158,
                    floor: 8,
                };
            activity.participants[0]
                .source_position
                .server_artifact_digest = native.source_digest();
            activity.participants[0]
                .source_position
                .coordinate_frame_digest = pin.frame_binding().digest();
            activity.participants[0].source_position.map_revision_digest =
                pin.frame_binding().digest();
            activity.participants[0].exit_position = activity.participants[0].source_position;
            activity.participants[0].destination_cell =
                crate::durability::character_last_position::LastPositionCell {
                    x: 32373,
                    y: 31151,
                    floor: 8,
                };
            let (family, production_key, revision_ref): (String, String, String) =
                sqlx::query_as("SELECT definition_family,definition_production_key,definition_revision_ref FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(71).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(production_key, core_key);
            assert!(native.item_policy(&production_key, &revision_ref).is_some());
            let core = SoulpitEntryRequest {
                source_definition: crate::durability::item_mint::TypedDefinitionRef {
                    family,
                    production_key,
                    revision_ref,
                },
                handoff_id: activity.handoff_id,
                command: CommandRef::new(
                    source.game_session_id,
                    CommandId::new(activity.command_id).map_err(debug)?,
                ),
                source: id(71),
                intent: vec![8, 1],
                started_at_micros: 1,
                participants: vec![SoulpitParticipantFact {
                    character: source.character_id,
                    game_master: false,
                }],
            };
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let lane =
                crate::durability::spell_owner_commit::SpellLane::for_scope(source.runtime_scope);
            let permit = lane.acquire().await.map_err(|_| "source lane unresolved")?;
            let owner = ActivityHandoffOwner::new(h.root.clone());
            if without_core_context {
                let outcome=owner.enter(&authority,&h.node,activity.clone()).await?;
                assert!(matches!(outcome,ActivityEnterOutcome::Refused(_)),
                    "the regular shared entry API cannot admit the reserved Soulpit activity without Core context: {outcome:?}");
                let groups:i64=sqlx::query_scalar("SELECT count(*) FROM game_activity_handoff_groups").fetch_one(&h.pool).await?;
                let quantity:i64=sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&h.pool).await?;
                assert_eq!((groups,quantity),(0,2),"refused Core-less entry has no group/debit");
                return Ok(());
            }
            let mut stale = activity.clone();
            stale.participants[0].source.connection_generation =
                crate::foundation::ConnectionGeneration::new(
                    stale.participants[0].source.connection_generation.get() + 1,
                )
                .map_err(debug)?;
            assert!(matches!(
                owner
                    .enter_soulpit(&authority, &h.node, &permit, stale, &core, native)
                    .await?,
                ActivityEnterOutcome::Refused(_)
            ));
            let mut missing_core = core.clone();
            missing_core.source = id(72);
            assert!(matches!(
                owner
                    .enter_soulpit(
                        &authority,
                        &h.node,
                        &permit,
                        activity.clone(),
                        &missing_core,
                        native
                    )
                    .await?,
                ActivityEnterOutcome::Refused(_)
            ));
            let rejected: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid),(SELECT count(*) FROM game_soulpit_encounters),(SELECT count(*) FROM game_activity_handoff_groups)")
                .bind(id(71).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                rejected,
                (2, 0, 0),
                "failed core proof rolls back the prepared group"
            );
            let entered = owner
                .enter_soulpit(
                    &authority,
                    &h.node,
                    &permit,
                    activity.clone(),
                    &core,
                    native,
                )
                .await
                .map_err(|e| format!("atomic Soulpit admission: {e}"))?;
            let ActivityEnterOutcome::Entered(receipt) = entered else {
                return Err(format!("Soulpit shared entry refused: {entered:?}").into());
            };
            assert_eq!(receipt.handoff_id, activity.handoff_id);
            let bound = h
                .root
                .read_soulpit_scope_binding(
                    &authority,
                    &h.node,
                    activity.destination_scope,
                    activity.destination_generation,
                )
                .await
                .map_err(debug)?
                .ok_or("current Soulpit scope binding")?;
            assert_eq!(bound.handoff_id, activity.handoff_id);
            assert_eq!(bound.members.len(), 1);
            assert_eq!(
                bound.members[0].character.as_bytes(),
                source.character_id.as_bytes()
            );
            assert_eq!(bound.members[0].slot, 0);
            assert_eq!(
                bound.members[0].destination_session,
                activity.participants[0].destination_game_session_id
            );
            let counts: (i64, i64, i64, i64) = sqlx::query_as("SELECT (SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid),(SELECT count(*) FROM game_soulpit_encounters),(SELECT count(*) FROM game_item_craft_records WHERE recipe='oteryn:soulpit.entry'),(SELECT count(*) FROM game_soulpit_entry_handoff_commits)")
                .bind(id(71).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(counts, (1, 1, 1, 1));
            let states: (i16, i16) = sqlx::query_as("SELECT (SELECT session_state FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid),(SELECT session_state FROM game_durability_reconnect_sessions WHERE game_session_id=encode($2,'hex')::uuid)")
                .bind(source.game_session_id.as_bytes().as_slice()).bind(activity.participants[0].destination_game_session_id.as_bytes().as_slice())
                .fetch_one(&h.pool).await?;
            assert_eq!(states, (3, 2));
            assert!(matches!(
                owner
                    .enter_soulpit(
                        &authority,
                        &h.node,
                        &permit,
                        activity.clone(),
                        &core,
                        native
                    )
                    .await?,
                ActivityEnterOutcome::Replayed(_)
            ));
            let mut rebound = core;
            rebound.source = id(72);
            assert!(matches!(
                owner
                    .enter_soulpit(&authority, &h.node, &permit, activity, &rebound, native)
                    .await?,
                ActivityEnterOutcome::Refused(_)
            ));
            let quantity: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(71).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(quantity, 1, "replay/rebind must never debit another core");
            let store =
                crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone());
            let in_room = crate::gameplay_transport::item_fence_of(
                store
                    .current_session_at(receipt.participants[0].destination_game_session_id)
                    .await?
                    .0,
            )
            .ok_or("current Soulpit Instance session")?;
            let placement = owner
                .read_return_placement(&authority, &h.node, in_room)
                .await?
                .ok_or("Soulpit return placement")?;
            let returning = crate::durability::activity_handoff::ActivityReturnRequest {
                return_id: id(94),
                handoff_id: receipt.handoff_id,
                command_id: None,
                source: in_room,
                destination_game_session_id: crate::foundation::GameSessionId::decode(&id(95))
                    .map_err(debug)?,
                destination_transport: Some(
                    crate::foundation::AuthenticatedTransportRefV1::decode(&[95; 16])
                        .map_err(debug)?,
                ),
                destination_generation: placement.destination_generation,
                reason: crate::durability::activity_handoff::ActivityReturnReason::Defeated,
                current_vitals: crate::durability::character_last_position::LastVitals::new(83, 17),
                current_rest: None,
            };
            assert!(
                matches!(
                    owner
                        .return_participant(&authority, &h.node, returning.clone())
                        .await?,
                    crate::durability::activity_handoff::ActivityReturnOutcome::Refused(
                        crate::durability::activity_handoff::ActivityRefusal::NotDue
                    )
                ),
                "a live Soulpit cannot request a terminal return"
            );
            if exit_while_live {
                let mut exit = returning;
                exit.command_id = Some(1);
                exit.reason = crate::durability::activity_handoff::ActivityReturnReason::Exit;
                let outcome = owner.return_participant(&authority, &h.node, exit).await?;
                assert!(
                    matches!(
                        outcome,
                        crate::durability::activity_handoff::ActivityReturnOutcome::Returned(_)
                    ),
                    "the live participant can use the native exit: {outcome:?}"
                );
                let stored = h
                    .root
                    .read_soulpit_encounter(
                        &authority,
                        &h.node,
                        receipt.handoff_id,
                        receipt.destination_scope,
                        receipt.destination_generation,
                    )
                    .await?
                    .ok_or("retained exited encounter")?;
                assert!(
                    stored.encounter.snapshot().phase.is_terminal(),
                    "the shared native exit must terminalize a fight after its last participant leaves"
                );
                return Ok(());
            }
            let interrupted = h
                .root
                .advance_soulpit(
                    &authority,
                    &h.node,
                    crate::durability::soulpit::SoulpitStepRequest {
                        handoff_id: receipt.handoff_id,
                        occurrence: id(96),
                        runtime_scope: receipt.destination_scope,
                        scope_ownership_generation: receipt.destination_generation,
                        expected_revision: 0,
                        now_micros: 2,
                        action: crate::durability::soulpit::SoulpitAction::Interrupt,
                        survivors: Vec::new(),
                    },
                )
                .await
                .map_err(debug)?;
            assert!(interrupted.encounter.snapshot().phase.is_terminal());
            let returned = owner
                .return_participant(&authority, &h.node, returning.clone())
                .await?;
            assert!(
                matches!(
                    returned,
                    crate::durability::activity_handoff::ActivityReturnOutcome::Returned(_)
                ),
                "terminal Soulpit must compose the shared native server return: {returned:?}"
            );
            assert!(matches!(
                owner
                    .return_participant(&authority, &h.node, returning)
                    .await?,
                crate::durability::activity_handoff::ActivityReturnOutcome::Replayed(_)
            ));
            assert!(
                h.root
                    .read_soulpit_scope_binding(
                        &authority,
                        &h.node,
                        receipt.destination_scope,
                        receipt.destination_generation
                    )
                    .await
                    .map_err(debug)?
                    .is_none(),
                "returned groups are no longer live"
            );

            Ok(())
            }.await;
            result?;
            h.cleanup().await
        })
    })?;
    eprintln!(
        "SRX3_SOULPIT POSTGRES_CASE_COMPLETED atomic_core_successor_replay exit_while_live={exit_while_live} without_core_context={without_core_context} duration={:?}",
        started.elapsed()
    );
    Ok(())
}
