//! Kevin's focused conversation advances the production Postman track.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::foundation::ChannelContentPin;
use std::sync::Arc;

async fn publish_item_content(
    h: &Harness,
    admitted: &crate::gameplay_transport::connection::AdmittedSession,
    pin: &crate::content::NativeEntryContentPin,
) -> TestResult {
    let world = admitted.world_id.as_bytes().as_slice();
    let channel = admitted.channel_id.as_bytes().as_slice();
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(world).bind(channel).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,NULL,$4,$5,$6)")
        .bind(world).bind(channel).bind(pin.activation_sequence().to_string())
        .bind(pin.identity().server_artifact_digest().as_slice())
        .bind(pin.identity().client_artifact_digest().as_slice())
        .bind(pin.frame_binding().digest().as_slice())
        .execute(&h.pool).await?;
    Ok(())
}

#[test]
fn crystal_kevin_postgres_requires_the_full_tour_briefing_and_takes_all_bones() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("postman-npc", || {
        run(async |admin| {
            reset();
            let h = Harness::create_with_admission_seed(admin, "postman_npc", true, false).await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
            let (controller, pin) = activated_with_loot_tables(world, None);
            let deps = Dependencies::with_content_and_capacity(
                ChannelContentPin::from_activation(
                    world,
                    pin.activation_sequence(),
                    pin.identity().server_artifact_digest(),
                    pin.identity().client_artifact_digest(),
                    pin.frame_binding().digest(),
                    pin.qualified_room().map_revision_digest(),
                    (2, 0, 7),
                ),
                pin.qualified_room().clone(),
                crate::spell::cast::v1_spell_book().map_err(debug)?,
                3,
            )?;
            // Crystal kevin.lua:53-60 consumes carried items. The item writers
            // check the durable Channel activation as well as the runtime pin.
            publish_item_content(&h, &deps.admitted, &pin).await?;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(controller.active().ok_or("active content")?);
            owner.quest_catalogue = Some(Arc::new(
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?
                    .catalogue()
                    .clone(),
            ));
            deps.initialize(&owner).await;
            let mut admitted = deps.admitted;
            super::qengine_outfit_stories_fixtures::seed_production_progression_context(
                &h,
                &owner,
                admitted.game_session_id,
            )
            .await?;
            owner.admit_progression_session(&mut admitted).await;
            deps.runtime
                .lock()
                .await
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            let [kevin, olrik] = super::classic_quests_fixtures::install_with_offsets(
                &owner,
                &deps.admitted,
                ["oteryn:npc.kevin", "oteryn:npc.olrik"],
                Some([(1, 0), (0, 1)]),
            )
            .await?;
            owner
                .refresh_quest_session(deps.admitted.game_session_id, true)
                .await;
            let mission = || async {
                sqlx::query_scalar::<_, i64>(
                    "SELECT COALESCE(MAX(value),0) FROM game_character_quest_tracks
                     WHERE track_key='oteryn:quest-progress/quest/u7_24/the_postman_missions/mission01'",
                ).fetch_one(&h.pool).await
            };
            for (index, text) in [
                "hi", "yes", "mission", "yes", "yes", "yes", "yes", "yes", "yes", "yes",
            ]
            .into_iter()
            .enumerate()
            {
                super::classic_quests_fixtures::talk(
                    &mut owner,
                    &deps.admitted,
                    kevin,
                    u64::try_from(index)? + 1,
                    text,
                )
                .await?;
                assert_eq!(
                    mission().await?,
                    i64::from(index == 9),
                    "command {index}: {text}"
                );
            }
            let mission04 = "oteryn:quest-progress/quest/u7_24/the_postman_missions/mission04";
            super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
                &h,
                &owner,
                &deps.admitted,
                &[(mission04.into(), 1)],
                91,
            )
            .await?;
            super::item_look_fixtures::seed_main_backpack(
                &h,
                &[
                    ("oteryn:item.tibia.i3115", 100),
                    ("oteryn:item.tibia.i3115", 1),
                    ("oteryn:item.tibia.i3116", 100),
                    ("oteryn:item.tibia.i3116", 1),
                    ("oteryn:item.tibia.i3031", 2),
                ],
            )
            .await?;
            owner
                .refresh_quest_session(deps.admitted.game_session_id, true)
                .await;
            for (command, message) in [(92, "mission"), (93, "yes")] {
                super::classic_quests_fixtures::talk(
                    &mut owner,
                    &deps.admitted,
                    kevin,
                    command,
                    message,
                )
                .await?;
            }
            let stage: i64 = sqlx::query_scalar(
                "SELECT value FROM game_character_quest_tracks WHERE track_key=$1",
            )
            .bind(mission04)
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(stage, 2);
            let consumed: Vec<(String, i64)> = sqlx::query_as(
                "SELECT before_item->>'definition_production_key' AS item, sum(quantity)::bigint
                   FROM game_quest_callback_consumptions
                  WHERE transition_key='oteryn:quest-npc/postman/kevin/bones_complete'
                  GROUP BY item ORDER BY item",
            )
            .fetch_all(&h.pool)
            .await?;
            assert_eq!(
                consumed,
                [
                    ("oteryn:item.tibia.i3115".to_owned(), 101),
                    ("oteryn:item.tibia.i3116".to_owned(), 101),
                ]
            );
            let mission07 = "oteryn:quest-progress/quest/u7_24/the_postman_missions/mission07";
            super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
                &h,
                &owner,
                &deps.admitted,
                &[(mission07.into(), 5)],
                94,
            )
            .await?;
            sqlx::query("INSERT INTO game_type2_audit_activation VALUES(1,now())")
                .execute(&h.pool)
                .await?;
            super::guild_bank_fixtures::bank_seed::balance(&h, id(40), id(41), 3, 180).await?;
            owner
                .refresh_quest_session(deps.admitted.game_session_id, true)
                .await;
            super::classic_quests_fixtures::talk(&mut owner, &deps.admitted, kevin, 95, "bye")
                .await?;
            for (command, text) in [(96, "hi"), (97, "measurements"), (98, "yes")] {
                super::classic_quests_fixtures::talk(&mut owner, &deps.admitted, olrik, command, text)
                    .await?;
            }
            let wager: (i64, i64, i64) = sqlx::query_as(
                "SELECT fee_gold_units,burned_gold_units,bank_debit_gold_units
                   FROM game_item_fee_burns
                  WHERE purchase_service_key='oteryn:quest-npc/postman/olrik/roll'",
            )
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(wager, (5, 2, 3));
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })?;
    eprintln!("POSTGRES_CASE_COMPLETED postman_kevin_tour_briefing_and_bones");
    Ok(())
}

#[test]
fn crystal_kevin_postgres_resumes_first_promotion_after_regreeting() -> TestResult {
    use super::classic_quests_fixtures::{install, progress, talk};
    use super::qengine_outfit_stories_fixtures::{
        seed_production_progression_context, seed_quest_prerequisites,
    };

    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("postman-promotion", || {
        run(async |admin| {
            reset();
            let h = Harness::create_with_admission_seed(admin, "postman_promotion", true, false)
                .await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
            let (controller, pin) = activated_with_loot_tables(world, None);
            let deps = Dependencies::with_content(
                ChannelContentPin::from_activation(
                    world,
                    pin.activation_sequence(),
                    pin.identity().server_artifact_digest(),
                    pin.identity().client_artifact_digest(),
                    pin.frame_binding().digest(),
                    pin.qualified_room().map_revision_digest(),
                    (2, 0, 7),
                ),
                pin.qualified_room().clone(),
                crate::spell::cast::v1_spell_book().map_err(debug)?,
            )?;
            publish_item_content(&h, &deps.admitted, &pin).await?;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(controller.active().ok_or("active content")?);
            owner.quest_catalogue = Some(Arc::new(
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?
                    .catalogue()
                    .clone(),
            ));
            deps.initialize(&owner).await;
            let mut admitted = deps.admitted;
            seed_production_progression_context(&h, &owner, admitted.game_session_id).await?;
            owner.admit_progression_session(&mut admitted).await;
            let admitted = &admitted;
            deps.runtime
                .lock()
                .await
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            let [kevin] = install(&owner, admitted, ["oteryn:npc.kevin"]).await?;
            let mission01 = "oteryn:quest-progress/quest/u7_24/the_postman_missions/mission01";
            let mission02 = "oteryn:quest-progress/quest/u7_24/the_postman_missions/mission02";
            let rank = "oteryn:quest-progress/quest/u7_24/the_postman_missions/rank";
            seed_quest_prerequisites(
                &h,
                &owner,
                admitted,
                &[(mission01.into(), 6), (mission02.into(), 2)],
                91,
            )
            .await?;
            owner.refresh_quest_session(admitted.game_session_id, true).await;

            talk(&mut owner, admitted, kevin, 1, "hi").await?;
            let offer = talk(&mut owner, admitted, kevin, 2, "mission").await?;
            assert_eq!(offer, ["The mailbox is working again. Are you ready for your next assignment?"]);
            assert_eq!(progress(&owner, admitted, &authority, mission02).await?, 3);
            assert_eq!(progress(&owner, admitted, &authority, rank).await?, 0);

            talk(&mut owner, admitted, kevin, 3, "hi").await?;
            talk(&mut owner, admitted, kevin, 4, "yes").await?;
            assert_eq!(progress(&owner, admitted, &authority, rank).await?, 0);
            assert_eq!(talk(&mut owner, admitted, kevin, 5, "mission").await?, offer);
            assert_eq!(progress(&owner, admitted, &authority, rank).await?, 0);
            assert_eq!(
                talk(&mut owner, admitted, kevin, 6, "yes").await?,
                ["You are now an Assistant Postofficer. Ready to deliver a bill?"]
            );
            assert_eq!(progress(&owner, admitted, &authority, rank).await?, 1);
            assert_eq!(progress(&owner, admitted, &authority, mission02).await?, 3);

            super::item_look_fixtures::seed_main_backpack(&h, &[]).await?;
            talk(&mut owner, admitted, kevin, 7, "hi").await?;
            assert_eq!(
                talk(&mut owner, admitted, kevin, 8, "mission").await?,
                ["Are you ready for another mission?"]
            );
            talk(&mut owner, admitted, kevin, 9, "yes").await?;
            let mission03 = "oteryn:quest-progress/quest/u7_24/the_postman_missions/mission03";
            assert_eq!(progress(&owner, admitted, &authority, mission03).await?, 1);
            assert_eq!(super::questpc7_outfit_items_fixtures::quantity(&h, 3216).await?, 1);

            const POSTMAN: &str = "oteryn:quest.the_postman_missions_quest";
            let mission09 = "oteryn:quest-progress/quest/u7_24/the_postman_missions/mission09";
            let mission10 = "oteryn:quest-progress/quest/u7_24/the_postman_missions/mission10";
            seed_quest_prerequisites(&h, &owner, admitted, &[(mission09.into(), 3)], 92).await?;
            owner.refresh_quest_session(admitted.game_session_id, true).await;
            talk(&mut owner, admitted, kevin, 10, "mission").await?;
            talk(&mut owner, admitted, kevin, 11, "yes").await?;
            assert_eq!(progress(&owner, admitted, &authority, mission10).await?, 1);
            assert_eq!(super::questpc7_outfit_items_fixtures::quantity(&h, 3220).await?, 1);

            seed_quest_prerequisites(
                &h, &owner, admitted, &[(mission10.into(), 3), (rank.into(), 4)], 93,
            ).await?;
            let mut predecessor = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *predecessor).await?;
            sqlx::query(
                "UPDATE game_character_quest_states SET definition_hash=decode(
                   '0af6fe65e65acefeb85edaee7d134251217cb9b5ae12e92c95b374d38ceff05d','hex')
                 WHERE quest_key=$1",
            ).bind(POSTMAN).execute(&mut *predecessor).await?;
            predecessor.commit().await?;
            owner.refresh_quest_session(admitted.game_session_id, true).await;
            talk(&mut owner, admitted, kevin, 12, "advancement").await?;
            talk(&mut owner, admitted, kevin, 13, "yes").await?;
            owner.refresh_quest_session(admitted.game_session_id, true).await;
            assert_eq!(progress(&owner, admitted, &authority, rank).await?, 5);
            let completed: bool = sqlx::query_scalar(
                "SELECT q.completed_character_revision IS NOT NULL AND EXISTS (
                    SELECT 1 FROM game_character_quest_receipts r
                    WHERE r.character_id=q.character_id AND r.quest_key=q.quest_key AND r.completes
                    AND r.transition_key='oteryn:quest-npc/postman/kevin/rank_5')
                 FROM game_character_quest_states q WHERE q.quest_key=$1",
            ).bind(POSTMAN).fetch_one(&h.pool).await?;
            assert!(completed, "the final promotion leaves durable completion and its receipt");
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}

#[test]
fn postman_olrik_postgres_only_uses_bank_after_type2_activation() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("postman-olrik", || {
        run(async |admin| {
            reset();
            let h = Harness::create_with_admission_seed(admin, "postman_olrik", true, false).await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
            let (controller, pin) = activated_with_loot_tables(world, None);
            let deps = Dependencies::with_content(
                ChannelContentPin::from_activation(
                    world,
                    pin.activation_sequence(),
                    pin.identity().server_artifact_digest(),
                    pin.identity().client_artifact_digest(),
                    pin.frame_binding().digest(),
                    pin.qualified_room().map_revision_digest(),
                    (2, 0, 7),
                ),
                pin.qualified_room().clone(),
                crate::spell::cast::v1_spell_book().map_err(debug)?,
            )?;
            // Crystal olrik.lua:69-72 charges the wager from carried coins and bank funds.
            publish_item_content(&h, &deps.admitted, &pin).await?;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(controller.active().ok_or("active content")?);
            owner.quest_catalogue = Some(Arc::new(
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?
                    .catalogue()
                    .clone(),
            ));
            deps.initialize(&owner).await;
            deps.runtime
                .lock()
                .await
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            let mut admitted = deps.admitted;
            super::qengine_outfit_stories_fixtures::seed_production_progression_context(
                &h,
                &owner,
                admitted.game_session_id,
            )
            .await?;
            owner.admit_progression_session(&mut admitted).await;
            assert!(owner.progression_binding(admitted.game_session_id).is_some());
            let [olrik] = super::classic_quests_fixtures::install(
                &owner,
                &admitted,
                ["oteryn:npc.olrik"],
            )
            .await?;
            super::item_look_fixtures::seed_main_backpack(
                &h,
                &[("oteryn:item.tibia.i3031", 2)],
            )
            .await?;
            let mission07 = "oteryn:quest-progress/quest/u7_24/the_postman_missions/mission07";
            super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
                &h,
                &owner,
                &deps.admitted,
                &[(mission07.into(), 5)],
                94,
            )
            .await?;
            super::guild_bank_fixtures::bank_seed::balance(&h, id(40), id(41), 3, 180).await?;
            owner
                .refresh_quest_session(deps.admitted.game_session_id, true)
                .await;
            for (command, text) in [(96, "hi"), (97, "measurements")] {
                super::classic_quests_fixtures::talk(&mut owner, &deps.admitted, olrik, command, text)
                    .await?;
            }
            let reply =
                super::classic_quests_fixtures::talk(&mut owner, &deps.admitted, olrik, 98, "yes")
                    .await?;
            assert!(reply.is_empty(), "V1 cannot fund the wager from the bank");
            let unpaid: bool = sqlx::query_scalar(
                "SELECT NOT EXISTS (SELECT 1 FROM game_item_fee_burns
                  WHERE purchase_service_key='oteryn:quest-npc/postman/olrik/roll')",
            )
            .fetch_one(&h.pool)
            .await?;
            assert!(unpaid);
            sqlx::query("INSERT INTO game_type2_audit_activation VALUES(1,now())")
                .execute(&h.pool)
                .await?;
            // The V1 refusal keeps the offer open. With V2, the same topic can use bank funds.
            super::classic_quests_fixtures::talk(&mut owner, &deps.admitted, olrik, 99, "yes").await?;
            let wager: (i64, i64, i64) = sqlx::query_as(
                "SELECT fee_gold_units,burned_gold_units,bank_debit_gold_units
                   FROM game_item_fee_burns
                  WHERE purchase_service_key='oteryn:quest-npc/postman/olrik/roll'",
            )
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(wager, (5, 2, 3));
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })?;
    eprintln!("POSTGRES_CASE_COMPLETED postman_olrik_type2_activation");
    Ok(())
}
