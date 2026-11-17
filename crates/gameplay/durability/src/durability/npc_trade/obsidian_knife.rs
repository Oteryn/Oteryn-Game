//! Uzgod: one draconian steel and one obsidian lance for a knife, once per20h.
use super::*;

use oteryn_gameplay_quests::quest::QuestStateCatalogue;

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

const QUEST: &str = "oteryn:quest.obsidian_knife_quest";

pub(super) struct QuestPin {
    revision: String,
    hash: [u8; 32],
}

/// The exchange has already proved and locked the Character item fence.
pub(super) async fn complete(
    tx: &mut Type2Transaction<'_>,
    character: CharacterId,
    request: &NpcTradeRequest,
    binding: &[u8],
    pin: &QuestPin,
) -> std::result::Result<bool, DurabilityError> {
    let id = character.as_bytes().as_slice();
    let state: Option<(String, Vec<u8>, bool)> = sqlx::query_as(
        "SELECT pinned_content_revision, definition_hash, completed_character_revision IS NOT NULL
         FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2",
    )
    .bind(id)
    .bind(QUEST)
    .fetch_optional(&mut ***tx)
    .await?;
    let (revision, hash) = match &state {
        Some((_, _, true)) => return Ok(true),
        Some((revision, hash, false)) => (revision.as_str(), hash.as_slice()),
        None => {
            if !super::super::quest_state::capacity::new_state_fits(tx, character).await? {
                return Ok(false);
            }
            (pin.revision.as_str(), pin.hash.as_slice())
        }
    };
    let committed: String = sqlx::query_scalar(
        "UPDATE game_character_roots SET character_revision=character_revision+1
         WHERE character_id=encode($1,'hex')::uuid RETURNING character_revision::text",
    )
    .bind(id)
    .fetch_one(&mut ***tx)
    .await?;
    let receipt = sqlx::query(
        "WITH progression AS (
            UPDATE game_character_progression_state SET character_revision=$2::text::numeric
            WHERE character_id=encode($1,'hex')::uuid AND character_revision=$2::text::numeric-1
            RETURNING *
         )
         INSERT INTO game_character_quest_receipts(
            character_id,cause_kind,cause_id,cause_ordinal,transition_key,quest_key,request_binding,
            pinned_content_revision,definition_hash,completes,track_keys,values_before,values_after,
            original_character_revision,committed_character_revision,level_before,level_after,
            experience_before,experience_after,profile_revision,ruleset_revision,content_revision,
            simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,
            committed_at
         )
         SELECT character_id,'command',encode($3,'hex')::uuid,$4::text::numeric,
            'oteryn:quest-transition/obsidian_knife_exchange',$5,$6,$7,$8,true,'{}','{}','{}',
            character_revision-1,character_revision,level,level,total_experience,total_experience,
            profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,
            declaration_revision,policy_revision,reward_revision,
            floor(extract(epoch FROM statement_timestamp())*1000)::bigint
         FROM progression",
    )
    .bind(id)
    .bind(&committed)
    .bind(request.command.game_session_id().as_bytes().as_slice())
    .bind(request.command.command_id().get().to_string())
    .bind(QUEST)
    .bind(binding)
    .bind(revision)
    .bind(hash)
    .execute(&mut ***tx)
    .await?;
    if receipt.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    sqlx::query(
        "INSERT INTO game_character_quest_states(
            character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,
            completed_character_revision,committed_character_revision
         ) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5::text::numeric,$5::text::numeric,$5::text::numeric)
         ON CONFLICT(character_id,quest_key) DO UPDATE SET
            completed_character_revision=EXCLUDED.completed_character_revision,
            committed_character_revision=EXCLUDED.committed_character_revision",
    )
    .bind(id)
    .bind(QUEST)
    .bind(revision)
    .bind(hash)
    .bind(&committed)
    .execute(&mut ***tx)
    .await?;
    Ok(true)
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
        catalogue: &QuestStateCatalogue,
    ) -> Result<NpcTradeOutcome> {
        if request.cause.side != NpcTradeSide::Exchange {
            return Err(NpcTradeError::InvalidInput);
        }
        validate(&request, false)?;
        let hash = catalogue.definition_hash(QUEST).ok_or(NpcTradeError::InvalidInput)?;
        self.commit_trade(
            authority,
            node,
            fence,
            request,
            TradeCommitOptions {
                obsidian_knife_quest: Some(QuestPin {
                    revision: catalogue.content_revision().to_owned(),
                    hash,
                }),
                weights: Some(weights),
                ..TradeCommitOptions::default()
            },
        )
        .await
    }
}
