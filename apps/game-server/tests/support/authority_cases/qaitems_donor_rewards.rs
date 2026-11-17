//! Crystal 201189f8 chest.lua:1182 grants the original Giant Smithhammer.
use super::engitem_reward_text::{Weights, command, definition, equip_backpack};
use crate::bestiary_postgres_harness as f;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::reward_claim_mint::{RewardClaimMintOutcome, RewardClaimMintRequest};

#[derive(Clone, Copy)]
struct Case {
    tag: &'static str,
    claim: &'static str,
    item: &'static str,
    shard: &'static str,
}
const HAMMER: Case = Case {
    tag: "hammer",
    claim: "oteryn:reward-claim.quest.u7_6.explorer_society.giant_smith_hammer.hammer",
    item: "oteryn:item.tibia.i3208",
    shard: "items-19000-19499.json",
};
// API returned HTTP 402: owner-directed lane qa TibiaWiki revs 1148768/1115206/1114329.
const BOOTS: Case = Case {
    tag: "boots",
    claim: "oteryn:reward-claim.quest.u8_4.the_hidden_city_of_beregar.firewalker_boots",
    item: "oteryn:item.tibia.i9018",
    shard: "items-33000-33499.json",
};
const POEM: Case = Case {
    tag: "poem",
    claim: "oteryn:reward-claim.quest.u8_1.secret_service.poem_scroll",
    item: "oteryn:item.tibia.i347",
    shard: "items-21000-21499.json",
};
const LOTTERY: Case = Case {
    tag: "lottery",
    claim: "oteryn:reward-claim.quest.u8_1.secret_service.lottery_ticket",
    item: "oteryn:item.tibia.i5957",
    shard: "items-29500-29999.json",
};

fn content_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")
}
fn reward_claim(case: Case) -> f::TestResult<serde_json::Value> {
    for path in std::fs::read_dir(content_root().join("interactions/reward_claims"))? {
        let path = path?.path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let shard: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
        if let Some(row) = shard["records"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|row| row["definition"]["identity"]["key"] == case.claim)
        {
            return Ok(row["definition"].clone());
        }
    }
    Err("reward claim missing".into())
}
fn content_case(case: Case) -> f::TestResult {
    let shard: serde_json::Value = serde_json::from_slice(&std::fs::read(
        content_root().join("items/definitions").join(case.shard),
    )?)?;
    let item = shard["records"]
        .as_array()
        .ok_or("item records")?
        .iter()
        .find(|row| row["definition"]["identity"]["key"] == case.item)
        .ok_or("reward item")?;
    assert_eq!(item["definition"]["materializable"], true, "{}", case.item);
    let claim = reward_claim(case)?;
    assert_eq!(
        claim["placements"][0]["reward"]["items"][0]["item"]["key"],
        case.item
    );
    assert_eq!(claim["readiness"], "ready");
    Ok(())
}
fn postgres_case(case: Case) -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    // Qualify the content before opening the live authority harness.
                    let (controller, _) =
                        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
                            f::scope()?.world_id(),
                            None,
                        );
                    let native = controller
                        .active()
                        .and_then(|g| g.native_gameplay())
                        .ok_or("native")?;
                    let h =
                        f::Harness::create(admin, &format!("qaitems_{}", case.tag), true).await?;
                    let result = exercise(&h, case, native).await;
                    let cleanup = h.cleanup().await;
                    result?;
                    cleanup?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "reward PG thread failed")?
        .map_err(Into::into)
}
macro_rules! cases {
    ($content:ident, $pg:ident, $case:ident) => {
        #[test]
        fn $content() -> f::TestResult {
            content_case($case)
        }
        #[test]
        fn $pg() -> f::TestResult {
            postgres_case($case)
        }
    };
}
cases!(qaitems_hammer_content, qaitems_hammer_inventory, HAMMER);
cases!(qaitems_boots_content, qaitems_boots_inventory, BOOTS);
cases!(qaitems_poem_content, qaitems_poem_inventory, POEM);
cases!(qaitems_lottery_content, qaitems_lottery_inventory, LOTTERY);

async fn exercise(
    h: &f::Harness,
    case: Case,
    native: &oteryn_gameplay_content::content::native_gameplay::NativeGameplayState,
) -> f::TestResult {
    use oteryn_gameplay_items::item_ref_admission::generation_item_facts;
    let claim = reward_claim(case)?;
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
            production_key: case.claim.into(),
            revision_ref: "reward-claim-r1".into(),
        },
        source_placement: format!(
            "oteryn:placement/{}/0",
            case.claim.strip_prefix("oteryn:").ok_or("namespace")?
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
        .bind(case.claim).fetch_all(&h.pool).await?;
    assert_eq!(items, vec![case.item]);
    eprintln!("QAITEMS_POSTGRES_EXECUTED {} inventory={items:?}", case.tag);
    Ok(())
}
