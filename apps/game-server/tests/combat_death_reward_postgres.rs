#![allow(clippy::expect_used)]
// Dedicated PostgreSQL 18 qualification for Combat D2b (creature death ->
// loot MINT + R7 P03 XP composition). Ordinary workspace runs report
// PRE-ROUTING/NONCANONICAL when the routed database is absent.
extern crate oteryn_game_server as production_server;
extern crate self as oteryn_game_server;

pub use production_server::admission_evidence;
#[allow(dead_code, unused_imports)]
#[path = "../../../crates/gameplay/durability/src/character_bootstrap_intent.rs"]
pub mod character_bootstrap_intent;
#[allow(dead_code, unused_imports)]
pub use oteryn_gameplay_character_recovery_fence as character_recovery_fence;
#[allow(dead_code, unused_imports)]
#[path = "../../../crates/gameplay/combat/src/combat/mod.rs"]
pub mod combat;
#[path = "support/domain_shim.rs"]
pub mod domain;
use oteryn_gameplay_durability::durability;
#[allow(dead_code, unused_imports)]
pub use oteryn_gameplay_entities as foundation;
#[allow(dead_code, unused_imports)]
pub use oteryn_gameplay_native_admission_source as native_admission_source;

// Shared PostgreSQL harness for this target's integration cases.
#[allow(dead_code)]
#[path = "support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;

#[path = "support/attack_kill_reward_postgres_cases.rs"]
mod attack_kill_reward_postgres_cases;
#[path = "support/combat_death_reward_postgres_cases.rs"]
mod combat_death_reward_postgres_cases;

#[allow(dead_code, unused_imports)]
#[path = "support/content_spell_data_shim.rs"]
mod content;

mod atomic_loot {
    use super::combat_death_reward_postgres_cases::{
        Harness, TestResult, capture, configured_admin, death_fixture, debug, definition, id,
        replace_scope_owner, runtime,
    };
    use crate::durability::item_mint::{
        CORPSE_MATERIALIZATION_PURPOSE_KEY, CorpseContainerPlacement, CorpseLootMintRequest,
        GroundPlacement, ItemMintCause, ItemMintError, ItemMintRequest,
    };
    use crate::foundation::CombatDeathFixture;

    #[test]
    fn a_stale_atomic_loot_batch_creates_no_items_or_mint_work() -> TestResult {
        let Some(admin) = configured_admin() else {
            return Ok(());
        };
        runtime()?.block_on(async move {
            let harness = Harness::create(admin, "atomic_loot_fence").await?;
            let seal = harness.recovery.seal_current().map_err(debug)?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let (mut corpse, mut children) = freeze_atomic_loot(&harness, &authority).await?;
            replace_scope_owner(&harness).await?;
            let result = harness
                .root
                .commit_kill_loot_mints(
                    &corpse.fresh_death_lane_permit().await,
                    &authority,
                    &harness.node,
                    &mut corpse,
                    &mut children,
                    id(41),
                    None,
                    None,
                )
                .await;
            eprintln!("ATOMIC_LOOT_GENERATION_FENCE_PG_EXECUTED");
            assert!(matches!(result, Err(ItemMintError::AuthorityRejected)));
            assert_eq!(harness.count("game_item_instances").await?, 0);
            assert_eq!(harness.count("game_item_mint_receipts").await?, 0);
            assert_eq!(harness.count("game_item_audit_outbox").await?, 0);
            assert_eq!(corpse.work_units_used(), 0);
            assert_eq!(children[0].work_units_used(), 0);
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT COALESCE(sum(work_units_used),0) FROM game_item_mint_reservations",
                )
                .fetch_one(&harness.pool)
                .await?,
                0,
            );
            drop(authority);
            drop(seal);
            harness.cleanup().await
        })
    }

    #[test]
    fn an_atomic_loot_receipt_replays_at_the_work_limit() -> TestResult {
        use crate::durability::item_mint::ItemMintOutcome;
        let Some(admin) = configured_admin() else {
            return Ok(());
        };
        runtime()?.block_on(async move {
            let harness = Harness::create(admin, "atomic_loot_replay").await?;
            let seal = harness.recovery.seal_current().map_err(debug)?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let (mut corpse, mut children) = freeze_atomic_loot(&harness, &authority).await?;
            let permit = corpse.fresh_death_lane_permit().await;
            sqlx::raw_sql(
                "CREATE FUNCTION fail_atomic_child() RETURNS trigger LANGUAGE plpgsql AS $$
                BEGIN RAISE EXCEPTION 'atomic loot child fault'; END $$;
                CREATE TRIGGER fail_atomic_child BEFORE INSERT ON game_item_corpse_container_entries
                FOR EACH ROW EXECUTE FUNCTION fail_atomic_child();",
            )
            .execute(&harness.pool)
            .await?;
            let limit = crate::durability::item_mint_audit::RL08_RETRY_WORK_UNITS_MAX;
            for used in 1..limit {
                let result = harness
                    .root
                    .commit_kill_loot_mints(
                        &permit,
                        &authority,
                        &harness.node,
                        &mut corpse,
                        &mut children,
                        id(41),
                        None,
                        None,
                    )
                    .await;
                assert!(matches!(result, Err(ItemMintError::Unavailable(_))));
                assert_eq!(corpse.work_units_used(), used);
                assert_eq!(children[0].work_units_used(), used);
                assert_eq!(harness.count("game_item_mint_receipts").await?, 0);
                harness.root.maintain_ready_once().await.map_err(debug)?;
            }
            sqlx::query("DROP TRIGGER fail_atomic_child ON game_item_corpse_container_entries")
                .execute(&harness.pool)
                .await?;
            let first = harness
                .root
                .commit_kill_loot_mints(
                    &permit,
                    &authority,
                    &harness.node,
                    &mut corpse,
                    &mut children,
                    id(41),
                    None,
                    None,
                )
                .await?;
            assert_eq!(first.len(), 2);
            assert_eq!(corpse.work_units_used(), limit);
            assert_eq!(children[0].work_units_used(), limit);
            let replay = harness
                .root
                .commit_kill_loot_mints(
                    &permit,
                    &authority,
                    &harness.node,
                    &mut corpse,
                    &mut children,
                    id(41),
                    None,
                    None,
                )
                .await;
            eprintln!("ATOMIC_LOOT_EXHAUSTED_REPLAY_PG_EXECUTED");
            assert_eq!(
                replay?,
                first
                    .into_iter()
                    .map(|mint| match mint {
                        ItemMintOutcome::Committed(receipt)
                        | ItemMintOutcome::AlreadyCommitted(receipt) =>
                            ItemMintOutcome::AlreadyCommitted(receipt),
                    })
                    .collect::<Vec<_>>()
            );
            assert_eq!(harness.count("game_item_mint_receipts").await?, 2);
            assert_eq!(corpse.work_units_used(), limit);
            assert_eq!(children[0].work_units_used(), limit);
            drop(authority);
            drop(seal);
            harness.cleanup().await
        })
    }

    async fn freeze_atomic_loot(
        harness: &Harness,
        authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    ) -> TestResult<(
        crate::durability::item_mint::ItemMintCandidate,
        [crate::durability::item_mint::ItemMintCandidate; 1],
    )> {
        let mut fixture = death_fixture()?;
        fixture
            .strike("fixture:reward.strike.lethal", CombatDeathFixture::HEALTH)
            .map_err(debug)?;
        fixture.project_death().map_err(debug)?;
        let actor = fixture.actor();
        let death = capture(&mut fixture, actor)?.death;
        let body = definition("ItemType", "fixture:atomic.corpse", "corpse-r1");
        let corpse = harness
            .root
            .freeze_item_mint(
                authority,
                &harness.node,
                ItemMintRequest {
                    cause: ItemMintCause::from_creature_death(
                        death,
                        body.clone(),
                        CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
                        0,
                    )
                    .map_err(debug)?,
                    item: body,
                    quantity: 1,
                    ground: GroundPlacement {
                        spatial_position: vec![0, 0, 0, 9, 0, 0, 0, 9, 0, 0],
                        corpse_ref: id(3).to_vec(),
                        map_revision: "map-1".into(),
                        content_revision: "content-1".into(),
                        native_room_placement_context: id(6).to_vec(),
                    },
                    content_revision: "content-1".into(),
                    ruleset_revision: "ruleset-1".into(),
                    sim_revision: "sim-1".into(),
                },
            )
            .await?;
        let child = harness
            .root
            .freeze_kill_loot_entry(
                authority,
                &harness.node,
                CorpseLootMintRequest {
                    cause: ItemMintCause::from_creature_death(
                        death,
                        definition("LootTable", "fixture:atomic.loot", "loot-r1"),
                        "fixture:atomic.child".into(),
                        1,
                    )
                    .map_err(debug)?,
                    item: definition("ItemType", "fixture:atomic.child", "item-r1"),
                    quantity: 1,
                    placement: CorpseContainerPlacement {
                        corpse_item_instance_id: *corpse.item_instance_id(),
                        placement_ordinal: 1,
                        fluid_subtype: None,
                    },
                    content_revision: "content-1".into(),
                    ruleset_revision: "ruleset-1".into(),
                    sim_revision: "sim-1".into(),
                },
            )
            .await?;
        Ok((corpse, [child]))
    }
}
