//! ADMIT0 field eligibility at the existing locked new-write boundary.
use super::spell_item_transaction::{SpellItemAuthority, SpellItemError};
use super::spell_items_abi::{
    SpellFieldClass, SpellFieldOrigin, SpellItemOperation, SpellItemTransactionRequest,
};
use sqlx::{Postgres, Transaction};

pub(super) async fn validate_new_write(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    request: &SpellItemTransactionRequest,
    origin: &SpellFieldOrigin,
) -> Result<(), SpellItemError> {
    let creates_field = request.operations.iter().any(|operation| match operation {
        SpellItemOperation::MintGround { definition, .. }
        | SpellItemOperation::MintInventory { definition, .. } => {
            definition.field_class != SpellFieldClass::NotField
        }
        _ => false,
    });
    if !creates_field {
        return Ok(());
    }
    let created=request.operations.iter().filter(|operation|matches!(operation,SpellItemOperation::MintGround{definition,..} if definition.field_class!=SpellFieldClass::NotField)).count();
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = authority.runtime_scope()
    else {
        return Err(SpellItemError::FieldCapabilityRefused);
    };
    let removed: Vec<Vec<u8>> = request
        .operations
        .iter()
        .filter_map(|operation| match operation {
            SpellItemOperation::RemoveGround(item) => Some(item.item_instance_id.to_vec()),
            _ => None,
        })
        .collect();
    let active:i64=sqlx::query_scalar("SELECT count(*) FROM (SELECT 1 FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) JOIN game_spell_item_lines l ON l.item_instance_id=i.item_instance_id AND l.transaction_id=i.minted_transaction_id AND l.operation_kind=1 WHERE g.world_id=encode($1,'hex')::uuid AND g.channel_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND NOT (uuid_send(i.item_instance_id)=ANY($3::bytea[])) AND (l.expires_at_unix_ms IS NOT NULL OR EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules p WHERE p.item_instance_id=i.item_instance_id AND p.source_transaction_id=l.transaction_id)) LIMIT $4) fields")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(&removed).bind(super::field_removal::CHANNEL_FIELDS_MAX+1).fetch_one(&mut **tx).await?;
    if active
        .checked_add(i64::try_from(created).map_err(|_| SpellItemError::FieldCapabilityRefused)?)
        .is_none_or(|count| count > super::field_removal::CHANNEL_FIELDS_MAX)
    {
        return Err(SpellItemError::Rejected("channel field count bound"));
    }
    match origin {
        SpellFieldOrigin::Player(check) => {
            if request.caster_origin.is_none()
                || !super::fresh_admission::FreshAdmissionStore::from_root(authority.root.clone())
                    .check_current_runtime_capabilities(
                        tx,
                        authority.runtime_scope(),
                        authority.scope_generation(),
                        check,
                    )
                    .await?
            {
                return Err(SpellItemError::FieldCapabilityRefused);
            }
        }
        SpellFieldOrigin::Monster | SpellFieldOrigin::System => {
            // A positive player creation history cannot be relabelled as an exemption.
            if request.caster_origin.is_some() {
                return Err(SpellItemError::FieldCapabilityRefused);
            }
        }
    }
    Ok(())
}
