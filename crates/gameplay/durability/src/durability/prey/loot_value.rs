//! PREY-DIFF measurement reads immutable committed mint facts, not mutable inventory.
use super::super::{
    DurabilityError, DurabilityRoot,
    character_authority::{ReconciledCharacterAuthority, assert_recovery_fence},
    character_progression::CharacterProgressionError as Error,
    db::{begin_semantic_transaction, commit_semantic_transaction},
    item_mint::CommittedItemMint,
    item_mint_audit,
};
use crate::domain::CharacterId;
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::collections::BTreeMap;

impl DurabilityRoot {
    /// Value only the committed descendants of the principal's own corpse. The mint
    /// reservation preserves each original quantity even after looting, splitting or
    /// retirement. Partial loot settlement therefore measures its committed part.
    pub async fn read_prey_committed_loot_value(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        corpse: &CommittedItemMint,
        prices: &BTreeMap<String, u64>,
    ) -> Result<u64, Error> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| Error::AuthorityRejected)?;
        let corpse = corpse.clone();
        let prices = prices.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                let owns: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM game_item_mint_receipts WHERE item_instance_id=encode($1,'hex')::uuid AND transaction_id=encode($2,'hex')::uuid AND corpse_top_damage_character_id=encode($3,'hex')::uuid AND envelope_sha256=$4)"
                )
                .bind(corpse.item_instance_id.as_slice())
                .bind(corpse.transaction_id.as_slice())
                .bind(character.as_bytes().as_slice())
                .bind(corpse.envelope_sha256.as_slice())
                .fetch_one(&mut *tx).await?;
                if !owns {
                    return Ok(Err(Error::AuthorityRejected));
                }
                let maximum = crate::foundation::loot_data::GAMEITEM01_CORPSE_CONTAINER_ENTRIES_MAX;
                let rows = sqlx::query(
                    "WITH RECURSIVE drops AS (\
                     SELECT item_instance_id,transaction_id,envelope_sha256 FROM game_item_mint_receipts WHERE destination_parent_item_instance_id=encode($1,'hex')::uuid \
                     UNION SELECT r.item_instance_id,r.transaction_id,r.envelope_sha256 FROM game_item_mint_receipts r JOIN drops d ON r.destination_parent_item_instance_id=d.item_instance_id) \
                     SELECT v.envelope,d.envelope_sha256 FROM drops d JOIN game_item_mint_reservations v USING(item_instance_id,transaction_id) LIMIT $2"
                )
                .bind(corpse.item_instance_id.as_slice())
                .bind(i64::try_from(maximum + 1).map_err(|_| DurabilityError::InvalidStoredState)?)
                .fetch_all(&mut *tx).await?;
                if rows.len() > maximum {
                    return Ok(Err(Error::InvalidInput));
                }
                let mut gold = 0_u64;
                for row in rows {
                    let bytes: Vec<u8> = row.try_get("envelope")?;
                    let digest: Vec<u8> = row.try_get("envelope_sha256")?;
                    if Sha256::digest(&bytes).as_slice() != digest {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let (_, mint) = item_mint_audit::decode_envelope(&bytes)
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    let item = mint.after.ok_or(DurabilityError::InvalidStoredState)?;
                    let definition = item.definition.ok_or(DurabilityError::InvalidStoredState)?;
                    gold = add_value(gold, &definition.production_key, item.quantity, &prices)?;
                }
                commit_semantic_transaction(tx, deadline).await?;
                Ok(Ok(gold))
            })
        }).await?
    }
}

fn add_value(
    total: u64,
    item: &str,
    count: u32,
    prices: &BTreeMap<String, u64>,
) -> Result<u64, DurabilityError> {
    // An absent item has no NPC buyer in the generated content; market prices are irrelevant.
    let unit = prices.get(item).copied().unwrap_or(0);
    unit.checked_mul(u64::from(count))
        .and_then(|value| total.checked_add(value))
        .ok_or(DurabilityError::InvalidStoredState)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prey_actual_loot_counts_quantity_and_ignores_items_without_npc_buyer()
    -> Result<(), DurabilityError> {
        let prices = BTreeMap::from([("gold-coin".into(), 1), ("gem".into(), 400)]);
        assert_eq!(add_value(17, "gem", 3, &prices)?, 1217);
        assert_eq!(add_value(1217, "gold-coin", 43, &prices)?, 1260);
        assert_eq!(add_value(1260, "untradeable", 1, &prices)?, 1260);
        assert!(add_value(u64::MAX, "gold-coin", 1, &prices).is_err());
        Ok(())
    }
}
