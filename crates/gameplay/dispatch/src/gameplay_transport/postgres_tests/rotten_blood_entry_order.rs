//! The dispatch prerequisite gate observes each committed Rotten Blood death.
use super::fixture::{Harness, debug};
use super::kill_reward_e2e_fixtures::scenario_thread::run_async;
use super::soul_war_fixtures::{current, kill};
use super::w11_chain::prerequisite;
use super::{Dependencies, TestResult, prepare_current_session, reset};
use oteryn_gameplay_encounters::bosses::endgame_entry::EntryRefusal;
use oteryn_gameplay_quests::quest::rotten_blood::{ACCESS_TRACK, REQUIRED_LEVEL, RottenBloodBoss};

#[test]
fn rotten_blood_postgres_bakragore_entry_refused_until_children() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    run_async(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "w11chain_rotten", false, false).await?;
        eprintln!(
            "w11chain-marker: rotten_blood_postgres_bakragore_entry_refused_until_children db ready"
        );
        let deps = Dependencies::new()?;
        let (controller, _) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(controller.active().ok_or("active generation")?);
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        super::qengine_outfit_stories_fixtures::earn_level(
            &owner,
            &admitted,
            &authority,
            REQUIRED_LEVEL,
        )
        .await?;

        let session = admitted.game_session_id;
        let character = current(&owner, session).await?.character_id;
        // Import only the completed access prerequisites, as in rotten_blood_progression.
        let mut seed = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *seed)
            .await?;
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,'oteryn:quest/rotten-blood',4,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(character.as_bytes().as_slice()).bind(ACCESS_TRACK).execute(&mut *seed).await?;
        seed.commit().await?;

        let outfit = "oteryn:appearance-grant/quest/rotten-blood/decaying-defender-base";
        let outfit_rows = || async {
            sqlx::query_as::<_, (i64, i64)>(
                "SELECT (SELECT count(*) FROM game_account_appearance_unlocks u
                  JOIN game_character_roots c USING(account_id)
                  WHERE c.character_id=encode($1,'hex')::uuid AND u.unlock_key=$2),
                  (SELECT count(*) FROM game_account_appearance_unlock_receipts
                  WHERE granting_character_id=encode($1,'hex')::uuid AND unlock_key=$2)",
            )
            .bind(character.as_bytes().as_slice())
            .bind(outfit)
            .fetch_one(&h.pool)
            .await
        };
        assert_eq!(outfit_rows().await?, (0, 0));
        let final_key = RottenBloodBoss::Bakragore.creature_key();
        for (index, boss) in RottenBloodBoss::CHILDREN.into_iter().enumerate() {
            assert_eq!(
                prerequisite(&owner, admitted, final_key).await?,
                Err(EntryRefusal::BossChain)
            );
            assert_eq!(
                prerequisite(&owner, admitted, boss.creature_key()).await?,
                Ok(())
            );
            kill(
                &owner,
                &h.pool,
                session,
                120 + u8::try_from(index)?,
                boss.creature_key(),
                true,
            )
            .await?;
            let state = h
                .root
                .read_rotten_blood_state(&authority, &h.node, current(&owner, session).await?)
                .await
                .map_err(debug)?;
            assert!(state.holds_child(boss));
            assert_eq!(state.regular_taints(), u8::try_from(index + 1)?);
        }

        assert_eq!(prerequisite(&owner, admitted, final_key).await?, Ok(()));
        kill(&owner, &h.pool, session, 125, final_key, true).await?;
        let state = h
            .root
            .read_rotten_blood_state(&authority, &h.node, current(&owner, session).await?)
            .await
            .map_err(debug)?;
        assert!(state.final_taint() && state.completed());
        assert_eq!((state.regular_taints(), state.child_flags()), (0, 0));
        assert_eq!(outfit_rows().await?, (1, 1));
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
