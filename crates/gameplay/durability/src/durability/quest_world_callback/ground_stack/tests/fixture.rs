//! Shared native Ground custody setup for the library and focused window PG target.
use super::*;

pub(super) fn context() -> QuestWorldContext {
    QuestWorldContext {
        map_revision: "map-1".into(),
        content_revision: "content-1".into(),
        placement_context: h::id(6).to_vec(),
        reset_epoch: 0,
        actor_placement: h::id(7),
        movement_revision: 1,
        party: Vec::new(),
    }
}

pub(super) async fn mint_at(
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
