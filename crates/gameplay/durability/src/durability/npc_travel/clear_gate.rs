//! A negative quest-track prerequisite for guarded return passages.
use crate::domain::CharacterId;
use crate::durability::DurabilityError;
use crate::durability::item_mint_audit::Type2Transaction;
use sha2::{Digest, Sha256};

pub(super) async fn permitted(
    tx: &mut Type2Transaction<'_>,
    character: CharacterId,
    track: &str,
) -> Result<bool, DurabilityError> {
    // Match the Quest track owner's key grammar. A malformed key must not turn
    // an outstanding-fine condition into an always-clear absent-row lookup.
    sqlx::query_scalar("SELECT $2::text ~ '^oteryn:[A-Za-z0-9._:/-]+$' AND octet_length($2::text)<=128 AND NOT EXISTS(SELECT 1 FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2 AND value<>0)")
        .bind(character.as_bytes().as_slice()).bind(track).fetch_one(&mut ***tx).await.map_err(Into::into)
}

pub(super) fn bind(hash: &mut Sha256, track: &str) {
    // Call only for Some: old route receipt bindings remain byte-identical.
    hash.update(b"quest-clear-gate:v1");
    hash.update((track.len() as u64).to_be_bytes());
    hash.update(track.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m38_travel_clear_gate_binding_distinguishes_tracks() {
        let mut before = Sha256::new();
        before.update(b"legacy paid travel material");
        let mut first = before.clone();
        bind(&mut first, "oteryn:quest-progress/fine_a");
        let mut second = before.clone();
        bind(&mut second, "oteryn:quest-progress/fine_b");
        assert_ne!(before.finalize(), first.clone().finalize());
        assert_ne!(first.finalize(), second.finalize());
    }
}
