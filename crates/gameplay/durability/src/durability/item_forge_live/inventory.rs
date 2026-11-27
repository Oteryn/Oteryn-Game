//! Current carried custody and physical Forge materials, using the existing item rows.
use super::*;
use crate::domain::death::EquipmentSlot;
use crate::domain::item_definition::{ItemDefinitionFacts, ProjectV2ItemForgeProfile};
use crate::durability::item_loop_move::{
    ITEM_LOOP_DEPTH_MAX, ItemLoopContent, LoopError, carried_weight,
};
use crate::durability::item_transfer::read_derived_transfer_capacity_centi_oz;
use oteryn_gameplay_content::content::{ReferenceEquipmentSlot, ReferenceItemField};
use oteryn_gameplay_items::item_ref_admission::generation_item_facts;
use oteryn_gameplay_items::item_transfer::GAMEITEM01_CONTAINER_ENTRIES_MAX;

use crate::domain::currency::{Coin, CoinStack, FeePlanError, plan_fee_within};
use crate::durability::item_fee_burn::bank_part;
use crate::durability::item_mint_audit::{Type2EventTuple, Type2Transaction};

const EXALTATION_CHEST_KEY: &str = "oteryn:item.tibia.i37561";

#[derive(Debug, Clone)]
pub(super) struct Item {
    pub id: [u8; 16],
    pub identity: String,
    pub before: Value,
    pub facts: ItemDefinitionFacts,
    pub tier: ItemTier,
    pub location: Value,
    pub plain: bool,
    pub imbued: bool,
    pub scheduled: bool,
    pub quantity: u32,
}

impl Item {
    pub fn line(&self, quantity: u32, tier: ItemTier) -> Value {
        json!({"item": self.identity, "before": self.before, "location": self.location,
            "quantity_after": quantity, "tier_before": self.tier.get(), "tier_after": tier.get()})
    }
    pub fn forge<'a>(&'a self, native: &NativeGameplayState) -> Option<ForgeItem<'a>> {
        let definition = &self.facts.definition;
        let policy = native.item_policy(&definition.production_key, &definition.revision_ref)?;
        let record = policy.record();
        let profile = record.authoring.forge?;
        let slot = forge_slot(&record.semantics.equipment)?;
        (self.quantity == 1 && !self.imbued && !self.scheduled).then_some(ForgeItem {
            instance: self.id,
            definition: &self.facts,
            profile: ProjectV2ItemForgeProfile {
                classification: profile.classification,
                max_tier: profile.max_tier,
            },
            tier: self.tier,
            slot,
        })
    }
}

fn forge_slot(
    equipment: &ReferenceItemField<oteryn_gameplay_content::content::ReferenceItemEquipment>,
) -> Option<EquipmentSlot> {
    let ReferenceItemField::Known(equipment) = equipment else {
        return None;
    };
    let ReferenceItemField::Known(patterns) = &equipment.patterns else {
        return None;
    };
    patterns
        .iter()
        .find_map(|pattern| match pattern.primary_slot {
            ReferenceItemField::Known(ReferenceEquipmentSlot::Head) => Some(EquipmentSlot::Head),
            ReferenceItemField::Known(ReferenceEquipmentSlot::Torso) => Some(EquipmentSlot::Armor),
            ReferenceItemField::Known(ReferenceEquipmentSlot::Legs) => Some(EquipmentSlot::Legs),
            ReferenceItemField::Known(ReferenceEquipmentSlot::Feet) => Some(EquipmentSlot::Feet),
            ReferenceItemField::Known(
                ReferenceEquipmentSlot::Weapon | ReferenceEquipmentSlot::Shield,
            ) => Some(EquipmentSlot::RightHand),
            _ => None,
        })
}

pub(super) struct Inventory {
    pub bag_identity: String,
    pub bag_facts: ItemDefinitionFacts,
    pub items: Vec<Item>,
}

pub(super) async fn read(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterItemFence,
    native: &NativeGameplayState,
) -> NativeResult<Option<Inventory>> {
    let bag = sqlx::query("SELECT i.item_instance_id::text,i.definition_family,i.definition_production_key,i.definition_revision_ref FROM game_item_container_slots s JOIN game_item_instances i USING(item_instance_id,world_id) WHERE s.character_id=encode($1,'hex')::uuid AND s.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 FOR UPDATE OF i")
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
        .fetch_optional(&mut **tx).await?;
    let Some(bag) = bag else { return Ok(None) };
    let bag_identity: String = bag.try_get("item_instance_id")?;
    let bag_definition = definition(&bag)?;
    let Ok(bag_facts) = generation_item_facts(native, &bag_definition) else {
        return Ok(None);
    };
    let rows = sqlx::query("WITH RECURSIVE carried(item_instance_id,parent,ordinal,depth,kind) AS (SELECT e.item_instance_id,e.parent_item_instance_id,e.placement_ordinal,1,1 FROM game_item_container_entries e WHERE e.character_id=encode($1,'hex')::uuid AND e.world_id=encode($2,'hex')::uuid AND e.parent_item_instance_id=$3::uuid UNION ALL SELECT n.item_instance_id,n.parent_item_instance_id,n.placement_ordinal,c.depth+1,2 FROM carried c JOIN game_item_nested_entries n ON n.parent_item_instance_id=c.item_instance_id AND n.world_id=encode($2,'hex')::uuid WHERE c.depth<8) SELECT i.item_instance_id::text,to_jsonb(i) AS before_item,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,COALESCE(t.tier,0)::smallint AS tier,jsonb_build_object('kind',c.kind,'parent',c.parent,'ordinal',c.ordinal::text,'depth',c.depth) AS location,EXISTS(SELECT 1 FROM game_item_imbuements m WHERE m.item_instance_id=i.item_instance_id) AS imbued,game_forge_pending_item_timer(i.item_instance_id) AS scheduled,NOT EXISTS(SELECT 1 FROM game_item_keys k WHERE k.item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_texts x WHERE x.item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_timed_states d WHERE d.item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_imbuements m WHERE m.item_instance_id=i.item_instance_id) AND t.item_instance_id IS NULL AND NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules t WHERE t.item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules t WHERE t.item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries n WHERE n.parent_item_instance_id=i.item_instance_id) AS plain FROM carried c JOIN game_item_instances i USING(item_instance_id) LEFT JOIN game_item_tiers t USING(item_instance_id) WHERE i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 ORDER BY i.item_instance_id LIMIT 501 FOR UPDATE OF i")
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(&bag_identity)
        .fetch_all(&mut **tx).await?;
    if rows.len() > 500 {
        return Ok(None);
    }
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let definition = definition(&row)?;
        let Ok(facts) = generation_item_facts(native, &definition) else {
            return Ok(None);
        };
        let identity: String = row.try_get("item_instance_id")?;
        let tier = u8::try_from(row.try_get::<i16, _>("tier")?)
            .ok()
            .and_then(ItemTier::new)
            .ok_or(DurabilityError::InvalidStoredState)?;
        items.push(Item {
            id: uuid_text(&identity)?,
            identity,
            before: row.try_get("before_item")?,
            facts,
            tier,
            location: row.try_get("location")?,
            plain: row.try_get("plain")?,
            imbued: row.try_get("imbued")?,
            scheduled: row.try_get("scheduled")?,
            quantity: u32::try_from(row.try_get::<i64, _>("quantity")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        });
    }
    Ok(Some(Inventory {
        bag_identity,
        bag_facts,
        items,
    }))
}

fn definition(row: &sqlx::postgres::PgRow) -> NativeResult<TypedDefinitionRef> {
    Ok(TypedDefinitionRef {
        family: row.try_get("definition_family")?,
        production_key: row.try_get("definition_production_key")?,
        revision_ref: row.try_get("definition_revision_ref")?,
    })
}

pub(super) fn current_definition(
    native: &NativeGameplayState,
    key: &str,
) -> Option<TypedDefinitionRef> {
    native.item_index()?;
    let policy =
        native.item_policy_for_source_id(key.strip_prefix("oteryn:item.tibia.i")?.parse().ok()?)?;
    let record = &policy.record().production_definition;
    let definition = TypedDefinitionRef {
        family: record.family.clone(),
        production_key: record.production_key.clone(),
        revision_ref: record.revision_ref.clone(),
    };
    // Resolve through the admitted policy and ensure the runtime index is present.
    generation_item_facts(native, &definition).ok()?;
    Some(definition)
}

pub(super) async fn material_count(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterItemFence,
    inventory: &Inventory,
    definition: &TypedDefinitionRef,
) -> NativeResult<u64> {
    let carried = inventory
        .items
        .iter()
        .filter(|item| item.plain && item.facts.definition == *definition)
        .map(|item| u64::from(item.quantity))
        .sum::<u64>();
    let stash: Option<i64> = sqlx::query_scalar("SELECT quantity FROM game_character_stash_balances WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND definition_key=$3 AND definition_revision=$4 FOR UPDATE")
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
        .bind(&definition.production_key).bind(&definition.revision_ref).fetch_optional(&mut **tx).await?;
    carried
        .checked_add(
            u64::try_from(stash.unwrap_or(0)).map_err(|_| DurabilityError::InvalidStoredState)?,
        )
        .ok_or(DurabilityError::InvalidStoredState)
}

pub(super) async fn consume_material(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterItemFence,
    inventory: &Inventory,
    definition: &TypedDefinitionRef,
    amount: u32,
    inputs: &mut Vec<Value>,
) -> NativeResult<Option<Value>> {
    let mut remaining = amount;
    for item in inventory
        .items
        .iter()
        .filter(|item| item.plain && item.facts.definition == *definition)
    {
        if remaining == 0 {
            break;
        }
        let take = remaining.min(item.quantity);
        inputs.push(item.line(item.quantity - take, ItemTier::ZERO));
        remaining -= take;
    }
    if remaining == 0 {
        return Ok(None);
    }
    let row = sqlx::query("SELECT quantity,last_entry_id::text FROM game_character_stash_balances WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND definition_key=$3 AND definition_revision=$4 FOR UPDATE")
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
        .bind(&definition.production_key).bind(&definition.revision_ref).fetch_one(&mut **tx).await?;
    let before: i64 = row.try_get("quantity")?;
    if before < i64::from(remaining) {
        return Err(DurabilityError::InvalidStoredState);
    }
    let entry: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
        .fetch_one(&mut **tx)
        .await?;
    Ok(Some(
        json!({"entry":entry,"key":definition.production_key,"revision":definition.revision_ref,
        "amount":remaining,"before":before,"after":before-i64::from(remaining),"previous":row.try_get::<String,_>("last_entry_id")?}),
    ))
}

impl Inventory {
    pub fn free_slot(&self, inputs: &[Value]) -> Option<(String, i16, u64)> {
        let candidates = std::iter::once((&self.bag_identity, &self.bag_facts, 1_i16)).chain(
            self.items
                .iter()
                .filter(|item| {
                    item.facts.container_capacity.is_some()
                        && item.location["depth"]
                            .as_u64()
                            .is_some_and(|depth| depth < 8)
                })
                .map(|item| (&item.identity, &item.facts, 2_i16)),
        );
        for (parent, facts, kind) in candidates {
            let capacity = u64::from(facts.container_capacity?.min(20));
            let children = self
                .items
                .iter()
                .filter(|item| item.location["parent"].as_str() == Some(parent.as_str()))
                .collect::<Vec<_>>();
            let occupied = children
                .iter()
                .filter(|item| {
                    !inputs
                        .iter()
                        .any(|line| line["item"] == item.identity && line["quantity_after"] == 0)
                })
                .count() as u64;
            if occupied < capacity {
                let ordinal = children
                    .iter()
                    .filter_map(|item| item.location["ordinal"].as_str()?.parse::<u64>().ok())
                    .max()
                    .unwrap_or(0)
                    .checked_add(1)?;
                return Some((parent.clone(), kind, ordinal));
            }
        }
        None
    }
}

// The fee owner burns direct-backpack coins in the same order and places its
// change before the chest. Project its occupancy without issuing a second debit.
fn fee_placement(
    coins: &[CoinStack],
    direct_count: usize,
    capacity: usize,
    gold: u64,
    tuple: Type2EventTuple,
    character: crate::domain::CharacterId,
) -> std::result::Result<(Vec<u64>, usize), &'static str> {
    let eligible = coins
        .iter()
        .map(|stack| u64::from(stack.quantity) * stack.coin.worth())
        .sum();
    let bank = bank_part(
        gold,
        eligible,
        tuple,
        crate::durability::bank::fee_payer_is_junior(character),
    )
    .map_err(|_| "INSUFFICIENT_GOLD")?;
    let paid = gold - bank;
    if paid == 0 {
        return Ok((Vec::new(), 0));
    }
    let plan =
        plan_fee_within(paid, coins, direct_count, capacity).map_err(|error| match error {
            FeePlanError::InsufficientFunds => "INSUFFICIENT_GOLD",
            FeePlanError::CapacityExceeded | FeePlanError::ChangeDoesNotFit => "NO_ROOM",
            FeePlanError::InvalidInput => "INVALID_INPUT",
        })?;
    let retired = plan
        .lines
        .iter()
        .filter_map(|line| {
            let coin = &coins[line.input];
            (coin.quantity == line.burned).then_some(coin.placement_ordinal)
        })
        .collect();
    Ok((retired, plan.change_outputs()))
}

impl Inventory {
    fn chest_location(
        &self,
        content: &dyn ItemLoopContent,
        inputs: &[Value],
        gold: u64,
        tuple: Type2EventTuple,
        character: crate::domain::CharacterId,
    ) -> std::result::Result<Value, &'static str> {
        let removed = |item: &Item| {
            inputs.iter().enumerate().any(|(index, line)| {
                line["item"] == item.identity && (index < 2 || line["quantity_after"] == 0)
            })
        };
        let mut children = std::collections::HashMap::<&str, Vec<(u64, &Item)>>::new();
        for item in self.items.iter().filter(|item| !removed(item)) {
            let parent = item.location["parent"].as_str().ok_or("INVALID_INPUT")?;
            let ordinal = item.location["ordinal"]
                .as_str()
                .and_then(|value| value.parse().ok())
                .ok_or("INVALID_INPUT")?;
            children.entry(parent).or_default().push((ordinal, item));
        }
        for entries in children.values_mut() {
            entries.sort_unstable_by_key(|entry| std::cmp::Reverse(entry.0));
        }

        let capacity = self
            .bag_facts
            .container_capacity
            .ok_or("UNSUPPORTED_ITEM")? as usize;
        let direct = children
            .get(self.bag_identity.as_str())
            .map_or(&[][..], Vec::as_slice);
        let coins = direct
            .iter()
            .filter_map(|(ordinal, item)| {
                Some(CoinStack {
                    coin: Coin::from_production_key(&item.facts.definition.production_key)?,
                    quantity: item.quantity,
                    placement_ordinal: *ordinal,
                })
            })
            .collect::<Vec<_>>();
        let (burned, change) =
            fee_placement(&coins, direct.len(), capacity, gold, tuple, character)?;
        let retired_resources = inputs
            .iter()
            .filter(|line| line["quantity_after"] == 0)
            .count();
        if self.items.len() + change + 1 - retired_resources - burned.len() > 500 {
            return Err("NO_ROOM");
        }

        let mut queue =
            std::collections::VecDeque::from([(self.bag_identity.as_str(), capacity, 0)]);
        // Crystal Player::queryDestination searches carried containers breadth-first.
        while let Some((parent, capacity, depth)) = queue.pop_front() {
            let entries = children.get(parent).map_or(&[][..], Vec::as_slice);
            let (occupied, last) = if depth == 0 {
                let count = entries.len() - burned.len() + change;
                let last = if change == 0 {
                    entries
                        .iter()
                        .find(|(ordinal, _)| !burned.contains(ordinal))
                        .map_or(0, |entry| entry.0)
                } else {
                    entries
                        .first()
                        .map_or(0, |entry| entry.0)
                        .checked_add(change as u64)
                        .ok_or("NO_ROOM")?
                };
                (count, last)
            } else {
                (entries.len(), entries.first().map_or(0, |entry| entry.0))
            };
            if occupied < capacity.min(GAMEITEM01_CONTAINER_ENTRIES_MAX as usize) {
                let ordinal = last.checked_add(1).ok_or("NO_ROOM")?;
                return Ok(
                    json!({"kind": if depth == 0 { 1 } else { 2 }, "parent":parent,
                    "ordinal":ordinal.to_string()}),
                );
            }
            // The resulting chest and its contents occupy the following two levels.
            if depth + 2 < ITEM_LOOP_DEPTH_MAX {
                queue.extend(entries.iter().filter_map(|(_, item)| {
                    let definition = &item.facts.definition;
                    if content.is_quiver(definition)
                        || definition.production_key == "oteryn:item.tibia.i23721"
                    {
                        return None;
                    }
                    Some((
                        item.identity.as_str(),
                        item.facts.container_capacity? as usize,
                        depth + 1,
                    ))
                }));
            }
        }
        Err("NO_ROOM")
    }
}

pub(super) async fn chest_output(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    native: &NativeGameplayState,
    inventory: &Inventory,
    content: &dyn ItemLoopContent,
    inputs: &[Value],
    gold: u64,
) -> NativeResult<std::result::Result<Value, &'static str>> {
    let Some(definition) = current_definition(native, EXALTATION_CHEST_KEY) else {
        return Ok(Err("UNSUPPORTED_ITEM"));
    };
    let location = match inventory.chest_location(
        content,
        inputs,
        gold,
        tx.tuple().tuple(),
        fence.character_id,
    ) {
        Ok(location) => location,
        Err(reason) => return Ok(Err(reason)),
    };
    let item: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
        .fetch_one(&mut ***tx)
        .await?;
    Ok(Ok(json!({"item":item,"key":definition.production_key,
        "revision":definition.revision_ref,"count":1,"location":location})))
}

pub(super) async fn carried_capacity_check(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterItemFence,
    content: &dyn ItemLoopContent,
) -> NativeResult<std::result::Result<(), &'static str>> {
    let carried = match carried_weight(
        tx,
        fence.character_id,
        *fence.runtime_scope.world_id().as_bytes(),
        content,
    )
    .await
    {
        Ok(weight) => weight,
        Err(LoopError::Refused(_)) => return Ok(Err("UNSUPPORTED_ITEM")),
        Err(LoopError::Durability(error)) => return Err(error),
        Err(LoopError::Database(error)) => return Err(error.into()),
    };
    let capacity = read_derived_transfer_capacity_centi_oz(tx, fence.character_id, content).await?;
    Ok(if carried > capacity {
        Err("OVERWEIGHT")
    } else {
        Ok(())
    })
}

pub(super) async fn produce(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterItemFence,
    content: &dyn ItemLoopContent,
    inventory: &Inventory,
    definition: &TypedDefinitionRef,
    amount: u32,
    inputs: &mut Vec<Value>,
) -> NativeResult<std::result::Result<Vec<Value>, &'static str>> {
    let Some(facts) = content.loop_facts(definition) else {
        return Ok(Err("UNSUPPORTED_ITEM"));
    };
    let Ok((maximum, _)) = super::super::super::item_transfer::stack_maximum(facts.stack) else {
        return Ok(Err("UNSUPPORTED_ITEM"));
    };
    let mut remaining = amount;
    for item in inventory.items.iter().filter(|item| {
        item.plain && item.facts.definition == *definition && item.quantity < maximum
    }) {
        let count = remaining.min(maximum - item.quantity);
        inputs.push(item.line(item.quantity + count, ItemTier::ZERO));
        remaining -= count;
        if remaining == 0 {
            break;
        }
    }
    let mut outputs = Vec::new();
    if remaining > 0 {
        // PR2785 Q3: a fresh output needs a slot before consuming any source.
        let Some((parent, kind, ordinal)) = inventory.free_slot(&[]) else {
            return Ok(Err("NO_ROOM"));
        };
        if remaining > maximum
            || inventory.items.len() + 1
                - inputs
                    .iter()
                    .filter(|line| line["quantity_after"] == 0)
                    .count()
                > 500
        {
            return Ok(Err("NO_ROOM"));
        }
        let item: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
            .fetch_one(&mut **tx)
            .await?;
        outputs.push(json!({"item":item,"key":definition.production_key,"revision":definition.revision_ref,"count":remaining,"location":{"kind":kind,"parent":parent,"ordinal":ordinal.to_string()}}));
    }
    let carried = match carried_weight(
        tx,
        fence.character_id,
        *fence.runtime_scope.world_id().as_bytes(),
        content,
    )
    .await
    {
        Ok(weight) => weight,
        Err(LoopError::Refused(_)) => return Ok(Err("UNSUPPORTED_ITEM")),
        Err(LoopError::Durability(error)) => return Err(error),
        Err(LoopError::Database(error)) => return Err(error.into()),
    };
    let capacity = read_derived_transfer_capacity_centi_oz(tx, fence.character_id, content).await?;
    let mut after = i128::from(carried);
    for line in inputs.iter() {
        let Some(item) = inventory
            .items
            .iter()
            .find(|item| line["item"] == item.identity)
        else {
            return Err(DurabilityError::InvalidStoredState);
        };
        let Some(weight) = item.facts.weight else {
            return Ok(Err("UNSUPPORTED_ITEM"));
        };
        after += (i128::from(
            line["quantity_after"]
                .as_u64()
                .ok_or(DurabilityError::InvalidStoredState)?,
        ) - i128::from(item.quantity))
            * i128::from(weight);
    }
    let Some(weight) = facts.weight else {
        return Ok(Err("UNSUPPORTED_ITEM"));
    };
    after += i128::from(remaining) * i128::from(weight);
    if after < 0 || after > i128::from(capacity) {
        return Ok(Err("OVERWEIGHT"));
    }
    Ok(Ok(outputs))
}
