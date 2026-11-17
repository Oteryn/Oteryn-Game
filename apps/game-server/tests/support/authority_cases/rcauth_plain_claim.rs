//! Prove the plain MINT owner persists a full container and its authored parchment.
use crate::bestiary_postgres_harness as f;
use crate::durability::item_transfer::ItemWeightLookup;
use crate::durability::reward_claim_mint::{
    RewardClaimMintContent, RewardClaimMintOutcome, RewardClaimMintRequest,
};
use oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables;
use oteryn_gameplay_items::item_ref_admission::generation_item_facts;

#[test]
fn rcauth_plain_claim_persists_filled_container_and_exact_parchment() -> f::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let h = f::Harness::create(admin, "rcauth_plain_claim", true).await?;
                    let result = claim(&h).await;
                    h.cleanup().await?;
                    result
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "plain claim PG thread failed")?
        .map_err(Into::into)
}

async fn claim(h: &f::Harness) -> f::TestResult {
    use super::engitem_reward_text::{Weights, command, definition, equip_backpack};
    const TEXT: &str = "Survey notes\nKeep the original café spelling.\n";

    let (controller, _) = activated_with_loot_tables(f::scope()?.world_id(), None);
    let native = controller
        .active()
        .and_then(|g| g.native_gameplay())
        .ok_or("native gameplay")?;
    let weights = Weights(native);
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    equip_backpack(h, &authority, native, &weights).await?;
    let fence = f::item_fence()?;
    let facts = |id| generation_item_facts(native, &definition(id)).map_err(f::debug);
    let digest = weights
        .generation_digest()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let request = RewardClaimMintRequest {
        command: command(9400)?,
        claim: crate::durability::item_mint::TypedDefinitionRef {
            family: "RewardClaim".into(),
            production_key: "oteryn:reward-claim.rcauth.filled_bag".into(),
            revision_ref: "reward-claim-r1".into(),
        },
        source_placement: "oteryn:placement/reward-claim.rcauth.filled_bag/0".into(),
        item: facts(2853)?,
        quantity: 1,
        contents: vec![
            RewardClaimMintContent {
                item: facts(2835)?,
                quantity: 1,
                text: Some(TEXT.into()),
                key_number: None,
                fluid_subtype: None,
                charges_remaining: None,
            },
            RewardClaimMintContent {
                item: facts(3031)?,
                quantity: 40,
                text: None,
                key_number: None,
                fluid_subtype: None,
                charges_remaining: None,
            },
        ],
        backpack_outputs: false,
        additional_items: Vec::new(),
        root_text: None,
        root_fluid_subtype: None,
        root_charges_remaining: None,
        backpack: facts(2854)?,
        content_revision: format!("sha256:{digest}"),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
        achievement: None,
        quest_transition: None,
        key_number: None,
    };
    let mut pending = h
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
            &mut pending,
            None,
            &weights,
        )
        .await
        .map_err(f::debug)?;
    assert!(
        matches!(outcome, RewardClaimMintOutcome::Committed(_)),
        "{outcome:?}"
    );
    let bag = h
        .root
        .read_character_backpack(&authority, fence.character_id)
        .await
        .map_err(f::debug)?
        .ok_or("character backpack")?;
    let [reward] = bag.entries.as_slice() else {
        return Err("expected exactly one reward in the backpack".into());
    };
    assert_eq!(
        reward.item.definition.production_key,
        "oteryn:item.tibia.i2853"
    );
    let children: Vec<(String, i64, Option<String>)> = sqlx::query_as(
        "SELECT i.definition_production_key,i.quantity,t.text \
         FROM game_item_nested_entries e JOIN game_item_instances i USING(item_instance_id) \
         LEFT JOIN game_item_texts t USING(item_instance_id) \
         WHERE e.parent_item_instance_id=encode($1,'hex')::uuid ORDER BY e.placement_ordinal",
    )
    .bind(reward.item.item_instance_id.as_slice())
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(
        children,
        [
            ("oteryn:item.tibia.i2835".into(), 1, Some(TEXT.into())),
            ("oteryn:item.tibia.i3031".into(), 40, None),
        ]
    );
    Ok(())
}
