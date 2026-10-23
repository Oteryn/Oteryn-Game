//! LOOP-TRAIN-1: real composed admission/dispatch against the registered PostgreSQL library tree.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_gameplay_progression::loop_training::TrainingRates;
use std::sync::Arc;

#[test]
fn loop_training_postgres_admission_marker_replay_and_restart() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "loop training PG panic")??;
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "loop_training", true, false).await?;
        eprintln!("POSTGRES_STARTED LOOP-TRAIN-1");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let (controller, _) = super::death_ml_loss_composed::production_generation_with_pin(
            deps.admitted.world_id,
            "content-1",
        )?;
        let active = controller.active().ok_or("active")?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        deps.initialize(&owner).await;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session(admitted.game_session_id)
                .await?;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        assert!(owner.states.training.config.is_none());
        owner.admit_loop_training(&mut admitted).await;
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_training_markers")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(count, 0, "shipped gate writes nothing");
        let mut rates = TrainingRates::from_json(include_bytes!(
            "../../../../../../rulesets/training/provisional-v1.json"
        ))
        .map_err(debug)?;
        rates.enabled = true;
        owner.states.training.config = Some(Arc::new(rates));
        owner.admit_loop_training(&mut admitted).await;
        assert_eq!(admitted.first_entry, FirstEntryOutcome::Positioned);
        let first:(String,i32,i32,i16)=sqlx::query_as("SELECT committed_character_revision::text,stamina_minutes,offline_pool_s,offline_skill FROM game_character_build_state").fetch_one(&h.pool).await?;
        assert_eq!(first, ("2".into(), 2520, 43200, -1));
        owner.admit_loop_training(&mut admitted).await;
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_training_markers")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(count, 1);
        let clock:(i64,Option<String>)=sqlx::query_as("SELECT training_created_at_us,training_predecessor_session::text FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid").bind(id(50).as_slice()).fetch_one(&h.pool).await?;
        assert!(clock.0 > 0);
        assert!(clock.1.is_none());
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
            .read_character_build_state(
                &reopened,
                crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?,
            )
            .await
            .map_err(debug)?;
        assert_eq!(
            build,
            crate::durability::character_build::DurableBuildState::default()
        );
        drop(reopened);
        drop(seal);
        h.cleanup().await
    })
}

#[path = "loop_training/exercise.rs"]
mod exercise;
#[path = "loop_training/world.rs"]
mod world;

#[test]
fn loop_training_postgres_missing_progression_is_atomic_with_settlement() -> TestResult {
    std::thread::Builder::new().stack_size(64<<20).spawn(||run(async |admin| {
        let h=Harness::create_with_admission_seed(admin,"loop_training_init",false,false).await?;
        prepare_current_session(&h).await?;
        let seal=h.recovery.seal_current().map_err(debug)?;let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
        let deps=Dependencies::new()?;
        let (controller,_)=super::death_ml_loss_composed::production_generation_with_pin(deps.admitted.world_id,"content-1")?;
        let active=controller.active().ok_or("active")?;
        let current=crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone()).current_session(deps.admitted.game_session_id).await?;
        let item=crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        let fence=crate::durability::character_progression::CurrentCharacterGameplayFence {character_id:item.character_id,game_session_id:item.game_session_id,connection_generation:item.connection_generation,character_lease_generation:item.character_lease_generation,runtime_scope:item.runtime_scope,scope_ownership_generation:item.scope_ownership_generation,expected_character_revision:crate::domain::CharacterRevision::new(1).map_err(debug)?};
        let mut rates=TrainingRates::from_json(include_bytes!("../../../../../../rulesets/training/provisional-v1.json")).map_err(debug)?;rates.enabled=true;
        let context=crate::durability::loop_training::TrainingContext {rates:Arc::new(rates),formula:active.native_gameplay().and_then(|n|n.training_formula()).ok_or("formula")?.clone(),progression:active.progression().cloned()};
        let mut stale=context.clone();stale.formula.content_revision="stale-content".into();
        assert!(h.root.settle_offline_training(&authority,&h.node,fence,stale).await.is_err());
        let rows:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_character_progression_state),(SELECT count(*) FROM game_character_training_markers)").fetch_one(&h.pool).await?;assert_eq!(rows,(0,0));
        let settled=h.root.settle_offline_training(&authority,&h.node,fence,context).await.map_err(debug)?;assert_eq!(settled.revision.get(),2);
        drop(authority);drop(seal);h.cleanup().await
    }).map_err(|e|e.to_string()))?.join().map_err(|_|"training initializer panic")??;
    Ok(())
}
