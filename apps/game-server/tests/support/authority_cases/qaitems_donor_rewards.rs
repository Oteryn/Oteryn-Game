//! Crystal 201189f8 chest.lua:1182 grants the original Giant Smithhammer.
use super::engitem_reward_text::{Weights, command, definition, equip_backpack};
use crate::bestiary_postgres_harness as f;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::reward_claim_mint::{RewardClaimMintOutcome, RewardClaimMintRequest};

const CLAIM: &str = "oteryn:reward-claim.quest.u7_6.explorer_society.giant_smith_hammer.hammer";
const ITEM: &str = "oteryn:item.tibia.i3208";

fn hammer_claim() -> f::TestResult<serde_json::Value> {
    let shard: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../content/interactions/reward_claims/reward-claims-00100-00199.json"
    ))?;
    shard["records"]
        .as_array()
        .ok_or("claim records")?
        .iter()
        .find(|row| row["definition"]["identity"]["key"] == CLAIM)
        .map(|row| row["definition"].clone())
        .ok_or("hammer claim".into())
}

#[test]
fn qaitems_hammer_content_is_materializable_and_grants_donor_id() -> f::TestResult {
    let shard: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../content/items/definitions/items-19000-19499.json"
    ))?;
    let item = shard["records"]
        .as_array()
        .ok_or("item records")?
        .iter()
        .find(|row| row["definition"]["identity"]["key"] == ITEM)
        .ok_or("hammer item")?;
    assert_eq!(item["definition"]["materializable"], true);
    assert_eq!(
        hammer_claim()?["placements"][0]["reward"]["items"][0]["item"]["key"],
        ITEM
    );
    Ok(())
}

#[test]
fn qaitems_hammer_claim_materializes_donor_item_in_inventory() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let h = f::Harness::create(admin, "qaitems_hammer", true).await?;
                    let result = exercise(&h).await;
                    let cleanup = h.cleanup().await;
                    result?;
                    cleanup?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "hammer PG thread failed")?
        .map_err(Into::into)
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    use oteryn_gameplay_items::item_ref_admission::generation_item_facts;
    let claim = hammer_claim()?;
    let reward = &claim["placements"][0]["reward"]["items"][0];
    let item = TypedDefinitionRef {
        family: reward["item"]["family"]
            .as_str()
            .ok_or("item family")?
            .into(),
        production_key: reward["item"]["key"].as_str().ok_or("item key")?.into(),
        revision_ref: reward["item"]["revision"]
            .as_str()
            .ok_or("item revision")?
            .into(),
    };
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
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    equip_backpack(h, &authority, native, &weights).await?;
    let request = RewardClaimMintRequest {
        backpack_outputs: false,
        root_charges_remaining: None,
        root_fluid_subtype: None,
        command: command(9600)?,
        claim: TypedDefinitionRef {
            family: "RewardClaim".into(),
            production_key: CLAIM.into(),
            revision_ref: "reward-claim-r1".into(),
        },
        source_placement: format!(
            "oteryn:placement/{}/0",
            CLAIM.strip_prefix("oteryn:").ok_or("namespace")?
        ),
        item: generation_item_facts(native, &item).map_err(f::debug)?,
        quantity: reward["count"].as_u64().ok_or("quantity")?.try_into()?,
        contents: Vec::new(),
        additional_items: Vec::new(),
        root_text: None,
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
        key_number: None,
    };
    let fence = f::item_fence()?;
    let mut candidate = h
        .root
        .freeze_reward_claim_mint_weighted(&authority, &h.node, fence, request, &weights)
        .await
        .map_err(f::debug)?;
    let (outcome, _) = h
        .root
        .commit_reward_claim_mint_weighted_noticed(
            &authority,
            &h.node,
            fence,
            &mut candidate,
            None,
            &weights,
        )
        .await
        .map_err(f::debug)?;
    assert!(matches!(outcome, RewardClaimMintOutcome::Committed(_)));
    let items: Vec<String> = sqlx::query_scalar("SELECT i.definition_production_key FROM game_item_instances i JOIN game_reward_claim_mint_receipts m USING(item_instance_id) JOIN game_item_container_entries e USING(item_instance_id) WHERE m.claim_production_key=$1 AND e.character_id=m.character_id")
        .bind(CLAIM).fetch_all(&h.pool).await?;
    assert_eq!(items, vec![ITEM]);
    eprintln!("QAITEMS_POSTGRES_EXECUTED hammer inventory={items:?}");
    Ok(())
}
