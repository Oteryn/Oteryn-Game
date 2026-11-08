//! Native death context is frozen at capture, then independently fenced in SQL.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeDeathSource {
    Party(Arc<crate::foundation::PartySnapshot>),
    /// The carrier supplies this unforgeable key only after a committed lethal.
    InstanceBoss {
        party: Arc<crate::foundation::PartySnapshot>,
        death: crate::foundation::CreatureDeathOccurrenceKey,
        died_at_unix_ms: u64,
    },
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
                NativeDeathSource::Party(_) | NativeDeathSource::InstanceBoss { .. } => {
                    oteryn_gameplay_quests::quest::native_kills::counter_for(
                        catalogue,
                        &self.transition,
                    )
                    .is_some_and(|counter| {
                        counter.creature == self.creature
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
                NativeDeathSource::Source(_) => {
                    self.definition_hash.is_none()
                        && oteryn_gameplay_quests::quest::source_deaths::rule(&self.transition)
                            .is_some_and(|rule| rule.creature == self.creature)
                }
            }
    }

    pub(super) fn permits_scope(&self, fence: &CurrentCharacterGameplayFence) -> bool {
        match &self.source {
            NativeDeathSource::InstanceBoss { death, .. } => {
                self.transition == "oteryn:quest-transition/native-kills/first-dragon-selected"
                    && self.creature == "oteryn:creature.the_first_dragon"
                    && matches!(fence.runtime_scope, RuntimeScopeRefV1::Instance { .. })
                    && death.runtime_scope() == fence.runtime_scope
                    && death.scope_ownership_generation() == fence.scope_ownership_generation
            }
            _ => matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. }),
        }
    }

    pub(super) fn party(&self) -> Option<&crate::foundation::PartySnapshot> {
        match &self.source {
            NativeDeathSource::Party(source)
            | NativeDeathSource::InstanceBoss { party: source, .. } => Some(source.as_ref()),
            NativeDeathSource::Source(_) => None,
        }
    }

    pub(super) fn bind(&self, binding: &mut [u8; 33]) -> Result<()> {
        match &self.source {
            NativeDeathSource::Party(source) => super::grizzly::bind_party(binding, source)?,
            NativeDeathSource::InstanceBoss {
                party,
                death,
                died_at_unix_ms,
            } => {
                super::grizzly::bind_party(binding, party)?;
                let digest = Sha256::new()
                    .chain_update(*binding)
                    .chain_update(b"native-instance-boss-v1")
                    .chain_update(death.native_bytes())
                    .chain_update(died_at_unix_ms.to_be_bytes())
                    .finalize();
                binding[1..].copy_from_slice(&digest);
            }
            NativeDeathSource::Source(source) => source.bind(binding),
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
        if let NativeDeathSource::InstanceBoss {
            death,
            died_at_unix_ms,
            ..
        } = &self.source
        {
            if !self.permits_scope(&fence) {
                return Ok(false);
            }
            let RuntimeScopeRefV1::Instance {
                world_id,
                instance_id,
            } = death.runtime_scope()
            else {
                return Ok(false);
            };
            let died =
                i64::try_from(*died_at_unix_ms).map_err(|_| DurabilityError::InvalidStoredState)?;
            // Crystal9f5a72c the_first_dragon/actions_lever.lua:6,40 and
            // monster/bosses/the_first_dragon.lua:60-62: only an on-time native
            // death in this player's admitted selected encounter may finish it.
            // The opaque carrier key binds the actual lethal scope/generation;
            // ordinary party/NPC callers never acquire this Instance exception.
            let permitted: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_boss_room_activities a JOIN game_activity_handoff_groups g ON g.handoff_id=a.admission JOIN game_activity_handoff_participants p ON p.handoff_id=a.admission JOIN game_durability_reconnect_sessions s ON s.game_session_id=encode($4,'hex')::uuid WHERE a.world_id=encode($1,'hex')::uuid AND a.instance_id=encode($2,'hex')::uuid AND g.ownership_generation=$3::text::numeric AND g.state=2 AND a.state IN(1,2) AND a.boss_key='oteryn:boss.the_first_dragon' AND a.boss_creature_key=$5 AND p.character_id=encode($6,'hex')::uuid AND s.character_id=p.character_id AND s.activity_handoff_id=a.admission AND s.session_state=2 AND a.opened_at <= $7 AND $7 <= a.deadline AND $7 <= floor(extract(epoch FROM clock_timestamp())*1000)::bigint AND NOT EXISTS(SELECT 1 FROM game_activity_runtime_returns r WHERE r.handoff_id=a.admission AND r.character_id=p.character_id))")
                .bind(world_id.as_bytes().as_slice()).bind(instance_id.as_slice())
                .bind(fence.scope_ownership_generation.get().to_string())
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(&self.creature)
                .bind(fence.character_id.as_bytes().as_slice()).bind(died)
                .fetch_one(&mut **tx).await?;
            if !permitted {
                return Ok(false);
            }
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
            Some(death),
            None,
        )
        .await
    }
}
