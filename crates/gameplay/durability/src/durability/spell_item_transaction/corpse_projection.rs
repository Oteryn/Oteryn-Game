//! Compose immutable loader-qualified Item DATA with a freshly locked, audited
//! materialized corpse. Neither history nor this projection grants mutation authority.
use super::*;

fn reference(
    value: &super::super::item_mint_audit::OneItemTypedDefinitionRevisionV1,
) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: value.family.clone(),
        production_key: value.production_key.clone(),
        revision_ref: value.revision_ref.clone(),
    }
}

fn qualified_flags(
    source: &crate::content::native_gameplay::NativeItemProfileSource,
    original: &TypedDefinitionRef,
    current: &TypedDefinitionRef,
) -> Result<[bool; 3]> {
    let mut definition = original.clone();
    let mut seen = std::collections::BTreeSet::new();
    // Visit up to 32 timed stages and their optional permanent terminal,
    // matching the existing qualified native Item decay-chain owner.
    for depth in 0..=32 {
        if !seen.insert((
            definition.production_key.clone(),
            definition.revision_ref.clone(),
        )) {
            return Err(SpellItemError::Rejected("cyclic corpse source Item chain"));
        }
        let policy = source
            .item_policy(&definition.production_key, &definition.revision_ref)
            .ok_or(SpellItemError::Rejected(
                "corpse source Item profile missing",
            ))?;
        let record = policy.record();
        let item = QualifiedItemDefinition::from_native_policy(policy)
            .map_err(SpellItemError::Rejected)?;
        if item.definition != definition
            || definition.family != "Item"
            || item.content_generation_digest != source.source_digest()
            || !item.materializable
            || !item.ground_destination
            || item.stack_maximum != 1
            || record.production_binding.disposition
                != crate::content::ProjectV2SourceIdentityDisposition::Exact
            || record.production_binding.target.key != definition.production_key
            || record.production_binding.target.revision != definition.revision_ref
        {
            return Err(SpellItemError::Rejected(
                "corpse source Item qualification mismatch",
            ));
        }
        if depth == 32 && item.decay.is_some() {
            return Err(SpellItemError::Rejected(
                "corpse source Item chain exceeds 32 stages",
            ));
        }
        if &definition == current {
            return Ok([
                record
                    .attributes
                    .blocks_movement
                    .ok_or(SpellItemError::Rejected("unknown corpse movement flag"))?,
                record
                    .attributes
                    .blocks_projectile
                    .ok_or(SpellItemError::Rejected("unknown corpse projectile flag"))?,
                record
                    .attributes
                    .immovable_block_solid
                    .ok_or(SpellItemError::Rejected(
                        "unknown corpse immovable blocking flag",
                    ))?,
            ]);
        }
        definition = item
            .decay
            .and_then(|decay| decay.target)
            .ok_or(SpellItemError::Rejected(
                "current Item is outside corpse source decay chain",
            ))?;
    }
    Err(SpellItemError::Rejected(
        "corpse source Item chain exceeds 32 stages",
    ))
}

pub(super) fn project_current_corpse_flags(
    row: &sqlx::postgres::PgRow,
    ownership_generation: u64,
    target: &SpellGroundTarget,
    flags: &mut [Option<bool>; 5],
) -> Result<()> {
    let Some(wire) = row.try_get::<Option<Vec<u8>>, _>("corpse_mint_envelope")? else {
        return Ok(()); // The unchanged decoder rejects missing ordinary Item flags.
    };
    let source = target
        .item_profile_source
        .as_ref()
        .ok_or(SpellItemError::Rejected(
            "corpse read lacks qualified Item source",
        ))?;
    let digest = source.source_digest();
    let content = format!(
        "sha256:{}",
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    if digest == [0; 32] || target.content_revision != content {
        return Err(SpellItemError::Rejected(
            "corpse Item source Content pin mismatch",
        ));
    }
    let (_, mint) = super::super::item_mint_audit::decode_envelope(&wire)
        .map_err(|_| SpellItemError::Rejected("corpse MINT envelope qualification"))?;
    let after = mint
        .after
        .as_ref()
        .ok_or(SpellItemError::Rejected("corpse MINT state absent"))?;
    let ground = mint
        .destination
        .as_ref()
        .ok_or(SpellItemError::Rejected("corpse MINT Ground absent"))?;
    let provenance = mint
        .source
        .as_ref()
        .ok_or(SpellItemError::Rejected("corpse MINT provenance absent"))?;
    let death = provenance
        .death_occurrence
        .as_ref()
        .ok_or(SpellItemError::Rejected("corpse MINT death absent"))?;
    let original = reference(
        after
            .definition
            .as_ref()
            .ok_or(SpellItemError::Rejected("corpse MINT definition absent"))?,
    );
    let cause_item = reference(
        provenance
            .loot_table
            .as_ref()
            .ok_or(SpellItemError::Rejected("corpse MINT cause Item absent"))?,
    );
    let id = uuid(&row.try_get::<String, _>("item_instance_id")?)?;
    let current_scope = row.try_get::<Vec<Option<String>>, _>("corpse_current_scope")?;
    let [world, channel, instance, kind] = current_scope.as_slice() else {
        return Err(SpellItemError::Rejected("corpse current scope shape"));
    };
    let current_world = uuid(
        world
            .as_deref()
            .ok_or(SpellItemError::Rejected("corpse current World absent"))?,
    )?;
    let current_channel = channel.as_deref().map(uuid).transpose()?;
    let current_instance = instance.as_deref().map(uuid).transpose()?;
    let scope_kind = kind
        .as_deref()
        .and_then(|kind| kind.parse::<i16>().ok())
        .ok_or(SpellItemError::Rejected("corpse current scope kind"))?;
    let scope_matches = match scope_kind {
        1 => {
            current_instance.is_none()
                && ground.instance_id.is_none()
                && current_channel
                    .is_some_and(|channel| ground.channel_id.as_slice() == channel.as_slice())
        }
        2 => {
            current_channel.is_none()
                && ground.channel_id.is_empty()
                && current_instance.is_some_and(|instance| {
                    ground.instance_id.as_deref() == Some(instance.as_slice())
                })
        }
        _ => false,
    };
    if provenance.loot_purpose_key != "CORPSE_MATERIALIZATION"
        || provenance.draw_ordinal != 0
        || original != cause_item
        || after.item_instance_id.as_slice() != id.as_slice()
        || after.world_id != ground.world_id
        || after.world_id.as_slice() != current_world.as_slice()
        || !scope_matches
        || after.quantity != 1
        || after.lifecycle != 1
        || !mint.before_semantically_absent
        || mint.corpse_container_entry.is_some()
        || provenance.content_revision != target.content_revision
        || ground.content_revision != target.content_revision
        || ground.map_revision != target.map_revision
        || ground.native_room_placement_context != target.placement_context
        || ground.runtime_scope_ownership_generation == 0
        || ground.runtime_scope_ownership_generation > ownership_generation
        || death.scope_ownership_generation != ground.runtime_scope_ownership_generation
    {
        return Err(SpellItemError::Rejected(
            "corpse MINT does not bind current scoped Content",
        ));
    }
    let qualified = qualified_flags(source, &original, &decoded_definition(row)?)?;
    for index in 0..3 {
        match flags[index] {
            Some(actual) if actual != qualified[index] => {
                return Err(SpellItemError::Rejected(
                    "corpse collision proof conflicts with current source",
                ));
            }
            Some(_) => {}
            None => flags[index] = Some(qualified[index]),
        }
    }
    Ok(())
}
