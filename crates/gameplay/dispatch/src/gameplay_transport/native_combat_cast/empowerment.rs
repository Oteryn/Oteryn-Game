//! Divine Empowerment mints timed items in the spell payment transaction. The paid caster
//! successor carries the matching footprint for outgoing damage at its current position.
use super::*;
use crate::durability::spell_items_abi::{
    QualifiedItemDefinition, SpellGroundTarget, SpellItemOperation,
};
use crate::spell::native_delayed::NativeDelayedPlan;

pub(super) fn operations(
    room: &QualifiedNativeEntryRoom,
    runtime: &ChannelRuntimeV1,
    content: &NativeGameplayState,
    batch: &OwnerCombatBatch,
    field: &NativeDelayedPlan,
) -> Result<Vec<SpellItemOperation>, SpellCastDisposition> {
    let NativeDelayedPlan::CreateField {
        positions,
        item_key,
        item_revision,
        expires_ms,
        ..
    } = field
    else {
        return reject();
    };
    let policy = content
        .item_policy(item_key, item_revision)
        .ok_or(SpellCastDisposition::NotAvailable)?;
    let flags = &policy.record().attributes;
    let definition = QualifiedItemDefinition::from_native_policy(policy)
        .map_err(|_| SpellCastDisposition::NotAvailable)?;
    if definition.movable
        || !definition.ground_destination
        || flags.blocks_movement != Some(false)
        || flags.blocks_projectile != Some(false)
    {
        return reject();
    }
    let lifetime = expires_ms
        .checked_sub(batch.now_ms)
        .ok_or(SpellCastDisposition::Rejected)?;
    let session = batch.command.game_session_id();
    let command = batch.command.command_id().get();
    let source = format!(
        "spell:{}:{command}",
        session
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    positions
        .iter()
        .enumerate()
        .map(|(ordinal, position)| {
            let target = SpellGroundTarget::for_native_tile_read(room, runtime, cell(*position))
                .map_err(|_| SpellCastDisposition::Rejected)?;
            let ordinal = u8::try_from(ordinal).map_err(|_| SpellCastDisposition::Rejected)?;
            Ok(SpellItemOperation::MintGround {
                item_instance_id: nonce(
                    b"oteryn:native-empowerment-item:v1",
                    batch.caster,
                    session,
                    command,
                    &[ordinal],
                ),
                definition: definition.clone(),
                quantity: 1,
                placement: target.placement(source.as_bytes().to_vec()),
                lifetime_millis: Some(
                    u32::try_from(lifetime).map_err(|_| SpellCastDisposition::Rejected)?,
                ),
                description: None,
                blocks_movement: false,
                blocks_projectile: false,
            })
        })
        .collect()
}
