use super::super::character_equipment::{EquipmentError, assert_current_equipment_in_transaction};
use super::super::item_mint::{TypedDefinitionRef, uuid_text};
use super::*;
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use sqlx::{Postgres, Row, Transaction};

pub(super) fn uuid(value: &Value, field: &str) -> Result<[u8; 16], DurabilityError> {
    uuid_text(
        value
            .get(field)
            .and_then(Value::as_str)
            .ok_or(DurabilityError::InvalidStoredState)?,
    )
}

pub(super) fn number(value: &Value, field: &str) -> Result<u64, DurabilityError> {
    value
        .get(field)
        .and_then(|n| n.as_u64().or_else(|| n.as_str()?.parse().ok()))
        .ok_or(DurabilityError::InvalidStoredState)
}

pub(super) fn definition(value: &Value) -> Result<TypedDefinitionRef, DurabilityError> {
    let text = |key| {
        value
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or(DurabilityError::InvalidStoredState)
    };
    Ok(TypedDefinitionRef {
        family: text("definition_family")?,
        production_key: text("definition_production_key")?,
        revision_ref: text("definition_revision_ref")?,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn fence_pair(
    tx: &mut Transaction<'_, Postgres>,
    record: &CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fences: &[CurrentCharacterItemFence; 2],
    commands: &[CommandRef; 2],
    digest: [u8; 32],
) -> Result<bool, DurabilityError> {
    super::super::character_authority::assert_recovery_fence(tx, record).await?;
    super::super::db::lock_admission_relations(tx).await?;
    let [a, b] = fences;
    let Ok((world, channel)) = scope_of(a) else {
        return Ok(false);
    };
    if a.character_id == b.character_id
        || a.game_session_id == b.game_session_id
        || scope_of(b).ok() != Some((world, channel))
    {
        return Ok(false);
    }
    // Match the equipment owner's Content-before-root order. Otherwise an
    // equipment writer holding this shared lock while waiting for our root,
    // with an activation queued exclusively, can deadlock our later shared read.
    sqlx::query("SELECT pg_advisory_xact_lock_shared(hashtextextended('oteryn:content-activation:' || encode($1::bytea, 'hex')::uuid::text || ':' || encode($2::bytea, 'hex')::uuid::text, 0))")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
        .execute(&mut **tx).await?;
    // Both fence-only checks precede either root lock. Reconnection/assignment
    // SHARE locks survive the ordered root locks and the final commit.
    for index in [0, 1] {
        if !super::super::item_transfer::character_item_fence_without_root(
            tx,
            node,
            &fences[index],
            commands[index],
            fences[index].character_id,
            *world.as_bytes(),
            *channel.as_bytes(),
        )
        .await?
        {
            return Ok(false);
        }
    }
    let order = if a.character_id < b.character_id {
        [0, 1]
    } else {
        [1, 0]
    };
    for index in order {
        if !super::super::item_transfer::character_item_fence_is_current(
            tx,
            node,
            &fences[index],
            commands[index],
            fences[index].character_id,
            *world.as_bytes(),
            *channel.as_bytes(),
        )
        .await?
        {
            return Ok(false);
        }
    }
    for index in order {
        match assert_current_equipment_in_transaction(
            tx,
            record,
            node,
            &fences[index],
            Some(commands[index]),
            digest,
        )
        .await
        {
            Ok(_) => {}
            Err(EquipmentError::Rejected(_)) => return Ok(false),
            Err(EquipmentError::Database(e)) => return Err(e.into()),
            Err(EquipmentError::Durability(e)) => return Err(e),
        }
    }
    let distinct: bool = sqlx::query_scalar(
        "SELECT count(DISTINCT account_id)=2 FROM game_character_roots \
          WHERE character_id IN(encode($1,'hex')::uuid,encode($2,'hex')::uuid)",
    )
    .bind(a.character_id.as_bytes().as_slice())
    .bind(b.character_id.as_bytes().as_slice())
    .fetch_one(&mut **tx)
    .await?;
    Ok(distinct)
}

pub(super) async fn read_pair(
    tx: &mut Transaction<'_, Postgres>,
    fences: &[CurrentCharacterItemFence; 2],
    items: [[u8; 16]; 2],
) -> Pass<[TradeLeaf; 2]> {
    if items[0] == items[1] {
        return Ok(Err(Refusal::NotTradeable));
    }
    let mut leaves = [None, None];
    let order = if items[0] < items[1] { [0, 1] } else { [1, 0] };
    for index in order {
        let row=sqlx::query(
            "SELECT to_jsonb(i) AS item,to_jsonb(e) AS entry, \
                (SELECT to_jsonb(k) FROM game_item_keys k WHERE k.item_instance_id=i.item_instance_id) AS key, \
                (SELECT to_jsonb(t) FROM game_item_timed_states t WHERE t.item_instance_id=i.item_instance_id) AS timed, \
                (SELECT jsonb_build_object('sha256',encode(sha256(convert_to(to_jsonb(t)::text,'UTF8')),'hex')) FROM game_item_texts t WHERE t.item_instance_id=i.item_instance_id) AS text, \
                (SELECT to_jsonb(t) FROM game_item_text_revisions t WHERE t.item_instance_id=i.item_instance_id) AS text_revision \
             FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id,world_id) \
             JOIN game_item_container_slots s ON s.character_id=e.character_id AND s.item_instance_id=e.parent_item_instance_id \
             WHERE i.item_instance_id=encode($1,'hex')::uuid AND e.character_id=encode($2,'hex')::uuid AND i.lifecycle=1 \
               AND NOT EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id) \
               AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id) \
               AND NOT EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=i.item_instance_id) \
             FOR UPDATE OF i",
        ).bind(items[index].as_slice()).bind(fences[index].character_id.as_bytes().as_slice())
            .fetch_optional(&mut **tx).await?;
        let Some(row) = row else {
            return Ok(Err(Refusal::NotTradeable));
        };
        leaves[index] = Some(TradeLeaf {
            item: row.try_get("item")?,
            entry: row.try_get("entry")?,
            key: row.try_get("key")?,
            timed: row.try_get("timed")?,
            text: row.try_get("text")?,
            text_revision: row.try_get("text_revision")?,
        });
    }
    match leaves {
        [Some(a), Some(b)] => Ok(Ok([a, b])),
        _ => Err(DurabilityError::InvalidStoredState),
    }
}

pub(super) async fn destinations<C: PlayerTradeContent>(
    tx: &mut Transaction<'_, Postgres>,
    current: &[CurrentCharacterItemFence; 2],
    content: &C,
    leaves: &[TradeLeaf; 2],
) -> Pass<[(u64, u32); 2]> {
    use super::super::item_loop_move::{LoopError, LoopMoveRefusal, carried_weight};
    use super::super::item_transfer::{
        load_entries, load_slot, read_derived_transfer_capacity_centi_oz,
    };
    use oteryn_gameplay_economy::player_trade::{Destination, LeafFacts};
    let mut result = [(0, 0); 2];
    for index in [0, 1] {
        let leaf = &leaves[index];
        let definition = definition(&leaf.item)?;
        let Some((tradeable, pickupable)) = content.player_trade_flags(&definition) else {
            return Ok(Err(Refusal::NotTradeable));
        };
        let quantity = u32::try_from(number(&leaf.item, "quantity")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        if let Err(e) = (LeafFacts {
            direct_backpack_entry: true,
            is_main_backpack: false,
            has_contents: false,
            pickupable,
            tradeable,
            quantity,
        })
        .check()
        {
            return Ok(Err(e));
        }
        let Some(facts) = content.loop_facts(&definition) else {
            return Ok(Err(Refusal::NotTradeable));
        };
        let maximum = super::super::item_transfer::stack_maximum(facts.stack);
        if facts.definition != definition || !maximum.is_ok_and(|(m, _)| quantity <= m) {
            return Ok(Err(Refusal::NotTradeable));
        }
        let target = &current[1 - index];
        let Some(backpack) = load_slot(tx, target.character_id).await? else {
            return Ok(Err(Refusal::NoRoom));
        };
        let Some(backpack_facts) = content.loop_facts(&backpack.definition) else {
            return Ok(Err(Refusal::NoRoom));
        };
        let Some(limit) = backpack_facts.container_capacity else {
            return Ok(Err(Refusal::NoRoom));
        };
        let limit = limit.min(super::super::item_transfer::GAMEITEM01_CONTAINER_ENTRIES_MAX);
        if backpack_facts.definition != backpack.definition {
            return Ok(Err(Refusal::Rejected));
        }
        let entries = load_entries(tx, backpack.item_instance_id).await?;
        let (world, _) = scope_of(target).map_err(|_| DurabilityError::InvalidStoredState)?;
        let weight = match carried_weight(tx, target.character_id, *world.as_bytes(), content).await
        {
            Ok(w) => w,
            Err(LoopError::Refused(LoopMoveRefusal::WeightUnknown)) => {
                return Ok(Err(Refusal::UnknownWeight));
            }
            Err(LoopError::Refused(_)) => return Ok(Err(Refusal::Rejected)),
            Err(LoopError::Database(e)) => return Err(e.into()),
            Err(LoopError::Durability(e)) => return Err(e),
        };
        let unit = content.weight_of(&definition).ok();
        if facts.weight != unit {
            return Ok(Err(Refusal::UnknownWeight));
        }
        let plan = Destination {
            entries: u32::try_from(entries.len())
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            entry_limit: limit,
            highest_ordinal: entries.first().map_or(0, |e| e.placement_ordinal),
            carried_weight: weight,
            capacity: read_derived_transfer_capacity_centi_oz(tx, target.character_id).await?,
            incoming_unit_weight: unit,
            incoming_quantity: quantity,
        };
        result[index] = match plan.next_entry() {
            Ok(ordinal) => (ordinal, limit),
            Err(e) => return Ok(Err(e)),
        };
    }
    Ok(Ok(result))
}
