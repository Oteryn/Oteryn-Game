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
