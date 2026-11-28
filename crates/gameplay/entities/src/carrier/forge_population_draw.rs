//! One-shot observation of the due Forge population turn in qualification builds.
use super::*;
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
    /// Newly classified incarnations in application order.
    pub selected: Vec<(ExactActorRef, ForgeCreature)>,
    /// Each native spawn's planner identity and exact incarnation, before selection.
    pub eligible: Vec<(u64, ExactActorRef)>,
    /// All live controlled summons, including familiars, in the Channel.
    pub excluded_owned_summons: Vec<ExactActorRef>,
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
            selected: Vec::new(),
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
