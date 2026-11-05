//! C10: the production command loop connects drag/count state, sequenced inventory, and refusal text.
use oteryn_client::{
    combat_input::{CombatCommand, CombatReport},
    play::{Stepper, play_channel, run_session},
};
use oteryn_client_ui::inventory::movement::DragItem;
use oteryn_session::{
    AppliedDelta, CharacterInventory, ItemEntry, ItemMoveDestination, ItemMoveOutcome,
    LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
    item_movement::{MoveRequest, MoveResponse, RefusalDetail},
};
use std::time::{Duration, Instant};
struct Items {
    inventory: CharacterInventory,
    events: Vec<SessionEvent>,
    moves: usize,
}
impl Stepper for Items {
    fn inventory(&self) -> Option<CharacterInventory> {
        Some(self.inventory.clone())
    }
    fn item_count_selected(&self) -> bool {
        true
    }
    fn drain(&mut self) -> Vec<SessionEvent> {
        std::mem::take(&mut self.events)
    }
    async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
        tokio::time::sleep(duration).await;
        Ok(())
    }
    async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        Ok(Some(LogoutResult::Accepted))
    }
    async fn move_item(
        &mut self,
        request: MoveRequest,
    ) -> Result<Option<MoveResponse>, SessionError> {
        self.moves += 1;
        assert_eq!(request.source, self.inventory.entries[0].handle);
        if self.moves == 1 {
            assert_eq!(request.count, Some(7));
            assert!(matches!(
                request.destination,
                ItemMoveDestination::Container(_)
            ));
            self.inventory.entries[0].count =
                13.try_into().map_err(|_| SessionError::SessionUnusable)?;
            self.events.push(SessionEvent::Inventory(AppliedDelta {
                server_sequence: 1,
                base_revision: 1,
                new_revision: 2,
                value: self.inventory.clone(),
            }));
            Ok(Some(MoveResponse {
                outcome: ItemMoveOutcome::Moved,
                detail: None,
            }))
        } else {
            assert!(matches!(
                request.destination,
                ItemMoveDestination::Ground(_)
            ));
            Ok(Some(MoveResponse {
                outcome: ItemMoveOutcome::NoRoom,
                detail: Some(RefusalDetail::NotEnoughCapacity),
            }))
        }
    }
}
#[test]
fn drag_split_move_and_refusal_scenario() -> Result<(), Box<dyn std::error::Error>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(scenario)?
        .join()
        .map_err(|_| "scenario failed")??;
    Ok(())
}
fn wait_for(mut condition: impl FnMut() -> bool) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !condition() {
        if Instant::now() >= deadline {
            return Err("scenario deadline".into());
        }
        std::thread::yield_now();
    }
    Ok(())
}
fn scenario() -> Result<(), String> {
    let entry = ItemEntry {
        handle: 2.try_into().map_err(|_| "handle")?,
        item_definition_ref: 3031.try_into().map_err(|_| "definition")?,
        count: 20.try_into().map_err(|_| "count")?,
        sub_type: 0,
    };
    let session = Items {
        inventory: CharacterInventory {
            entries: vec![entry],
            ..Default::default()
        },
        events: vec![],
        moves: 0,
    };
    let (link, commands, outbox) = play_channel();
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let task = runtime.spawn(run_session(session, commands, outbox));
    wait_for(|| link.inventory_panel().inventory.is_some())?;
    assert!(link.item_drag_begin(DragItem {
        equipped: false,
        handle: entry.handle,
        count: 20
    }));
    link.item_drag_drop(Some(ItemMoveDestination::Container(
        3.try_into().map_err(|_| "container")?,
    )));
    assert!(link.item_movement().prompt.is_some());
    link.item_move_confirm(Some(0));
    assert!(link.item_movement().prompt.is_some());
    link.item_move_confirm(Some(7));
    wait_for(|| {
        link.inventory_panel()
            .rows()
            .first()
            .is_some_and(|e| e.count.get() == 13)
    })?;
    assert_eq!(link.item_movement().feedback, Some("Item moved"));
    assert!(link.item_drag_begin(DragItem {
        equipped: false,
        handle: entry.handle,
        count: 13
    }));
    link.item_drag_drop(Some(ItemMoveDestination::Ground(
        oteryn_session::item_movement::WorldTilePosition {
            x: 1,
            y: 0,
            floor: 0,
        },
    )));
    link.item_move_confirm(Some(13));
    wait_for(|| !link.item_movement().pending)?;
    assert_eq!(
        link.item_movement().feedback,
        Some("This object is too heavy for you to carry.")
    );
    assert_eq!(link.inventory_panel().rows()[0].count.get(), 13);
    link.request_logout();
    runtime.block_on(task).map_err(|e| e.to_string())?;
    assert_eq!(link.item_movement(), Default::default());
    assert!(!link.item_drag_begin(DragItem {
        equipped: false,
        handle: entry.handle,
        count: 13
    }));
    Ok(())
}
