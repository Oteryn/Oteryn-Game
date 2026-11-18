//! Crystal 201189f creaturescripts_izcandar.lua:11-72.
use crate::foundation::{CarrierError, ChannelRuntimeV1, ExactActorRef, MovementLocalPosition};
use oteryn_gameplay_quests::quest::world_callbacks::PositionForms;
use sha2::{Digest, Sha256};

pub(in crate::gameplay_transport) struct Forms {
    base: String,
    spec: PositionForms,
    pending: Option<(u64, usize)>,
}

impl Forms {
    pub(in crate::gameplay_transport) const fn new(base: String, spec: PositionForms) -> Self {
        Self {
            base,
            spec,
            pending: None,
        }
    }

    pub(in crate::gameplay_transport) fn next_due_us(&self) -> Option<u64> {
        self.pending.map(|(due, _)| due)
    }

    pub(in crate::gameplay_transport) fn observe(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        boss: &mut ExactActorRef,
        now: u64,
    ) -> Result<bool, CarrierError> {
        let subject = runtime.companion_snapshot(*boss)?;
        let at = runtime.read_actor_position(*boss)?.position();
        let Some((index, region)) = self.spec.regions.iter().enumerate().find(|(_, region)| {
            at.floor == -i16::from(region.min.floor)
                && (i32::from(region.min.x)..=i32::from(region.max.x)).contains(&at.x)
                && (i32::from(region.min.y)..=i32::from(region.max.y)).contains(&at.y)
        }) else {
            return Ok(false);
        };
        let current = &subject.state.policy.definition_key;
        if current == &region.creature {
            return Ok(false);
        }
        if current != &self.base {
            *boss = replace_form(runtime, *boss, &self.base, subject.health, at)?;
            self.pending = None;
        }
        let [first, last] = self.spec.delay_seconds.map(u64::from);
        let digest = Sha256::new()
            .chain_update(b"oteryn:quest-position-form:v1")
            .chain_update(boss.placement_identity())
            .chain_update(now.to_be_bytes())
            .finalize();
        let mut sample = [0; 8];
        sample.copy_from_slice(&digest[..8]);
        let delay = first + u64::from_be_bytes(sample) % (last - first + 1);
        let due = now
            .checked_add(delay * 1_000_000)
            .ok_or(CarrierError::PlanConflict)?;
        // Each think may schedule a callback. Once the earliest callback replaces
        // this incarnation, later callbacks for it cannot match a live source.
        if self.pending.is_none_or(|(previous, _)| due < previous) {
            self.pending = Some((due, index));
            return Ok(true);
        }
        Ok(false)
    }

    pub(in crate::gameplay_transport) fn publish_due(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        boss: &mut ExactActorRef,
        now: u64,
    ) -> Result<(), CarrierError> {
        let Some((_, index)) = self.pending.filter(|(due, _)| *due <= now) else {
            return Ok(());
        };
        self.pending = None;
        let subject = runtime.companion_snapshot(*boss)?;
        if subject.state.policy.definition_key != self.base {
            return Ok(());
        }
        let at = runtime.read_actor_position(*boss)?.position();
        *boss = replace_form(
            runtime,
            *boss,
            &self.spec.regions[index].creature,
            subject.health,
            at,
        )?;
        Ok(())
    }
}

fn replace_form(
    runtime: &mut ChannelRuntimeV1,
    boss: ExactActorRef,
    key: &str,
    health: i64,
    at: MovementLocalPosition,
) -> Result<ExactActorRef, CarrierError> {
    let health = health.min(runtime.companion_policy(key)?.maximum_health);
    let (_, replacement) = runtime.commit_quest_creature_spawn_with_owner(
        &[(key, health, at)],
        |runtime, actors| {
            let [replacement] = actors else {
                return Err(CarrierError::PlanConflict);
            };
            runtime.remove_encounter_actor(boss)?;
            Ok(*replacement)
        },
    )?;
    Ok(replacement)
}
