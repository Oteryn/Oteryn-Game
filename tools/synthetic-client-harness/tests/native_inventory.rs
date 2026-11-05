//! C08: production session loop -> mailbox -> inventory panel and selection.
use oteryn_client::combat_input::{CombatCommand, CombatReport};
use oteryn_client::play::{Stepper, play_channel, run_session};
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
use oteryn_session::{
    AppliedDelta, CharacterInventory, ItemEntry, LogoutResult, SessionError, SessionEvent,
    SpellCastDisposition, StepDirection, StepOutcome,
};
use std::time::{Duration, Instant};

struct InventorySession {
    inventory: CharacterInventory,
    events: Vec<SessionEvent>,
}
impl Stepper for InventorySession {
    fn feature_selected(&self, capability: u32) -> bool {
        capability == 15
    }
    fn inventory(&self) -> Option<CharacterInventory> {
        Some(self.inventory.clone())
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
        self.inventory.entries.clear();
        self.events.push(SessionEvent::Inventory(AppliedDelta {
            server_sequence: 1,
            base_revision: 1,
            new_revision: 2,
            value: self.inventory.clone(),
        }));
        Ok(CombatReport::Cast(SpellCastDisposition::Cast))
    }
    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        Ok(Some(LogoutResult::Accepted))
    }
}

#[test]
fn synthetic_inventory_session_scenario() -> Result<(), Box<dyn std::error::Error>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(scenario)?
        .join()
        .map_err(|_| "scenario thread failed")??;
    Ok(())
}
fn scenario() -> Result<(), String> {
    let item = |handle: u64| -> Result<ItemEntry, String> {
        Ok(ItemEntry {
            handle: handle.try_into().map_err(|_| "handle")?,
            item_definition_ref: 2854.try_into().map_err(|_| "definition")?,
            count: 1.try_into().map_err(|_| "count")?,
            sub_type: 0,
        })
    };
    let backpack = item(1)?;
    let carried = item(2)?;
    let session = InventorySession {
        inventory: CharacterInventory {
            main_backpack: Some(backpack),
            entries: vec![carried],
            equipment: vec![],
        },
        events: vec![],
    };
    let (link, commands, outbox) = play_channel();
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let task = runtime.spawn(run_session(session, commands, outbox));
    let deadline = Instant::now() + Duration::from_secs(3);
    while link.inventory_panel().inventory.is_none() && Instant::now() < deadline {
        std::thread::yield_now();
    }
    assert_eq!(link.inventory_panel().rows(), vec![backpack, carried]);
    assert!(link.inventory_input(&[NormalizedInputEvent::Key {
        code: KeyCode::new(12).map_err(|e| e.to_string())?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control),
        repeat: false,
    }]));
    assert!(link.inventory_panel().open);
    assert!(link.inventory_select(carried.handle));
    assert_eq!(link.inventory_panel().selected_item(), Some(carried));
    assert!(link.inventory_use_with());
    link.request_combat(CombatCommand::Cast(1.try_into().map_err(|_| "spell")?));
    while link.inventory_panel().rows().len() == 2 && Instant::now() < deadline {
        std::thread::yield_now();
    }
    assert_eq!(link.inventory_panel().rows(), vec![backpack]);
    assert_eq!(link.inventory_panel().selected_item(), None);
    while !link
        .world_context_feedback()
        .is_some_and(|text| text.contains("disappeared"))
        && Instant::now() < deadline
    {
        std::thread::yield_now();
    }
    assert!(
        link.world_context_feedback()
            .is_some_and(|text| text.contains("disappeared"))
    );
    link.request_logout();
    runtime.block_on(task).map_err(|e| e.to_string())?;
    assert!(link.inventory_panel().inventory.is_none());
    assert!(!link.inventory_panel().open);
    assert!(!link.inventory_select(backpack.handle));
    Ok(())
}
