//! Uzgod: one draconian steel and one obsidian lance for a knife, once per20h.
use super::*;

pub(super) const NPC: &str = "oteryn:npc.uzgod";

pub(super) fn steel() -> &'static TypedDefinitionRef {
    static VALUE: std::sync::OnceLock<TypedDefinitionRef> = std::sync::OnceLock::new();
    VALUE.get_or_init(|| definition("oteryn:item.tibia.i5889"))
}
pub(super) fn lance() -> &'static TypedDefinitionRef {
    static VALUE: std::sync::OnceLock<TypedDefinitionRef> = std::sync::OnceLock::new();
    VALUE.get_or_init(|| definition("oteryn:item.tibia.i3313"))
}
pub(super) fn knife() -> &'static TypedDefinitionRef {
    static VALUE: std::sync::OnceLock<TypedDefinitionRef> = std::sync::OnceLock::new();
    VALUE.get_or_init(|| definition("oteryn:item.tibia.i5908"))
}
fn definition(key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: "definition-r1".into(),
    }
}

pub(super) fn validate(request: &NpcTradeRequest, sell_all: bool) -> Result<()> {
    if sell_all
        || request.cause.npc_key != NPC
        || request.cause.catalogue_revision == 0
        || request.cause.offer_index != 0
        || request.quantity != 1
        || request.unit_price != 0
        || request.offer_count != 1
        || request.item.definition != *knife()
        || request.item.container_capacity.is_some()
    {
        return Err(NpcTradeError::InvalidInput);
    }
    Ok(())
}

pub(super) async fn ready(
    tx: &mut Type2Transaction<'_>,
    character: CharacterId,
) -> std::result::Result<bool, DurabilityError> {
    // character_item_fence_is_current already holds the Character root. The same
    // lock serializes every channel; the server's committed_at is the time authority.
    let cooling: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_npc_trades WHERE character_id=encode($1,'hex')::uuid AND side=3 AND npc_key='oteryn:npc.uzgod' AND committed_at>floor(extract(epoch FROM statement_timestamp())*1000)::bigint-72000000)")
        .bind(character.as_bytes().as_slice()).fetch_one(&mut ***tx).await?;
    Ok(!cooling)
}

pub(super) fn plan(request: &NpcTradeRequest, capacity: usize, entries: &[Entry]) -> Result<Plan> {
    let steel = entries
        .iter()
        .find(|e| {
            e.live && !e.not_default && e.definition == *steel() && (1..=100).contains(&e.quantity)
        })
        .ok_or(NpcTradeError::ItemNotFound)?;
    let lance = entries
        .iter()
        .find(|e| e.live && !e.not_default && e.definition == *lance() && e.quantity == 1)
        .ok_or(NpcTradeError::ItemNotFound)?;
    let remaining = entries
        .len()
        .checked_sub(1 + usize::from(steel.quantity == 1))
        .and_then(|n| n.checked_add(1))
        .ok_or(NpcTradeError::InvalidInput)?;
    if remaining > capacity {
        return Err(NpcTradeError::DoesNotFit);
    }
    let ordinal = entries
        .iter()
        .map(|e| e.placement_ordinal)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or(NpcTradeError::CapacityExceeded)?;
    let output = request.output_item_instance_ids[0];
    Ok(Plan {
        lines: vec![
            NpcTradeLine {
                kind: NpcTradeLineKind::ItemBurn,
                item_instance_id: steel.item_instance_id,
                placement_ordinal: steel.placement_ordinal,
                coin: None,
                quantity_before: steel.quantity,
                quantity_after: steel.quantity - 1,
            },
            NpcTradeLine {
                kind: NpcTradeLineKind::ItemBurn,
                item_instance_id: lance.item_instance_id,
                placement_ordinal: lance.placement_ordinal,
                coin: None,
                quantity_before: 1,
                quantity_after: 0,
            },
            NpcTradeLine {
                kind: NpcTradeLineKind::ItemMint,
                item_instance_id: output,
                placement_ordinal: ordinal,
                coin: None,
                quantity_before: 0,
                quantity_after: 1,
            },
        ],
        burned_gold_units: 0,
        minted_gold_units: 0,
        bank_debit_gold_units: 0,
        bank_debit: None,
        item_instance_id: output,
    })
}

pub(super) fn input_for(
    line: &NpcTradeLine,
    lines: &[NpcTradeLine],
) -> &'static TypedDefinitionRef {
    if lines
        .first()
        .is_some_and(|first| first.item_instance_id == line.item_instance_id)
    {
        steel()
    } else {
        lance()
    }
}

impl DurabilityRoot {
    /// Uzgod's fixed recipe and cross-channel cooldown on the existing NPC trade writer.
    pub async fn exchange_obsidian_knife(
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
