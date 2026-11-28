//! One-shot observation of the due Forge population turn in qualification builds.
use super::*;
use crate::carrier::runtime_actor_companion::CompanionSnapshot;
use tokio::sync::oneshot;
use tokio::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgePopulationDrawRefusal {
    LeaseRequired,
    Deadline,
    Busy,
}

#[derive(Debug)]
pub struct ForgePopulationCensus {
    pub now_micros: u64,
    pub due_micros: u64,
    pub seed: u64,
    pub artifact_digest: [u8; 32],
    pub population_tick_micros: u64,
    /// Owner slots after observing deaths and before releasing expired reservations.
    pub fiendish_slots: Vec<(ExactActorRef, Option<u64>)>,
    pub influenced_count: usize,
    pub influenced_available_slots: usize,
    /// Readable live native spawn cells in source and point order, before either draw.
    pub candidates: Vec<ForgePopulationCandidate>,
    /// Newly classified incarnations in application order.
    pub selected: Vec<ForgePopulationSelection>,
    /// Previously classified native actors, including fiends still holding a dead slot.
    pub retained: Vec<(ExactActorRef, ForgeCreature)>,
    /// Planner deficit after counting live retained fiends.
    pub fiendish_vacancy: usize,
    /// Owner allowance after counting all held slots, including dead fiends.
    pub fiendish_available_slots: usize,
    /// Each native spawn's planner identity and exact incarnation, before selection.
    pub eligible: Vec<(u64, ExactActorRef)>,
    /// All live controlled summons, including familiars, in the Channel.
    pub excluded_owned_summons: Vec<ExactActorRef>,
}

#[derive(Debug)]
pub struct ForgePopulationCandidate {
    pub actor: ExactActorRef,
    pub spawn_identity: u64,
    pub kind: Option<ForgeCreature>,
    pub owned: bool,
    pub forge_difficulty: Option<u8>,
    pub creature_key: String,
    pub position: MovementLocalPosition,
}

#[derive(Debug)]
pub struct ForgePopulationSelection {
    pub actor: ExactActorRef,
    pub kind: ForgeCreature,
    pub creature_key: String,
    pub position: MovementLocalPosition,
}

type DrawResult = Result<ForgePopulationCensus, ForgePopulationDrawRefusal>;

#[derive(Debug)]
pub(in crate::carrier) struct PendingDraw {
    session: GameSessionId,
    actor: ExactActorRef,
    lease: CharacterLease,
    control_revision: u64,
    seed: u64,
    deadline: Instant,
    reply: oneshot::Sender<DrawResult>,
}

pub(super) struct ActiveDraw {
    pub(super) seed: u64,
    pub(super) census: ForgePopulationCensus,
    pub(super) reply: oneshot::Sender<DrawResult>,
}

impl ActiveDraw {
    pub(super) fn record_candidate(&mut self, spawn_identity: u64, snapshot: &CompanionSnapshot) {
        self.census.candidates.push(ForgePopulationCandidate {
            actor: snapshot.actor,
            spawn_identity,
            kind: snapshot.forge,
            owned: snapshot.state.master.is_some(),
            forge_difficulty: snapshot.state.policy.forge_difficulty,
            creature_key: snapshot.state.policy.definition_key.clone(),
            position: snapshot.position,
        });
    }

    pub(super) fn record_selection(&mut self, snapshot: &CompanionSnapshot, kind: ForgeCreature) {
        self.census.selected.push(ForgePopulationSelection {
            actor: snapshot.actor,
            kind,
            creature_key: snapshot.state.policy.definition_key.clone(),
            position: snapshot.position,
        });
    }
}

impl ChannelRuntimeV1 {
    pub fn arm_qualification_forge_draw(
        &mut self,
        session: GameSessionId,
        lease: CharacterLease,
        scope: RuntimeScopeRefV1,
        scope_generation: ScopeOwnershipGeneration,
        seed: u64,
        deadline: Instant,
    ) -> Result<oneshot::Receiver<DrawResult>, ForgePopulationDrawRefusal> {
        use ForgePopulationDrawRefusal::*;
        let now = Instant::now();
        if now >= deadline {
            return Err(Deadline);
        }
        if !self
            .owner_fence()
            .is_ok_and(|f| f.is_current_for_scope(scope, scope_generation))
        {
            return Err(LeaseRequired);
        }
        let (actor, _) = self
            .positioned_player_for_session(session)
            .map_err(|_| LeaseRequired)?
            .ok_or(LeaseRequired)?;
        let control_revision = self
            .forge_draw_control_revision(session, actor, lease)
            .ok_or(LeaseRequired)?;
        if self
            .forge_population_draw
            .as_ref()
            .is_some_and(|draw| !draw.reply.is_closed() && now < draw.deadline)
        {
            return Err(Busy);
        }
        let (reply, receive) = oneshot::channel();
        self.forge_population_draw = Some(PendingDraw {
            session,
            actor,
            lease,
            control_revision,
            seed,
            deadline,
            reply,
        });
        Ok(receive)
    }

    #[cfg(feature = "test-support")]
    pub fn qualification_forge_draw_pending(&self) -> bool {
        self.forge_population_draw
            .as_ref()
            .is_some_and(|draw| !draw.reply.is_closed())
    }

    fn forge_draw_control_revision(
        &self,
        session: GameSessionId,
        actor: ExactActorRef,
        lease: CharacterLease,
    ) -> Option<u64> {
        if self.bound_attacker_lease(actor, session) != Ok(lease) {
            return None;
        }
        let index = self
            .carrier
            .player_slot_index(&self.continuity, actor.0, session)
            .ok()?;
        let Slot::Occupied { lifecycle, .. } = &self.carrier.slots[index] else {
            return None;
        };
        lifecycle
            .control_loss
            .is_none()
            .then_some(lifecycle.control_revision)
    }

    pub(super) fn take_qualification_forge_draw(&mut self, now_micros: u64) -> Option<ActiveDraw> {
        use ForgePopulationDrawRefusal::*;
        let draw = self.forge_population_draw.take()?;
        if draw.reply.is_closed() {
            return None;
        }
        if Instant::now() >= draw.deadline {
            let _ = draw.reply.send(Err(Deadline));
            return None;
        }
        if self.forge_draw_control_revision(draw.session, draw.actor, draw.lease)
            != Some(draw.control_revision)
        {
            let _ = draw.reply.send(Err(LeaseRequired));
            return None;
        }
        let excluded_owned_summons = self
            .carrier
            .occupied
            .iter()
            .filter_map(|&index| {
                let Slot::CreatureOccupied {
                    generation,
                    health,
                    companion: Some(state),
                    ..
                } = &self.carrier.slots[index as usize]
                else {
                    return None;
                };
                (*health > 0 && state.master.is_some()).then(|| {
                    ExactActorRef(ActorRef::from_scoped_owner_parts(
                        self.binding.runtime_scope,
                        self.binding.scope_generation,
                        ActorLocalId(index + 1),
                        ActorLocalGeneration(*generation),
                    ))
                })
            })
            .collect();
        let census = ForgePopulationCensus {
            now_micros,
            due_micros: self.forge_next_tick_micros,
            seed: draw.seed,
            artifact_digest: self.content.server_artifact_digest(),
            population_tick_micros: super::POPULATION_TICK_MICROS,
            fiendish_slots: self.forge_fiends.clone(),
            influenced_count: 0,
            influenced_available_slots: 0,
            candidates: Vec::new(),
            selected: Vec::new(),
            retained: Vec::new(),
            fiendish_vacancy: 0,
            fiendish_available_slots: 0,
            eligible: Vec::new(),
            excluded_owned_summons,
        };
        Some(ActiveDraw {
            seed: draw.seed,
            census,
            reply: draw.reply,
        })
    }
}
