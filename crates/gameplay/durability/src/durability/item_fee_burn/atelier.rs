//! Crystal 201189f game.cpp:2862-2969: carried coins, smaller denominations/stacks first,
//! then the bank. The same bounded native carried reader also serves source-bound Quest payments.
use super::*;
#[cfg(test)]
use crate::domain::currency::FeeBurnLine;
use serde_json::{Value, json};

// Native carried reader: 500 tree rows including the main bag, plus 9 equipment slots.
// At least one carried root is a container when descendants exist, hence 508 coin leaves.

pub(super) fn is_cause(cause: &FeeBurnCause) -> bool {
    matches!(
        cause,
        FeeBurnCause::QuestRewardPayment { .. } | FeeBurnCause::GemReveal { .. }
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

pub(super) use super::carried_coins::PhysicalInputs as CarriedInputs;
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
    super::carried_coins::lock_inputs(
        connection,
        fence,
        request,
        world,
        equipment_before,
        equipment_already_advanced,
    )
    .await
}
#[cfg(test)]
pub(super) use super::carried_coins::plan;

impl CarriedInputs {
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
            FeeBurnCauseV1::QuestRewardPayment(value) => value.purchase.as_mut().ok_or(FeeBurnError::InvalidInput)?,
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
