//! Read-only market projections under the existing recovery/session/depot fence.
use super::*;
use crate::durability::{
    DurabilityRoot,
    character_authority::{ReconciledCharacterAuthority, assert_recovery_fence},
    character_progression::assert_market_item_fence,
    db::{begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations},
    item_transfer::CurrentCharacterItemFence,
    runtime_scope_assignment::NodeIncarnationProof,
};
use oteryn_protocol_oteryn::market_details::{PriceSummary, Statistics};
use std::collections::BTreeMap;
pub struct Projection {
    pub statistics: Statistics,
    pub depot: BTreeMap<String, u64>,
    pub book: Vec<Offer>,
    pub has_next: bool,
}
impl DurabilityRoot {
    pub async fn market_projection(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        definition: Option<String>,
        town: u16,
        page: Option<u32>,
    ) -> Result<Projection> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        if page.is_some_and(|p| p > 21) {
            return Err(invalid());
        }
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            assert_market_item_fence(&mut tx,&fence,&node).await?.map_err(|_|invalid())?;
            let mut result=Projection{statistics:Statistics::default(),depot:BTreeMap::new(),book:Vec::new(),has_next:false};
            if let Some(key)=&definition {
                // The two private history observations of one settlement are one execution.
                // Read one SQL snapshot and use the stored execution price, never today's offer price.
                let row=sqlx::query("WITH stamp AS (SELECT floor(extract(epoch FROM statement_timestamp())*1000000)::bigint AS now_us), trades AS (SELECT DISTINCT ON (h.transaction_id) h.transaction_id,h.price,h.amount,o.side FROM stamp,game_market_settlements s JOIN game_market_history h USING(transaction_id) JOIN game_market_offers o ON o.offer_id=s.offer_id WHERE s.world_id=encode($1,'hex')::uuid AND o.world_id=s.world_id AND o.definition_key=$2 AND s.kind IN(2,5) AND s.occurred_at_us>=stamp.now_us-2592000000000 AND s.occurred_at_us<=stamp.now_us ORDER BY h.transaction_id,h.character_id) SELECT stamp.now_us,stamp.now_us-2592000000000 AS since_us,count(t.transaction_id)::text AS trades,coalesce(sum(t.amount),0)::text AS units,min(t.price)::text AS minimum,max(t.price)::text AS maximum,floor(avg(t.price))::text AS average,count(t.transaction_id) FILTER (WHERE t.side=1)::text AS buy_trades,coalesce(sum(t.amount) FILTER (WHERE t.side=1),0)::text AS buy_units,min(t.price) FILTER (WHERE t.side=1)::text AS buy_minimum,max(t.price) FILTER (WHERE t.side=1)::text AS buy_maximum,floor(avg(t.price) FILTER (WHERE t.side=1))::text AS buy_average,count(t.transaction_id) FILTER (WHERE t.side=2)::text AS sell_trades,coalesce(sum(t.amount) FILTER (WHERE t.side=2),0)::text AS sell_units,min(t.price) FILTER (WHERE t.side=2)::text AS sell_minimum,max(t.price) FILTER (WHERE t.side=2)::text AS sell_maximum,floor(avg(t.price) FILTER (WHERE t.side=2))::text AS sell_average FROM stamp LEFT JOIN trades t ON true GROUP BY stamp.now_us")
                    .bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(key).fetch_one(&mut *tx).await?;
                let number=|name:&str| -> Result<u64> {row.try_get::<String,_>(name)?.parse().map_err(|_|invalid())};
                let optional=|name:&str| -> Result<Option<u64>> {row.try_get::<Option<String>,_>(name)?.map(|s|s.parse().map_err(|_|invalid())).transpose()};
                let summary=|prefix:&str| -> Result<PriceSummary> {Ok(PriceSummary{
                    trades:number(&format!("{prefix}_trades"))?,units:number(&format!("{prefix}_units"))?,
                    minimum:optional(&format!("{prefix}_minimum"))?,maximum:optional(&format!("{prefix}_maximum"))?,average:optional(&format!("{prefix}_average"))?
                })};
                result.statistics=Statistics{buy:summary("buy")?,sell:summary("sell")?,definition:0,as_of_us:row.try_get("now_us")?,since_us:row.try_get("since_us")?,trades:number("trades")?,units:number("units")?,minimum:optional("minimum")?,maximum:optional("maximum")?,average:optional("average")?};
            } else {
                // Catalogue possession filter only; placement requalifies instances atomically.
                let rows=sqlx::query("SELECT i.definition_production_key, sum(i.quantity)::text AS units FROM game_item_instances i JOIN game_item_character_depot_locations d USING(item_instance_id,world_id) WHERE d.world_id=encode($1,'hex')::uuid AND d.character_id=encode($2,'hex')::uuid AND d.town_id=$3 AND i.lifecycle=1 GROUP BY i.definition_production_key ORDER BY i.definition_production_key LIMIT 65536")
                    .bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(i32::from(town)).fetch_all(&mut *tx).await?;
                if rows.len()>65535 {return Err(invalid());}
                for row in rows {result.depot.insert(row.try_get("definition_production_key")?,row.try_get::<String,_>("units")?.parse().map_err(|_|invalid())?);}
            }
            if let (Some(key),Some(page))=(&definition,page) {
                let key=BookKey{world:WorldId::from_bytes(*fence.runtime_scope.world_id().as_bytes()).map_err(|_|invalid())?,definition:MarketDefinitionKey::new(key)?};
                for side in [Side::Buy,Side::Sell] {
                    let mut cursor=None;
                    for current in 0..=page {
                        let mut batch=book_page(&mut tx,&key,side,cursor.as_ref()).await?;
                        if current==page {
                            result.has_next |= page<21 && batch.next.is_some();
                            batch.offers.truncate((700-page*32).min(32) as usize);
                            result.book.extend(batch.offers);
                            break;
                        }
                        let Some(next)=batch.next else {break;};cursor=Some(next);
                    }
                }
            }
            commit_semantic_transaction(tx,deadline).await?;
            Ok(result)
        })).await
    }
}
