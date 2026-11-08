//! One closed repeatable item barter on the existing NPC Trade owner and audit receipt.
//! Wiki Twenty Miles 69583@1149608 sha4e4593cc.. line158: Yeo exchanges ten Shrimp
//! for one Bottle of Gloo. NPC placement/dialogue are explicit native authoring.
use super::*;
use std::sync::OnceLock;
pub const NPC: &str = "oteryn:npc.yeo";
pub const SHRIMP: &str = "oteryn:item.tibia.i3581";
pub const GLOO: &str = "oteryn:item.tibia.i21803";
/// Explicit native placement, not a claimed donor coordinate.
pub const POSITION: (u16, u16, i8) = (33554, 31624, 14);
pub fn shrimp_definition() -> &'static TypedDefinitionRef {
    static DEFINITION: OnceLock<TypedDefinitionRef> = OnceLock::new();
    DEFINITION.get_or_init(|| TypedDefinitionRef {
        family: "Item".into(),
        production_key: SHRIMP.into(),
        revision_ref: "definition-r1".into(),
    })
}
pub(super) fn matches(request: &NpcTradeRequest) -> bool {
    request.cause.npc_key == NPC
        && request.cause.side == NpcTradeSide::Buy
        && request.item.definition.production_key == GLOO
        && request.item.definition.family == "Item"
        && request.item.definition.revision_ref == "definition-r1"
        && request.quantity == 1
        && request.offer_count == 1
        && request.unit_price == 0
}
pub(super) fn definition<'a>(
    request: &'a NpcTradeRequest,
    line: &NpcTradeLine,
) -> &'a TypedDefinitionRef {
    if request.cause.npc_key == NPC && line.kind == NpcTradeLineKind::ItemBurn {
        shrimp_definition()
    } else if let Some(coin) = line.coin {
        request.coins.of(coin)
    } else {
        &request.item.definition
    }
}
pub(super) fn bind(binding: Vec<u8>, position: Option<(u16, u16, i8)>) -> Vec<u8> {
    let Some((x, y, z)) = position else {
        return binding;
    };
    Sha256::new()
        .chain_update(b"oteryn:npc-yeo-barter-source:v1\0")
        .chain_update(binding)
        .chain_update(x.to_be_bytes())
        .chain_update(y.to_be_bytes())
        .chain_update(z.to_be_bytes())
        .finalize()
        .to_vec()
}
pub(super) fn plan(
    request: &NpcTradeRequest,
    capacity: usize,
    entries: &[Entry],
    tuple: Type2EventTuple,
) -> Result<Plan> {
    if !matches(request) {
        return Err(NpcTradeError::InvalidInput);
    }
    let mut after = entries.to_vec();
    let mut remaining = 10u32;
    let mut burns = Vec::new();
    for entry in &mut after {
        if remaining == 0 {
            break;
        }
        if !entry.live || entry.definition.production_key != SHRIMP {
            continue;
        }
        if entry.definition != *shrimp_definition() || entry.not_default {
            return Err(NpcTradeError::InvalidInput);
        }
        let units = remaining.min(entry.quantity);
        burns.push(NpcTradeLine {
            kind: NpcTradeLineKind::ItemBurn,
            item_instance_id: entry.item_instance_id,
            placement_ordinal: entry.placement_ordinal,
            coin: None,
            quantity_before: entry.quantity,
            quantity_after: entry.quantity - units,
        });
        entry.quantity -= units;
        remaining -= units;
    }
    if remaining != 0 {
        return Err(NpcTradeError::ItemNotFound);
    }
    after.retain(|entry| entry.quantity > 0);
    let mut result = super::plan_buy(request, 0, 1, capacity, &after, tuple, false)?;
    // Keep the existing monotonic placement lineage even when the highest input retires.
    let next = entries
        .iter()
        .map(|e| e.placement_ordinal)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or(NpcTradeError::DoesNotFit)?;
    if let Some(output) = result.lines.last_mut() {
        output.placement_ordinal = next;
    }
    burns.extend(result.lines);
    result.lines = burns;
    Ok(result)
}
impl DurabilityRoot {
    /// The actual native NPC route freezes this source position and original command.
    pub async fn commit_yeo_barter(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: NpcTradeRequest,
        source_position: (u16, u16, i8),
    ) -> Result<NpcTradeOutcome> {
        if !matches(&request) || source_position != POSITION {
            return Err(NpcTradeError::InvalidInput);
        }
        self.commit_trade(
            authority,
            node,
            fence,
            request,
            TradeCommitOptions {
                yeo_position: Some(source_position),
                ..TradeCommitOptions::default()
            },
        )
        .await
    }
}
