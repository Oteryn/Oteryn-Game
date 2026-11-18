//! P01: a real corpse, its immutable minted race and a current carried skinning tool.
use super::*;
use oteryn_gameplay_progression::charm_effects::{CharmCatalogueRead, CharmEffect};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug)]
pub struct SkinningRequest {
    pub craft: ItemCraftRequest,
    pub lord_ashes: bool,
    pub corpse: [u8; 16],
    /// Runtime actor coordinates captured under the lane, never wire coordinates.
    pub actor_position: [i32; 3],
}

pub(super) struct Prepared {
    pub corpse: [u8; 16],
    pub before: Value,
    pub proof: Value,
    pub effect: Effect,
    pub conversion: oteryn_gameplay_items::item_craft::CookingConversion,
}

pub(super) enum Effect {
    Transform(TypedDefinitionRef),
    LordAshes(i16),
}

/// Positive relative chance, capped at certainty. The donor's declining-roll bug is rejected.
pub(super) fn chance_numerator(base: u32, bonus_hundredths: u32) -> u64 {
    (u64::from(base) * (10_000 + u64::from(bonus_hundredths))).min(1_000_000_000)
}

impl DurabilityRoot {
    #[allow(
        clippy::too_many_arguments,
        reason = "the existing craft owner receives its exact fence, content and corpse operand"
    )]
    pub async fn skin_corpse<C: ItemLoopContent>(
        &self,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: SkinningRequest,
        native: &NativeGameplayState,
        content: &C,
        catalogue: &oteryn_gameplay_content::content::charm_source::CanonicalCharmCatalogue,
        probe: bool,
    ) -> Result<Option<UseDisposition>, DurabilityError> {
        permit.check_scope(fence.runtime_scope)?;
        if request.craft.command.game_session_id() != fence.game_session_id
            || request.craft.intent.is_empty()
            || request.craft.intent.len()
                > oteryn_protocol_oteryn::world_object::MAX_USE_INTENT_BYTES
            || native.source_digest() != content.loop_content_digest()
            || request.craft.tool.is_some()
            || request.craft.ingredient.is_some()
            || request.craft.station.is_some()
            || request.craft.gathering.is_some()
            || request.craft.enchanting.is_some()
        {
            return Ok(Some(UseDisposition::Rejected));
        }
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = (native, content, catalogue, permit);
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder,deadline,context| Box::pin(async move {
            let mut tx=begin_type2_transaction(holder,deadline).await?;
            super::super::character_authority::assert_recovery_fence(&mut tx,&record).await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:item-craft:' || encode($1,'hex') || ':' || $2,0))")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.craft.command.command_id().get().to_string()).execute(&mut **tx).await?;
            let previous:Option<(Vec<u8>,Vec<u8>)>=sqlx::query_as("SELECT intent,uuid_send(character_id) FROM game_item_craft_records WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.craft.command.command_id().get().to_string()).fetch_optional(&mut **tx).await?;
            if let Some((intent,character))=previous {
                let result=if intent==request.craft.intent && character==fence.character_id.as_bytes() { UseDisposition::Committed } else { UseDisposition::Rejected };
                tx.commit(deadline).await?; return Ok(Some(result));
            }
            if probe { tx.commit(deadline).await?; return Ok(None); }
            if assert_current_equipment_in_transaction(&mut tx,&record,&node,&fence,Some(request.craft.command),context.1.loop_content_digest()).await.is_err() {
                return Ok(Some(UseDisposition::Rejected));
            }
            let result=super::write_with_skinning(&mut tx,context.3,fence,&request.craft,context.0,context.1,Some((&request,context.2))).await?;
            if result==UseDisposition::Committed { tx.commit(deadline).await?; }
            Ok(Some(result))
        })).await
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "the single prepared corpse proof keeps provenance, custody, chance and source output qualification together"
)]
pub(super) async fn prepare(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &SkinningRequest,
    tool: &TypedDefinitionRef,
    native: &NativeGameplayState,
    catalogue: &oteryn_gameplay_content::content::charm_source::CanonicalCharmCatalogue,
) -> Result<Option<Prepared>, DurabilityError> {
    let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex') || encode($2,'hex'),33))",
    )
    .bind(world.as_bytes().as_slice())
    .bind(channel.as_bytes().as_slice())
    .execute(&mut ***tx)
    .await?;
    if request.lord_ashes {
        return prepare_lord_ashes(tx, fence, request, tool, native).await;
    }
    let row=sqlx::query("SELECT to_jsonb(i) AS before_item,i.definition_production_key,g.spatial_position,p.race_key,p.race_definition_revision FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id,world_id) JOIN game_corpse_source_races p USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid AND g.channel_id=encode($3,'hex')::uuid AND g.runtime_scope_ownership_generation=$4::text::numeric AND i.lifecycle=1 AND i.quantity=1 AND p.mint_transaction_id=i.minted_transaction_id AND NOT EXISTS(SELECT 1 FROM game_corpse_skinning_records s WHERE s.corpse_item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_timed_states t WHERE t.item_instance_id=i.item_instance_id) FOR UPDATE OF i,g")
        .bind(request.corpse.as_slice()).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(fence.scope_ownership_generation.get().to_string()).fetch_optional(&mut ***tx).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let position: Vec<u8> = row.try_get("spatial_position")?;
    if !in_reach(request, &position)? {
        return Ok(None);
    }
    let source_id = |key: &str| {
        key.strip_prefix("oteryn:item.tibia.i")
            .and_then(|v| v.parse::<u16>().ok())
    };
    let corpse_key: String = row.try_get("definition_production_key")?;
    let before: Value = row.try_get("before_item")?;
    if before["definition_revision_ref"]
        .as_str()
        .is_none_or(|revision| {
            native
                .item_index()
                .and_then(|index| index.revision(&corpse_key))
                != Some(revision)
        })
    {
        return Ok(None);
    }
    let Some((target, yield_rule)) = source_id(&tool.production_key)
        .zip(source_id(&corpse_key))
        .and_then(|(tool, corpse)| oteryn_gameplay_items::fishing::gathering_rule(tool, corpse))
    else {
        return Ok(None);
    };
    let oteryn_gameplay_items::fishing::GatheringTarget::Corpse { spent_item } = target else {
        return Ok(None);
    };
    if spent_item == 0
        || native
            .item_policy(&tool.production_key, &tool.revision_ref)
            .is_none()
    {
        return Ok(None);
    }
    // This transforms an existing source corpse, not a new materialization grant.
    // The exact donor row supplies its spent definition; the current index supplies its revision.
    let spent_key = format!("oteryn:item.tibia.i{spent_item}");
    let Some(spent_revision) = native
        .item_index()
        .and_then(|index| index.revision(&spent_key))
    else {
        return Ok(None);
    };
    // A timed corpse must retain an exact supported original-source decay chain.
    // The same predicate fences the durable skinning receipt's admission guard.
    let temporal_allowed: bool = sqlx::query_scalar(
        "SELECT game_corpse_skinning_temporal_allowed(encode($1,'hex')::uuid,$2,$3)",
    )
    .bind(request.corpse.as_slice())
    .bind(&spent_key)
    .bind(spent_revision)
    .fetch_one(&mut ***tx)
    .await?;
    if !temporal_allowed {
        return Ok(None);
    }
    let spent = TypedDefinitionRef {
        family: "Item".into(),
        production_key: spent_key,
        revision_ref: spent_revision.to_owned(),
    };
    let race: String = row.try_get("race_key")?;
    let stage:Option<i16>=sqlx::query_scalar("SELECT u.unlocked_stage FROM game_character_charm_assignments a JOIN game_character_charm_unlocks u USING(character_id,charm_key) WHERE a.character_id=encode($1,'hex')::uuid AND a.race_key=$2 AND a.charm_key='oteryn:charm.scavenge'")
        .bind(fence.character_id.as_bytes().as_slice()).bind(&race).fetch_optional(&mut ***tx).await?;
    let bonus = stage
        .and_then(|s| u8::try_from(s).ok())
        .and_then(|s| {
            catalogue
                .charm("oteryn:charm.scavenge")
                .filter(|d| d.effect() == CharmEffect::SkinningChanceBonus)
                .and_then(|d| d.stage(s))
        })
        .map_or(0, |v| v.hundredths());
    let mut hash = Sha256::new();
    hash.update(fence.game_session_id.as_bytes());
    hash.update(request.craft.command.command_id().get().to_be_bytes());
    hash.update(request.corpse);
    let digest: [u8; 32] = hash.finalize().into();
    let mut occurrence = [0; 16];
    occurrence.copy_from_slice(&digest[..16]);
    let draw = deterministic_decision_u64(
        &GameplayDecisionRoot::from_bytes(native.source_digest()),
        DecisionOccurrenceId::from_bytes(occurrence),
        "oteryn.skinning.v1",
        0,
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let success = u128::from(draw) * 1_000_000_000
        < (u128::from(chance_numerator(yield_rule.threshold, bonus)) << 64);
    let mut outputs = Vec::new();
    if success {
        let key = format!("oteryn:item.tibia.i{}", yield_rule.item);
        let Some(revision) = native.item_index().and_then(|index| index.revision(&key)) else {
            return Ok(None);
        };
        let Some(output) = oteryn_gameplay_items::item_craft::output_definition(
            native,
            &oteryn_gameplay_content::content::ProjectV2DefinitionRef {
                family: oteryn_gameplay_content::content::ProjectV2Family::Item,
                key,
                revision: revision.to_owned(),
            },
        ) else {
            return Ok(None);
        };
        outputs.push((output, 1));
    }
    Ok(Some(Prepared {
        corpse: request.corpse,
        before,
        proof: json!({"source_race":race,"source_race_revision":row.try_get::<String,_>("race_definition_revision")?,
            "stage":stage,"bonus_hundredths":bonus,"base_threshold":yield_rule.threshold,"draw":draw}),
        effect: Effect::Transform(spent),
        conversion: oteryn_gameplay_items::item_craft::CookingConversion {
            recipe: "oteryn:skinning.crystal".into(),
            outputs,
        },
    }))
}

fn in_reach(request: &SkinningRequest, position: &[u8]) -> Result<bool, DurabilityError> {
    let cell = super::super::spell_items_abi::decode_ground_cell(position)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(request.actor_position[0].abs_diff(cell.x) <= 1
        && request.actor_position[1].abs_diff(cell.y) <= 1
        && request.actor_position[2] == -cell.z)
}

// Crystal skinning.lua:259–268 advances the Character's claim stage but
// leaves the ashes available to any other Character who reaches them.
async fn prepare_lord_ashes(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &SkinningRequest,
    tool: &TypedDefinitionRef,
    native: &NativeGameplayState,
) -> Result<Option<Prepared>, DurabilityError> {
    if tool.production_key != "oteryn:item.tibia.i5908"
        || native
            .item_policy(&tool.production_key, &tool.revision_ref)
            .is_none()
    {
        return Ok(None);
    }
    let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    let row: Option<(Value, Vec<u8>, i16)> = sqlx::query_as(
        "SELECT to_jsonb(i),g.spatial_position,a.phase
         FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id,world_id)
         CROSS JOIN LATERAL (SELECT game_lord_ashes_phase(i.item_instance_id) AS phase) a
         WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid
           AND g.channel_id=encode($3,'hex')::uuid AND g.scope_kind=1
           AND g.runtime_scope_ownership_generation=$4::text::numeric AND a.phase IS NOT NULL
           AND NOT EXISTS(SELECT 1 FROM game_lord_ashes_claims c
                          WHERE c.character_id=encode($5,'hex')::uuid AND c.phase=2)
         FOR UPDATE OF i,g",
    )
    .bind(request.corpse.as_slice())
    .bind(world.as_bytes().as_slice())
    .bind(channel.as_bytes().as_slice())
    .bind(fence.scope_ownership_generation.get().to_string())
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_optional(&mut ***tx)
    .await?;
    let Some((before, position, phase)) = row else {
        return Ok(None);
    };
    if !in_reach(request, &position)? {
        return Ok(None);
    }
    let output = oteryn_gameplay_items::item_craft::output_definition(
        native,
        &oteryn_gameplay_content::content::ProjectV2DefinitionRef {
            family: oteryn_gameplay_content::content::ProjectV2Family::Item,
            key: "oteryn:item.tibia.i954".into(),
            revision: "definition-r1".into(),
        },
    );
    Ok(output.map(|output| Prepared {
        corpse: request.corpse,
        before,
        proof: json!({"phase":phase}),
        effect: Effect::LordAshes(phase),
        conversion: oteryn_gameplay_items::item_craft::CookingConversion {
            recipe: "oteryn:skinning.lord-ashes".into(),
            outputs: vec![(output, 1)],
        },
    }))
}

pub(super) async fn record(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    prepared: &Prepared,
    transaction: &str,
) -> Result<(), DurabilityError> {
    let spent = match &prepared.effect {
        Effect::Transform(spent) => spent,
        Effect::LordAshes(phase) => {
            sqlx::query(
                "INSERT INTO game_lord_ashes_claims(transaction_id,character_id,corpse_item_instance_id,phase)
                 VALUES($1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4)",
            )
            .bind(transaction)
            .bind(fence.character_id.as_bytes().as_slice())
            .bind(prepared.corpse.as_slice())
            .bind(phase)
            .execute(&mut ***tx)
            .await?;
            return Ok(());
        }
    };
    let (_, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_corpse_skinning_records(transaction_id,corpse_item_instance_id,before_item,spent_key,spent_revision,channel_id,scope_ownership_generation) VALUES($1::uuid,encode($2,'hex')::uuid,$3,$4,$5,encode($6,'hex')::uuid,$7::text::numeric)")
        .bind(transaction).bind(prepared.corpse.as_slice()).bind(&prepared.before).bind(&spent.production_key).bind(&spent.revision_ref).bind(channel.as_bytes().as_slice()).bind(fence.scope_ownership_generation.get().to_string()).execute(&mut ***tx).await?;
    Ok(())
}

pub(super) async fn transform(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    prepared: &Prepared,
    transaction: &str,
) -> Result<(), DurabilityError> {
    let Effect::Transform(spent) = &prepared.effect else {
        return Ok(());
    };
    sqlx::query("UPDATE game_item_instances SET definition_production_key=$2,definition_revision_ref=$3,state_revision=state_revision+1,last_transaction_id=$4::uuid WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(prepared.corpse.as_slice()).bind(&spent.production_key).bind(&spent.revision_ref).bind(transaction).execute(&mut ***tx).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::chance_numerator;
    #[test]
    fn scavenge_stages_increase_relative_probability_exactly_and_cap_at_certainty() {
        assert_eq!(chance_numerator(7_000, 0), 70_000_000);
        assert_eq!(chance_numerator(7_000, 6_000), 112_000_000);
        assert_eq!(chance_numerator(7_000, 9_000), 133_000_000);
        assert_eq!(chance_numerator(7_000, 12_000), 154_000_000);
        assert_eq!(chance_numerator(50_000, 12_000), 1_000_000_000);
        assert_eq!(chance_numerator(1, 6_001), 16_001);
    }
}
