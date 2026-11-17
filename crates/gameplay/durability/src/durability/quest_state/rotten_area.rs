//! Sealed native Rotten onThink input to the existing sole Quest writer.
//! The current Scope/Map reader constructs this input; no wire command does.
use super::*;
mod current;
pub use current::{
    CurrentRottenAreaPlayerFacts, PreparedRottenAreaRead, prepare_current_rotten_area_read,
    read_current_rotten_area_players_in_transaction,
    read_prepared_rotten_area_players_in_transaction,
};
use sqlx::Row;
mod source;
pub use source::{QualifiedRottenAreaSource, QualifiedRottenAreaTarget};

#[derive(Debug, Clone)]
pub struct QualifiedRottenAreaThink {
    fence: CurrentCharacterGameplayFence,
    cause: SourceThinkOccurrence,
    mask: u8,
    clocks_before: [i64; 3],
    original_binding: [u8; 32],
}
impl QualifiedRottenAreaThink {
    pub fn request(&self, fence: &CurrentCharacterGameplayFence) -> Result<QuestTransitionRequest> {
        if self.fence.item_fence() != fence.item_fence() {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        let transition_key =
            oteryn_gameplay_quests::quest::rotten_blood::area_icons::transition(self.mask)
                .ok_or(CharacterProgressionError::InvalidInput)?;
        Ok(QuestTransitionRequest {
            transition_key,
            cause: QuestCause::SourceThink(self.cause),
        })
    }
    pub fn binding(&self, character: CharacterId, request: &QuestTransitionRequest) -> [u8; 33] {
        let mut binding = request_binding(character, request);
        let mut hash = Sha256::new();
        hash.update(binding);
        hash.update(b"oteryn:rotten-area-native-think:v1");
        hash.update(self.original_binding);
        for value in self.clocks_before {
            hash.update(value.to_be_bytes());
        }
        hash.update([self.mask]);
        binding[1..].copy_from_slice(&hash.finalize());
        binding
    }
    /// The enclosing sole writer already proves the current recovery, Node and
    /// gameplay fence. Lock the exact three predecessor KV rows before mutation.
    pub(super) async fn validate_current_in_transaction(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        fence: CurrentCharacterGameplayFence,
    ) -> StoredResult<bool> {
        if self.fence.item_fence() != fence.item_fence() {
            return Ok(false);
        }
        let tracks = oteryn_gameplay_quests::quest::rotten_blood::area_icons::TRACKS;
        let rows = sqlx::query("SELECT track_key,value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=ANY($2::text[]) ORDER BY track_key FOR UPDATE")
            .bind(fence.character_id.as_bytes().as_slice()).bind(tracks.to_vec()).fetch_all(&mut **tx).await?;
        let mut observed = [0_i64; 3];
        for row in rows {
            let key: String = row.try_get("track_key")?;
            let index = tracks
                .iter()
                .position(|candidate| *candidate == key)
                .ok_or(DurabilityError::InvalidStoredState)?;
            observed[index] = row.try_get("value")?;
        }
        Ok(observed == self.clocks_before)
    }
}

impl DurabilityRoot {
    pub async fn commit_rotten_area_think(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        input: QualifiedRottenAreaThink,
        catalogue: Arc<QuestStateCatalogue>,
    ) -> Result<QuestTransitionOutcome> {
        let request = input.request(&fence)?;
        self.commit_quest_inner::<_, 0>(
            authority,
            node,
            fence,
            request,
            catalogue,
            None,
            None,
            None,
            None,
            &grizzly::NoGrizzlyPremium,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(super::QuestSourceAction::RottenArea(input)),
        )
        .await
    }
}
