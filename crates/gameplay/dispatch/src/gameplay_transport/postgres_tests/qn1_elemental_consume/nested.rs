//! Genuine atomic kill-MINT producer: a bag with coins and a fluid child.
use super::{SpellLanePermit, f};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::item_mint::{
    CorpseContainerPlacement, CorpseLootMintRequest, ItemMintCandidate, ItemMintCause,
    ItemMintOutcome,
};
use crate::foundation::{ChannelId, ScopeOwnershipGeneration, WorldId};

pub(super) async fn mint(
    h: &f::Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    body: &mut ItemMintCandidate,
) -> f::TestResult<Vec<[u8; 16]>> {
    let mut parent = *body.item_instance_id();
    let mut children = Vec::new();
    let mut ids = Vec::new();
    for (ordinal, key, quantity, fluid) in [
        (1, "oteryn:item.tibia.i2853", 1, None),
        (2, "oteryn:item.tibia.i3031", 60, None),
        (3, "oteryn:item.tibia.i2874", 1, Some(1)),
    ] {
        let child = h
            .root
            .freeze_kill_loot_entry(
                authority,
                &h.node,
                CorpseLootMintRequest {
                    cause: ItemMintCause::for_test(
                        WorldId::decode(&f::id(f::WORLD)).map_err(f::debug)?,
                        ChannelId::decode(&f::id(f::CHANNEL)).map_err(f::debug)?,
                        ScopeOwnershipGeneration::new(1).map_err(f::debug)?,
                        2,
                        1,
                        f::definition("LootTable", "fixture:elemental"),
                        "fixture:elemental-nested".into(),
                        ordinal,
                    ),
                    item: f::definition("Item", key),
                    quantity,
                    placement: CorpseContainerPlacement {
                        corpse_item_instance_id: parent,
                        placement_ordinal: ordinal,
                        fluid_subtype: fluid,
                    },
                    content_revision: "content-1".into(),
                    ruleset_revision: "ruleset-1".into(),
                    sim_revision: "sim-1".into(),
                },
            )
            .await
            .map_err(f::debug)?;
        ids.push(*child.item_instance_id());
        if ordinal == 1 {
            parent = *child.item_instance_id();
        }
        children.push(child);
    }
    let permit = SpellLanePermit::of_fresh_scope(f::scope()?)
        .await
        .map_err(f::debug)?;
    let outcomes = h
        .root
        .commit_kill_loot_mints(
            &permit,
            authority,
            &h.node,
            body,
            &mut children,
            f::id(f::CHARACTER),
            None,
        )
        .await
        .map_err(f::debug)?;
    assert_eq!(outcomes.len(), 4);
    assert!(
        outcomes
            .iter()
            .all(|outcome| matches!(outcome, ItemMintOutcome::Committed(_)))
    );
    let nested: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_nested_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid")
        .bind(ids[0].as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        nested, 2,
        "must exercise the actual nested kill-MINT producer"
    );
    eprintln!("QN1_ELEMENTAL_NESTED_LOOT_PG_READY");
    Ok(ids)
}
