//! Composes the real kill drain with one atomic multi-Character XP transaction.
use super::{
    ComposedFreshAdmission,
    kill_reward::PendingKillSettlement,
    party_xp::{PartyXpPlan, plan_shared_xp_in_transaction},
};
use crate::combat::{CombatDeathRewardXpError, RewardProgressionBinding};
use crate::content::character_progression_content::CHARACTER_EXPERIENCE_TABLE_LEVELS;
use crate::durability::{
    DurabilityError,
    character_progression::{
        CharacterProgressionError, CurrentCharacterGameplayFence, ExperienceAwardRequest,
        ExperienceCommitOutcome, ExperienceRewardOccurrence, ProgressionInitializationRequest,
    },
    character_revision_sequencer::RevisionSlot,
    party_kill_xp::{self, PartyKillXpMember, PartyKillXpRequest},
    spell_item_transaction as item_tx,
    spell_owner_commit::SpellLanePermit,
};
use crate::foundation::RuntimeScopeRefV1;

pub(super) struct SettledPartyXp {
    pub xp: Result<ExperienceCommitOutcome, CombatDeathRewardXpError>,
    pub slots: Vec<RevisionSlot>,
}

fn rejected() -> CharacterProgressionError {
    CharacterProgressionError::AuthorityRejected
}
fn key(entry: &PendingKillSettlement) -> [u8; 52] {
    let d = entry.facts.death;
    let mut key = [0; 52];
    key[..16].copy_from_slice(d.world_id().as_bytes());
    key[16..32].copy_from_slice(d.channel_id().as_bytes());
    key[32..40].copy_from_slice(&d.scope_ownership_generation().get().to_be_bytes());
    key[40..44].copy_from_slice(&d.actor_local_id().to_be_bytes());
    key[44..52].copy_from_slice(&d.actor_local_generation().to_be_bytes());
    key
}

impl ComposedFreshAdmission<'_, '_, '_> {
    async fn discover_party_xp(
        &self,
        permit: &SpellLanePermit,
        entry: &PendingKillSettlement,
    ) -> Result<PartyXpPlan, CharacterProgressionError> {
        let runtime = self.runtime.lock().await;
        let states = self.states.spell_states.lock().await;
        let attack = self.states.attack.lock().await;
        let mut ctx = (self, &*runtime, &*states, &*attack, entry);
        self.root
            .try_issue_semantic_pass()?
            .run_with_context(&mut ctx, move |holder, deadline, ctx| {
                Box::pin(async move {
                    let (owner, runtime, states, _attack, entry) = ctx;
                    let b = runtime.binding();
                    let mut tx = item_tx::begin_spell_owner_transaction(holder, deadline).await?;
                    let scope = item_tx::assert_spell_item_scope_in_transaction(
                        &mut tx,
                        permit,
                        owner.root,
                        owner.character,
                        owner.holder,
                        RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
                        b.scope_generation().get(),
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                    let plan = plan_shared_xp_in_transaction(
                        &mut tx, owner.root, &scope, runtime, states, entry,
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                    tx.rollback().await?;
                    Ok(plan)
                })
            })
            .await
            .map_err(Into::into)
    }

    pub(super) async fn settle_party_experience(
        &self,
        permit: &SpellLanePermit,
        entry: &PendingKillSettlement,
    ) -> Result<SettledPartyXp, CharacterProgressionError> {
        let plan = self.discover_party_xp(permit, entry).await?;
        if !plan
            .members
            .iter()
            .any(|m| m.character.as_bytes() == entry.facts.principal.character.as_bytes())
        {
            return Err(rejected());
        }
        let mut slots = Vec::with_capacity(plan.members.len());
        let mut members = Vec::with_capacity(plan.members.len());
        for member in &plan.members {
            let mut slot = self.revision_sequencer.acquire(member.character).await;
            let mut fence = self
                .current_quest_fence(member.session)
                .await
                .map_err(|_| CharacterProgressionError::Unavailable(DurabilityError::Unavailable))?
                .ok_or_else(rejected)?;
            if fence.character_id != member.character {
                return Err(rejected());
            }
            if member.character.as_bytes() == entry.facts.principal.character.as_bytes()
                && (fence.game_session_id != entry.facts.principal.session
                    || fence.character_lease_generation != entry.facts.principal.lease_generation)
            {
                return Err(rejected());
            }
            fence.expected_character_revision = slot
                .cursor(self.root, self.character)
                .await
                .map_err(|error|match error {
                    crate::durability::character_authority::CharacterAuthorityError::Unavailable(error)=>CharacterProgressionError::Unavailable(error),
                    _=>rejected(),
                })?;
            let binding = self
                .progression_binding(member.session)
                .ok_or(CharacterProgressionError::MissingProgressionState)?;
            initialize(self, fence, &binding).await?;
            // The carrier memoizes the occurrence per death and Character. This happens before
            // the final transaction; it creates no grant and writes no XP.
            let occurrence = {
                let mut runtime = self.runtime.lock().await;
                let (bytes, _) = runtime
                    .borrow_combat_death()
                    .party_reward_occurrence(
                        entry.target,
                        crate::foundation::CharacterId::decode(member.character.as_bytes())
                            .map_err(|_| rejected())?,
                    )
                    .map_err(|_| rejected())?;
                ExperienceRewardOccurrence::from_bytes(bytes)?
            };
            members.push(PartyKillXpMember {
                fence,
                request: ExperienceAwardRequest {
                    occurrence,
                    amount: oteryn_simulation_determinism::ExactI64::new(member.share),
                    context: binding.context.clone(),
                    policy_revision: binding.policy_revision.clone(),
                    reward_revision: binding.reward_revision.clone(),
                    policy: binding.policy.clone(),
                },
                premium: member.premium,
                shared: member.shared,
                party_id: member.party_id,
                party_revision: member.party_revision,
            });
            slots.push(slot);
        }
        let runtime = self.runtime.lock().await;
        let states = self.states.spell_states.lock().await;
        let attack = self.states.attack.lock().await;
        let mut ctx = (
            self,
            &*runtime,
            &*states,
            &*attack,
            entry,
            Some(members),
            &plan,
        );
        let result = self
            .root
            .try_issue_semantic_pass()?
            .run_with_context(&mut ctx, move |holder, deadline, ctx| {
                Box::pin(async move {
                    let (owner, runtime, states, _attack, entry, members, previous) = ctx;
                    let b = runtime.binding();
                    let mut tx = item_tx::begin_spell_owner_transaction(holder, deadline).await?;
                    let scope = item_tx::assert_spell_item_scope_in_transaction(
                        &mut tx,
                        permit,
                        owner.root,
                        owner.character,
                        owner.holder,
                        RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
                        b.scope_generation().get(),
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                    let death = key(entry);
                    if let Some(awards) = party_kill_xp::reconcile_in_transaction(
                        &mut tx,
                        &scope,
                        &death,
                        crate::domain::CharacterId::from_bytes(
                            *entry.facts.principal.character.as_bytes(),
                        )
                        .map_err(|_| DurabilityError::InvalidStoredState)?,
                        &entry.creature,
                        entry.row.xp_amount.get(),
                        (
                            entry.facts.death_at_ms,
                            entry
                                .activity_observed_at
                                .ok_or(DurabilityError::InvalidStoredState)?
                                .get(),
                        ),
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?
                    {
                        party_kill_xp::commit_party_kill_transaction(tx, deadline).await?;
                        return Ok(Ok(awards));
                    }
                    let current = plan_shared_xp_in_transaction(
                        &mut tx, owner.root, &scope, runtime, states, entry,
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                    if current.source.party_id != previous.source.party_id
                        || current.source.revision != previous.source.revision
                        || current
                            .members
                            .iter()
                            .map(|m| m.character)
                            .ne(previous.members.iter().map(|m| m.character))
                    {
                        return Err(DurabilityError::Unavailable);
                    }
                    let mut members = members.take().ok_or(DurabilityError::InvalidStoredState)?;
                    for (request, current) in members.iter_mut().zip(&current.members) {
                        request.request.amount =
                            oteryn_simulation_determinism::ExactI64::new(current.share);
                        request.premium = current.premium;
                        request.shared = current.shared;
                        request.party_id = current.party_id;
                        request.party_revision = current.party_revision;
                        if !runtime
                            .bound_attacker_lease(current.actor, current.session)
                            .is_ok_and(|lease| {
                                lease.character_id().as_bytes()
                                    == request.fence.character_id.as_bytes()
                                    && lease.generation()
                                        == request.fence.character_lease_generation
                            })
                        {
                            return Err(DurabilityError::Unavailable);
                        }
                    }
                    let awards = match party_kill_xp::commit_in_transaction(
                        &mut tx,
                        &scope,
                        owner.holder,
                        PartyKillXpRequest {
                            death_key: death,
                            principal: crate::domain::CharacterId::from_bytes(
                                *entry.facts.principal.character.as_bytes(),
                            )
                            .map_err(|_| DurabilityError::InvalidStoredState)?,
                            creature: entry.creature.clone(),
                            base_xp: entry.row.xp_amount.get(),
                            death_at_ms: entry.facts.death_at_ms,
                            death_at_us: entry
                                .activity_observed_at
                                .ok_or(DurabilityError::InvalidStoredState)?
                                .get(),
                            shared: current.shared,
                            party_id: current.source.party_id,
                            party_revision: current.source.revision,
                            members,
                        },
                    )
                    .await
                    {
                        Ok(awards) => awards,
                        Err(error) => return Ok(Err(error)),
                    };
                    party_kill_xp::commit_party_kill_transaction(tx, deadline).await?;
                    Ok(Ok(awards))
                })
            })
            .await;
        drop(attack);
        drop(states);
        drop(runtime);
        let result = result
            .map_err(CharacterProgressionError::from)
            .and_then(std::convert::identity);
        let awards = match result {
            Ok(awards) => awards,
            Err(error) => {
                for slot in &mut slots {
                    slot.observe_party_experience(&Err(CharacterProgressionError::Unavailable(
                        DurabilityError::Unavailable,
                    )));
                }
                return Err(error);
            }
        };
        let mut principal = None;
        for award in awards {
            if let Some(xp) = award.xp {
                if let Some(slot) = slots
                    .iter_mut()
                    .find(|s| s.character_id() == award.character)
                {
                    slot.observe_party_experience(&Ok(xp.clone()));
                }
                if award.character.as_bytes() == entry.facts.principal.character.as_bytes() {
                    principal = Some(xp);
                }
            }
        }
        Ok(SettledPartyXp {
            xp: principal.ok_or(CombatDeathRewardXpError::NoExperienceAward),
            slots,
        })
    }
}

async fn initialize(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    fence: CurrentCharacterGameplayFence,
    binding: &RewardProgressionBinding<CHARACTER_EXPERIENCE_TABLE_LEVELS>,
) -> Result<(), CharacterProgressionError> {
    owner
        .root
        .initialize_character_progression(
            owner.character,
            owner.holder,
            fence,
            ProgressionInitializationRequest {
                context: binding.context.clone(),
                policy_revision: binding.policy_revision.clone(),
                reward_revision: binding.reward_revision.clone(),
                policy: binding.policy.clone(),
            },
        )
        .await?;
    Ok(())
}
