//! Crystal 201189f feaster_of_souls/creaturescripts_brother_worm_death.lua:3-20.
use super::*;
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub(super) struct Link {
    pub(super) delayed: ExactActorRef,
    pub(super) original: ExactActorRef,
    pub(super) radius: u8,
    pub(super) effect: u16,
    pub(super) cells: crate::content::NativeEntryMovementCells,
}

impl Registry {
    pub(in crate::gameplay_transport) fn link(
        &self,
        runtime: &ChannelRuntimeV1,
        delayed: ExactActorRef,
        original: ExactActorRef,
        radius: u8,
        effect: u16,
        cells: crate::content::NativeEntryMovementCells,
    ) -> Result<(), CarrierError> {
        let mut links = self.links.lock().map_err(|_| CarrierError::PlanConflict)?;
        links.retain(|link| runtime.contains_live_creature(link.delayed));
        if !runtime.contains_live_creature(delayed)
            || delayed == original
            || links.len() >= MAX_ENCOUNTERS
            || links.iter().any(|link| link.delayed == delayed)
        {
            return Err(CarrierError::PlanConflict);
        }
        links
            .try_reserve(1)
            .map_err(|_| CarrierError::AllocationFailed)?;
        links.push(Link {
            delayed,
            original,
            radius,
            effect,
            cells,
        });
        Ok(())
    }

    /// Consume the registered pair when its second member actually dies.
    pub(in crate::gameplay_transport) fn finish_link(
        &self,
        runtime: &mut ChannelRuntimeV1,
        delayed: ExactActorRef,
        now_us: u64,
    ) -> Option<ExactActorRef> {
        let source = runtime
            .borrow_combat_death()
            .committed_lethal_receipt(delayed)
            .ok()?
            .statistics_death_key()
            .ok()?;
        let mut links = self.links.lock().ok()?;
        let index = links.iter().position(|link| link.delayed == delayed)?;
        let link = &links[index];
        let died = runtime.read_actor_position(delayed).ok()?.position();
        let Ok(original) = runtime.companion_snapshot(link.original) else {
            links.swap_remove(index);
            return None;
        };
        let at = runtime.read_actor_position(link.original).ok()?.position();
        let radius = u32::from(link.radius);
        if at.floor != died.floor
            || at.x.abs_diff(died.x) > radius
            || at.y.abs_diff(died.y) > radius
        {
            links.swap_remove(index);
            return None;
        }
        let digest: [u8; 32] = Sha256::new()
            .chain_update(b"oteryn:linked-encounter-death:v1")
            .chain_update(source.native_bytes())
            .finalize()
            .into();
        let result = runtime
            .borrow_exact_actor_commit_at_us(now_us)
            .commit_damage(
                link.original,
                oteryn_gameplay_entities::OwnerDamageCommand {
                    target: original.state.policy.definition_key.as_bytes(),
                    occurrence: &digest,
                    binding: &digest,
                    damage: original.health,
                },
            )
            .ok()?;
        if result.health_after != 0 {
            return None;
        }
        let finished = links.swap_remove(index);
        if let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
            finished.cells.index()
            && let (Ok(x), Ok(y), Ok(floor)) = (
                u16::try_from(at.x),
                u16::try_from(at.y),
                i8::try_from(-at.floor),
            )
        {
            index.map().facts.emit_movement_effect(
                crate::map::overlay::TilePos { x, y, floor },
                finished.effect,
            );
        }
        Some(finished.original)
    }
}
