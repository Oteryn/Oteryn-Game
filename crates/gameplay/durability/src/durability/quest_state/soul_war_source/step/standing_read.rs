//! Adapt the existing SQL-fenced standing-tile read; detached Item names or
//! client handles cannot construct this bridge's actual Ground selection.
use super::*;
use crate::durability::spell_item_transaction::StandingTileRead;
use crate::durability::spell_items_abi::DurableTileItem;

struct CurrentStandingItems<'a> {
    read: &'a StandingTileRead,
    actor: ExactActorRef,
    session: GameSessionId,
    movement: MovementPositionSnapshot,
    selected: Option<&'a DurableTileItem>,
}
impl SoulWarStepItemRead for CurrentStandingItems<'_> {
    fn current(&self, runtime: &ChannelRuntimeV1) -> bool {
        let fence = self.read.fence();
        runtime.owner_fence().is_ok()
            && fence.game_session_id == self.session
            && fence.runtime_scope == runtime.binding().runtime_scope()
            && fence.scope_ownership_generation == runtime.binding().scope_generation()
            && self.read.content_digest() == runtime.content_pin().server_artifact_digest()
            && runtime.read_actor_position(self.actor) == Ok(self.movement)
    }
    fn items(&self, position: MovementLocalPosition) -> Vec<SourceStepGroundItem> {
        if position != self.movement.position() {
            return Vec::new();
        }
        self.selected
            .into_iter()
            .map(|item| SourceStepGroundItem {
                item: item.item_instance_id,
                definition: item.definition.clone(),
                position,
            })
            .collect()
    }
}
impl SoulWarSourceStep {
    /// The caller keeps the physical lane through the subsequent fused write;
    /// that write independently rechecks each original UID and reconciles first.
    #[allow(
        clippy::too_many_arguments,
        reason = "existing SQL read, current movement and native room are independent owners"
    )]
    pub fn from_standing_read(
        runtime: &ChannelRuntimeV1,
        native: &crate::content::native_gameplay::NativeGameplayState,
        room: &crate::content::QualifiedNativeEntryRoom,
        actor: ExactActorRef,
        session: GameSessionId,
        movement: MovementPositionSnapshot,
        reset_epoch: u64,
        read: &StandingTileRead,
    ) -> Option<Vec<Self>> {
        let expected =
            SpellGroundTarget::for_native_tile_read(room, runtime, movement.position()).ok()?;
        if read.target() != &expected {
            return None;
        }
        let reader = CurrentStandingItems {
            read,
            actor,
            session,
            movement,
            selected: None,
        };
        if !reader.current(runtime) {
            return None;
        }
        Some(
            read.items()
                .iter()
                .filter_map(|item| {
                    let reader = CurrentStandingItems {
                        read,
                        actor,
                        session,
                        movement,
                        selected: Some(item),
                    };
                    Self::from_current_owner(
                        runtime,
                        native,
                        room,
                        actor,
                        session,
                        movement,
                        reset_epoch,
                        item.item_instance_id,
                        &reader,
                    )
                })
                .collect(),
        )
    }
}
