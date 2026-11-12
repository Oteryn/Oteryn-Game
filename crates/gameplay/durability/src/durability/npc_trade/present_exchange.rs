//! Seymour's present box exchange, CRAFT-1 C8 on the existing NPC trade writer.
//! TIBIAWIKI_STRUCTURED: Present Quest/Spoiler transcript; captured items.xml
//! lines 5953 and 9797 join the present and legion helmet identities.
use super::*;

pub(super) const NPC: &str = "oteryn:npc.seymour";

pub(super) fn input() -> &'static TypedDefinitionRef {
    static VALUE: std::sync::OnceLock<TypedDefinitionRef> = std::sync::OnceLock::new();
    VALUE.get_or_init(|| TypedDefinitionRef {
        family: "Item".into(),
        production_key: "oteryn:item.tibia.i2856".into(),
        revision_ref: "definition-r1".into(),
    })
}

pub(super) fn output() -> &'static TypedDefinitionRef {
    static VALUE: std::sync::OnceLock<TypedDefinitionRef> = std::sync::OnceLock::new();
    VALUE.get_or_init(|| TypedDefinitionRef {
        family: "Item".into(),
        production_key: "oteryn:item.tibia.i3374".into(),
        revision_ref: "definition-r1".into(),
    })
}

pub(super) fn validate(request: &NpcTradeRequest, sell_all: bool) -> Result<()> {
    if sell_all
        || request.cause.npc_key != NPC
        || request.cause.catalogue_revision == 0
        || request.cause.offer_index != 0
        || request.quantity != 1
        || request.unit_price != 0
        || request.offer_count != 1
        || request.item.definition != *output()
        || request.item.container_capacity.is_some()
    {
        return Err(NpcTradeError::InvalidInput);
    }
    Ok(())
}

impl DurabilityRoot {
    /// Exchange one empty source present for Seymour's legion helmet atomically.
    pub async fn exchange_present_for_helmet(
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
