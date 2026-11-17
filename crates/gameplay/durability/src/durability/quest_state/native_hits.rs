//! Fenced successful-hit credit backed by a native-minted retained damage witness.
use super::*;
use oteryn_gameplay_entities::runtime_actor_carrier::CommittedNativeHit;
use oteryn_gameplay_quests::quest::authored_bindings::hits;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuestHitOccurrence([u8; 16]);
impl QuestHitOccurrence {
    pub(super) fn from_bytes(bytes: [u8; 16]) -> StoredResult<Self> {
        if bytes[6] >> 4 != 7 || bytes[8] & 0xc0 != 0x80 {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(Self(bytes))
    }
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub struct NativeQuestHit {
    transition: String,
    creature: String,
    item: String,
    spell: Option<String>,
    observed_revision: u64,
    definition_hash: [u8; 32],
    witness: CommittedNativeHit,
}
impl NativeQuestHit {
    pub fn from_committed(
        transition: String,
        creature: String,
        item: String,
        spell: Option<String>,
        observed_revision: u64,
        definition_hash: [u8; 32],
        witness: CommittedNativeHit,
    ) -> Self {
        Self {
            transition,
            creature,
            item,
            spell,
            observed_revision,
            definition_hash,
            witness,
        }
    }
    pub fn occurrence(&self) -> QuestHitOccurrence {
        let (character, lease, session, sequence, sub) = self.witness.origin();
        let digest = Sha256::new()
            .chain_update(b"native-quest-hit-occurrence-v1")
            .chain_update(self.witness.attacker().placement_identity())
            .chain_update(self.witness.target().placement_identity())
            .chain_update(character.as_bytes())
            .chain_update(lease.to_be_bytes())
            .chain_update(session.as_bytes())
            .chain_update(sequence.to_be_bytes())
            .chain_update(sub.to_be_bytes())
            .chain_update(self.witness.ordinal().to_be_bytes())
            .finalize();
        let mut bytes = [0; 16];
        bytes.copy_from_slice(&digest[..16]);
        bytes[6] = (bytes[6] & 0x0f) | 0x70;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        QuestHitOccurrence(bytes)
    }
    pub(super) fn valid(
        &self,
        request: &QuestTransitionRequest,
        catalogue: &QuestStateCatalogue,
    ) -> bool {
        self.transition == request.transition_key
            && self.observed_revision > 0
            && request.cause == QuestCause::NativeHit(self.occurrence())
            && hits::objective_for(catalogue, &self.transition).is_some_and(|(quest, objective)| {
                catalogue.definition_hash(quest) == Some(self.definition_hash)
                    && objective.matches(&self.creature, &self.item, self.spell.as_deref())
            })
    }
    pub(super) fn bind(&self, binding: &mut [u8; 33]) {
        let mut digest = Sha256::new()
            .chain_update(*binding)
            .chain_update(b"native-quest-hit-binding-v1")
            .chain_update(self.occurrence().as_bytes())
            .chain_update(self.observed_revision.to_be_bytes())
            .chain_update(self.definition_hash)
            .chain_update(self.witness.health_loss().get().to_be_bytes());
        for source in [&self.creature, &self.item] {
            digest.update((source.len() as u64).to_be_bytes());
            digest.update(source.as_bytes());
        }
        digest.update([u8::from(self.spell.is_some())]);
        if let Some(spell) = &self.spell {
            digest.update(spell.as_bytes());
        }
        binding[1..].copy_from_slice(&digest.finalize());
    }
    pub(super) async fn admit(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        fence: CurrentCharacterGameplayFence,
        catalogue: &QuestStateCatalogue,
    ) -> StoredResult<bool> {
        let (character, lease, session, _, _) = self.witness.origin();
        if self.observed_revision > fence.expected_character_revision.get()
            || character.as_bytes() != fence.character_id.as_bytes()
            || lease != fence.character_lease_generation
            || session != fence.game_session_id
            || self.witness.attacker().runtime_scope() != fence.runtime_scope
            || self.witness.attacker().scope_generation() != fence.scope_ownership_generation
        {
            return Ok(false);
        }
        let (_, objective) = hits::objective_for(catalogue, &self.transition)
            .ok_or(DurabilityError::InvalidStoredState)?;
        let initial = catalogue
            .track(&objective.gate)
            .ok_or(DurabilityError::InvalidStoredState)?
            .initial;
        Ok(sqlx::query_scalar("SELECT coalesce((SELECT value=$4 AND committed_character_revision <= $3::text::numeric FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),$5)")
            .bind(fence.character_id.as_bytes().as_slice()).bind(&objective.gate).bind(self.observed_revision.to_string())
            .bind(objective.gate_value).bind(initial==objective.gate_value).fetch_one(&mut **tx).await?)
    }
}
impl DurabilityRoot {
    pub async fn commit_character_native_quest_hit(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        hit: NativeQuestHit,
        catalogue: Arc<QuestStateCatalogue>,
    ) -> Result<QuestTransitionOutcome> {
        let request = QuestTransitionRequest {
            transition_key: hit.transition.clone(),
            cause: QuestCause::NativeHit(hit.occurrence()),
        };
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
            &super::grizzly::NoGrizzlyPremium,
            None,
            None,
            None,
            Some(super::native_events::NativeQuestEvent::Hit(hit)),
            None,
        None, None, None,)
        .await
    }
}
