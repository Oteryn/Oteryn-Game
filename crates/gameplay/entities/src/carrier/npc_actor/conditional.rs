//! One source-qualified conditional placement, independent of other copies of the same NPC.
use super::{ChannelRuntimeV1, ExactActorRef, NpcPlacement, NpcPlacementError, cell, key_ok};

/// Runtime projection of one already admitted, conditionally available NPC placement.
#[derive(Debug)]
pub struct ConditionalNpcActor {
    key: String,
    placement: NpcPlacement,
    actor: Option<ExactActorRef>,
}

impl ConditionalNpcActor {
    /// The caller supplies a placement admitted against the active map and NPC catalogue.
    pub fn new(key: String, placement: NpcPlacement) -> Result<Self, NpcPlacementError> {
        if !key_ok(&key) {
            return Err(NpcPlacementError::InvalidKey);
        }
        cell(placement)?;
        Ok(Self {
            key,
            placement,
            actor: None,
        })
    }

    /// Retry a return on a later tick when another actor occupies the source anchor.
    /// Other admission and ownership errors remain errors.
    pub fn reconcile_or_defer(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        present: bool,
    ) -> Result<Option<ExactActorRef>, NpcPlacementError> {
        match self.reconcile(runtime, present) {
            Err(NpcPlacementError::CellOccupied) => Ok(None),
            result => result,
        }
    }

    /// Project authoritative availability without removing another occurrence of this NPC.
    pub fn reconcile(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        present: bool,
    ) -> Result<Option<ExactActorRef>, NpcPlacementError> {
        runtime
            .carrier
            .validate_current_continuity(&runtime.continuity)
            .map_err(NpcPlacementError::Carrier)?;
        runtime.owner_fence().map_err(NpcPlacementError::Carrier)?;
        if !present {
            if let Some(actor) = self.actor
                && runtime.npc_key(actor) == Some(self.key.as_str())
            {
                runtime.remove_dynamic_npc_actor(actor)?;
            }
            self.actor = None;
            return Ok(None);
        }
        if let Some(actor) = self.actor
            && runtime.npc_key(actor) == Some(self.key.as_str())
        {
            return Ok(Some(actor));
        }
        self.actor = None;
        let actor = runtime.place_dynamic_npc(&self.key, self.placement)?;
        self.actor = Some(actor);
        Ok(Some(actor))
    }
}

#[cfg(test)]
#[path = "conditional/tests.rs"]
mod tests;
