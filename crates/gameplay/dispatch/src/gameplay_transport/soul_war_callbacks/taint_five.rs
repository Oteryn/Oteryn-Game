//! The real native one-second player cadence, after periodic conditions.
use super::super::ComposedFreshAdmission;
use super::*;
use crate::foundation::GameSessionId;

#[derive(Debug, Default)]
pub(super) struct FifthCadence {
    actors: Vec<(ExactActorRef, GameSessionId, Option<u64>, u8)>,
}
impl FifthCadence {
    fn observe(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        now: u64,
    ) -> bool {
        self.actors.retain(|(actor, session, _, _)| {
            runtime.player_control_facts(*actor, *session).is_ok()
        });
        let index = if let Some(index) = self
            .actors
            .iter()
            .position(|(a, s, _, _)| *a == actor && *s == session)
        {
            index
        } else {
            self.actors.push((actor, session, None, 0));
            self.actors.len() - 1
        };
        let (_, _, previous, count) = &mut self.actors[index];
        if previous.is_some_and(|previous| now.saturating_sub(previous) < 1_000_000) {
            return false;
        }
        // A late actual callback contributes one interval, never invented ticks.
        *previous = Some(now);
        *count += 1;
        if *count < 10 {
            return false;
        }
        *count = 0;
        true
    }
}
/// Constructed only immediately after the canonical scoped Quest reader below;
/// a public DTO or a client-supplied taint count cannot manufacture this value.
pub(in crate::gameplay_transport) struct QualifiedFifthTaint {
    binding: taints::Binding,
    actor: ExactActorRef,
    session: GameSessionId,
    now: u64,
    geometry: taints::Geometry,
    count: u8,
    revision: u64,
}
impl QualifiedFifthTaint {
    pub(in crate::gameplay_transport) fn permits(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        now: u64,
        revision: u64,
    ) -> bool {
        self.revision == revision
            && self.count >= 5
            && self.binding == taints::binding(runtime)
            && self.actor == actor
            && self.session == session
            && self.now == now
            && runtime.owner_fence().is_ok()
            && runtime.player_control_facts(actor, session).is_ok()
            && runtime
                .read_actor_position(actor)
                .is_ok_and(|p| self.geometry.contains(p.position()))
    }
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn tick_soul_war_fifth_taint(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<(u64, super::super::actor_spell::ActorVitals)> {
        let permit = self.spell_lane_permit().await?;
        let now = self.owner_now().get();
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if runtime
            .player_control_facts(actor, session)
            .ok()?
            .control_loss
            .is_some()
            || states.is_dead(actor)
            || states.has_pending_spell_commit(actor, session)
        {
            return None;
        }
        states.get(&runtime, actor, session)?;
        if !states
            .soul_war_callbacks
            .fifth
            .observe(&runtime, actor, session, now)
        {
            return None;
        }
        let room = self.effective_rooms().gate_monster_ai()?;
        let active = self.active_generation?;
        let native = active.native_gameplay()?;
        if !qualifies_source(&runtime, native) {
            return None;
        }
        let geometry = if let Some(role) = room
            .pinned_activity_source_room()
            .filter(|r| taints::boss_room(*r))
        {
            if runtime.content_pin().frame_binding_digest() != room.frame_binding().digest()
                || runtime.map_revision_digest() != room.map_revision_digest()
            {
                return None;
            }
            taints::Geometry::Boss(role)
        } else {
            let proof = active.qualify_published_world_context(room, runtime.content_pin())?;
            taints::Geometry::World(proof.source_frame())
        };
        if !geometry.contains(runtime.read_actor_position(actor).ok()?.position()) {
            return None;
        }
        let pass = self.root.try_issue_semantic_pass().ok()?;
        let mut context = (self, &mut *runtime, &mut *states, &permit, geometry);
        pass.run_with_context(&mut context,move|holder,deadline,context|Box::pin(async move {
            use crate::durability::spell_item_transaction::{assert_spell_item_scope_in_transaction,begin_spell_owner_transaction,check_scope_transaction};
            let (owner,runtime,states,permit,geometry)=context;
            let mut tx=begin_spell_owner_transaction(holder,deadline).await?;
            let scope=runtime.binding().runtime_scope();let generation=runtime.binding().scope_generation();
            let authority=assert_spell_item_scope_in_transaction(&mut tx,permit,owner.root,owner.character,owner.holder,scope,generation.get())
                .await.map_err(|_|crate::durability::DurabilityError::Unavailable)?;
            let facts=crate::durability::quest_state::soul_war::read_current_soul_war_taints_in_transaction(
                &mut tx,owner.root,&authority,runtime,&[(actor,session)]).await?;
            let Some(fact)=facts.first() else{return Ok(None);};
            let proof=QualifiedFifthTaint{binding:taints::binding(runtime),actor,session,now,geometry:*geometry,count:fact.count,revision:states.get(runtime,actor,session).ok_or(crate::durability::DurabilityError::Unavailable)?.revision()};
            // A failed read/transaction cannot publish partial HP or mana.
            check_scope_transaction(&mut tx,&authority).await.map_err(|_|crate::durability::DurabilityError::Unavailable)?;
            tx.rollback().await?;
            let Some(hit)=states.apply_soul_war_fifth_taint(runtime,actor,session,&proof,crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now)) else{return Ok(None);};
            Ok(states.get(runtime,actor,session).map(|state|(hit.vitals_revision,state.vitals())))
        })).await.ok().flatten()
    }
}
