//! Crystal 201189f game.cpp:2862-2969: carried coins, smaller denominations/stacks first,
//! then the bank. This child is reachable only from the three Atelier fee causes.
use super::*;
use crate::domain::currency::FeeBurnLine;
use serde_json::{Value, json};

// Native carried reader: 500 tree rows including the main bag, plus 9 equipment slots.
// At least one carried root is a container when descendants exist, hence 508 coin leaves.
const COIN_INPUTS_MAX: usize = 508;

pub(super) fn is_cause(cause: &FeeBurnCause) -> bool {
    matches!(
        cause,
        FeeBurnCause::GemReveal { .. }
            | FeeBurnCause::GemSwitchDomain { .. }
            | FeeBurnCause::GemGradeUp { .. }
    )
}

pub(super) fn fee_gold_units_max(tuple: Type2EventTuple) -> u64 {
    match tuple {
        Type2EventTuple::V1 => 508_000_000,
        Type2EventTuple::V2 => 1_000_507_999_999,
    }
}

pub(super) struct CarriedInputs {
    pub(super) inputs: Vec<(CoinStack, [u8; 16], String)>,
    sources: Vec<Value>,
    equipment_before: Option<Value>,
    equipment_already_advanced: bool,
}

pub(super) async fn lock_inputs<C: FeeChangeInput>(
    connection: &mut PgConnection,
    fence: &CurrentCharacterGameplayFence,
    request: &FeeBurnRequest<C>,
    world: WorldId,
) -> Result<CarriedInputs> {
    let equipment: Option<Value> = sqlx::query_scalar(
        "SELECT to_jsonb(s) FROM game_character_equipment_state s WHERE character_id=encode($1,'hex')::uuid FOR UPDATE",
    ).bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *connection).await?;
    let material_equipment: Option<Value> = sqlx::query_scalar(
        "SELECT equipment_before FROM game_atelier_item_records WHERE transaction_id=encode($1,'hex')::uuid AND created_xact_id=pg_current_xact_id()",
    ).bind(request.transaction_id.as_slice()).fetch_optional(&mut *connection).await?.flatten();
    // Material custody can already have advanced the same equipment epoch in this transaction.
    let equipment_already_advanced = material_equipment.is_some();
    let equipment_before = material_equipment.or(equipment);
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
    Ok(CarriedInputs {
        inputs,
        sources,
        equipment_before,
        equipment_already_advanced,
    })
}

pub(super) fn plan(
    fee: u64,
    stacks: &[CoinStack],
    carried: &CarriedInputs,
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

impl CarriedInputs {
    fn equipment_used(&self, plan: &FeePlan) -> bool {
        plan.lines
            .iter()
            .any(|line| self.sources[line.input]["surface"] == 3)
    }

    pub(super) fn audit_cause<C>(
        &self,
        fence: &CurrentCharacterGameplayFence,
        request: &FeeBurnRequest<C>,
        plan: &FeePlan,
    ) -> Result<FeeBurnCauseV1> {
        let mut cause = request.cause.audit();
        let value = match &mut cause {
            FeeBurnCauseV1::GemReveal(value)
            | FeeBurnCauseV1::GemSwitchDomain(value)
            | FeeBurnCauseV1::GemGradeUp(value) => value,
            _ => return Err(FeeBurnError::InvalidInput),
        };
        let mut proof: Value = if value.atelier_proof.is_empty() {
            json!({"character":fence.character_id.as_bytes(),"occurrence":request.cause.occurrence()})
        } else {
            serde_json::from_slice(&value.atelier_proof).map_err(|_| FeeBurnError::InvalidInput)?
        };
        let mut selected = Vec::with_capacity(plan.lines.len());
        for line in &plan.lines {
            let mut source = self.sources[line.input].clone();
            source
                .as_object_mut()
                .ok_or(FeeBurnError::InvalidInput)?
                .remove("before");
            selected.push(source);
        }
        proof
            .as_object_mut()
            .ok_or(FeeBurnError::InvalidInput)?
            .insert("coin_inputs".into(), Value::Array(selected));
        value.atelier_proof = serde_json::to_vec(&proof).map_err(|_| FeeBurnError::InvalidInput)?;
        Ok(cause)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "one source proof binds the existing fee transaction"
    )]
    pub(super) async fn record<C>(
        &self,
        connection: &mut PgConnection,
        fence: &CurrentCharacterGameplayFence,
        request: &FeeBurnRequest<C>,
        plan: &FeePlan,
        backpack: Option<[u8; 16]>,
        capacity: usize,
        entries: &[u64],
    ) -> Result<()> {
        let mut sources = self.sources.clone();
        for (index, source) in sources.iter_mut().enumerate() {
            let burned = plan
                .lines
                .iter()
                .find(|line| line.input == index)
                .map_or(0, |line| line.burned);
            source
                .as_object_mut()
                .ok_or(FeeBurnError::InvalidInput)?
                .insert("burned".into(), json!(burned));
        }
        let equipment = if self.equipment_used(plan) {
            Some(
                self.equipment_before
                    .as_ref()
                    .ok_or(DurabilityError::InvalidStoredState)?,
            )
        } else {
            None
        };
        sqlx::query("INSERT INTO game_atelier_fee_coin_records(transaction_id,character_id,world_id,backpack_item_instance_id,backpack_capacity,backpack_ordinals,inputs,equipment_before) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,$6,$7,$8)")
            .bind(request.transaction_id.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(backpack.map(|id| id.to_vec()))
            .bind(i16::try_from(capacity).map_err(|_| FeeBurnError::InvalidInput)?).bind(json!(entries)).bind(Value::Array(sources)).bind(equipment).execute(&mut *connection).await?;
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn carried(surfaces: &[u8]) -> CarriedInputs {
        CarriedInputs {
            inputs: Vec::new(),
            sources: surfaces
                .iter()
                .enumerate()
                .map(|(i, surface)| json!({"surface":surface,"selection_order":i+1}))
                .collect(),
            equipment_before: None,
            equipment_already_advanced: false,
        }
    }

    #[test]
    fn atelier_coins_use_denomination_then_stack_worth_and_physical_capacity() {
        let stacks = [
            CoinStack {
                coin: Coin::Crystal,
                quantity: 100,
                placement_ordinal: 8,
            },
            CoinStack {
                coin: Coin::Crystal,
                quantity: 13,
                placement_ordinal: 1,
            },
            CoinStack {
                coin: Coin::Gold,
                quantity: 50,
                placement_ordinal: 8,
            },
        ];
        let carried = carried(&[2, 2, 1]);
        assert!(matches!(
            plan(125_000, &stacks, &carried, 20, 20),
            Err(FeeBurnError::ChangeDoesNotFit)
        ));
        let result = plan(125_000, &stacks, &carried, 19, 20);
        assert!(result.is_ok());
        if let Ok(plan) = result {
            assert_eq!(
                plan.lines,
                vec![
                    FeeBurnLine {
                        input: 2,
                        burned: 50
                    },
                    FeeBurnLine {
                        input: 1,
                        burned: 13
                    }
                ]
            );
            assert_eq!(plan.change, 5_050);
        }
    }

    #[test]
    fn atelier_full_coin_grade_can_burn_more_than_twenty_stacks() {
        let stacks: Vec<_> = (1..=500)
            .map(|ordinal| CoinStack {
                coin: Coin::Crystal,
                quantity: 100,
                placement_ordinal: ordinal,
            })
            .collect();
        let carried = carried(&[2; 500]);
        let result = plan(75_000_000, &stacks, &carried, 0, 0);
        assert!(result.is_ok());
        if let Ok(plan) = result {
            assert_eq!(plan.lines.len(), 75);
            assert_eq!(plan.change, 0);
            assert!(plan.lines.iter().all(|line| line.burned == 100));
        }
    }

    #[test]
    fn atelier_nested_partial_coin_requires_a_real_change_slot() {
        let stacks = [CoinStack {
            coin: Coin::Crystal,
            quantity: 100,
            placement_ordinal: 500,
        }];
        let carried = carried(&[2]);
        for (entries, capacity) in [(0, 0), (20, 20)] {
            assert!(matches!(
                plan(125_000, &stacks, &carried, entries, capacity),
                Err(FeeBurnError::ChangeDoesNotFit)
            ));
        }
        let result = plan(125_000, &stacks, &carried, 19, 20);
        assert!(result.is_ok());
        if let Ok(plan) = result {
            assert_eq!(plan.lines[0].burned, 13);
            assert_eq!(plan.change_platinum, 50);
        }
    }
}
