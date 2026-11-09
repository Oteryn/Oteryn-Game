//! Native death context is frozen at capture, then independently fenced in SQL.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeDeathSource {
    Party(Arc<crate::foundation::PartySnapshot>),
    Encounter(super::encounter_deaths::EncounterDeathContext),
    Source(super::source_deaths::SourceDeathContext),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeQuestDeath {
    pub transition: String,
    pub creature: String,
    pub observed_revision: u64,
    /// Only authored bindings carry this extra immutable definition witness.
    pub definition_hash: Option<[u8; 32]>,
    pub source: NativeDeathSource,
}

impl NativeQuestDeath {
    pub(super) fn valid(
        &self,
        request: &QuestTransitionRequest,
        catalogue: &QuestStateCatalogue,
    ) -> bool {
        self.transition == request.transition_key
            && self.observed_revision > 0
            && matches!(request.cause, QuestCause::CreatureDeath(_))
            && match &self.source {
                NativeDeathSource::Party(_) => {
                    oteryn_gameplay_quests::quest::native_kills::counter_for(
                        catalogue,
                        &self.transition,
                    )
                    .is_some_and(|counter| {
                        counter.creature == self.creature
                            && counter.encounter_source.is_none()
                            && if oteryn_gameplay_quests::quest::native_kills::counter(
                                &self.transition,
                            )
                            .is_some()
                            {
                                self.definition_hash.is_none()
                            } else {
                                self.definition_hash.is_some_and(|hash| {
                                    catalogue.definition_hash(counter.quest) == Some(hash)
                                })
                            }
                    })
                }
                NativeDeathSource::Encounter(source) => {
                    oteryn_gameplay_quests::quest::native_kills::counter_for(
                        catalogue,
                        &self.transition,
                    )
                    .is_some_and(|counter| {
                        counter.creature == self.creature
                            && counter.encounter_source == Some(source.source.as_str())
                            && self.definition_hash.is_some_and(|hash| {
                                catalogue.definition_hash(counter.quest) == Some(hash)
                            })
                    })
                }
                NativeDeathSource::Source(_) => {
                    self.definition_hash.is_none()
                        && oteryn_gameplay_quests::quest::source_deaths::rule(&self.transition)
                            .is_some_and(|rule| rule.creature == self.creature)
                }
            }
    }

    pub(super) fn party(&self) -> Option<&crate::foundation::PartySnapshot> {
        match &self.source {
            NativeDeathSource::Party(source) => Some(source.as_ref()),
            NativeDeathSource::Source(_) | NativeDeathSource::Encounter(_) => None,
        }
    }

    pub(super) fn bind(&self, binding: &mut [u8; 33]) -> Result<()> {
        match &self.source {
            NativeDeathSource::Party(source) => super::grizzly::bind_party(binding, source)?,
            NativeDeathSource::Source(source) => source.bind(binding),
            NativeDeathSource::Encounter(source) => source.bind(binding),
        }
        let mut digest = Sha256::new()
            .chain_update(*binding)
            .chain_update(b"native-quest-death-v1")
            .chain_update(self.creature.as_bytes())
            .chain_update(self.observed_revision.to_be_bytes());
        if let Some(hash) = self.definition_hash {
            digest.update(b"authored-definition-v1");
            digest.update(hash);
        }
        let digest = digest.finalize();
        binding[1..].copy_from_slice(&digest);
        Ok(())
    }

    pub(super) async fn admit(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        fence: CurrentCharacterGameplayFence,
        catalogue: &QuestStateCatalogue,
    ) -> StoredResult<bool> {
        if let NativeDeathSource::Source(source) = &self.source {
            return source.admit(tx, fence, self).await;
        }
        if let NativeDeathSource::Encounter(source) = &self.source
            && !source.matches(fence)
        {
            return Ok(false);
        }
        let character = fence.character_id;
        let counter =
            oteryn_gameplay_quests::quest::native_kills::counter_for(catalogue, &self.transition)
                .ok_or(DurabilityError::InvalidStoredState)?;
        let initial_gate = catalogue
            .track(counter.gate)
            .ok_or(DurabilityError::InvalidStoredState)?
            .initial;
        // Character root and the track rows are already locked. A queued death
        // cannot enter a mission started or reset after its captured snapshot.
        Ok(sqlx::query_scalar("SELECT coalesce((SELECT value=$4 AND committed_character_revision <= $3::text::numeric FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),$5)")
            .bind(character.as_bytes().as_slice()).bind(counter.gate)
            .bind(self.observed_revision.to_string()).bind(counter.gate_value).bind(initial_gate==counter.gate_value).fetch_one(&mut **tx).await?)
    }
}

impl DurabilityRoot {
    pub async fn commit_character_native_quest_kill(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        cause: QuestCause,
        death: NativeQuestDeath,
        catalogue: Arc<QuestStateCatalogue>,
    ) -> Result<QuestTransitionOutcome> {
        let request = QuestTransitionRequest {
            transition_key: death.transition.clone(),
            cause,
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
            Some(super::native_events::NativeQuestEvent::Death(death)),
            None,
        )
        .await
    }
}
