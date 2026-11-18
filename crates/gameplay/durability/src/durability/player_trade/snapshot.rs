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

pub(super) async fn read_leaves<const N: usize>(
    tx: &mut Transaction<'_, Postgres>,
    fences: &[CurrentCharacterItemFence; N],
    items: [[u8; 16]; N],
) -> Pass<[TradeLeaf; N]> {
    if N == 2 && items[0] == items[1] {
        return Ok(Err(Refusal::NotTradeable));
    }
    let roots: Vec<Vec<u8>> = items.iter().map(|id| id.to_vec()).collect();
    let candidates = sqlx::query("SELECT r.ordinal,t.item_instance_id::text AS item FROM unnest($1::bytea[]) WITH ORDINALITY AS r(root,ordinal) CROSS JOIN LATERAL (SELECT item_instance_id FROM game_item_loop_subtree_edges(encode(r.root,'hex')::uuid) LIMIT 101) t")
        .bind(roots).fetch_all(&mut **tx).await?;
    let mut counts = [0usize; N];
    let mut identities = Vec::with_capacity(candidates.len());
    for row in candidates {
        let ordinal = usize::try_from(row.try_get::<i64, _>("ordinal")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let count = counts
            .get_mut(ordinal.saturating_sub(1))
            .ok_or(DurabilityError::InvalidStoredState)?;
        *count += 1;
        if *count > 100 {
            return Ok(Err(Refusal::NotTradeable));
        }
        identities.push(row.try_get::<String, _>("item")?);
    }
    // Both Character roots are already locked. Acquire every offered ItemInstance
    // in one UUID order before reading either tree's immutable snapshots.
    let locked: Vec<bool> = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_bindings b WHERE b.item_instance_id=i.item_instance_id) AS bound FROM game_item_instances i WHERE item_instance_id IN (SELECT value::uuid FROM unnest($1::text[]) AS value) ORDER BY item_instance_id FOR UPDATE OF i")
        .bind(identities).fetch_all(&mut **tx).await?;
    if locked.into_iter().any(|bound| bound) {
        return Ok(Err(Refusal::NotTradeable));
    }
    let mut leaves: [Option<TradeLeaf>; N] = std::array::from_fn(|_| None);
    let mut order: Vec<_> = (0..N).collect();
    order.sort_by_key(|index| items[*index]);
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
               AND NOT EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=i.item_instance_id) \
             FOR UPDATE OF i",
        ).bind(items[index].as_slice()).bind(fences[index].character_id.as_bytes().as_slice())
            .fetch_optional(&mut **tx).await?;
        let Some(row) = row else {
            return Ok(Err(Refusal::NotTradeable));
        };
        leaves[index] = Some(TradeLeaf {
            contents: read_contents(tx, items[index]).await?,
            item: row.try_get("item")?,
            entry: row.try_get("entry")?,
            key: row.try_get("key")?,
            timed: row.try_get("timed")?,
            text: row.try_get("text")?,
            text_revision: row.try_get("text_revision")?,
            attributes: None,
        });
    }
    let leaves = leaves
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or(DurabilityError::InvalidStoredState)?;
    Ok(Ok(leaves
        .try_into()
        .map_err(|_| DurabilityError::InvalidStoredState)?))
}

// The shared subtree walk stops at a sentinel beyond the carried-tree bound. Reject
// oversized, cyclic and non-nested descendants rather than exposing a truncated offer.
async fn read_contents(
    tx: &mut Transaction<'_, Postgres>,
    root: [u8; 16],
) -> Result<Vec<TradeLeaf>, DurabilityError> {
    let rows = sqlx::query(
        "SELECT t.depth,t.cyclic,t.source_kind,(i.world_id=r.world_id AND n.world_id=r.world_id) AS same_world,to_jsonb(i) AS item,to_jsonb(n) AS entry, \
         (SELECT to_jsonb(k) FROM game_item_keys k WHERE k.item_instance_id=i.item_instance_id) AS key, \
         (SELECT to_jsonb(t) FROM game_item_timed_states t WHERE t.item_instance_id=i.item_instance_id) AS timed, \
         (SELECT jsonb_build_object('sha256',encode(sha256(convert_to(to_jsonb(t)::text,'UTF8')),'hex')) FROM game_item_texts t WHERE t.item_instance_id=i.item_instance_id) AS text, \
         (SELECT to_jsonb(t) FROM game_item_text_revisions t WHERE t.item_instance_id=i.item_instance_id) AS text_revision, \
         NULLIF(jsonb_strip_nulls(jsonb_build_object( \
           'fluid', (SELECT to_jsonb(f) FROM game_item_fluid_states f WHERE f.item_instance_id=i.item_instance_id), \
           'tier', (SELECT to_jsonb(f) FROM game_item_tiers f WHERE f.item_instance_id=i.item_instance_id), \
           'imbuements', (SELECT jsonb_agg(to_jsonb(b) ORDER BY b.slot) FROM game_item_imbuements b WHERE b.item_instance_id=i.item_instance_id) \
         )), '{}'::jsonb) AS attributes \
         FROM game_item_loop_subtree_edges(encode($1,'hex')::uuid) t \
         JOIN game_item_instances i USING(item_instance_id) \
         JOIN game_item_instances r ON r.item_instance_id=encode($1,'hex')::uuid \
         LEFT JOIN game_item_nested_entries n ON n.item_instance_id=i.item_instance_id \
         WHERE t.depth>0 ORDER BY t.depth,t.parent_item_instance_id,t.placement_ordinal DESC \
         LIMIT 100 FOR UPDATE OF i"
    ).bind(root.as_slice()).fetch_all(&mut **tx).await?;
    if rows.len() >= 100 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut contents = Vec::with_capacity(rows.len());
    for row in rows {
        let depth: i32 = row.try_get("depth")?;
        let item: Value = row.try_get("item")?;
        let entry: Option<Value> = row.try_get("entry")?;
        if row.try_get::<Option<bool>, _>("same_world")? != Some(true)
            || !(1..=7).contains(&depth)
            || row.try_get::<bool, _>("cyclic")?
            || row.try_get::<i16, _>("source_kind")? != 2
            || item["lifecycle"] != 1
            || contents
                .iter()
                .any(|leaf: &TradeLeaf| leaf.item["item_instance_id"] == item["item_instance_id"])
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        contents.push(TradeLeaf {
            contents: Vec::new(),
            item,
            entry: entry.ok_or(DurabilityError::InvalidStoredState)?,
            key: row.try_get("key")?,
            timed: row.try_get("timed")?,
            text: row.try_get("text")?,
            text_revision: row.try_get("text_revision")?,
            attributes: row.try_get("attributes")?,
        });
    }
    Ok(contents)
}

pub(super) async fn read_pair(
    tx: &mut Transaction<'_, Postgres>,
    fences: &[CurrentCharacterItemFence; 2],
    items: [[u8; 16]; 2],
) -> Pass<[TradeLeaf; 2]> {
    read_leaves(tx, fences, items).await
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
        let destination_size: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_item_loop_subtree_edges(encode($1,'hex')::uuid)",
        )
        .bind(backpack.item_instance_id.as_slice())
        .fetch_one(&mut **tx)
        .await?;
        let incoming = i64::try_from(1 + leaf.contents.len())
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let outgoing = i64::try_from(1 + leaves[1 - index].contents.len())
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        if destination_size > 501 || destination_size - outgoing + incoming > 501 {
            return Ok(Err(Refusal::NoRoom));
        }

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
        let mut child_weight = 0u64;
        for child in &leaf.contents {
            let (definition, count) = child.definition_and_quantity()?;
            let Some((tradeable, pickupable)) = content.player_trade_flags(&definition) else {
                return Ok(Err(Refusal::NotTradeable));
            };
            if !pickupable
                || !matches!(
                    tradeable,
                    oteryn_gameplay_economy::player_trade::TradeFlag::Unknown
                        | oteryn_gameplay_economy::player_trade::TradeFlag::Known(true)
                )
            {
                return Ok(Err(Refusal::NotTradeable));
            }
            let Some(facts) = content.loop_facts(&definition) else {
                return Ok(Err(Refusal::NotTradeable));
            };
            if !super::super::item_transfer::stack_maximum(facts.stack)
                .is_ok_and(|(maximum, _)| (1..=maximum).contains(&count))
            {
                return Ok(Err(Refusal::NotTradeable));
            }
            let Ok(unit) = content.weight_of(&definition) else {
                return Ok(Err(Refusal::UnknownWeight));
            };
            child_weight = child_weight
                .checked_add(u64::from(unit) * u64::from(count))
                .ok_or(DurabilityError::InvalidStoredState)?;
        }
        let capacity =
            read_derived_transfer_capacity_centi_oz(tx, target.character_id, content).await?;
        let Some(capacity) = capacity.checked_sub(child_weight) else {
            return Ok(Err(Refusal::Overweight));
        };
        let plan = Destination {
            entries: u32::try_from(entries.len())
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            entry_limit: limit,
            highest_ordinal: entries.first().map_or(0, |e| e.placement_ordinal),
            carried_weight: weight,
            capacity,
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
