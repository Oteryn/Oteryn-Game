//! C11: live equipment projection and equip/unequip/drop through the production command loop.
use oteryn_client::{
    combat_input::{CombatCommand, CombatReport},
    play::{Stepper, play_channel, run_session},
};
use oteryn_client_ui::equipment::{EquipmentSlot, EquippedItem};
use oteryn_client_ui::inventory::movement::DragItem;
use oteryn_session::{
    AppliedDelta, CharacterInventory, ItemEntry, ItemMoveDestination, ItemMoveOutcome,
    LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
    item_movement::{MoveRequest, MoveResponse, WorldTilePosition},
};
use std::time::{Duration, Instant};
struct Items {
    inventory: CharacterInventory,
    item: ItemEntry,
    events: Vec<SessionEvent>,
    moves: u64,
}
impl Stepper for Items {
    fn inventory(&self) -> Option<CharacterInventory> {
        Some(self.inventory.clone())
    }
    fn equipment_selected(&self) -> bool {
        true
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
        assert_eq!(request.source, self.item.handle);
        assert_eq!(request.count, (self.moves == 7).then_some(50));
        match self.moves {
            1 | 3 => {
                assert_eq!(
                    request.destination,
                    ItemMoveDestination::Equipment(EquipmentSlot::Ammo)
                );
                self.inventory.entries.clear();
                self.inventory.equipment = vec![EquippedItem {
                    slot: EquipmentSlot::Ammo,
                    item: self.item,
                }];
            }
            2 | 6 => {
                assert_eq!(request.destination, ItemMoveDestination::MainBackpack);
                self.inventory.equipment.clear();
                self.inventory.entries = vec![self.item];
            }
            4 | 5 => {
                if self.moves == 4 {
                    assert_eq!(
                        request.destination,
                        ItemMoveDestination::Container(
                            8.try_into().map_err(|_| SessionError::SessionUnusable)?
                        )
                    );
                } else {
                    assert!(matches!(
                        request.destination,
                        ItemMoveDestination::Ground(_)
                    ));
                }
                // The current server explicitly refuses equipped sources in loop_move.
                return Ok(Some(MoveResponse {
                    outcome: ItemMoveOutcome::NotSupported,
                    detail: None,
                }));
            }
            7 => {
                assert!(matches!(
                    request.destination,
                    ItemMoveDestination::Ground(_)
                ));
                self.inventory.equipment.clear();
                self.inventory.entries.clear();
            }
            _ => return Err(SessionError::SessionUnusable),
        }
        self.events.push(SessionEvent::Inventory(AppliedDelta {
            server_sequence: self.moves,
            base_revision: self.moves,
            new_revision: self.moves + 1,
            value: self.inventory.clone(),
        }));
        Ok(Some(MoveResponse {
            outcome: ItemMoveOutcome::Moved,
            detail: None,
        }))
    }
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
#[test]
fn equip_unequip_container_drop_refusal_and_disconnect() -> Result<(), Box<dyn std::error::Error>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(scenario)?
        .join()
        .map_err(|_| "scenario failed")??;
    Ok(())
}
fn scenario() -> Result<(), String> {
    let item = ItemEntry {
        handle: 7.try_into().map_err(|_| "handle")?,
        item_definition_ref: 42.try_into().map_err(|_| "definition")?,
        count: 50.try_into().map_err(|_| "count")?,
        sub_type: 0,
    };
    let session = Items {
        inventory: CharacterInventory {
            entries: vec![item],
            ..Default::default()
        },
        item,
        events: vec![],
        moves: 0,
    };
    let (link, commands, outbox) = play_channel();
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let task = runtime.spawn(run_session(session, commands, outbox));
    wait_for(|| link.inventory_panel().inventory.is_some())?;
    let ground = ItemMoveDestination::Ground(WorldTilePosition {
        x: 1,
        y: 0,
        floor: 0,
    });
    for (destination, equipped, refused) in [
        (
            ItemMoveDestination::Equipment(EquipmentSlot::Ammo),
            true,
            false,
        ),
        (ItemMoveDestination::MainBackpack, false, false),
        (
            ItemMoveDestination::Equipment(EquipmentSlot::Ammo),
            true,
            false,
        ),
        (
            ItemMoveDestination::Container(8.try_into().map_err(|_| "container")?),
            true,
            true,
        ),
        (ground, true, true),
        (ItemMoveDestination::MainBackpack, false, false),
        (ground, false, false),
    ] {
        let source = if link.equipment_slots()[9].item.is_some() {
            link.equipment_slots()[9]
                .drag_item()
                .ok_or("equipped source")?
        } else {
            DragItem {
                equipped: false,
                handle: item.handle,
                count: 50,
            }
        };
        assert!(link.item_drag_begin(source));
        link.item_drag_drop(Some(destination));
        if destination == ground && !equipped {
            assert!(link.item_movement().prompt.is_some());
            link.item_move_confirm(Some(50));
        } else {
            assert!(link.item_movement().prompt.is_none());
        }
        wait_for(|| {
            !link.item_movement().pending
                && link.equipment_slots()[9].item.is_some() == equipped
                && (destination != ground || equipped || link.inventory_panel().rows().is_empty())
        })?;
        assert_eq!(link.equipment_slots()[9].item, equipped.then_some(item));
        if refused {
            assert_eq!(
                link.item_movement().feedback,
                Some("This item cannot be moved there")
            );
        } else {
            assert_eq!(link.item_movement().feedback, Some("Item moved"));
        }
    }
    assert!(link.inventory_panel().rows().is_empty());
    link.request_logout();
    runtime.block_on(task).map_err(|e| e.to_string())?;
    assert!(
        link.equipment_slots()
            .iter()
            .all(|slot| !slot.available && slot.item.is_none())
    );
    Ok(())
}
