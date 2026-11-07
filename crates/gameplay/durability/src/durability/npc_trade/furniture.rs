//! Source-qualified ordinary-NPC furniture BUY, through the existing trade graph.
use super::*;
use oteryn_gameplay_items::furniture::{NpcFurnitureOffer, npc_offer_for_row};

#[derive(Clone, Debug)]
pub struct NpcFurnitureSelection {
    pub source_row: u32,
}
impl NpcFurnitureSelection {
    fn source(&self) -> Result<&'static NpcFurnitureOffer> {
        npc_offer_for_row(self.source_row).ok_or(NpcTradeError::InvalidInput)
    }
    pub(super) fn validate(&self, request: &NpcTradeRequest, sell_all: bool) -> Result<()> {
        let source = self.source()?;
        if sell_all
            || request.cause.side != NpcTradeSide::Buy
            || request.cause.npc_key != source.npc_key
            || request.quantity != 1
            || request.offer_count != 1
            || request.unit_price != source.price
            || request.item.definition.family != "Item"
            || request.item.definition.production_key != "oteryn:item.tibia.i23398"
            || request.item.definition.revision_ref != "definition-r1"
        {
            return Err(NpcTradeError::InvalidInput);
        }
        Ok(())
    }
    pub(super) fn bind(&self, binding: Vec<u8>) -> Vec<u8> {
        use sha2::{Digest, Sha256};
        Sha256::new()
            .chain_update(binding)
            .chain_update(b"oteryn:normal-npc-furniture.v1")
            .chain_update(self.source_row.to_be_bytes())
            .finalize()
            .to_vec()
    }
    pub(super) fn kit_audit(
        &self,
        item: [u8; 16],
    ) -> Option<super::super::npc_trade_audit::OneItemHouseFurnitureKitV1> {
        let source = self.source().ok()?;
        Some(super::super::npc_trade_audit::OneItemHouseFurnitureKitV1 {
            item_instance_id: item.to_vec(),
            source_row: self.source_row,
            target: Some(OneItemTypedDefinitionRevisionV1 {
                family: "Item".into(),
                production_key: format!("oteryn:item.tibia.i{}", source.item_id),
                revision_ref: "definition-r1".into(),
            }),
        })
    }
    pub(super) async fn apply(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        request: &NpcTradeRequest,
        item: [u8; 16],
    ) -> Result<()> {
        let source = self.source()?;
        super::super::house_wrap::record_purchased_kit(
            tx,
            request.transaction_id,
            item,
            self.source_row as usize,
            &TypedDefinitionRef {
                family: "Item".into(),
                production_key: format!("oteryn:item.tibia.i{}", source.item_id),
                revision_ref: "definition-r1".into(),
            },
        )
        .await?;
        Ok(())
    }
}
impl DurabilityRoot {
    pub async fn commit_npc_furniture_trade(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: NpcTradeRequest,
        selection: NpcFurnitureSelection,
    ) -> Result<NpcTradeOutcome> {
        self.commit_trade(
            authority,
            node,
            fence,
            request,
            TradeCommitOptions {
                furniture: Some(selection),
                ..TradeCommitOptions::default()
            },
        )
        .await
    }
}

/// Crystal201189f data-global/npc/hairycles.lua:497-503. Greeting state is advisory;
/// the trade writer reads the durable quest track under its current Character lock.
pub(super) async fn permitted(
    tx: &mut Type2Transaction<'_>,
    character: CharacterId,
    request: &NpcTradeRequest,
) -> std::result::Result<bool, sqlx::Error> {
    if request.cause.npc_key != "oteryn:npc.hairycles" {
        return Ok(true);
    }
    let stage:Option<i64>=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 AND track_key=$3")
  .bind(character.as_bytes().as_slice()).bind("oteryn:quest.the_ape_city_quest")
  .bind("oteryn:quest-progress/quest/u7_6/the_ape_city/questline").fetch_optional(&mut ***tx).await?;
    Ok(stage.is_some_and(|value| value >= 18))
}
