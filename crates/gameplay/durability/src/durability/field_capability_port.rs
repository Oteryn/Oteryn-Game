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
    let active:i64=sqlx::query_scalar("SELECT game_live_channel_field_count(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::bytea[])")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(&removed).fetch_one(&mut **tx).await?;
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
