//! The copied Soul War goal runs in the existing atomic pre-taint XP transaction.
use super::*;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use oteryn_gameplay_entities::carrier::native_source_death::SourceCreatureDeathSeal;
use oteryn_gameplay_quests::quest::soul_war::SoulWarBoss;
use std::sync::Arc;

#[derive(Debug)]
struct Killer {
    actor: ExactActorRef,
    session: GameSessionId,
    character: [u8; 16],
    lease: u64,
    damage: u64,
}

#[derive(Debug)]
struct Retained {
    begin: SourceCreatureDeathSeal,
    index: usize,
    killers: Vec<Killer>,
    actors: Vec<ExactActorRef>,
    reward: Option<(
        super::super::kill_reward::PendingKillSettlement,
        oteryn_gameplay_foundation::scheduled_events::EventRates,
    )>,
}

#[derive(Debug, Clone)]
pub(in crate::gameplay_transport) struct SoulWarAction(Arc<Retained>);

impl PartialEq for SoulWarAction {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for SoulWarAction {}

impl SoulWarAction {
    pub(super) fn capture(
        runtime: &ChannelRuntimeV1,
        attack: &attack::ChannelAttackStates,
        begin: &SourceCreatureDeathSeal,
        index: usize,
        now_us: u64,
    ) -> Result<Option<Self>, ()> {
        let boss = match begin.events().get(index).ok_or(())?.name() {
            "SoulWarBossesDeath" => {
                let boss = SoulWarBoss::from_creature(begin.definition_key()).ok_or(())?;
                if boss == SoulWarBoss::Megalomania {
                    return Err(());
                }
                boss
            }
            "MegalomaniaDeath" => SoulWarBoss::Megalomania,
            _ => return Ok(None),
        };
        if SoulWarBoss::from_creature(begin.definition_key()) != Some(boss)
            || !begin.current_for(runtime)
        {
            return Err(());
        }
        let census = runtime.positioned_actor_census().map_err(|_| ())?;
        let mut killers = Vec::new();
        // Crystal201189f creature.lua:219-240 uses Player(cid), not a summon master,
        // and the callback's current clock. config.lua.dist:40 fixes PZ_LOCKED=60000ms.
        for share in runtime
            .source_player_damage_shares(begin.actor())
            .map_err(|_| ())?
        {
            if share.total == 0
                || !share.last_damage_at_ms.is_some_and(|at| {
                    (now_us / 1000)
                        .checked_sub(at)
                        .is_some_and(|age| age <= 60_000)
                })
            {
                continue;
            }
            let Some(session) = census
                .iter()
                .find(|(actor, _, _)| *actor == share.actor)
                .and_then(|(_, _, session)| *session)
            else {
                continue;
            };
            let lease = runtime
                .bound_attacker_lease(share.actor, session)
                .map_err(|_| ())?;
            if lease.character_id() != share.character {
                return Err(());
            }
            killers.push(Killer {
                actor: share.actor,
                session,
                character: *share.character.as_bytes(),
                lease: lease.generation(),
                damage: share.total,
            });
        }
        killers.sort_by(|a, b| b.damage.cmp(&a.damage).then(a.character.cmp(&b.character)));
        let reward = attack.kills().source_death_experience(runtime, begin)?;
        if reward.is_none() && (!killers.is_empty() || begin.has_reward_damage_credit()) {
            return Err(());
        }
        let actors = killers.iter().map(|killer| killer.actor).collect();
        Ok(Some(Self(Arc::new(Retained {
            begin: begin.clone(),
            index,
            killers,
            actors,
            reward,
        }))))
    }

    pub(super) fn actors(&self) -> &[ExactActorRef] {
        &self.0.actors
    }

    pub(in crate::gameplay_transport) fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.0.begin.current_for(runtime)
            && runtime
                .source_creature_death_event_index(&self.0.begin)
                .ok()
                == Some(Some(self.0.index))
            && self.0.killers.iter().all(|killer| {
                runtime
                    .bound_attacker_lease(killer.actor, killer.session)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == &killer.character
                            && lease.generation() == killer.lease
                    })
            })
    }

    pub(in crate::gameplay_transport) fn matches_entry(
        &self,
        entry: &super::super::kill_reward::PendingKillSettlement,
    ) -> bool {
        entry.target == self.0.begin.actor()
            && entry.facts.death == self.0.begin.death_key()
            && entry.creature == self.0.begin.definition_key()
    }

    pub(in crate::gameplay_transport) fn credits(
        &self,
        fence: &CurrentCharacterGameplayFence,
    ) -> bool {
        fence.runtime_scope == self.0.begin.death_key().runtime_scope()
            && fence.scope_ownership_generation
                == self.0.begin.death_key().scope_ownership_generation()
            && self.0.killers.iter().any(|killer| {
                fence.character_id.as_bytes() == &killer.character
                    && fence.game_session_id == killer.session
                    && fence.character_lease_generation == killer.lease
            })
    }

    pub(in crate::gameplay_transport) fn contains_all_killers(
        &self,
        participants: &[(CurrentCharacterGameplayFence, bool)],
    ) -> bool {
        self.0.killers.iter().all(|killer| {
            participants.iter().any(|(fence, credited)| {
                *credited
                    && fence.character_id.as_bytes() == &killer.character
                    && fence.game_session_id == killer.session
                    && fence.character_lease_generation == killer.lease
            })
        })
    }

    pub(super) async fn execute(
        &self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    ) -> Result<(), ()> {
        let Some((entry, rates)) = &self.0.reward else {
            return Ok(());
        };
        let mut prepared = entry.clone();
        super::super::kill_reward::prepare_experience(owner, &mut prepared, *rates)
            .await
            .map_err(|_| ())?;
        let settled = owner
            .settle_source_soul_war_experience(permit, &prepared, self)
            .await
            .map_err(|_| ())?;
        if matches!(
            settled.xp,
            Err(crate::combat::CombatDeathRewardXpError::Progression(_))
        ) {
            return Err(());
        }
        owner
            .states
            .level_refresh
            .mark(settled.leveled.iter().copied());
        owner.states.title_refresh.mark(settled.leveled);
        if let Ok(xp) = &settled.xp {
            owner
                .states
                .soul_kills
                .mark(super::super::soul_kill_live::soul_hook(xp));
        }
        Ok(())
    }

    pub(super) async fn refresh_after_release(&self, owner: &ComposedFreshAdmission<'_, '_, '_>) {
        for killer in &self.0.killers {
            owner.schedule_quest_refresh(killer.session);
        }
        for killer in &self.0.killers {
            owner.refresh_quest_session(killer.session, false).await;
        }
    }
}
