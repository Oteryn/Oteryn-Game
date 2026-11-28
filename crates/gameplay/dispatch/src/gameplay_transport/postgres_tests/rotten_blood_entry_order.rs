//! Bloodshade's paid access, ordered captured-death settlement, and durable readmission.
#[path = "w11_journey/session.rs"]
mod session_fixture;
use super::fixture::{Harness, debug};
use super::kill_reward_e2e_fixtures::scenario_thread::run_async;
use super::soul_war_fixtures::{current, kill};
use super::w11_chain::prerequisite;
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::domain::currency::{COIN_STACK_MAXIMUM, Coin};
use oteryn_gameplay_encounters::bosses::endgame_entry::EntryRefusal;
use oteryn_gameplay_quests::quest::rotten_blood::{ACCESS_PRICE, REQUIRED_LEVEL, RottenBloodBoss};

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
        let world = super::fixture::fence(1)?.runtime_scope.world_id();
        let (controller, pin) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                world,
                "content-1",
            )?;
        let room = pin.qualified_room().clone();
        let deps = Dependencies::with_content(
            pin.into_channel_pin(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        // Give the player spending money as starting inventory. The access
        // track, the fee burn and every boss credit must come from their owners.
        let crystal = Coin::Crystal;
        let mut coins = ACCESS_PRICE.div_ceil(crystal.worth());
        let mut stacks = Vec::new();
        while coins > 0 {
            let quantity = coins.min(u64::from(COIN_STACK_MAXIMUM));
            stacks.push((crystal.production_key(), i64::try_from(quantity)?));
            coins -= quantity;
        }
        super::item_look_fixtures::seed_main_backpack(&h, &stacks).await?;
        sqlx::query("INSERT INTO game_type2_audit_activation VALUES(1,now())")
            .execute(&h.pool)
            .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(controller.active().ok_or("active generation")?);
        let catalogue =
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?;
        owner.quest_catalogue = Some(std::sync::Arc::new(catalogue.catalogue().clone()));
        let mut admitted = deps.admitted;
        session_fixture::prepare(&deps, &owner, &mut admitted).await?;
        super::qengine_outfit_stories_fixtures::earn_level(
            &owner,
            &admitted,
            &authority,
            REQUIRED_LEVEL,
        )
        .await?;

        let session = admitted.game_session_id;
        let character = current(&owner, session).await?.character_id;
        let first_child = RottenBloodBoss::CHILDREN[0].creature_key();
        assert_eq!(
            prerequisite(&owner, admitted, first_child).await?,
            Err(EntryRefusal::QuestAccess)
        );
        // TibiaWiki Rotten Blood Quest/Spoiler (oldid 1107075), Method.
        let npc = session_fixture::npc(&owner, admitted, "oteryn:npc.bloodshade_sacrifice").await?;
        session_fixture::converse(
            &mut owner,
            admitted,
            npc,
            40,
            &["hi", "quest", "detour", "yes"],
        )
        .await?;
        assert_eq!(
            prerequisite(&owner, admitted, first_child).await?,
            Err(EntryRefusal::QuestAccess)
        );
        let fees = || {
            sqlx::query_scalar::<_, i64>(
                "SELECT burn.fee_gold_units FROM game_rotten_blood_access_fees access
                 JOIN game_item_fee_burns burn ON burn.cause_occurrence_id=access.fee_occurrence
                 WHERE access.character_id=encode($1,'hex')::uuid LIMIT 2",
            )
            .bind(character.as_bytes().as_slice())
            .fetch_all(&h.pool)
        };
        assert!(fees().await?.is_empty());
        session_fixture::converse(&mut owner, admitted, npc, 44, &["yes"]).await?;
        assert_eq!(fees().await?, [i64::try_from(ACCESS_PRICE)?]);

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
        let fresh = session_fixture::relog(&h, &owner, admitted).await?;
        let retained = h
            .root
            .read_rotten_blood_state(&authority, &h.node, fresh)
            .await
            .map_err(debug)?;
        assert_eq!(retained, state);
        assert_eq!(outfit_rows().await?, (1, 1));
        assert_eq!(fees().await?, [i64::try_from(ACCESS_PRICE)?]);
        eprintln!(
            "w11chain-marker: Rotten Blood completion and outfit retained after fresh durable admission"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
