//! Real current movement snapshots drive the existing Quest cooldown receipt owner.
use super::*;
use crate::durability::quest_state::{
    NativeWorldChangeEntry, QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
};
use crate::foundation::{CharacterLease, CommandId, CommandRef, MovementLocalPosition};
use oteryn_gameplay_quests::quest::{QuestRefusal, world_change_entry as rules};
use std::sync::Arc;

#[test]
fn wvar_entry_native_step_clock_refresh_replay_and_fences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("WVAR_ENTRY_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "World entry PG thread")??;
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        let start = std::time::Instant::now();
        reset();
        let h = Harness::create_with_admission_seed(admin, "wvar_entry", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let world = deps.admitted.world_id;
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let generation = controller.active().ok_or("generation")?;
        let definition = oteryn_gameplay_world_change::variants::definitions()?
            .iter()
            .find(|d| d.key == rules::CHANGE)
            .ok_or("Warpath")?;
        let position = definition
            .variants
            .iter()
            .flat_map(|v| &v.portals)
            .find(|p| !p.exit)
            .ok_or("entry")?
            .position;
        let epoch = (1..100_000)
            .find(|e| {
                crate::durability::world_event::changes::roll(world, *e, 0)
                    .is_ok_and(|d| d.variant(rules::CHANGE).is_some())
            })
            .ok_or("active epoch")?;
        let closed = (epoch + 1..epoch + 100_000)
            .find(|e| {
                crate::durability::world_event::changes::roll(world, *e, 0)
                    .is_ok_and(|d| d.variant(rules::CHANGE).is_none())
            })
            .ok_or("closed epoch")?;
        let fence = deps
            .owner(&h, &authority)
            .current_quest_fence(deps.admitted.game_session_id)
            .await
            .map_err(|()| "current quest fence")?
            .ok_or("current quest fence")?;
        let pin = ChannelContentPin::from_activation(
            world,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            [8; 32],
            (
                i32::from(position.0),
                i32::from(position.1),
                i16::from(position.2),
            ),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                actor,
                MovementLocalPosition {
                    x: i32::from(position.0),
                    y: i32::from(position.1),
                    floor: i16::from(position.2),
                },
            )
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                deps.admitted.game_session_id,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice()).bind(generation.identity().client_artifact_digest().as_slice()).bind(activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        super::lumberjack::seed_epoch(
            &h,
            world,
            epoch,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
        )
        .await?;
        let catalogue = Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        );
        let capture =
            |runtime: &ChannelRuntimeV1, day| -> TestResult<(CommandRef, NativeWorldChangeEntry)> {
                let entered = runtime.read_actor_position(actor).map_err(debug)?;
                let command = CommandRef::new(
                    fence.game_session_id,
                    CommandId::new(entered.revision()).map_err(debug)?,
                );
                let source = NativeWorldChangeEntry::from_current_runtime(
                    runtime,
                    actor,
                    command,
                    entered,
                    day,
                    fence.item_fence(),
                )
                .map_err(debug)?
                .ok_or("source portal")?;
                Ok((command, source))
            };
        let request = |key: &str, command| QuestTransitionRequest {
            transition_key: key.into(),
            cause: QuestCause::Step(command),
        };
        let sequencer =
            crate::durability::character_revision_sequencer::CharacterRevisionSequencer::new();
        let mut slot = sequencer.acquire(fence.character_id).await;
        let (first, source) = capture(&runtime, epoch)?;
        assert!(
            h.root
                .commit_character_quest_transition(
                    &authority,
                    &h.node,
                    fence,
                    request(rules::ENTER, first),
                    Arc::clone(&catalogue)
                )
                .await
                .is_err()
        );
        assert!(matches!(
            slot.commit_world_change_entry(
                &h.root,
                &authority,
                &h.node,
                fence,
                request(rules::ENTER, first),
                Arc::clone(&catalogue),
                source.clone()
            )
            .await?,
            QuestTransitionOutcome::Committed(_)
        ));
        let initial = clock(&h).await?;
        assert!(initial > 0);
        let count = receipts(&h).await?;
        assert!(matches!(
            slot.commit_world_change_entry(
                &h.root,
                &authority,
                &h.node,
                fence,
                request(rules::ENTER, first),
                Arc::clone(&catalogue),
                source
            )
            .await?,
            QuestTransitionOutcome::AlreadyCommitted(_)
        ));
        assert_eq!(clock(&h).await?, initial);
        assert_eq!(receipts(&h).await?, count);
        next_step(&mut runtime, actor)?;
        let (early, source) = capture(&runtime, epoch)?;
        assert!(matches!(
            slot.commit_world_change_entry(
                &h.root,
                &authority,
                &h.node,
                fence,
                request(rules::ENTER, early),
                Arc::clone(&catalogue),
                source.clone()
            )
            .await?,
            QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
        ));
        assert_eq!(
            clock(&h).await?,
            initial,
            "denied entry itself cannot silently rewrite time"
        );
        assert!(matches!(
            slot.commit_world_change_entry(
                &h.root,
                &authority,
                &h.node,
                fence,
                request(rules::REFRESH, early),
                Arc::clone(&catalogue),
                source.clone()
            )
            .await?,
            QuestTransitionOutcome::Committed(_)
        ));
        let refreshed = clock(&h).await?;
        assert!(refreshed >= initial);
        let count = receipts(&h).await?;
        assert!(matches!(
            slot.commit_world_change_entry(
                &h.root,
                &authority,
                &h.node,
                fence,
                request(rules::REFRESH, early),
                Arc::clone(&catalogue),
                source
            )
            .await?,
            QuestTransitionOutcome::AlreadyCommitted(_)
        ));
        assert_eq!(clock(&h).await?, refreshed);
        assert_eq!(receipts(&h).await?, count);
        next_step(&mut runtime, actor)?;
        let (attempt, source) = capture(&runtime, epoch)?;
        let mut revision3 = fence;
        revision3.expected_character_revision =
            crate::domain::CharacterRevision::new(3).map_err(debug)?;
        let alias = "oteryn:quest-transition/world-change/warpath/renamed";
        let malicious = Arc::new(
            oteryn_gameplay_quests::quest::QuestStateCatalogue::new(
                "content-1",
                vec![catalogue.track(rules::TRACK).ok_or("track")?.clone()],
                vec![oteryn_gameplay_quests::quest::QuestTransition {
                    key: alias.into(),
                    quest: rules::QUEST.into(),
                    effects: vec![oteryn_gameplay_quests::quest::QuestEffect {
                        track: rules::TRACK.into(),
                        from: oteryn_gameplay_quests::quest::QuestComparison::Any,
                        effect: oteryn_gameplay_quests::quest::QuestEffectKind::SetNow,
                    }],
                    completes: false,
                    experience: None,
                }],
            )
            .map_err(debug)?,
        );
        assert!(
            h.root
                .commit_character_world_change_entry(
                    &authority,
                    &h.node,
                    revision3,
                    request(alias, attempt),
                    malicious,
                    source.clone()
                )
                .await
                .is_err()
        );
        let mut stale = revision3;
        stale.character_lease_generation += 1;
        assert!(
            h.root
                .commit_character_world_change_entry(
                    &authority,
                    &h.node,
                    stale,
                    request(rules::REFRESH, attempt),
                    Arc::clone(&catalogue),
                    source.clone()
                )
                .await
                .is_err()
        );
        super::lumberjack::seed_epoch(
            &h,
            world,
            closed,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
        )
        .await?;
        assert!(
            slot.commit_world_change_entry(
                &h.root,
                &authority,
                &h.node,
                fence,
                request(rules::REFRESH, attempt),
                Arc::clone(&catalogue),
                source
            )
            .await
            .is_err()
        );
        let (_, closed_source) = capture(&runtime, closed)?;
        assert!(
            slot.commit_world_change_entry(
                &h.root,
                &authority,
                &h.node,
                fence,
                request(rules::REFRESH, attempt),
                catalogue,
                closed_source
            )
            .await
            .is_err()
        );
        assert_eq!(clock(&h).await?, refreshed);
        assert_eq!(receipts(&h).await?, count);
        drop(slot);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        eprintln!(
            "WVAR_ENTRY POSTGRES_CASE_COMPLETED native-step,first-enter,early-denial,refresh-receipt,replay,alias,stale-lease,stale-epoch,closed-variant duration={:?}",
            start.elapsed()
        );
        Ok(())
    })
}
fn next_step(
    runtime: &mut ChannelRuntimeV1,
    actor: crate::foundation::ExactActorRef,
) -> TestResult {
    let before = runtime
        .borrow_movement_position()
        .read(actor)
        .map_err(debug)?;
    let mut next = before.position();
    next.x += 1;
    let moved = runtime
        .borrow_movement_position()
        .commit_cardinal(before, next)
        .map_err(debug)?;
    runtime
        .borrow_movement_position()
        .commit_cardinal(moved, before.position())
        .map_err(debug)?;
    Ok(())
}
async fn clock(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key='oteryn:quest-progress/world-change/warpath/last-entry'").fetch_one(&h.pool).await?)
}
async fn receipts(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE quest_key='oteryn:quest.world_change_warpath'").fetch_one(&h.pool).await?)
}
