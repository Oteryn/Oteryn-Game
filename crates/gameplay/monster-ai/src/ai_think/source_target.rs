//! Crystal201189f creature_functions.cpp:315 and creature.cpp:953-977.
//! A source setTarget changes the attack pointer, not the committed report,
//! follow goal, shared attack clock or taunt override slots.
use super::*;
use crate::foundation::ChannelRuntimeV1;
use oteryn_gameplay_entities::CarrierError;

impl CreatureAiTable {
    /// Consume the Elder Bloodjaw's source onThink selection under the current owner.
    pub fn set_current_bloodjaw_target(
        &mut self,
        runtime: &ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
        actor: ExactActorRef,
        target: ExactActorRef,
        now_us: u64,
    ) -> Result<bool, CarrierError> {
        const KEY: &str = "oteryn:creature.elder_bloodjaw";
        if !runtime.owner_fence()?.accepts_stamp(stamp) {
            return Err(CarrierError::WrongScope);
        }
        let creature = runtime.companion_snapshot(actor)?;
        if creature.state.policy.definition_key != KEY
            || creature.health <= 0
            || creature.state.master.is_some()
            || runtime.native_summon_master(actor).is_some()
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        if !runtime
            .positioned_melee_census()?
            .iter()
            .any(|(player, _, session)| *player == target && session.is_some())
        {
            return Err(CarrierError::InvalidCreatureTarget);
        }
        let position = runtime.read_actor_position(target)?.position();
        let visible = creature.position.floor == position.floor
            && targeting::sees_native(creature.position, position);
        let entry = self
            .entry_mut(actor)
            .ok_or(CarrierError::InvalidCreatureTarget)?;
        if !entry.state.profile.is_native() || entry.state.creature_key.as_deref() != Some(KEY) {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        if !visible {
            entry.state.target = None;
            return Ok(false);
        }
        entry.state.target = Some(target);
        // Setting an attacked creature wakes an idle monster. Retain one deadline;
        // the report and any already selected casts keep their original recipient.
        if entry.due_us.is_none() {
            entry.due_us = Some(now_us.saturating_add(CREATURE_THINK_INTERVAL_MILLIS * 1000));
        }
        Ok(true)
    }
}
