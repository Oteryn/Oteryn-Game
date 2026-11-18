//! The map timer advances only the character that paid for its source USE.
use super::*;

#[derive(Clone, Debug)]
pub(in crate::durability) struct PresenceProgress {
    source: String,
    transition: String,
    quest: String,
    command: CommandRef,
}

impl PresenceProgress {
    pub(super) fn request(
        &self,
        fence: &CurrentCharacterGameplayFence,
    ) -> Result<QuestTransitionRequest> {
        if self.command.game_session_id() != fence.game_session_id {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        Ok(QuestTransitionRequest {
            transition_key: self.transition.clone(),
            cause: QuestCause::Use(self.command),
        })
    }

    pub(super) async fn paid(
        &self,
        tx: &mut sqlx::PgConnection,
        character: CharacterId,
    ) -> StoredResult<bool> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM game_character_quest_receipts \
             WHERE character_id=encode($1,'hex')::uuid AND cause_id=encode($2,'hex')::uuid \
               AND cause_ordinal=$3::text::numeric AND cause_kind='use' \
               AND transition_key=$4 AND quest_key=$5)",
        )
        .bind(character.as_bytes().as_slice())
        .bind(self.command.game_session_id().as_bytes().as_slice())
        .bind(self.command.command_id().get().to_string())
        .bind(&self.source)
        .bind(&self.quest)
        .fetch_one(tx)
        .await?)
    }
}

impl DurabilityRoot {
    #[allow(
        clippy::too_many_arguments,
        reason = "binds both the character fence and the paid world occurrence"
    )]
    pub async fn commit_paid_presence_progress(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        catalogue: Arc<QuestStateCatalogue>,
        source: &str,
        command: CommandRef,
    ) -> Result<QuestTransitionOutcome> {
        let source = catalogue
            .authored_world_callbacks()
            .and_then(|callbacks| callbacks.iter().find(|callback| callback.key == source))
            .ok_or(CharacterProgressionError::InvalidInput)?;
        let transition = source
            .presence_progress_transition()
            .ok_or(CharacterProgressionError::InvalidInput)?;
        if catalogue.transition(&transition.key) != Some(&transition) {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let input = PresenceProgress {
            source: source.key.clone(),
            transition: transition.key,
            quest: transition.quest,
            command,
        };
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
            Some(QuestSourceAction::Presence(input)),
        )
        .await
    }
}
