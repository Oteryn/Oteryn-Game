//! Quest reward facts use the existing account appearance owner and immutable receipts.
//! The caller has already fenced and locked the granting Character and accepted its Quest
//! transition in this same transaction. A recorded grant does not manufacture a catalogue:
//! selection still requires an admitted, explicit grant mapping compatible with its provenance.
use crate::domain::CharacterId;
use crate::durability::DurabilityError;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum QuestOutfitGrant {
    /// Crystal 201189f data-global/npc/flickering_soul.lua:122-138.
    Revenant,
    /// PR2784 answer6547536027: base only; no unverified addon.
    DecayingDefender,
}

impl QuestOutfitGrant {
    fn key(self) -> &'static str {
        match self {
            Self::Revenant => "oteryn:appearance-grant/quest/soul-war/revenant-base",
            Self::DecayingDefender => {
                "oteryn:appearance-grant/quest/rotten-blood/decaying-defender-base"
            }
        }
    }

    fn provenance(self) -> &'static str {
        match self {
            Self::Revenant => "crystal-201189f-soul-war-revenant-v1",
            Self::DecayingDefender => "owner-pr2784-decaying-defender-base-v1",
        }
    }
}

fn identity(
    character: CharacterId,
    occurrence: &[u8],
    grant: QuestOutfitGrant,
) -> Result<([u8; 16], Vec<u8>), DurabilityError> {
    if occurrence.is_empty() || occurrence.len() > 256 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut event = b"oteryn:quest-outfit-grant:v1\0".to_vec();
    event.extend_from_slice(character.as_bytes());
    event.extend_from_slice(&(occurrence.len() as u16).to_be_bytes());
    event.extend_from_slice(occurrence);
    event.extend_from_slice(grant.key().as_bytes());
    let digest = Sha256::digest(&event);
    let mut id = [0; 16];
    id.copy_from_slice(&digest[..16]);
    // This is a deterministic receipt UUID, not a new time-ordered actor or session ID.
    id[6] = (id[6] & 0x0f) | 0x80;
    id[8] = (id[8] & 0x3f) | 0x80;
    Ok((id, event))
}

/// Return true only when this transaction earns a new account entitlement. Concurrent
/// Characters on the same account serialize on the same grant key; a second completion
/// retains the original receipt. The caller must roll back on every error.
pub(crate) async fn grant_quest_outfit(
    tx: &mut Transaction<'_, Postgres>,
    character: CharacterId,
    occurrence: &[u8],
    grant: QuestOutfitGrant,
) -> Result<bool, DurabilityError> {
    let (id, event) = identity(character, occurrence, grant)?;
    let account: sqlx::types::Uuid = sqlx::query_scalar(
        "SELECT account_id FROM game_character_roots \
         WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_bytes().as_slice())
    .fetch_one(&mut **tx)
    .await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(format!("oteryn:quest-appearance:{account}:{}", grant.key()))
        .execute(&mut **tx)
        .await?;
    let owned: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_account_appearance_unlocks \
         WHERE account_id=$1 AND unlock_key=$2)",
    )
    .bind(account)
    .bind(grant.key())
    .fetch_one(&mut **tx)
    .await?;
    if owned {
        return Ok(false);
    }
    sqlx::query(
        "INSERT INTO game_account_appearance_unlock_receipts \
         (grant_occurrence_id,account_id,unlock_key,content_revision,granting_character_id,source_kind,source_event_id) \
         VALUES(encode($1,'hex')::uuid,$2,$3,$4,encode($5,'hex')::uuid,'quest',$6)",
    )
    .bind(id.as_slice())
    .bind(account)
    .bind(grant.key())
    .bind(grant.provenance())
    .bind(character.as_bytes().as_slice())
    .bind(event)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_account_appearance_unlocks(account_id,unlock_key,grant_occurrence_id) \
         VALUES($1,$2,encode($3,'hex')::uuid)",
    )
    .bind(account)
    .bind(grant.key())
    .bind(id.as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn character(n: u8) -> Result<CharacterId, DurabilityError> {
        let mut bytes = [n; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        CharacterId::from_bytes(bytes).map_err(|_| DurabilityError::InvalidStoredState)
    }

    #[test]
    fn quest_outfit_receipt_identity_binds_character_event_and_exact_base_grant() -> Result<(), DurabilityError> {
        let a = identity(character(1)?, &[7; 32], QuestOutfitGrant::Revenant)?;
        assert_eq!(
            a,
            identity(character(1)?, &[7; 32], QuestOutfitGrant::Revenant)?
        );
        for other in [
            identity(character(2)?, &[7; 32], QuestOutfitGrant::Revenant)?,
            identity(character(1)?, &[8; 32], QuestOutfitGrant::Revenant)?,
            identity(character(1)?, &[7; 32], QuestOutfitGrant::DecayingDefender)?,
        ] {
            assert_ne!(a.0, other.0);
            assert_ne!(a.1, other.1);
        }
        assert!(identity(character(1)?, &[], QuestOutfitGrant::Revenant).is_err());
        assert!(identity(character(1)?, &[1; 257], QuestOutfitGrant::Revenant).is_err());
        assert!(identity(character(1)?, &[1; 256], QuestOutfitGrant::Revenant).is_ok());
        Ok(())
    }
}
