//! Live journal observation must refresh save-scoped facts without a Character write.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::gameplay_transport::quest_log::{QuestLogLines, QuestLogObservation};
use oteryn_gameplay_quests::quest::{
    QuestComparison as C, QuestEffect, QuestEffectKind as E, QuestStateCatalogue, QuestTrack,
    QuestTransition, shared_world_progress,
};
const QUEST: &str = "oteryn:quest.qm55_observer_event";
const TRACK: &str = "oteryn:quest-progress/qm55/observer";
#[test]
fn qm55_live_quest_log_refreshes_on_save_without_character_mutation() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "M55 journal worker failed")?
        .map_err(Into::into)
}
fn scenario() -> TestResult {
    run(async |admin| {
        let started = std::time::Instant::now();
        let h = Harness::create_with_admission_seed(admin, "qm55_log_refresh", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let d = Dependencies::new()?;
        let mut owner = d.owner(&h, &authority);
        let world = owner.world_id;
        for epoch in 1..=2_i64 {
            sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,'2031-06-13T12:00:00Z','2031-06')").bind(world.as_bytes().as_slice()).bind(epoch).bind([7_u8;32].as_slice()).execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_world_event_epochs(world_id,reset_epoch,active,announcement,seasonal_calendar_revision,seasonal_event_keys) VALUES(encode($1,'hex')::uuid,$2,false,'',1,'{}')").bind(world.as_bytes().as_slice()).bind(epoch).execute(&h.pool).await?;
        }
        sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
            .bind(world.as_bytes().as_slice())
            .execute(&h.pool)
            .await?;
        assert!(
            h.root
                .world_changes(world, 1)
                .await?
                .ok_or("current day")?
                .grimvale_full_moon
        );
        let reset = shared_world_progress::save_event_restart_key(QUEST);
        let finish = "oteryn:quest-transition/qm55/observer-finish";
        let transition = |key: &str, from, after, completes| QuestTransition {
            key: key.into(),
            quest: QUEST.into(),
            effects: vec![QuestEffect {
                track: TRACK.into(),
                from,
                effect: E::Set(after),
            }],
            completes,
            experience: None,
            world_change: Some("grimvale".into()),
        };
        let catalogue = std::sync::Arc::new(
            QuestStateCatalogue::new(
                "content-1",
                vec![QuestTrack {
                    key: TRACK.into(),
                    quest: QUEST.into(),
                    initial: 0,
                    min: 0,
                    max: 1,
                }],
                vec![
                    transition(&reset, C::Any, 0, false),
                    transition(finish, C::Between(0, 0), 1, true),
                ],
            )
            .map_err(debug)?,
        );
        let loaded = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?;
        owner.quest_log_content = Some(
            QuestLogLines::new(
                oteryn_gameplay_quests::quest::log::loader::load_embedded_quest_log(&loaded)?
                    .content,
            )
            .ok_or("log content")?,
        );
        owner.quest_catalogue = Some(catalogue.clone());
        d.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(d.actor)
            .map_err(debug)?;
        let mut admitted = d.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let session = admitted.game_session_id;
        for (ordinal, key) in [(1, reset.as_str()), (2, finish)] {
            let fence = owner
                .current_quest_fence(session)
                .await
                .map_err(|_| "fence")?
                .ok_or("live session")?;
            let request = QuestTransitionRequest {
                transition_key: key.into(),
                cause: QuestCause::Command(crate::foundation::CommandRef::new(
                    session,
                    crate::foundation::CommandId::new(ordinal).map_err(debug)?,
                )),
            };
            assert!(matches!(
                h.root
                    .commit_character_quest_transition(
                        &authority,
                        &h.node,
                        fence,
                        request,
                        catalogue.clone()
                    )
                    .await?,
                QuestTransitionOutcome::Committed(_)
            ));
        }
        owner.admit_quest_session(&admitted).await;
        let QuestLogObservation::Changed(first) =
            owner.observe_quest_log(d.actor, session, None).await
        else {
            return Err("initial journal unavailable".into());
        };
        assert!(
            first
                .copy
                .quest_state(QUEST)
                .ok_or("quest log state")?
                .completed
        );
        assert_eq!(first.copy.track_value(TRACK), Some(1));
        let before: String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(owner.current_quest_fence(session).await.map_err(|_|"fence")?.ok_or("live")?.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        sqlx::query("UPDATE game_world_reset_active SET reset_epoch=2 WHERE world_id=encode($1,'hex')::uuid").bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
        h.root.world_changes(world, 2).await?.ok_or("next day")?;
        let observation = owner
            .observe_quest_log(d.actor, session, Some(first.version))
            .await;
        let QuestLogObservation::Changed(next) = observation else {
            return Err("journal retained a retired save without a Character write".into());
        };
        assert!(next.version > first.version);
        assert!(
            !next
                .copy
                .quest_state(QUEST)
                .ok_or("next quest log state")?
                .completed
        );
        assert_eq!(next.copy.track_value(TRACK), Some(0));
        let raw = owner
            .root
            .read_character_quest_state(
                &authority,
                owner
                    .current_quest_fence(session)
                    .await
                    .map_err(|_| "fence")?
                    .ok_or("live")?
                    .character_id,
            )
            .await?;
        assert_eq!(raw.tracks().get(TRACK), Some(&1));
        assert!(matches!(
            owner
                .observe_quest_log(d.actor, session, Some(next.version))
                .await,
            QuestLogObservation::Unchanged
        ));
        let after: String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(owner.current_quest_fence(session).await.map_err(|_|"fence")?.ok_or("live")?.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(before, after, "journal scope refresh must remain read-only");
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        assert!(started.elapsed().as_secs_f64() > 0.01);
        eprintln!(
            "M55_PG_EXECUTED live_journal_refresh duration={:?}",
            started.elapsed()
        );
        Ok(())
    })
}
