//! Reward-boss participation captured from committed combat receipts, under the runtime lock.
//! Crystal 201189f `data/scripts/systems/reward_chest.lua:141-171` credits damage from the
//! boss and healing another active participant. Damage dealt is frozen from the carrier's
//! existing per-creature ledger, so the reward path cannot invent or reroll damage shares.

use super::*;
use oteryn_gameplay_encounter_rules::boss::RewardParticipation;

#[cfg(test)]
#[path = "boss_reward_contribution_tests.rs"]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::gameplay_transport) struct FrozenBossParticipant {
    pub character: crate::foundation::CharacterId,
    pub session: GameSessionId,
    pub lease_generation: u64,
    pub contribution: RewardParticipation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::gameplay_transport) struct FrozenBossDeath {
    pub participants: Vec<FrozenBossParticipant>,
    pub reward: Arc<CreatureRewardRow>,
    pub died_at_unix_ms: u64,
}

#[derive(Debug, Clone)]
struct Participant {
    actor: ExactActorRef,
    frozen: FrozenBossParticipant,
    active: bool,
    received_revision: u64,
}

#[derive(Debug)]
struct Fight {
    boss: ExactActorRef,
    participants: Vec<Participant>,
    overflow: bool,
    /// Retained with the real corpse projection after the settlement queue drains.
    frozen: Option<FrozenBossDeath>,
}

#[derive(Debug, Default)]
pub(super) struct BossContributions {
    fights: Vec<Fight>,
}

// The native death owner already bounds a group reward occurrence to fifty principals.
// Exceeding that boundary refuses the personal settlement, never a prefix of its census.
const MAX_PARTICIPANTS: usize = 50;

fn reward_boss(runtime: &ChannelRuntimeV1, boss: ExactActorRef) -> bool {
    runtime
        .companion_snapshot_including_dead(boss)
        .is_ok_and(|view| {
            view.state.master.is_none()
                && !view.state.policy.is_familiar
                && view.state.policy.reward_boss == Some(true)
        })
}

impl BossContributions {
    pub(super) fn prune(&mut self, runtime: &ChannelRuntimeV1) {
        self.fights.retain(|fight| {
            reward_boss(runtime, fight.boss) || runtime.retains_corpse_projection(fight.boss)
        });
    }

    pub(super) fn involves(&self, boss: ExactActorRef, session: GameSessionId) -> bool {
        self.fights.iter().any(|fight| {
            fight.boss == boss
                && (fight
                    .participants
                    .iter()
                    .any(|participant| participant.frozen.session == session)
                    || fight.frozen.as_ref().is_some_and(|death| {
                        death
                            .participants
                            .iter()
                            .any(|participant| participant.session == session)
                    }))
        })
    }

    fn participant<'a>(
        &'a mut self,
        runtime: &ChannelRuntimeV1,
        boss: ExactActorRef,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<&'a mut Participant> {
        if !reward_boss(runtime, boss) {
            return None;
        }
        let lease = runtime.bound_attacker_lease(actor, session).ok()?;
        // Exact actor generations are the identity. Despawn/reuse cannot carry old scores
        // into a fresh encounter, while a retained dead actor can still be frozen once.
        self.prune(runtime);
        let index = match self.fights.iter().position(|fight| fight.boss == boss) {
            Some(index) => index,
            None => {
                self.fights.push(Fight {
                    boss,
                    participants: Vec::new(),
                    overflow: false,
                    frozen: None,
                });
                self.fights.len() - 1
            }
        };
        let fight = &mut self.fights[index];
        if fight.frozen.is_some() {
            return None;
        }
        let character = lease.character_id();
        let index = match fight
            .participants
            .iter()
            .position(|p| p.frozen.character == character)
        {
            Some(index) => index,
            None if fight.participants.len() < MAX_PARTICIPANTS => {
                fight.participants.push(Participant {
                    actor,
                    frozen: FrozenBossParticipant {
                        character,
                        session,
                        lease_generation: lease.generation(),
                        contribution: RewardParticipation {
                            damage_dealt: 0,
                            damage_received: 0,
                            healing_others: 0,
                        },
                    },
                    active: true,
                    received_revision: 0,
                });
                fight.participants.len() - 1
            }
            None => {
                fight.overflow = true;
                return None;
            }
        };
        let participant = &mut fight.participants[index];
        if participant.actor != actor
            || participant.frozen.session != session
            || participant.frozen.lease_generation != lease.generation()
        {
            // Contribution belongs to the character. Crystal 201189f login.lua:174-176
            // updates playerId after reconnect without clearing the existing GUID's score.
            // Only the genuinely current bound lease can replace this audit identity; chest
            // collection independently requires its current gameplay fence.
            participant.actor = actor;
            participant.frozen.session = session;
            participant.frozen.lease_generation = lease.generation();
            participant.received_revision = 0;
        }
        participant.active = true;
        Some(participant)
    }

    fn freeze(
        &self,
        boss: ExactActorRef,
        shares: &[crate::foundation::RewardDamageShare],
    ) -> Option<Vec<FrozenBossParticipant>> {
        let fight = self.fights.iter().find(|fight| fight.boss == boss);
        if let Some(death) = fight.and_then(|fight| fight.frozen.as_ref()) {
            return Some(death.participants.clone());
        }
        if fight.is_some_and(|fight| fight.overflow) {
            return None;
        }
        let mut frozen = fight.map_or_else(Vec::new, |fight| {
            fight
                .participants
                .iter()
                .map(|p| p.frozen.clone())
                .collect()
        });
        for share in shares {
            let (Some(session), Some(lease_generation)) = (share.session, share.lease_generation)
            else {
                return None;
            };
            if let Some(participant) = frozen.iter_mut().find(|p| p.character == share.character) {
                if lease_generation > participant.lease_generation {
                    participant.session = session;
                    participant.lease_generation = lease_generation;
                }
                participant.contribution.damage_dealt = share.total;
            } else {
                frozen.push(FrozenBossParticipant {
                    character: share.character,
                    session,
                    lease_generation,
                    contribution: RewardParticipation {
                        damage_dealt: share.total,
                        damage_received: 0,
                        healing_others: 0,
                    },
                });
            }
        }
        if frozen.len() > MAX_PARTICIPANTS {
            return None;
        }
        frozen.sort_by_key(|p| p.character);
        Some(frozen)
    }

    fn freeze_death(
        &mut self,
        runtime: &ChannelRuntimeV1,
        boss: ExactActorRef,
        shares: &[crate::foundation::RewardDamageShare],
        reward: Arc<CreatureRewardRow>,
    ) -> Option<FrozenBossDeath> {
        if !runtime.retains_corpse_projection(boss) {
            return None;
        }
        self.prune(runtime);
        if let Some(frozen) = self
            .fights
            .iter()
            .find(|fight| fight.boss == boss)
            .and_then(|fight| fight.frozen.as_ref())
        {
            return Some(frozen.clone());
        }
        if !reward_boss(runtime, boss) {
            return None;
        }
        let participants = self.freeze(boss, shares)?;
        let died_at_unix_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|at| u64::try_from(at.as_millis()).ok())?;
        let frozen = FrozenBossDeath {
            participants,
            reward,
            died_at_unix_ms,
        };
        if let Some(fight) = self.fights.iter_mut().find(|fight| fight.boss == boss) {
            fight.frozen = Some(frozen.clone());
        } else {
            self.fights.push(Fight {
                boss,
                participants: Vec::new(),
                overflow: false,
                frozen: Some(frozen.clone()),
            });
        }
        Some(frozen)
    }
}

impl KillSettlementQueue {
    pub(super) fn freeze_boss_death(
        &self,
        runtime: &ChannelRuntimeV1,
        boss: ExactActorRef,
        shares: &[crate::foundation::RewardDamageShare],
        reward: Arc<CreatureRewardRow>,
    ) -> Option<FrozenBossDeath> {
        self.state()
            .boss_contributions
            .freeze_death(runtime, boss, shares, reward)
    }

    /// Called only after the existing owner confirms a new applied outgoing hit.
    pub(in crate::gameplay_transport) fn boss_damage_dealt(
        &self,
        runtime: &ChannelRuntimeV1,
        boss: ExactActorRef,
        actor: ExactActorRef,
        session: GameSessionId,
    ) {
        self.state()
            .boss_contributions
            .participant(runtime, boss, actor, session);
    }

    /// The AI's current legal target list bounds who is actively in this boss fight.
    pub(in crate::gameplay_transport) fn boss_targets(
        &self,
        runtime: &ChannelRuntimeV1,
        boss: ExactActorRef,
        targets: impl Iterator<Item = (ExactActorRef, GameSessionId)>,
    ) {
        if !reward_boss(runtime, boss) {
            return;
        }
        let mut state = self.state();
        if let Some(fight) = state
            .boss_contributions
            .fights
            .iter_mut()
            .find(|fight| fight.boss == boss)
        {
            if fight.frozen.is_some() {
                return;
            }
            for participant in &mut fight.participants {
                participant.active = false;
            }
        }
        for (actor, session) in targets {
            state
                .boss_contributions
                .participant(runtime, boss, actor, session);
        }
    }

    /// The player's monotone vitals revision prevents a replayed AI/spell receipt from
    /// crediting its already-applied damage again.
    pub(in crate::gameplay_transport) fn boss_damage_received(
        &self,
        runtime: &ChannelRuntimeV1,
        boss: ExactActorRef,
        actor: ExactActorRef,
        session: GameSessionId,
        amount: u64,
        vitals_revision: u64,
    ) {
        if amount == 0 {
            return;
        }
        let mut state = self.state();
        if let Some(participant) = state
            .boss_contributions
            .participant(runtime, boss, actor, session)
        {
            if vitals_revision <= participant.received_revision {
                return;
            }
            participant.received_revision = vitals_revision;
            participant.frozen.contribution.damage_received = participant
                .frozen
                .contribution
                .damage_received
                .saturating_add(amount);
        }
    }

    pub(super) fn boss_spell_contribution(
        &self,
        runtime: &ChannelRuntimeV1,
        receipt: &CombatBatchReceipt,
    ) {
        if !receipt.applied {
            return;
        }
        let session = receipt.command.game_session_id();
        if runtime
            .bound_attacker_lease(receipt.caster, session)
            .is_err()
        {
            return;
        }
        for effect in &receipt.effects {
            let Some(health) = effect.health.filter(|health| health.applied) else {
                continue;
            };
            if health.health_before > health.health_after {
                self.boss_damage_dealt(runtime, effect.target, receipt.caster, session);
                continue;
            }
            if health.health_after <= health.health_before || effect.target == receipt.caster {
                continue;
            }
            let amount = u64::try_from(health.health_after - health.health_before).unwrap_or(0);
            let mut state = self.state();
            // Crystal 201189f player.lua:803-815 chooses the first active fight. Use the
            // owner's stable encounter registration order, crediting one fight only.
            let boss = state
                .boss_contributions
                .fights
                .iter()
                .find(|fight| {
                    runtime.companion_snapshot(fight.boss).is_ok()
                        && fight.participants.iter().any(|p| {
                            p.actor == effect.target
                                && p.active
                                && runtime
                                    .bound_attacker_lease(p.actor, p.frozen.session)
                                    .is_ok_and(|lease| {
                                        lease.character_id() == p.frozen.character
                                            && lease.generation() == p.frozen.lease_generation
                                    })
                        })
                })
                .map(|fight| fight.boss);
            let Some(boss) = boss else {
                continue;
            };
            if let Some(participant) =
                state
                    .boss_contributions
                    .participant(runtime, boss, receipt.caster, session)
            {
                participant.frozen.contribution.healing_others = participant
                    .frozen
                    .contribution
                    .healing_others
                    .saturating_add(amount);
            }
        }
    }

    #[cfg(test)]
    pub(super) fn freeze_boss_participation(
        &self,
        boss: ExactActorRef,
        shares: &[crate::foundation::RewardDamageShare],
    ) -> Option<Vec<FrozenBossParticipant>> {
        self.state().boss_contributions.freeze(boss, shares)
    }
}
