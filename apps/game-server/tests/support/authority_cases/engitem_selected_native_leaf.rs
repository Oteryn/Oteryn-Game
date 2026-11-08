//! A selected source leaf uses its original chest claim, never a second reward ledger.
use crate::bestiary_postgres_harness as f;
use crate::durability::item_mint::{
    GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
};
use crate::durability::item_transfer::{
    ItemTransferDestination, ItemTransferOutcome, ItemTransferRequest, ItemWeightLookup,
    ItemWeightRefused,
};
use crate::durability::reward_claim_mint::{
    RewardClaimMintError, RewardClaimMintOutcome, RewardClaimMintRequest, RewardClaimRefusal,
};
use crate::foundation::{
    ChannelId, CommandId, CommandRef, GameSessionId, ScopeOwnershipGeneration, WorldId,
};
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_content::content::world_reward_claims::{
    self as claims, BoundCell, RewardClaimBase, RewardClaimTile, UniqueEntry,
};
use oteryn_gameplay_items::item_ref_admission::generation_item_facts;

#[test]
fn engitem_selected_native_leaf_mint_refusal_replay_and_once_postgres() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = f::Harness::create(admin, "engitem_selected_leaf", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    eprintln!(
                        "ENGITEM_PG_EXECUTED selected_native_leaf duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "selected leaf PG thread failed")?
        .map_err(Into::into)
}
struct Weights<'a>(&'a NativeGameplayState);
impl ItemWeightLookup for Weights<'_> {
    fn generation_digest(&self) -> [u8; 32] {
        self.0.source_digest()
    }
    fn weight_of(&self, d: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        generation_item_facts(self.0, d)
            .map_err(|_| ItemWeightRefused::DefinitionMismatch)?
            .weight
            .ok_or(ItemWeightRefused::Unknown)
    }
}
fn definition(id: u32) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: format!("oteryn:item.tibia.i{id}"),
        revision_ref: "definition-r1".into(),
    }
}
fn command(id: u64) -> f::TestResult<CommandRef> {
    Ok(CommandRef::new(
        GameSessionId::decode(&f::id(f::SESSION)).map_err(f::debug)?,
        CommandId::new(id).map_err(f::debug)?,
    ))
}
#[derive(Clone, Copy)]
struct Tile {
    appearance: u16,
}
impl RewardClaimTile for Tile {
    fn depths(&self) -> &[u8] {
        &[0]
    }
    fn unique(&self, ordinal: u8) -> Option<UniqueEntry> {
        (ordinal == 0).then_some(UniqueEntry {
            unique: 6063,
            appearance: Some(self.appearance),
        })
    }
}
struct Base {
    cell: BoundCell,
    tile: Tile,
}
impl RewardClaimBase for Base {
    type Tile<'a> = Tile;
    fn tile(&self, x: u16, y: u16, floor: i8) -> Option<Tile> {
        (self.cell == BoundCell { x, y, floor }).then_some(self.tile)
    }
}
fn selected_claim(native: &NativeGameplayState) -> f::TestResult<claims::ServedClaim> {
    let mut definition = None;
    for shard in claims::REWARD_CLAIM_SHARDS {
        let doc: serde_json::Value = serde_json::from_str(shard)?;
        if let Some(row) = doc["records"]
            .as_array()
            .ok_or("records")?
            .iter()
            .find(|r| {
                r["definition"]["identity"]["key"].as_str()
                    == Some("oteryn:reward-claim.kv.ornamentedshield")
            })
        {
            definition = Some(row["definition"].clone());
            break;
        }
    }
    let mut d = definition.ok_or("qualified source fixture")?;
    let recipe = &mut d["native_contents"]["placements"][0];
    assert_eq!(recipe["unique_id"], 6063);
    assert_eq!(recipe["contents"].as_array().ok_or("children")?.len(), 5);
    let child = recipe["contents"][3].clone();
    recipe["selected_leaf"] = serde_json::json!({"source_unique_id":6063,"path":[3],"item":child["item"],"count":child["count"]});
    let p = &d["placements"][0];
    let pos = &p["source_binding"]["project_position"];
    let base = Base {
        cell: BoundCell {
            x: u16::try_from(pos["x"].as_u64().ok_or("x")?)?,
            y: u16::try_from(pos["y"].as_u64().ok_or("y")?)?,
            floor: i8::try_from(-pos["z"].as_i64().ok_or("z")?)?,
        },
        tile: Tile {
            appearance: u16::try_from(p["appearance_tibia_id"].as_u64().ok_or("appearance")?)?,
        },
    };
    let shard = serde_json::json!({"records":[{"definition":d}]}).to_string();
    let report = claims::bind_reward_claims(&base, &[&shard], &|item| {
        generation_item_facts(
            native,
            &TypedDefinitionRef {
                family: "Item".into(),
                production_key: item.key().as_str().into(),
                revision_ref: "definition-r1".into(),
            },
        )
        .is_ok()
    })?;
    if !report.left_out.is_empty() || !report.unbound.is_empty() {
        return Err(format!("selected source refused: {report:?}").into());
    }
    report
        .served
        .into_iter()
        .next()
        .ok_or_else(|| "selected source not served".into())
}
async fn exercise(h: &f::Harness) -> f::TestResult {
    let (controller, _) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            f::scope()?.world_id(),
            None,
        );
    let native = controller
        .active()
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let weights = Weights(native);
    let source = selected_claim(native)?;
    let placement = source.placements.first().ok_or("placement")?;
    assert!(placement.contents.is_empty());
    assert_eq!(placement.placement.items.len(), 1);
    let leaf = &placement.placement.items[0];
    assert_eq!(leaf.item.key().as_str(), "oteryn:item.tibia.i3035");
    assert_eq!(leaf.count, 5);
    let request = RewardClaimMintRequest {
        command: command(9500)?,
        claim: TypedDefinitionRef {
            family: "RewardClaim".into(),
            production_key: source.key.as_str().into(),
            revision_ref: source.revision.as_str().into(),
        },
        source_placement: placement.placement.placement.as_str().into(),
        item: generation_item_facts(native, &definition(3035)).map_err(f::debug)?,
        quantity: leaf.count,
        contents: vec![],
        root_text: placement.root_text.clone(),
        backpack: generation_item_facts(native, &definition(2854)).map_err(f::debug)?,
        content_revision: format!(
            "sha256:{}",
            native
                .source_digest()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        ),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
        achievement: None,
        quest_transition: None,
        key_number: placement.placement.key_number,
    };
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    let item_fence = f::item_fence()?;
    assert!(matches!(
        h.root
            .freeze_reward_claim_mint_weighted(
                &authority,
                &h.node,
                item_fence,
                request.clone(),
                &weights
            )
            .await,
        Err(RewardClaimMintError::Refused(
            RewardClaimRefusal::NoMainBackpack
        ))
    ));
    let absent:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_reward_claims),(SELECT count(*) FROM game_reward_claim_mint_reservations)").fetch_one(&h.pool).await?;
    assert_eq!(absent, (0, 0));
    let mut minted = h
        .root
        .freeze_item_mint(
            &authority,
            &h.node,
            ItemMintRequest {
                cause: ItemMintCause::for_test(
                    WorldId::decode(&f::id(f::WORLD)).map_err(f::debug)?,
                    ChannelId::decode(&f::id(f::CHANNEL)).map_err(f::debug)?,
                    ScopeOwnershipGeneration::new(1).map_err(f::debug)?,
                    1,
                    1,
                    TypedDefinitionRef {
                        family: "LootTable".into(),
                        production_key: "fixture:engitem".into(),
                        revision_ref: "definition-r1".into(),
                    },
                    "fixture:engitem".into(),
                    0,
                ),
                item: definition(2854),
                quantity: 1,
                ground: GroundPlacement {
                    spatial_position: vec![0, 0, 0, 1, 0, 0, 0, 2, 0, 3],
                    corpse_ref: f::id(3).to_vec(),
                    map_revision: "map-1".into(),
                    content_revision: "content-1".into(),
                    native_room_placement_context: f::id(6).to_vec(),
                },
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
        )
        .await
        .map_err(f::debug)?;
    let permit = minted.fresh_death_lane_permit().await;
    let ItemMintOutcome::Committed(bag) = h
        .root
        .commit_item_mint(&permit, &authority, &h.node, &mut minted)
        .await
        .map_err(f::debug)?
    else {
        return Err("bag mint".into());
    };
    let fence = f::item_fence()?;
    let mut transfer = h
        .root
        .freeze_item_transfer(
            &authority,
            &h.node,
            fence,
            ItemTransferRequest {
                weight_generation_digest: native.source_digest(),
                command: command(9300)?,
                source_item_instance_id: bag.item_instance_id,
                destination: ItemTransferDestination::ContainerSlot,
                item: generation_item_facts(native, &definition(2854)).map_err(f::debug)?,
                backpack: None,
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
            &weights,
        )
        .await
        .map_err(f::debug)?;
    let lane =
        crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(fence.runtime_scope)
            .await
            .map_err(f::debug)?;
    assert!(matches!(
        h.root
            .commit_item_transfer(&lane, &authority, &h.node, fence, &mut transfer, &weights)
            .await
            .map_err(f::debug)?,
        ItemTransferOutcome::Committed(_)
    ));
    let mut frozen = h
        .root
        .freeze_reward_claim_mint_weighted(&authority, &h.node, fence, request.clone(), &weights)
        .await
        .map_err(f::debug)?;
    let outcome = h
        .root
        .commit_reward_claim_mint(&authority, &h.node, fence, &mut frozen)
        .await
        .map_err(f::debug)?;
    let RewardClaimMintOutcome::Committed(receipt) = outcome else {
        return Err(format!("selected leaf: {outcome:?}").into());
    };
    assert_eq!(receipt.quantity, 5);
    let replay = h
        .root
        .commit_reward_claim_mint(&authority, &h.node, fence, &mut frozen)
        .await
        .map_err(f::debug)?;
    assert_eq!(replay, RewardClaimMintOutcome::AlreadyCommitted(receipt));
    let mut again = request;
    again.command = command(9501)?;
    again.claim.revision_ref = "selection-r2".into();
    assert!(matches!(
        h.root
            .freeze_reward_claim_mint_weighted(&authority, &h.node, fence, again, &weights)
            .await,
        Err(RewardClaimMintError::Refused(
            RewardClaimRefusal::AlreadyClaimed
        ))
    ));
    let counts:(i64,i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_reward_claims),(SELECT count(*) FROM game_reward_claim_mint_receipts),(SELECT count(*) FROM game_item_container_entries),(SELECT count(*) FROM game_item_nested_entries)").fetch_one(&h.pool).await?;
    assert_eq!(
        counts,
        (1, 1, 1, 0),
        "one leaf MINT under the existing source claim; no bag or sibling MINTs"
    );
    let quantity:i64=sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3035' AND lifecycle=1").fetch_one(&h.pool).await?;
    assert_eq!(quantity, 5);
    Ok(())
}
