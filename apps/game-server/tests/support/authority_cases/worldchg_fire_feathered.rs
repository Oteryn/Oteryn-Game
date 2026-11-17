//! Guardian quest receipt composes with the existing compound world reset.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::{
    quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest},
    world_event::changes,
};
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::fire_feathered_native;
#[test]
fn worldchg_postgres_fire_feathered_guardian_completion_waits_for_quara_reset()
-> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64<<20).spawn(move ||->Result<(),String> {
        fixture::runtime().map_err(fixture::debug)?.block_on(async {
            let start=std::time::Instant::now();
            let h=fixture::Harness::create(admin,"worldchg_fire_feathered",true).await?;
            let result=exercise(&h).await;h.cleanup().await?;result?;
            assert!(start.elapsed().as_secs_f64()>1.0);
            println!("WORLDCHG-Q PG EXECUTED: native FireFeathered Guardian orbcompletion+Worldcondition atomicreceipt,replay,incomplete refusal,compoundQuarareset; elapsed={:?}",start.elapsed());
            Ok::<(),Box<dyn std::error::Error>>(())
        }).map_err(fixture::debug)
    })?.join().map_err(|_|"FireFeathered PG thread failed")?.map_err(Into::into)
}
async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    let scope = fixture::scope()?;
    let world = scope.world_id();
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0),'1970-01')")
        .bind(world.as_bytes().as_slice()).bind([7_u8;32].as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    let _day = h.root.world_changes(world, 1).await?.ok_or("current day")?;
    // Fixture is already in the awake phase; physical phase producer is covered separately.
    sqlx::query("UPDATE game_world_change_counters SET state=jsonb_set(state,'{stage}','3') WHERE world_id=encode($1,'hex')::uuid AND change_key='the_fire_feathered_serpent'")
        .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
    let mut catalogue = std::sync::Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(fixture::debug)?
            .catalogue()
            .clone(),
    );
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let session = GameSessionId::decode(&fixture::id(fixture::SESSION)).map_err(fixture::debug)?;
    // Materialize independently earned progress through the real Quest receipt writer.
    // The fixture transition represents prior gathering; all deferred receipt/track guards run.
    {
        use oteryn_gameplay_quests::quest::{
            QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
        };
        let fixture_tracks = fire_feathered_native::TRACKS;
        let values = [1, 1, 1];
        let seed_key = "oteryn:quest-transition/worldchg-fixture/earned-progress";
        let seeded = std::sync::Arc::new(
            QuestStateCatalogue::new(
                "content-1",
                fixture_tracks
                    .iter()
                    .map(|track| catalogue.track(track).cloned().ok_or("source track"))
                    .collect::<Result<Vec<_>, _>>()?,
                vec![
                    catalogue
                        .transition(fire_feathered_native::COMPLETE)
                        .ok_or("native completion")?
                        .clone(),
                    QuestTransition {
                        key: seed_key.into(),
                        quest: fire_feathered_native::QUEST.into(),
                        effects: fixture_tracks
                            .iter()
                            .zip(values)
                            .map(|(track, value)| QuestEffect {
                                track: (*track).into(),
                                from: QuestComparison::Between(0, 0),
                                effect: QuestEffectKind::Set(value),
                            })
                            .collect(),
                        completes: false,
                        experience: None, world_change: None,
                    },
                ],
            )
            .map_err(fixture::debug)?,
        );
        assert!(matches!(
            h.root
                .commit_character_quest_transition(
                    &authority,
                    &h.node,
                    fixture::fence(1)?,
                    QuestTransitionRequest {
                        transition_key: seed_key.into(),
                        cause: QuestCause::Command(CommandRef::new(
                            session,
                            CommandId::new(99).map_err(fixture::debug)?
                        )),
                    },
                    seeded.clone()
                )
                .await?,
            QuestTransitionOutcome::Committed(_)
        ));
        // The native completion shares the definition pinned by the earned-progress receipt.
        catalogue = seeded;
    }
    let request = |ordinal| -> fixture::TestResult<QuestTransitionRequest> {
        Ok(QuestTransitionRequest {
            transition_key: fire_feathered_native::COMPLETE.into(),
            cause: QuestCause::Command(CommandRef::new(
                session,
                CommandId::new(ordinal).map_err(fixture::debug)?,
            )),
        })
    };
    let completed = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            fixture::fence(2)?,
            request(1)?,
            catalogue.clone(),
        )
        .await?;
    assert!(
        matches!(&completed, QuestTransitionOutcome::Committed(_)),
        "native completion: {completed:?}"
    );
    let state = || async {
        Ok::<serde_json::Value,Box<dyn std::error::Error>>(sqlx::query_scalar("SELECT state FROM game_world_change_counters WHERE world_id=encode($1,'hex')::uuid AND change_key='the_fire_feathered_serpent'").bind(world.as_bytes().as_slice()).fetch_one(&h.pool).await?)
    };
    let after = state().await?;
    assert_eq!(after["stage"], 3);
    assert_eq!(after["condition"], true);
    for track in fire_feathered_native::TRACKS {
        let value:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2").bind(fixture::id(fixture::CHARACTER).as_slice()).bind(track).fetch_one(&h.pool).await?;
        assert_eq!(value, 0);
    }
    assert!(matches!(
        h.root
            .commit_character_quest_transition(
                &authority,
                &h.node,
                fixture::fence(3)?,
                request(1)?,
                catalogue.clone()
            )
            .await?,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert!(matches!(
        h.root
            .commit_character_quest_transition(
                &authority,
                &h.node,
                fixture::fence(3)?,
                request(2)?,
                catalogue
            )
            .await?,
        QuestTransitionOutcome::Refused(_)
    ));
    assert_eq!(state().await?, after);
    // Mission completion alone did not end the change. The final native-counter
    // Renegade death reaches the independent 2000-kill arm of the conjunction.
    sqlx::query("UPDATE game_world_change_counters SET state=jsonb_set(state,'{counts,1}','1999') WHERE world_id=encode($1,'hex')::uuid AND change_key='the_fire_feathered_serpent'").bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
    let day = h.root.world_changes(world, 1).await?.ok_or("awake day")?;
    let crate::foundation::RuntimeScopeRefV1::Channel { channel_id, .. } = scope else {
        return Err("channel".into());
    };
    let mut occurrence = world.as_bytes().to_vec();
    occurrence.extend_from_slice(channel_id.as_bytes());
    occurrence.extend_from_slice(&1_u64.to_be_bytes());
    occurrence.extend_from_slice(&100_u32.to_be_bytes());
    occurrence.extend_from_slice(&1_u64.to_be_bytes());
    let event = changes::counters::CreditEvent {
        source_kind: 1,
        occurrence,
        credits: vec![(
            changes::counters::CounterKind::FireFeathered,
            changes::counters::Credit::Kill(1),
        )],
        counter_generations: day.counter_generations,
        occurred_at_ms: 100,
        character: Some(
            crate::foundation::CharacterId::decode(fixture::fence(3)?.character_id.as_bytes())
                .map_err(fixture::debug)?,
        ),
    };
    let generation = crate::foundation::ScopeOwnershipGeneration::new(1).map_err(fixture::debug)?;
    assert!(matches!(
        h.root
            .credit_world_change_event(&authority, &h.node, scope, generation, 1, event)
            .await?,
        changes::counters::CreditOutcome::Applied { .. }
    ));
    assert_eq!(state().await?["stage"], 1);
    Ok(())
}
