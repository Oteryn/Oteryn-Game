//! Wyda's blood-herb exchange, committed by the NPC trade owner.
use super::*;
use oteryn_gameplay_world_change::bored;

pub(super) fn input() -> &'static TypedDefinitionRef {
    static HERB: std::sync::OnceLock<TypedDefinitionRef> = std::sync::OnceLock::new();
    HERB.get_or_init(|| TypedDefinitionRef {
        family: "Item".into(),
        production_key: bored::BLOOD_HERB.into(),
        revision_ref: "definition-r1".into(),
    })
}

pub(super) fn validate(request: &NpcTradeRequest, sell_all: bool) -> Result<()> {
    if sell_all
        || request.cause.npc_key != bored::WYDA
        || request.cause.catalogue_revision == 0
        || request.cause.offer_index != 0
        || request.quantity != 1
        || request.unit_price != 0
        || request.offer_count != 1
        || request.item.definition.family != "Item"
        || request.item.definition.revision_ref != "definition-r1"
        || request.item.definition.production_key != bored::exchange_reward(&request.cause.occurrence)
        || request.item.container_capacity.is_some()
    {
        return Err(NpcTradeError::InvalidInput);
    }
    Ok(())
}

impl DurabilityRoot {
    pub async fn exchange_wyda_blood_herb(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: NpcTradeRequest,
        weights: &dyn super::super::item_transfer::ItemWeightLookup,
    ) -> Result<NpcTradeOutcome> {
        if request.cause.side != NpcTradeSide::Exchange {
            return Err(NpcTradeError::InvalidInput);
        }
        validate(&request, false)?;
        self.commit_trade(
            authority,
            node,
            fence,
            request,
            TradeCommitOptions {
                weights: Some(weights),
                ..TradeCommitOptions::default()
            },
        )
        .await
    }
}
