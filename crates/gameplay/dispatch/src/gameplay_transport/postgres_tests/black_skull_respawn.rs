use super::fixture::{Harness, debug, fence, id};
use super::{
    Dependencies, TestResult, death_ml_loss_composed_fixtures, kill_reward_e2e_fixtures,
    prepare_current_session, reset, track,
};
use crate::foundation::{CharacterId, CharacterLease};
use crate::gameplay_transport::{AdmissionRetryBudget, SessionDeath};

#[test]
fn black_skull_live_respawn_publishes_and_saves_reduced_vitals() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, false)
}

#[test]
fn black_skull_pending_respawn_recovers_reduced_vitals_after_skull_expiry() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, true)
}

fn scenario(admin: String, recover_pending: bool) -> TestResult {
    kill_reward_e2e_fixtures::scenario_thread::run_async(async move |_| {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "black_skull_respawn", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let (generation, _) = death_ml_loss_composed_fixtures::production_generation_with_pin(
            deps.admitted.world_id,
            "content-1",
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation.active().ok_or("active generation")?);
        let mut admitted = deps.admitted;
        let actor = deps.actor;
        let session = admitted.game_session_id;
        let lease =
            CharacterLease::new(CharacterId::decode(&id(41)).map_err(debug)?, 2).map_err(debug)?;
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        track(&owner, &deps)?;
        sqlx::query("INSERT INTO game_character_pvp_state(character_id,world_id,skull,skull_until) \
            VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,2,clock_timestamp()+interval '1 day')")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).execute(&h.pool).await?;
        eprintln!("black_skull_respawn PostgreSQL fixture ready, pending={recover_pending}");

        let death = {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(actor, session, lease)
                .map_err(debug)?;
            let mut states = owner.states.spell_states.lock().await;
            states
                .apply_creature_damage_native(
                    &runtime,
                    actor,
                    session,
                    100_000,
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
                        owner.owner_now().get(),
                    ),
                )
                .ok_or("lethal hit")?;
            states
                .player_death(&runtime, actor, session)
                .ok_or("death")?
        };

        let vitals = if recover_pending {
            assert_eq!(
                owner.settle_death(actor, session, death, false).await.0,
                SessionDeath::Durable
            );
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role = replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE game_character_pvp_state SET skull_until=clock_timestamp()-interval '1 day' \
                WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;

            // Rebuild the runtime owner with no live death record, as after a lost process.
            let restored = Dependencies::new()?;
            let restored_owner = restored.owner(&h, &authority);
            let actor = restored.actor;
            {
                let mut runtime = restored.runtime.lock().await;
                runtime
                    .initialize_first_entry_position(actor)
                    .map_err(debug)?;
                runtime
                    .bind_attacker_lease(actor, session, lease)
                    .map_err(debug)?;
            }
            restored.initialize(&restored_owner).await;
            let budget = AdmissionRetryBudget::for_root(&session, &h.root);
            assert!(
                restored_owner
                    .consume_admitted_respawn(&budget, session, actor, None)
                    .await
            );
            let runtime = restored.runtime.lock().await;
            let states = restored_owner.states.spell_states.lock().await;
            states
                .get(&runtime, actor, session)
                .ok_or("restored actor")?
                .vitals()
        } else {
            owner
                .respawn_after_death(actor, session, death)
                .await
                .ok_or("temple respawn")?
                .1
        };
        assert_eq!(
            (vitals.health, vitals.mana),
            (40, 0),
            "the production respawn must apply black-skull vitals"
        );
        let mut current = fence(2)?;
        current.character_lease_generation = 2;
        let saved = h
            .root
            .read_character_last_position(&authority, &h.node, current)
            .await
            .map_err(debug)?
            .ok_or("saved temple position")?;
        let saved_vitals = saved.record.vitals.ok_or("saved temple vitals")?;
        assert_eq!((saved_vitals.health(), saved_vitals.mana()), (40, 0));
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
