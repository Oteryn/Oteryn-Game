use super::*;
use crate::gameplay_transport::{ComposedFreshAdmission, source_death::PreparedSourceDeathAction};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Aspect,
    Focus,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::gameplay_transport) struct CopiedDeath {
    kind: Kind,
    unmastered: bool,
}
impl CopiedDeath {
    pub(in crate::gameplay_transport) fn capture(
        runtime: &ChannelRuntimeV1,
        begin: &SourceCreatureDeathSeal,
        event: &crate::content::QualifiedNativeDeathEvent,
    ) -> Result<Option<Self>, ()> {
        let (kind, expected) = match event.name() {
            "SoulWarAspectOfPowerDeath" => (Kind::Aspect, ASPECT),
            "NecromanticFocusDeath" => (Kind::Focus, "oteryn:creature.necromantic_focus"),
            _ => return Ok(None),
        };
        if begin.definition_key() != expected
            || !begin.current_for(runtime)
            || !begin.events().contains(event)
        {
            return Err(());
        }
        let current = runtime
            .companion_snapshot_including_dead(begin.actor())
            .map_err(|_| ())?;
        Ok(Some(Self {
            kind,
            unmastered: current.state.master.is_none()
                && runtime.native_summon_master(begin.actor()).is_none(),
        }))
    }

    pub(in crate::gameplay_transport) async fn execute(
        &self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        action: &PreparedSourceDeathAction,
    ) -> Result<(), ()> {
        if !self.unmastered || self.kind == Kind::Focus {
            // The retained Focus callback calls increaseNecromaticMegalomaniaStrength
            // after 300s; its undefined necromanticRemainsId makes it a literal
            // no-op (accepted W11 qualification). It grants no +5 or Item effect.
            return Ok(());
        }
        let room = owner.effective_rooms().gate_monster_ai().ok_or(())?;
        let native = owner
            .active_generation
            .and_then(|active| active.native_gameplay())
            .ok_or(())?;
        if room.pinned_activity_source_room() != Some(crate::content::bosses::activity_template::PinnedActivitySourceRoom::GoshnarsMegalomania) {
            return Err(());
        }
        let plan = {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.try_lock().map_err(|_| ())?;
            if !action.current_for(&runtime) {
                return Err(());
            }
            let mego = states.soul_war_callbacks.megalomania.as_ref().ok_or(())?;
            if mego.phases.completed_aspect.as_ref() == Some(action.begin()) {
                return Ok(());
            }
            if mego.phases.aspect.is_some() {
                None
            } else {
                Some(owner.prepare_source_names(&runtime, &[NAME])?)
            }
        };
        let names = if let Some(plan) = &plan {
            Some(owner.read_source_names(permit, plan).await?)
        } else {
            None
        };
        let mut runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.try_lock().map_err(|_| ())?;
        if !action.current_for(&runtime) {
            return Err(());
        }
        let names = if let Some(read) = names {
            owner.seal_source_names(&runtime, read)?
        } else {
            Vec::new()
        };
        let mut mego = states.soul_war_callbacks.megalomania.take().ok_or(())?;
        let result = consume_aspect(
            &mut runtime,
            &mut mego,
            room,
            native,
            action.begin(),
            &names,
            action.now_us(),
        );
        states.soul_war_callbacks.megalomania = Some(mego);
        result
    }

    pub(in crate::gameplay_transport) fn completed(
        &self,
        states: &ChannelSpellStates,
        runtime: &ChannelRuntimeV1,
        action: &PreparedSourceDeathAction,
    ) -> bool {
        action.current_for(runtime)
            && (!self.unmastered
                || self.kind == Kind::Focus
                || states
                    .soul_war_callbacks
                    .megalomania
                    .as_ref()
                    .is_some_and(|owner| {
                        owner.phases.completed_aspect.as_ref() == Some(action.begin())
                    }))
    }
}
