use super::*;
use crate::bestiary_postgres_harness as h;
use crate::durability::item_mint::{
    GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
};
use crate::foundation::{ChannelId, ScopeOwnershipGeneration, WorldId};
use oteryn_gameplay_quests::quest::world_callbacks::{GroundStackItem, Position};

const CRATE: &str = "oteryn:item.m02.fixture_crate";
const URN: &str = "oteryn:item.m02.fixture_urn";

#[test]
fn m02_ground_stack_postgres_exact_custody_order_quantity_and_no_consumption() -> h::TestResult {
    let Some(admin) = h::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            h::runtime()
                .map_err(h::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let harness =
                        h::Harness::create(admin, "srvwld_m02_ground_stack", true).await?;
                    let result = exercise(&harness).await;
                    harness.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "SRVWLD_PG_EXECUTED m02_ground_stack duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(h::debug)
        })?
        .join()
        .map_err(|_| "M02 Ground stack test thread failed")?
        .map_err(Into::into)
}

fn context() -> QuestWorldContext {
    QuestWorldContext {
        map_revision: "map-1".into(),
        content_revision: "content-1".into(),
        placement_context: h::id(6).to_vec(),
        reset_epoch: 0,
        actor_placement: h::id(7),
        movement_revision: 1,
    }
}
fn position() -> Position {
    Position {
        x: 120,
        y: 5,
        floor: -1,
    }
}

async fn mint(harness: &h::Harness, actor: u32, key: &str, quantity: u32) -> h::TestResult {
    mint_at(harness, actor, key, quantity, position(), &context()).await
}
async fn mint_at(
    harness: &h::Harness,
    actor: u32,
    key: &str,
    quantity: u32,
    at: Position,
    context: &QuestWorldContext,
) -> h::TestResult {
    let seal = harness.recovery.seal_current().map_err(h::debug)?;
    let recovery = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(h::debug)?;
    let mut candidate = harness
        .root
        .freeze_item_mint(
            &recovery,
            &harness.node,
            ItemMintRequest {
                cause: ItemMintCause::for_test(
                    WorldId::decode(&h::id(h::WORLD)).map_err(h::debug)?,
                    ChannelId::decode(&h::id(h::CHANNEL)).map_err(h::debug)?,
                    ScopeOwnershipGeneration::new(1).map_err(h::debug)?,
                    actor,
                    1,
                    TypedDefinitionRef {
                        family: "LootTable".into(),
                        production_key: "fixture:ground.loot".into(),
                        revision_ref: "definition-r1".into(),
                    },
                    "fixture:m02.drop".into(),
                    0,
                ),
                item: TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: key.into(),
                    revision_ref: "definition-r1".into(),
                },
                quantity,
                ground: GroundPlacement {
                    spatial_position: super::spatial(at),
                    corpse_ref: h::id(3).to_vec(),
                    map_revision: context.map_revision.clone(),
                    content_revision: context.content_revision.clone(),
                    native_room_placement_context: context.placement_context.clone(),
                },
                content_revision: context.content_revision.clone(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
        )
        .await
        .map_err(h::debug)?;
    let permit = candidate.fresh_death_lane_permit().await;
    assert!(matches!(
        harness
            .root
            .commit_item_mint(&permit, &recovery, &harness.node, &mut candidate)
            .await
            .map_err(h::debug)?,
        ItemMintOutcome::Committed(_)
    ));
    Ok(())
}
async fn check(
    harness: &h::Harness,
    requirement: WorldRequirement,
    context: QuestWorldContext,
    generation: u64,
) -> h::TestResult<bool> {
    let mut tx = harness.pool.begin().await?;
    let permit = SpellLanePermit::of_fresh_scope(h::scope()?)
        .await
        .map_err(h::debug)?;
    let mut fence = h::fence(1)?;
    fence.scope_ownership_generation =
        ScopeOwnershipGeneration::new(generation).map_err(h::debug)?;
    let result = matches_requirements(&mut tx, &permit, fence, &context, &[requirement]).await?;
    tx.rollback().await?;
    Ok(result)
}
async fn exercise(harness: &h::Harness) -> h::TestResult {
    mint(harness, 1001, CRATE, 1).await?;
    mint(harness, 1002, CRATE, 1).await?;
    mint(harness, 1003, URN, 2).await?;
    let requirement = WorldRequirement {
        position: position(),
        appearance: 0,
        ground_stack: Some(vec![
            GroundStackItem {
                item: CRATE.into(),
                count: 1,
            },
            GroundStackItem {
                item: CRATE.into(),
                count: 1,
            },
            GroundStackItem {
                item: URN.into(),
                count: 2,
            },
        ]),
    };
    let before: Vec<(String, i64)> = sqlx::query_as(
        "SELECT item_instance_id::text,quantity FROM game_item_instances ORDER BY item_instance_id",
    )
    .fetch_all(&harness.pool)
    .await?;
    assert!(check(harness, requirement.clone(), context(), 1).await?);
    let mut wrong_order = requirement.clone();
    wrong_order.ground_stack.as_mut().ok_or("stack")?.swap(0, 2);
    assert!(!check(harness, wrong_order, context(), 1).await?);
    let mut wrong_quantity = requirement.clone();
    wrong_quantity.ground_stack.as_mut().ok_or("stack")?[2].count = 1;
    assert!(!check(harness, wrong_quantity, context(), 1).await?);
    let mut wrong_position = requirement.clone();
    wrong_position.position.x += 1;
    assert!(!check(harness, wrong_position, context(), 1).await?);
    let mut old = context();
    old.map_revision = "stale-map".into();
    assert!(!check(harness, requirement.clone(), old, 1).await?);
    assert!(!check(harness, requirement.clone(), context(), 2).await?);
    assert_eq!(before, sqlx::query_as::<_,(String,i64)>("SELECT item_instance_id::text,quantity FROM game_item_instances ORDER BY item_instance_id").fetch_all(&harness.pool).await?);
    mint(harness, 1004, CRATE, 1).await?;
    assert!(
        !check(harness, requirement, context(), 1).await?,
        "extra root must not be ignored"
    );
    Ok(())
}

mod stateless;
