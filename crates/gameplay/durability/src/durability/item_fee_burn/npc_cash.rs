//! Closed horse and cosmetic NPC cash payments, using the existing physical coin owner.
use super::*;
use serde_json::{Value, json};
pub(super) fn is_cause(cause: &FeeBurnCause) -> bool {
    matches!(
        cause,
        FeeBurnCause::HorseRental { .. } | FeeBurnCause::NpcCosmeticPurchase { .. }
    )
}
/// Source removeMoney is carried-only; removeMoneyBank admits the residual.
/// The aggregate must first cover every cash-only effect from actual carried
/// value, even when a transition also contains a later bank-capable effect.
pub(super) fn minimum_carried_gold(cause: &FeeBurnCause) -> Result<u64> {
    match cause {
        FeeBurnCause::HorseRental { .. } => Ok(0),
        FeeBurnCause::NpcCosmeticPurchase { service, .. } => {
            super::super::npc_cosmetics::source_fee_cash_minimum(service)
                .ok_or(FeeBurnError::InvalidInput)
        }
        _ => Err(FeeBurnError::InvalidInput),
    }
}
pub(super) struct Inputs {
    pub(super) physical: super::carried_coins::PhysicalInputs,
    cost_transaction: Option<sqlx::types::Uuid>,
}
pub(super) async fn lock_inputs<C: FeeChangeInput>(
    connection: &mut PgConnection,
    fence: &CurrentCharacterGameplayFence,
    request: &FeeBurnRequest<C>,
    world: WorldId,
) -> Result<Inputs> {
    if !is_cause(&request.cause) {
        return Err(FeeBurnError::InvalidInput);
    }
    let current:Option<Value>=sqlx::query_scalar("SELECT to_jsonb(s) FROM game_character_equipment_state s WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
  .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *connection).await?;
    let prior: Option<(Value, sqlx::types::Uuid)> = if matches!(
        request.cause,
        FeeBurnCause::HorseRental { .. }
    ) {
        None
    } else {
        sqlx::query_as("SELECT equipment_before,cost_transaction_id FROM game_npc_cash_prior_equipment(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid)")
        .bind(request.transaction_id.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).fetch_optional(&mut *connection).await?
    };
    let cost_transaction = prior.as_ref().map(|(_, tx)| *tx);
    let before = prior.map(|(before, _)| before).or(current);
    let physical = super::carried_coins::lock_inputs(
        connection,
        fence,
        request,
        world,
        before,
        cost_transaction.is_some(),
    )
    .await?;
    Ok(Inputs {
        physical,
        cost_transaction,
    })
}
impl Inputs {
    pub(super) fn audit_cause<C>(
        &self,
        fence: &CurrentCharacterGameplayFence,
        request: &FeeBurnRequest<C>,
        plan: &FeePlan,
    ) -> Result<FeeBurnCauseV1> {
        let mut cause = request.cause.audit();
        let value = match &mut cause {
            FeeBurnCauseV1::HorseRental(v) | FeeBurnCauseV1::NpcCosmeticPurchase(v) => v,
            _ => return Err(FeeBurnError::InvalidInput),
        };
        let mut selected = Vec::with_capacity(plan.lines.len());
        for line in &plan.lines {
            let mut source = self.physical.sources[line.input].clone();
            source
                .as_object_mut()
                .ok_or(FeeBurnError::InvalidInput)?
                .remove("before");
            selected.push(source);
        }
        value.atelier_proof=serde_json::to_vec(&json!({"kind":"npc_carried_cash_v1","character":fence.character_id.as_bytes(),"occurrence":request.cause.occurrence(),"coin_inputs":selected})).map_err(|_|FeeBurnError::InvalidInput)?;
        Ok(cause)
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "each argument is one already-fenced physical payment witness"
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
        let mut sources = self.physical.sources.clone();
        for (index, source) in sources.iter_mut().enumerate() {
            let burned = plan
                .lines
                .iter()
                .find(|l| l.input == index)
                .map_or(0, |l| l.burned);
            source
                .as_object_mut()
                .ok_or(FeeBurnError::InvalidInput)?
                .insert("burned".into(), json!(burned));
        }
        let equipment = self
            .physical
            .equipment_used(plan)
            .then_some(self.physical.equipment_before.as_ref())
            .flatten();
        if self.physical.equipment_used(plan) && equipment.is_none() {
            return Err(DurabilityError::InvalidStoredState.into());
        }
        sqlx::query("INSERT INTO game_npc_fee_cash_records(transaction_id,character_id,world_id,backpack_item_instance_id,backpack_capacity,backpack_ordinals,inputs,equipment_before,equipment_cost_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,$6,$7,$8,$9)")
   .bind(request.transaction_id.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
   .bind(backpack.map(|id|id.to_vec())).bind(i16::try_from(capacity).map_err(|_|FeeBurnError::InvalidInput)?).bind(json!(entries)).bind(Value::Array(sources)).bind(equipment)
   .bind(equipment.and(self.cost_transaction)).execute(connection).await?;
        Ok(())
    }
}
