//! Current cast reads without an escaping equipment writer capability.
use super::super::character_wheel::{CurrentWheelCastRead, WheelRuleset};
use super::*;

/// Bound to one physical transaction; exposes Character/Wheel reads only.
/// Its private equipment authority cannot be passed to any Item writer.
pub struct CastEquipmentReader {
    authority: EquipmentAuthority,
}

impl CastEquipmentReader {
    pub async fn read_facts(
        &self,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<RawCastDurableFacts> {
        read_raw_cast_facts_in_transaction(tx, &self.authority).await
    }

    /// Read the projectile owner in this cast transaction when the held
    /// weapon uses ammunition. Container identities come from the locked read.
    pub async fn read_facts_with_quiver<C: super::super::item_loop_move::ItemLoopContent>(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        content: &C,
        needs_ammunition: impl FnOnce(&EquipmentSnapshot) -> bool,
    ) -> Result<(RawCastDurableFacts, Vec<EquippedItem>)> {
        let facts = self.read_facts(tx).await?;
        if facts.equipment.content_digest != content.loop_content_digest() {
            return Err(rejected("cast quiver content"));
        }
        let ammunition = if needs_ammunition(&facts.equipment) {
            read_quiver_entries(tx, &self.authority.world, &facts.equipment, content)
                .await
                .map_err(EquipmentError::from)?
        } else {
            Vec::new()
        };
        Ok((facts, ammunition))
    }

    pub async fn read_wheel(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        ruleset: &WheelRuleset,
    ) -> std::result::Result<CurrentWheelCastRead, DurabilityError> {
        super::super::character_wheel::read_current_allocation_in_transaction(
            tx,
            &self.authority,
            ruleset,
        )
        .await
    }
}

pub async fn read_cast_equipment_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    recovery: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    command: CommandRef,
    content_digest: [u8; 32],
) -> Result<CastEquipmentReader> {
    let authority = if matches!(
        fence.runtime_scope,
        crate::foundation::RuntimeScopeRefV1::Instance { .. }
    ) {
        if command.game_session_id() != fence.game_session_id {
            return Err(rejected("activity cast command session"));
        }
        let record = recovery
            .record_for(root)
            .map_err(|_| rejected("activity cast recovery root"))?;
        let mut authority = activity_read::assert_current_activity_equipment_in_transaction(
            tx,
            &record,
            node,
            fence,
            content_digest,
        )
        .await?;
        authority.command = Some(command);
        authority
    } else {
        assert_equipment_authority_in_transaction(
            tx,
            root,
            recovery,
            node,
            fence,
            command,
            content_digest,
        )
        .await?
    };
    Ok(CastEquipmentReader { authority })
}
