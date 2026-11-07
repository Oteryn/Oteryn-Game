//! Data-driven exact retained shop contracts. Only the new five-shop test reads this packet.
use super::*;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Contract {
    pub npc: String,
    pub service: String,
    pub offers: Vec<crate::content::ProjectV2ServiceOffer>,
}

pub(super) fn find(key: &str) -> TestResult<Option<Contract>> {
    if !matches!(key, "dixi" | "lubo" | "pompan" | "shiantis" | "toffee") {
        return Ok(None);
    }
    let path = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../tools/content-schema/npc-authoring/npcfix-retained-shop-contracts.json"));
    let cases: Vec<Contract> = serde_json::from_slice(&std::fs::read(path)?)?;
    assert_eq!(cases.len(), 5);
    let npc = format!("oteryn:npc.{key}");
    let mut matching = cases.into_iter().filter(|c| c.npc == npc);
    let case = matching.next().ok_or("exact shop contract absent")?;
    assert!(matching.next().is_none());
    assert_eq!(case.service, format!("oteryn:service.trade.{key}"));
    assert!(!case.offers.is_empty() && case.offers.len() <= 32);
    assert!(case.offers.iter().all(|o| o.count.unwrap_or(1) == 1
        && o.sub_type.is_none() && !o.parity_pending && o.unit_price < 20_000_000));
    Ok(Some(case))
}

pub(super) async fn quantity(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id,world_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.lifecycle=1 AND i.definition_production_key=$2")
        .bind(id(41).as_slice()).bind(key).fetch_one(&h.pool).await?)
}

pub(super) async fn gold(h: &Harness) -> TestResult<i64> {
    let carried: i64 = sqlx::query_scalar("SELECT COALESCE(sum(i.quantity * CASE i.definition_production_key WHEN 'oteryn:item.tibia.i3031' THEN 1 WHEN 'oteryn:item.tibia.i3035' THEN 100 WHEN 'oteryn:item.tibia.i3043' THEN 10000 ELSE 0 END),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id,world_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.lifecycle=1")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    let bank: i64 = sqlx::query_scalar("SELECT COALESCE((SELECT balance::bigint FROM game_account_bank_balances WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid),0)")
        .bind(id(40).as_slice()).bind(id(42).as_slice()).fetch_one(&h.pool).await?;
    Ok(carried + bank)
}
