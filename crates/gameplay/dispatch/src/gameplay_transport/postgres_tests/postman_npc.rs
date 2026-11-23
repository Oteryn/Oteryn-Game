//! Kevin's focused conversation advances the production Postman track.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::foundation::ChannelContentPin;
use std::sync::Arc;

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
            let [kevin] = super::classic_quests_fixtures::install_with_offsets(
                &owner,
                &deps.admitted,
                ["oteryn:npc.kevin"],
                Some([(1, 0)]),
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
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })?;
    eprintln!("POSTGRES_CASE_COMPLETED postman_kevin_tour_briefing_and_bones");
    Ok(())
}
