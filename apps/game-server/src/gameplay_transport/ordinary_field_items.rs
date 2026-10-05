//! Actual ordinary create_item effects share the original caster-cost SQL receipt.
//! Exact active Item source bindings select definitions, replaceable old fields,
//! and every duration/transform stage. No detached item map is introduced.
use super::*;
use crate::durability::spell_item_transaction::{self as item_tx, SpellItemError};
use crate::durability::spell_items_abi::SpellItemOperation;
use crate::spell::world_items_execution::QualifiedItemDefinition;

use crate::spell::world_items_execution::{
    qualify_source_field_chain as field_chain, source_magic_field as magic_field,
};

#[allow(clippy::too_many_arguments)]
pub(super) async fn append_creations(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    root: &crate::durability::DurabilityRoot,
    runtime: &ChannelRuntimeV1,
    room: &QualifiedNativeEntryRoom,
    objects: &LocalObjectRuntime,
    content: &NativeGameplayState,
    prepared: &mut PreparedNativeCombatCast,
    request: &mut crate::durability::spell_items_abi::SpellItemTransactionRequest,
) -> Result<(), SpellItemError> {
    if prepared.item_creations.is_empty() {
        return Ok(());
    }
    if prepared.item_creations.len() > MAX_WORLD_TILES {
        return Err(SpellItemError::Rejected("field footprint bound"));
    }
    let world = crate::durability::spell_field_policy::read_world_field_policy_in_transaction(
        tx,
        root,
        authority.runtime_scope(),
        authority.scope_generation(),
    )
    .await
    .map_err(SpellItemError::Durability)?
    .ok_or(SpellItemError::Rejected("field World policy unknown"))?;
    let policy = world.bind(&*tx);
    prepared.field_policy_revision = Some(policy.revision().to_owned());
    let mode = policy.mode();
    let mut positions = BTreeSet::new();
    for (ordinal, creation) in prepared.item_creations.iter().enumerate() {
        if !positions.insert(creation.position) {
            return Err(SpellItemError::Rejected("duplicate source field creation"));
        }
        let source = content
            .item_policy(&creation.item.key, &creation.item.revision)
            .ok_or(SpellItemError::Rejected(
                "created Item active binding missing",
            ))?;
        if !magic_field(source)? {
            return Err(SpellItemError::Rejected(
                "ordinary create_item source family",
            ));
        }
        let source_id = source
            .record()
            .production_binding
            .external_id
            .parse::<u32>()
            .map_err(|_| SpellItemError::Rejected("created field source identity"))?;
        let tile = crate::spell::world_execution::qualified_combat_tile_in_transaction(
            tx,
            authority,
            room,
            runtime,
            objects,
            cell(creation.position),
        )
        .await
        .map_err(|_| SpellItemError::Rejected("field current tile unknown"))?;
        let no_pvp = tile
            .no_pvp_zone()
            .ok_or(SpellItemError::Rejected("field tile PvP policy unknown"))?;
        if !tile.ground_present()
            || tile.block_solid()
            || tile.floor_change()
            || tile.protection_zone()
        {
            return Err(SpellItemError::Rejected("field target tile changed"));
        }
        // Canary999025 combat.cpp1189–1244 and utils_definitions.hpp589–603.
        let output = select_source_field_id(source_id, true, no_pvp, mode);
        // Source in-fight side effects require the current attack history owner;
        // ordinary damaging field creation is presently admitted only in safe
        // source contexts, never by inventing a non-PvP default.
        if !no_pvp && mode != crate::durability::spell_field_policy::FieldWorldType::NoPvp {
            return Err(SpellItemError::Rejected("field in-fight owner required"));
        }
        let selected =
            content
                .item_policy_for_source_id(output)
                .ok_or(SpellItemError::Rejected(
                    "selected field source binding missing",
                ))?;
        let flags = selected.record().attributes.clone();
        let qualified = field_chain(
            content,
            QualifiedItemDefinition::from_native_policy(selected)
                .map_err(SpellItemError::Rejected)?,
        )?;
        let target =
            SpellGroundTarget::for_native_tile_read(room, runtime, cell(creation.position))
                .map_err(SpellItemError::Rejected)?;
        let items = item_tx::read_spell_tile_in_transaction(tx, authority, &target).await?;
        for existing in items.items {
            let policy = content
                .item_policy(
                    &existing.definition.production_key,
                    &existing.definition.revision_ref,
                )
                .ok_or(SpellItemError::Rejected(
                    "existing tile Item source unknown",
                ))?;
            if magic_field(policy)? {
                match policy.record().attributes.field_replaceable {
                    Some(true) => {
                        let removal = item_tx::prepare_ground_removal_in_transaction(
                            tx,
                            authority,
                            &target,
                            existing.item_instance_id,
                        )
                        .await?;
                        request
                            .operations
                            .push(SpellItemOperation::RemoveGround(removal));
                    }
                    Some(false) => {
                        return Err(SpellItemError::Rejected("source field is not replaceable"));
                    }
                    None => {
                        return Err(SpellItemError::Rejected(
                            "source field replacement policy unknown",
                        ));
                    }
                }
                break; // Source replaces first down-item MagicField only.
            }
        }
        let id = nonce(
            b"oteryn:ordinary-field-item:v1",
            prepared.batch.caster,
            request.command.game_session_id(),
            request.command.command_id().get(),
            &u64::try_from(ordinal)
                .map_err(|_| SpellItemError::Rejected("field ordinal"))?
                .to_be_bytes(),
        );
        request.operations.push(SpellItemOperation::MintGround {
            item_instance_id: id,
            definition: qualified,
            quantity: 1,
            placement: target.placement(Vec::new()),
            lifetime_millis: None,
            description: None,
            blocks_movement: flags
                .blocks_movement
                .ok_or(SpellItemError::Rejected("field block source unknown"))?,
            blocks_projectile: flags
                .blocks_projectile
                .ok_or(SpellItemError::Rejected("field projectile source unknown"))?,
        });
    }
    Ok(())
}

/// Shared exact source Item identity normalization; current World/tile policy remains mandatory.
/// Pure helper grants no creature timer or durable Item authority.
pub(crate) fn select_source_field_id(
    source_id: u32,
    player_owned: bool,
    no_pvp: bool,
    mode: crate::durability::spell_field_policy::FieldWorldType,
) -> u32 {
    let pvp = match source_id {
        2123 => 2118,
        2124 => 2119,
        2125 => 2120,
        2126 => 2122,
        2121 => 105,
        other => other,
    };
    if player_owned
        && (no_pvp || mode == crate::durability::spell_field_policy::FieldWorldType::NoPvp)
    {
        match pvp {
            2118 => 21465,
            105 => 2134,
            2122 => 2135,
            other => other,
        }
    } else {
        pvp
    }
}
#[cfg(test)]
mod source_field_selection_tests {
    use super::*;
    use crate::durability::spell_field_policy::FieldWorldType as M;
    #[test]
    fn source_field_ids_apply_safe_world_conversion_only_to_actual_player_owned_casters() {
        for (source, safe) in [(2118, 21465), (2122, 2135), (105, 2134)] {
            assert_eq!(
                select_source_field_id(source, false, true, M::NoPvp),
                source
            );
            assert_eq!(select_source_field_id(source, true, false, M::Pvp), source);
            assert_eq!(select_source_field_id(source, true, true, M::Pvp), safe);
            assert_eq!(select_source_field_id(source, true, false, M::NoPvp), safe);
        }
        assert_eq!(
            select_source_field_id(2123, true, false, M::PvpEnforced),
            2118
        );
        assert_eq!(
            select_source_field_id(2123, true, true, M::PvpEnforced),
            21465
        );
    }
}
