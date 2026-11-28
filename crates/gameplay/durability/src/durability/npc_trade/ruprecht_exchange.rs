//! Global source: https://tibia.fandom.com/wiki/Christmas_Present_Bag
use super::*;

pub const NPC: &str = "oteryn:npc.ruprecht";
pub(crate) const BAG: &str = "oteryn:item.tibia.i6496";
pub const TOKEN: &str = "oteryn:item.tibia.i6526";

pub(super) fn input() -> &'static TypedDefinitionRef {
    static BAG_DEFINITION: std::sync::OnceLock<TypedDefinitionRef> = std::sync::OnceLock::new();
    BAG_DEFINITION.get_or_init(|| TypedDefinitionRef {
        family: "Item".into(),
        production_key: BAG.into(),
        revision_ref: "definition-r1".into(),
    })
}

pub(super) fn validate(request: &NpcTradeRequest, sell_all: bool) -> Result<()> {
    let definition = &request.item.definition;
    if sell_all
        || request.cause.npc_key != NPC
        || request.cause.catalogue_revision == 0
        || request.cause.offer_index != 0
        || request.quantity != 1
        || request.unit_price != 0
        || request.offer_count != 1
        || request.fluid_subtype.is_some()
        || request.full_charges.is_some()
        || definition.family != "Item"
        || definition.production_key != TOKEN
        || definition.revision_ref != "definition-r1"
        || request.item.container_capacity.is_some()
    {
        return Err(NpcTradeError::InvalidInput);
    }
    Ok(())
}

impl DurabilityRoot {
    /// Redeem one stolen present bag for one Christmas Token.
    pub async fn exchange_ruprecht_present(
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
