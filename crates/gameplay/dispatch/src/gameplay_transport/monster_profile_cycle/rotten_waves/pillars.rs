//! Source Pillar callbacks keep their current physical incarnation and native sequence.
use super::*;
use oteryn_gameplay_encounters::bosses::rotten_blood::native_pillar::NativePillarThinkOwner;

#[derive(Default)]
pub(super) struct PillarStates {
    owners: BTreeMap<[u8; 16], NativePillarThinkOwner>,
}

impl PillarStates {
    pub(super) fn consume(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        table: &mut CreatureAiTable,
        actor: ExactActorRef,
        content: [u8; 32],
        sequence: u64,
    ) -> Result<(), ()> {
        self.owners
            .retain(|_, owner| runtime.contains_live_creature(owner.actor()));
        let id = actor.placement_identity();
        if !self
            .owners
            .get(&id)
            .is_some_and(|owner| owner.actor() == actor)
        {
            self.owners.insert(
                id,
                NativePillarThinkOwner::for_actor(runtime, actor, content).map_err(|_| ())?,
            );
        }
        if self
            .owners
            .get_mut(&id)
            .ok_or(())?
            .consume(runtime, sequence)
            .map_err(|_| ())?
        {
            table.remove(actor);
            self.owners.remove(&id);
        }
        Ok(())
    }
}
