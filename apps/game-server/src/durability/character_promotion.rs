//! #2256: the durable promotion read (migration 0137, PREMIUM_ACTIVATION §4.2).
//!
//! `game_character_promotions` holds one row per promoted Character, written only by the D1106
//! NPC promotion purchase in the transaction that advances the CharacterRevision. Every
//! gameplay reader takes it from here, on the transaction that holds its own current fence:
//! the death writer (D76), the login projection (A4.2), the Wheel change (WHEEL-0 §6.1) and the
//! Bestiary Charm facts. A promotion is never revoked, so the read needs no row lock: the
//! Character root lock each caller holds orders it after any purchase that committed first.

use super::DurabilityError;
use super::character_progression::{CharacterProgressionError, CurrentCharacterGameplayFence};
use super::character_revision_sequencer::SequencedOutcome;
use super::item_mint_audit::Type2Transaction;
use super::npc_purchase::{NpcPurchaseFuture, NpcPurchaseOperation, NpcPurchaseWriteContext};
use crate::domain::{CharacterId, CharacterRevision};
use sqlx::PgConnection;

/// Whether `character` holds a durable promotion, read on the caller's transaction.
pub(crate) async fn read_durable_promotion(
    connection: &mut PgConnection,
    character: CharacterId,
) -> Result<bool, DurabilityError> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_character_promotions \
          WHERE character_id = encode($1,'hex')::uuid AND promoted)",
    )
    .bind(character.as_bytes().as_slice())
    .fetch_one(connection)
    .await?)
}

/// The durable promotion a fenced read found. It is a fact of one transaction, not a permit.
#[allow(
    dead_code,
    reason = "standalone durability suites path-load this module without the login caller"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DurablePromotion(pub(crate) bool);

impl SequencedOutcome for DurablePromotion {
    fn committed_revision(&self) -> Option<CharacterRevision> {
        None
    }
}

/// The login read (A4.2): the durable promotion under the exact current gameplay fence, in the
/// Character's revision slot. Nothing is written.
#[allow(
    dead_code,
    reason = "standalone durability suites path-load this module without the login caller"
)]
pub(crate) struct DurablePromotionRead {
    pub(crate) fence: CurrentCharacterGameplayFence,
}

impl NpcPurchaseOperation for DurablePromotionRead {
    type Output = DurablePromotion;
    type Error = CharacterProgressionError;

    fn apply<'a, 't>(
        self,
        context: &'a NpcPurchaseWriteContext,
        tx: &'a mut Type2Transaction<'t>,
    ) -> NpcPurchaseFuture<'a, Self::Output, Self::Error>
    where
        Self: 'a,
    {
        Box::pin(async move {
            context.admit_read(tx, &self.fence).await?;
            let promoted = read_durable_promotion(tx, self.fence.character_id).await?;
            Ok(DurablePromotion(promoted))
        })
    }
}
