//! Existing bounded carried coin physical owner, shared by explicit source causes only.
use super::*;
use crate::domain::currency::FeeBurnLine;
use serde_json::{Value, json};
pub(super) const COIN_INPUTS_MAX: usize = 508;
pub(super) struct PhysicalInputs {
    pub(super) inputs: Vec<(CoinStack, [u8; 16], String)>,
    pub(super) sources: Vec<Value>,
    pub(super) equipment_before: Option<Value>,
    pub(super) equipment_already_advanced: bool,
}

pub(super) async fn lock_inputs<C: FeeChangeInput>(
    connection: &mut PgConnection,
    fence: &CurrentCharacterGameplayFence,
    request: &FeeBurnRequest<C>,
    world: WorldId,
    equipment_before: Option<Value>,
    equipment_already_advanced: bool,
) -> Result<PhysicalInputs> {
    let rows = sqlx::query(
        "WITH carried AS MATERIALIZED (SELECT * FROM game_atelier_carried_coin_sources(encode($1,'hex')::uuid,encode($2,'hex')::uuid)),
         locked AS MATERIALIZED (SELECT i.item_instance_id FROM game_item_instances i JOIN carried c USING(item_instance_id) ORDER BY i.item_instance_id FOR UPDATE OF i)
         SELECT c.item_instance_id::text,c.parent_item_instance_id::text,c.placement_ordinal::text,c.surface,c.slot,c.selection_order,
                to_jsonb(i) AS before,i.definition_revision_ref,i.definition_production_key,i.quantity
           FROM carried c JOIN locked l USING(item_instance_id) JOIN game_item_instances i USING(item_instance_id)
          ORDER BY c.selection_order",
    ).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).fetch_all(&mut *connection).await?;
    if rows.len() > COIN_INPUTS_MAX {
        return Err(FeeBurnError::CapacityExceeded);
    }
    let facts = request.change.facts().ok_or(FeeBurnError::InvalidInput)?;
    let mut inputs = Vec::with_capacity(rows.len());
    let mut sources = Vec::with_capacity(rows.len());
    for row in rows {
        let coin =
            Coin::from_production_key(&row.try_get::<String, _>("definition_production_key")?)
                .ok_or(DurabilityError::InvalidStoredState)?;
        let compatible = match coin {
            Coin::Gold => &facts.gold,
            Coin::Platinum => &facts.platinum,
            Coin::Crystal => &facts.crystal,
        };
        let revision: String = row.try_get("definition_revision_ref")?;
        if revision != compatible.revision_ref {
            return Err(FeeBurnError::InvalidInput);
        }
        let quantity = u32::try_from(row.try_get::<i64, _>("quantity")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        if !(1..=100).contains(&quantity) {
            return Err(DurabilityError::InvalidStoredState.into());
        }
        let item: String = row.try_get("item_instance_id")?;
        let ordinal = numeric_u64(&row, "placement_ordinal")?;
        let item_id = uuid_text(&item)?;
        let surface: i16 = row.try_get("surface")?;
        sources.push(json!({"item":item,"parent":row.try_get::<Option<String>,_>("parent_item_instance_id")?,"ordinal":ordinal,"surface":surface,"slot":row.try_get::<i16,_>("slot")?,"selection_order":row.try_get::<i64,_>("selection_order")?,"before":row.try_get::<Value,_>("before")?}));
        inputs.push((
            CoinStack {
                coin,
                quantity,
                placement_ordinal: ordinal,
            },
            item_id,
            revision,
        ));
    }
    Ok(PhysicalInputs {
        inputs,
        sources,
        equipment_before,
        equipment_already_advanced,
    })
}

pub(super) fn plan(
    fee: u64,
    stacks: &[CoinStack],
    carried: &PhysicalInputs,
    entries: usize,
    capacity: usize,
) -> Result<FeePlan> {
    if fee == 0 || stacks.len() != carried.sources.len() || stacks.len() > COIN_INPUTS_MAX {
        return Err(FeeBurnError::InvalidInput);
    }
    let mut order: Vec<usize> = (0..stacks.len()).collect();
    // std::sort leaves equal donor stacks unordered. Keep the server traversal order for ties.
    order.sort_by_key(|index| (stacks[*index].coin.worth(), stacks[*index].quantity));
    let mut remaining = fee;
    let mut lines = Vec::new();
    let mut change = 0;
    for input in order {
        if remaining == 0 {
            break;
        }
        let stack = stacks[input];
        if !(1..=100).contains(&stack.quantity) {
            return Err(DurabilityError::InvalidStoredState.into());
        }
        let burned = u64::from(stack.quantity).min(remaining.div_ceil(stack.coin.worth()));
        let worth = burned * stack.coin.worth();
        if worth >= remaining {
            change = worth - remaining;
            remaining = 0;
        } else {
            remaining -= worth;
        }
        lines.push(FeeBurnLine {
            input,
            burned: u32::try_from(burned).map_err(|_| FeeBurnError::InvalidInput)?,
        });
    }
    if remaining != 0 {
        return Err(FeeBurnError::InsufficientFunds);
    }
    let plan = FeePlan {
        lines,
        change,
        change_platinum: u32::try_from(change / 100).map_err(|_| FeeBurnError::InvalidInput)?,
        change_gold: u32::try_from(change % 100).map_err(|_| FeeBurnError::InvalidInput)?,
    };
    // Native change retains the existing fresh main-bag output contract. Nested/equipped
    // burns free no slot in that bag. Refusal rolls the entire Atelier transaction back.
    let freed = plan
        .lines
        .iter()
        .filter(|line| {
            line.burned == stacks[line.input].quantity
                && carried.sources[line.input]["surface"] == 1
        })
        .count();
    if entries
        .checked_sub(freed)
        .and_then(|n| n.checked_add(plan.change_outputs()))
        .is_none_or(|n| n > capacity)
    {
        return Err(FeeBurnError::ChangeDoesNotFit);
    }
    Ok(plan)
}

impl PhysicalInputs {
    pub(super) fn equipment_used(&self, plan: &FeePlan) -> bool {
        plan.lines
            .iter()
            .any(|line| self.sources[line.input]["surface"] == 3)
    }

    pub(super) async fn remove(
        &self,
        connection: &mut PgConnection,
        line: &BurnedCoinStack,
    ) -> Result<()> {
        let index = self
            .inputs
            .iter()
            .position(|(_, item, _)| *item == line.item_instance_id)
            .ok_or(DurabilityError::InvalidStoredState)?;
        let source = &self.sources[index];
        let query = match source["surface"].as_i64() {
            Some(1) => {
                "DELETE FROM game_item_container_entries WHERE item_instance_id=($1->>'item')::uuid AND parent_item_instance_id=($1->>'parent')::uuid AND placement_ordinal=($1->>'ordinal')::numeric"
            }
            Some(2) => {
                "DELETE FROM game_item_nested_entries WHERE item_instance_id=($1->>'item')::uuid AND parent_item_instance_id=($1->>'parent')::uuid AND placement_ordinal=($1->>'ordinal')::numeric"
            }
            Some(3) => {
                "DELETE FROM game_character_equipment_slots WHERE item_instance_id=($1->>'item')::uuid AND slot=($1->>'slot')::smallint"
            }
            _ => return Err(DurabilityError::InvalidStoredState.into()),
        };
        if sqlx::query(query)
            .bind(source)
            .execute(&mut *connection)
            .await?
            .rows_affected()
            != 1
        {
            return Err(DurabilityError::InvalidStoredState.into());
        }
        Ok(())
    }

    pub(super) async fn advance_equipment<C>(
        &self,
        connection: &mut PgConnection,
        fence: &CurrentCharacterGameplayFence,
        request: &FeeBurnRequest<C>,
        plan: &FeePlan,
    ) -> Result<()> {
        if self.equipment_already_advanced || !self.equipment_used(plan) {
            return Ok(());
        }
        let before = self
            .equipment_before
            .as_ref()
            .ok_or(DurabilityError::InvalidStoredState)?;
        if sqlx::query("UPDATE game_character_equipment_state SET revision=revision+1,last_transaction_id=encode($1,'hex')::uuid WHERE character_id=encode($2,'hex')::uuid AND revision=($3->>'revision')::numeric")
            .bind(request.transaction_id.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(before).execute(&mut *connection).await?.rows_affected() != 1 { return Err(DurabilityError::InvalidStoredState.into()); }
        Ok(())
    }
}
