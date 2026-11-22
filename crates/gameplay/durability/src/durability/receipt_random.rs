//! One bounded decision per durable owner occurrence. Call after receipt lookup.
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

use super::DurabilityError;
use crate::domain::CharacterId;

pub(super) async fn draw(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    (occurrence, ordinal): ([u8; 16], u64),
    purpose: &str,
    faces: u8,
) -> Result<u8, DurabilityError> {
    if faces == 0 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let secret: Vec<u8> =
        sqlx::query_scalar("SELECT secret FROM game_receipt_random_root WHERE singleton")
            .fetch_one(&mut **tx)
            .await?;
    let secret: [u8; 32] = secret
        .try_into()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let bound = u64::from(faces);
    let threshold = bound.wrapping_neg() % bound;
    let occurrence = DecisionOccurrenceId::from_bytes(occurrence);
    for retry in 0_u64..16 {
        let root = GameplayDecisionRoot::from_bytes(
            Sha256::new()
                .chain_update(b"oteryn:receipt-random:v1")
                .chain_update(secret)
                .chain_update(character.as_bytes())
                .chain_update(retry.to_be_bytes())
                .finalize()
                .into(),
        );
        let sample = deterministic_decision_u64(&root, occurrence, purpose, ordinal)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        // Reject the incomplete residue set so all faces are equally likely.
        if sample >= threshold {
            return u8::try_from(1 + sample % bound)
                .map_err(|_| DurabilityError::InvalidStoredState);
        }
    }
    Err(DurabilityError::InvalidStoredState)
}
