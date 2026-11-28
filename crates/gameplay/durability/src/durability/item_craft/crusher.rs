//! GEM-CRUSH-1 uses the fenced craft receipt and the ordinary timed-item writers.
use super::super::character_progression::CurrentCharacterGameplayFence;
use super::super::item_mint::{TypedDefinitionRef, uuid_text};
use super::super::item_mint_audit::Type2Transaction;
use super::super::item_timed_state::{
    TimedCheckpointRequest, TimedDefinitionFacts, TimedExpiryRequest, load_timed_state,
    write_checkpoint_with_nested_crusher, write_expiry_with_nested_crusher,
};
use super::*;
use crate::domain::{
    CharacterRevision,
    timed_item::{ExpireReason, TimedValues},
};
use oteryn_gameplay_content::content::ReferenceItemField::Known;
use oteryn_gameplay_progression::wheel_gem_data::WheelGemData;

pub const CRUSHER_KEY: &str = "oteryn:item.tibia.i46627";
pub(super) const RECIPE: &str = "oteryn:gem-atelier.crush";

pub(super) struct Charge {
    item: [u8; 16],
    definition: TimedDefinitionFacts,
    revision: u64,
    before: u32,
    transaction: String,
    nested_location: Option<Value>,
}

impl Charge {
    pub(super) fn retires(&self) -> bool {
        self.before == 1
    }

    pub(super) fn frees_output_slot(&self) -> bool {
        self.retires() && self.nested_location.is_none()
    }

    pub(super) fn proof(&self) -> Value {
        json!({
            "item": self.item,
            "definition": self.definition.definition.production_key,
            "definition_revision": self.definition.definition.revision_ref,
            "expected_revision": self.revision,
            "charges_before": self.before,
            "charges_after": self.before - 1,
            "transaction": self.transaction,
            "location": self.nested_location,
        })
    }

    pub(super) async fn spend(
        self,
        tx: &mut Type2Transaction<'_>,
        fence: CurrentCharacterItemFence,
        craft_transaction: &str,
        occurred_at_unix_ms: i64,
    ) -> Result<(), DurabilityError> {
        let transaction_id = uuid_text(&self.transaction)?;
        if let Some(location) = &self.nested_location {
            sqlx::query(
                "INSERT INTO game_item_crusher_nested_tool_proofs
                    (craft_transaction_id,timed_transaction_id,item_instance_id,location)
                 VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,$4)",
            )
            .bind(craft_transaction)
            .bind(&self.transaction)
            .bind(self.item.as_slice())
            .bind(location)
            .execute(&mut ***tx)
            .await?;
        }
        let nested = self.nested_location.is_some();
        if !self.retires() {
            let after = TimedValues::new(Some(self.before - 1), None)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            write_checkpoint_with_nested_crusher(
                tx,
                fence.character_id,
                &TimedCheckpointRequest {
                    item_instance_id: self.item,
                    expected_revision: self.revision,
                    transaction_id,
                    definition: self.definition,
                    values: after,
                },
                nested,
            )
            .await
            .map_err(timed_error)?;
        } else {
            let (revision, event): (String, String) = sqlx::query_as(
                "SELECT character_revision::text,game_character_uuid_v7()::text
                   FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
            )
            .bind(fence.character_id.as_bytes().as_slice())
            .fetch_one(&mut ***tx)
            .await?;
            let gameplay = CurrentCharacterGameplayFence {
                character_id: fence.character_id,
                game_session_id: fence.game_session_id,
                connection_generation: fence.connection_generation,
                character_lease_generation: fence.character_lease_generation,
                runtime_scope: fence.runtime_scope,
                scope_ownership_generation: fence.scope_ownership_generation,
                expected_character_revision: CharacterRevision::new(
                    revision
                        .parse()
                        .map_err(|_| DurabilityError::InvalidStoredState)?,
                )
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            };
            let tuple = tx.tuple();
            write_expiry_with_nested_crusher(
                tx,
                &gameplay,
                &TimedExpiryRequest {
                    item_instance_id: self.item,
                    expected_revision: self.revision,
                    transaction_id,
                    event_id: uuid_text(&event)?,
                    reason: ExpireReason::ChargesExhausted,
                    definition: self.definition,
                    target: None,
                    occurred_at_unix_ms,
                    server_build_id: super::super::character_authority::SERVER_BUILD_ID.into(),
                },
                tuple,
                None,
                nested,
            )
            .await
            .map_err(timed_error)?;
        }
        Ok(())
    }
}

fn timed_error(error: super::super::item_timed_state::TimedWriteError) -> DurabilityError {
    match error {
        super::super::item_timed_state::TimedWriteError::Unavailable(error) => error,
        _ => DurabilityError::InvalidStoredState,
    }
}

pub(super) async fn prepare(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &ItemCraftRequest,
    native: &NativeGameplayState,
    gem: &TypedDefinitionRef,
) -> Result<Result<(Charge, CookingConversion), UseDisposition>, DurabilityError> {
    let (Some(item), Some(tool)) = (request.tool, &request.station) else {
        return Ok(Err(UseDisposition::Rejected));
    };
    if item == request.source
        || tool.family != "Item"
        || request.ingredient.is_some()
        || request.gathering.is_some()
    {
        return Ok(Err(UseDisposition::Rejected));
    }
    let Some(policy) = native.item_policy(CRUSHER_KEY, &tool.revision_ref) else {
        return Ok(Err(UseDisposition::StaleState));
    };
    let Known(charges) = &policy.record().semantics.charges else {
        return Ok(Err(UseDisposition::StaleState));
    };
    let Known(full_charges) = charges.count else {
        return Ok(Err(UseDisposition::StaleState));
    };
    let full = TimedValues::new(Some(full_charges), None)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let world = fence.runtime_scope.world_id();
    let direct: bool = sqlx::query_scalar(
        "SELECT EXISTS(
           SELECT 1 FROM game_item_instances i
             JOIN game_item_container_entries e USING(item_instance_id)
             JOIN game_item_container_slots b ON b.character_id=e.character_id
               AND b.item_instance_id=e.parent_item_instance_id AND b.world_id=e.world_id
            WHERE i.item_instance_id=encode($1,'hex')::uuid AND e.character_id=encode($2,'hex')::uuid
              AND i.world_id=encode($3,'hex')::uuid AND i.world_id=e.world_id
              AND i.definition_family='Item' AND i.definition_production_key=$4
              AND i.definition_revision_ref=$5 AND i.lifecycle=1 AND i.quantity=1
            FOR UPDATE OF i)",
    )
    .bind(item.as_slice())
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(world.as_bytes().as_slice())
    .bind(CRUSHER_KEY)
    .bind(&tool.revision_ref)
    .fetch_one(&mut ***tx)
    .await?;
    let nested_location = if direct {
        None
    } else {
        let Some(row) = event_containers::nested_source(tx, fence, item, *world.as_bytes()).await?
        else {
            return Ok(Err(UseDisposition::StaleState));
        };
        if row.try_get::<String, _>("definition_family")? != "Item"
            || row.try_get::<String, _>("definition_production_key")? != CRUSHER_KEY
            || row.try_get::<String, _>("definition_revision_ref")? != tool.revision_ref
            || row.try_get::<i64, _>("quantity")? != 1
        {
            return Ok(Err(UseDisposition::StaleState));
        }
        Some(row.try_get::<Value, _>("nested_source")?)
    };
    let (revision, before) = match load_timed_state(tx, &item).await.map_err(timed_error)? {
        None => (0, full_charges),
        Some(state) => {
            let Some(charges) = state
                .values
                .filter(|values| values.remaining_ms().is_none())
                .and_then(TimedValues::charges)
                .filter(|count| (1..=full_charges).contains(count))
            else {
                return Ok(Err(UseDisposition::StaleState));
            };
            if state.deadline_at.is_some() {
                return Ok(Err(UseDisposition::StaleState));
            }
            (state.revision, charges)
        }
    };
    if gem.family != "Item"
        || native
            .item_policy(&gem.production_key, &gem.revision_ref)
            .is_none()
    {
        return Ok(Err(UseDisposition::StaleState));
    }
    let data = WheelGemData::embedded().map_err(|_| DurabilityError::InvalidStoredState)?;
    let Some((output_key, low, high)) = fragment_yield(&data, &gem.production_key) else {
        return Ok(Err(UseDisposition::NothingToUse));
    };
    let Some(mut outputs) = gathering_outputs(native, Some(output_key)) else {
        return Ok(Err(UseDisposition::NotPossible));
    };
    let secret: Vec<u8> =
        sqlx::query_scalar("SELECT secret FROM game_atelier_decision_root WHERE singleton=true")
            .fetch_one(&mut ***tx)
            .await?;
    if secret.len() != 32 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let root = Sha256::new()
        .chain_update(secret)
        .chain_update(b"gem_crush")
        .chain_update(fence.character_id.as_bytes())
        .chain_update(data.revision().as_bytes())
        .finalize();
    let binding = Sha256::new()
        .chain_update(request.command.game_session_id().as_bytes())
        .chain_update(request.command.command_id().get().to_be_bytes())
        .finalize();
    let mut occurrence = [0; 16];
    occurrence.copy_from_slice(&binding[..16]);
    let draw = deterministic_decision_u64(
        &GameplayDecisionRoot::from_bytes(root.into()),
        DecisionOccurrenceId::from_bytes(occurrence),
        "gem_crush",
        0,
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let [(_, count)] = outputs.as_mut_slice() else {
        return Err(DurabilityError::InvalidStoredState);
    };
    *count = u32::try_from(oteryn_gameplay_domain::domain::uniform_draw::uniform_draw(
        draw, low, high,
    ))
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let transaction = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
        .fetch_one(&mut ***tx)
        .await?;
    Ok(Ok((
        Charge {
            item,
            definition: TimedDefinitionFacts {
                definition: tool.clone(),
                full,
                lit_continuous: false,
            },
            revision,
            before,
            transaction,
            nested_location,
        },
        CookingConversion {
            recipe: RECIPE.into(),
            outputs,
        },
    )))
}

fn fragment_yield(data: &WheelGemData, gem: &str) -> Option<(u16, i64, i64)> {
    let vocations = data.gems()["vocations"].as_object()?;
    vocations.values().find_map(|vocation| {
        let items = &vocation["gem_family"]["items"];
        // Fandom's archived Wheel of Destiny, Fragment Workshop, unrevealed row.
        [
            ("lesser", 46625, 1, 4),
            ("regular", 46625, 2, 8),
            ("greater", 46626, 1, 4),
        ]
        .into_iter()
        .find_map(|(quality, fragment, low, high)| {
            (items[quality].as_str() == Some(gem)).then_some((fragment, low, high))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crusher_global_ranges_cover_each_vocation() -> Result<(), Box<dyn std::error::Error>> {
        let data = WheelGemData::embedded()?;
        let vocations = data.gems()["vocations"]
            .as_object()
            .ok_or("vocation catalogue")?;
        for vocation in vocations.values() {
            let items = &vocation["gem_family"]["items"];
            for (quality, expected) in [
                ("lesser", (46625, 1, 4)),
                ("regular", (46625, 2, 8)),
                ("greater", (46626, 1, 4)),
            ] {
                let gem = items[quality].as_str().ok_or("gem identity")?;
                assert_eq!(fragment_yield(&data, gem), Some(expected));
            }
        }
        assert_eq!(fragment_yield(&data, CRUSHER_KEY), None);
        assert_eq!(fragment_yield(&data, "oteryn:item.tibia.i46625"), None);
        Ok(())
    }
}
