use super::*;
use crate::content::native_gameplay::QualifiedItemPolicy;
use crate::durability::item_loop_move::monster_push::{self, MonsterPushOccurrence};
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::durability::spell_items_abi::DurableTileItem;

pub(super) fn movable_blocker(item: &DurableTileItem, policy: QualifiedItemPolicy<'_>) -> bool {
    use crate::content::ReferenceItemField::Known;
    item.blocks_movement
        && !item.immovable_block_solid
        && item.expires_at_unix_ms.is_none()
        && matches!(&policy.record().semantics.physical, Known(physical) if physical.movable == Known(true))
}

impl MonsterMap {
    pub fn admits_monster_approach(&self, at: MovementLocalPosition, push_items: bool) -> bool {
        self.tile(at)
            .is_some_and(|tile| tile.admits || push_items && tile.pushable_blockers)
    }
}

impl<'a> CurrentMonsterMap<'a, '_> {
    pub fn bind_monster_push_owner(&mut self, node: &'a NodeIncarnationProof, reset_epoch: u64) {
        self.monster_push_owner = Some((node, reset_epoch));
    }

    /// Stage a whole pile of blockers in this physical owner transaction.
    /// The caller must commit before advancing the prepared Creature STEP.
    pub async fn push_blocking_items(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        due: u64,
        at: MovementLocalPosition,
        landings: &[MovementLocalPosition],
    ) -> Result<bool, SpellItemError> {
        let Some((node, reset_epoch)) = self.monster_push_owner else {
            return Ok(false);
        };
        let occurrence = MonsterPushOccurrence {
            actor,
            due,
            node,
            reset_epoch,
        };
        self.read(runtime, at).await;
        if !self
            .view
            .tile(at)
            .is_some_and(|tile| tile.pushable_blockers)
        {
            return Ok(false);
        }
        let source = self.read_items(runtime, at).await?;
        let mut landings_with_room = Vec::with_capacity(landings.len());
        for &landing in landings {
            if self.read(runtime, landing).await != Some(true) {
                continue;
            }
            let items = self.read_items(runtime, landing).await?;
            let room = crate::durability::ground_cell_limit::GROUND_CELL_ROOT_LIMIT
                .saturating_sub(items.items.len());
            landings_with_room.push((items.target, room));
        }
        let blockers = source.items.iter().filter(|item| item.blocks_movement);
        if blockers.clone().count() > landings_with_room.iter().map(|(_, room)| room).sum() {
            return Ok(false);
        }
        for item in blockers.clone() {
            if !monster_push::unprotected_blocker(self.tx, item).await? {
                return Ok(false);
            }
        }
        let mut destinations = landings_with_room
            .iter()
            .flat_map(|(target, room)| std::iter::repeat_n(target, *room));
        for (item, destination) in blockers.zip(&mut destinations) {
            monster_push::push_ground_blocker(
                self.tx,
                self.authority,
                &occurrence,
                &source.target,
                destination,
                item,
            )
            .await?;
        }
        Ok(true)
    }
}
