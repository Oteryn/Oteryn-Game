use super::super::kill_reward_e2e_fixtures::summer_settlement::handoff;
use super::*;
use crate::foundation::{ChannelId, WorldId};

#[test]
fn summer_corpse_process_restart() -> TestResult {
    let Ok(database) = std::env::var("SUMMER_RESTART_DATABASE") else {
        return Ok(());
    };
    let parent = std::env::var("SUMMER_RESTART_PARENT")?;
    let mode = std::env::var("SUMMER_RESTART_MODE")?;
    run(async |admin| {
        let prefix = admin
            .strip_suffix("/postgres")
            .ok_or("test admin database")?;
        let url = format!("{prefix}/{database}");
        let root = crate::durability::DurabilityRoot::connect_test_runtime(&url)?;
        assert!(root.maintain_ready_once().await?);
        let retained = std::env::temp_dir()
            .join(format!("oteryn-revision-sequencer-parent-{parent}"))
            .join(&database);
        let recovery = crate::character_recovery_fence::CharacterRecoveryStore::open(
            retained,
            "character-primary",
            "game-ops",
        )
        .map_err(debug)?;
        let seal = recovery.seal_current().map_err(debug)?;
        let authority = root.open_character_authority(&seal).await.map_err(debug)?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let channel = ChannelId::decode(&id(43)).map_err(debug)?;
        let schedule = root
            .read_corpse_decay_schedule(&authority, world, channel)
            .await
            .map_err(debug)?;
        assert_eq!(schedule.corpses.len(), 1);
        let item = schedule.corpses[0].corpse_item_instance_id;
        let pool = sqlx::PgPool::connect(&url).await?;
        if mode == "transforms" || mode == "handoff-transforms" {
            let node = if mode == "handoff-transforms" {
                handoff::move_scope(&root, &pool, world, channel).await?
            } else {
                super::super::fixture::registered_node(&root, 1).await?
            };
            let generation = if mode == "handoff-transforms" { 2 } else { 1 };
            let permit = crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_lane(
                world, channel,
            )
            .await;
            assert_eq!(
                root.drain_spell_item_deadlines(
                    &permit,
                    &authority,
                    &node,
                    crate::foundation::RuntimeScopeRefV1::channel(world, channel),
                    generation
                )
                .await
                .map_err(debug)?,
                3
            );
            assert_eq!(
                root.drain_spell_item_deadlines(
                    &permit,
                    &authority,
                    &node,
                    crate::foundation::RuntimeScopeRefV1::channel(world, channel),
                    generation
                )
                .await
                .map_err(debug)?,
                0
            );
        } else if mode == "retire" {
            let generation: i32 = sqlx::query_scalar("SELECT ownership_generation::int FROM game_runtime_scope_assignments WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid")
                .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).fetch_one(&pool).await?;
            let node =
                super::super::fixture::registered_node(&root, u8::try_from(generation)?).await?;
            let permit = crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_lane(
                world, channel,
            )
            .await;
            root.retire_decayed_corpse(
                &permit,
                &authority,
                &node,
                crate::durability::item_decay_retire::CorpseDecayFence {
                    // W02 keeps this pinned Channel fixture in its exact tagged scope.
                    runtime_scope: crate::foundation::RuntimeScopeRefV1::channel(world, channel),
                    scope_ownership_generation: crate::foundation::ScopeOwnershipGeneration::new(
                        u64::try_from(generation)?,
                    )
                    .map_err(debug)?,
                },
                item,
            )
            .await
            .map_err(debug)?;
            assert!(
                root.read_corpse_decay_schedule(&authority, world, channel)
                    .await
                    .map_err(debug)?
                    .corpses
                    .is_empty()
            );
        } else {
            assert_eq!(mode, "read");
            assert_eq!(
                schedule.corpses[0]
                    .decay_at_unix_ms
                    .ok_or("timed source corpse deadline")?
                    - schedule.corpses[0].materialized_at_unix_ms,
                670_000
            );
            assert!(
                root.read_item_instance(&authority, item)
                    .await
                    .map_err(debug)?
                    .is_some()
            );
        }
        let kills: i64 = sqlx::query_scalar(
            "SELECT sum(kill_count)::bigint FROM game_character_bestiary_progress",
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(kills, 1);
        let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_xp_receipts")
            .fetch_one(&pool)
            .await?;
        assert_eq!(receipts, 1);
        pool.close().await;
        Ok(())
    })
}
