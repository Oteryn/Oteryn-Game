//! A bounded source party rite reuses native group movement and altar receipts.
use super::*;
use crate::content::spell_data::Vocation;
use crate::durability::quest_world_callback::QuestWorldPartyMember;

pub(super) struct Member {
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
}
pub(super) struct Plan<P> {
    pub(super) proofs: Vec<P>,
    members: Vec<Member>,
}
impl<P> Plan<P> {
    pub(super) fn sessions(&self) -> Vec<GameSessionId> {
        self.members.iter().map(|m| m.session).collect()
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn prepare_party_transfer<'a>(
        &'a self,
        runtime: &ChannelRuntimeV1,
        states: &ChannelSpellStates,
        closed: &'a dyn crate::map::floor::CellBlocker,
        callback: &Callback,
        initiator: GameSessionId,
    ) -> Result<Plan<impl crate::foundation::SourceStepCommitProof + 'a>, UseDisposition> {
        let party = callback
            .effects
            .party_transfer
            .as_ref()
            .ok_or(UseDisposition::Rejected)?;
        let census = runtime
            .positioned_melee_census()
            .map_err(|_| UseDisposition::Rejected)?;
        let mut members = Vec::new();
        let mut proofs = Vec::new();
        for station in &party.stations {
            let mut occupants = census
                .iter()
                .filter(|(_, at, _)| at.position() == local(station.from));
            let Some(&(actor, expected, Some(session))) = occupants.next() else {
                return Err(UseDisposition::Rejected);
            };
            let state = states
                .get(runtime, actor, session)
                .ok_or(UseDisposition::Rejected)?;
            let base = match state.source_party_vocation() {
                Vocation::Druid | Vocation::ElderDruid => "druid",
                Vocation::Sorcerer | Vocation::MasterSorcerer => "sorcerer",
                Vocation::Paladin | Vocation::RoyalPaladin => "paladin",
                Vocation::Knight | Vocation::EliteKnight => "knight",
                Vocation::Monk | Vocation::ExaltedMonk => "monk",
                Vocation::None => return Err(UseDisposition::Rejected),
            };
            if occupants.next().is_some()
                || base != station.vocation
                || states.is_dead(actor)
                || !states.spell_idle(runtime, actor, session)
                || !runtime
                    .player_control_facts(actor, session)
                    .is_ok_and(|f| f.control_loss.is_none())
                || runtime.bound_attacker_lease(actor, session).is_err()
                || !self.states.player_trades.physical(session, None)
                || self.party_member_has_foreign_pending(session, initiator)
            {
                return Err(UseDisposition::Rejected);
            }
            let blocked =
                super::super::pvp_live::pz_blocked(states, actor, session, self.owner_now());
            let proof = crate::map::floor::prepare_quest_callback_teleport(
                runtime,
                &self.movement_cells,
                actor,
                session,
                expected,
                local(station.to),
                blocked,
                closed,
            )
            .map_err(|_| UseDisposition::NotPossible)?;
            members.push(Member {
                actor,
                session,
                expected,
            });
            proofs.push(proof);
        }
        Ok(Plan { proofs, members })
    }

    pub(super) async fn party_context<P>(
        &self,
        runtime: &ChannelRuntimeV1,
        plan: &Plan<P>,
    ) -> Option<Vec<QuestWorldPartyMember>> {
        let mut result = Vec::new();
        for member in &plan.members {
            let fence = self.current_quest_fence(member.session).await.ok()??;
            let lease = runtime
                .bound_attacker_lease(member.actor, member.session)
                .ok()?;
            if lease.character_id().as_bytes() != fence.character_id.as_bytes()
                || lease.generation() != fence.character_lease_generation
            {
                return None;
            }
            result.push(QuestWorldPartyMember {
                character: *fence.character_id.as_bytes(),
                session: *member.session.as_bytes(),
                connection_generation: fence.connection_generation.get(),
                lease_generation: fence.character_lease_generation,
                scope_generation: fence.scope_ownership_generation.get(),
                actor_placement: member.actor.placement_identity(),
                movement_revision: member.expected.revision(),
            });
        }
        Some(result)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "one owner turn projects the prepared exact party and lever"
    )]
    pub(super) fn commit_party_transfer<P: crate::foundation::SourceStepCommitProof>(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        map: &BundleMap,
        plan: Plan<P>,
        key: u64,
        revision: u64,
        after: crate::map::view::EntryFacts,
        close: bool,
        initiator: GameSessionId,
        optional_pvp: Option<bool>,
    ) -> UseDisposition {
        let Ok(snapshots) = runtime.commit_source_steps_and_project(plan.proofs, || {
            map.facts
                .apply_quest_callback(key, revision, after, &[], &[], close, None)
        }) else {
            return UseDisposition::Rejected;
        };
        for (member, snapshot) in plan.members.into_iter().zip(snapshots) {
            states.stage_special_movement_step(
                runtime,
                map,
                member.actor,
                member.session,
                self.owner_now().get(),
                Some(member.expected.position()),
                snapshot.position(),
                optional_pvp,
            );
            states.stage_underwater_step(
                runtime,
                member.actor,
                member.session,
                self.owner_now().get(),
                underwater(map, member.expected.position()),
                underwater(map, snapshot.position()),
            );
            if let Some(at) = native(snapshot.position()) {
                map.facts.emit_movement_effect(at, 11);
            }
            if let Some(at) = native(member.expected.position()) {
                map.facts.emit_movement_effect(at, 3);
            }
        }
        if let Ok(mut pending) = self.states.quest_callbacks.records.lock() {
            pending.remove(&initiator);
        }
        UseDisposition::Committed
    }

    fn party_member_has_foreign_pending(
        &self,
        member: GameSessionId,
        initiator: GameSessionId,
    ) -> bool {
        if self.states.grizzly_boss.has_pending(member)
            || self
                .states
                .quest_callbacks
                .soul_rewards
                .lock()
                .map_or(true, |p| p.contains_key(&member))
        {
            return true;
        }
        self.states
            .quest_callbacks
            .records
            .lock()
            .map_or(true, |p| {
                p.iter().any(|(owner, pending)| {
                    *owner != initiator
                        && (*owner == member || pending.participants.contains(&member))
                })
            })
    }
}
