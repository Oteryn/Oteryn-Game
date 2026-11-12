//! C09: production session task -> mailbox -> nested carried and ground container browser.
use oteryn_client::combat_input::{CombatCommand, CombatReport};
use oteryn_client::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_session::container_tree::{
    ContainerView, ContainerViewIntent, ContainerViewOutcome, ContainerViews,
};
use oteryn_session::{
    AppliedDelta, CharacterInventory, ItemEntry, LogoutResult, SessionError, SessionEvent,
    StepDirection, StepOutcome,
};
use std::time::{Duration, Instant};

fn item(id: u64) -> Result<ItemEntry, String> {
    Ok(ItemEntry {
        handle: id.try_into().map_err(|_| "handle")?,
        item_definition_ref: 2854.try_into().map_err(|_| "definition")?,
        count: 1.try_into().map_err(|_| "count")?,
        sub_type: 0,
    })
}
fn view(id: u8, container: u64, parent: Option<u64>, entry: u64) -> Result<ContainerView, String> {
    Ok(ContainerView {
        view_id: id,
        container_handle: item(container)?.handle,
        parent_handle: parent.map(item).transpose()?.map(|e| e.handle),
        capacity: 20,
        entries: vec![item(entry)?],
        browse: None,
    })
}
struct ContainerSession {
    views: ContainerViews,
    events: Vec<SessionEvent>,
    revision: u64,
}
impl Stepper for ContainerSession {
    fn inventory(&self) -> Option<CharacterInventory> {
        Some(CharacterInventory {
            main_backpack: item(10).ok(),
            ..Default::default()
        })
    }
    fn container_views(&self) -> Option<ContainerViews> {
        Some(self.views.clone())
    }
    async fn container_view(
        &mut self,
        intent: ContainerViewIntent,
    ) -> Result<Option<ContainerViewOutcome>, SessionError> {
        let next = match intent {
            ContainerViewIntent::Open { handle, .. } if handle.get() == 10 => {
                vec![view(3, 10, None, 11)]
            }
            ContainerViewIntent::Open {
                handle,
                replace_view: Some(3),
            } if handle.get() == 11 => vec![view(3, 11, Some(10), 12)],
            ContainerViewIntent::Open { handle, .. } if handle.get() == 42 => {
                let mut views: Vec<_> = self.views.views.iter().cloned().map(Ok).collect();
                views.push(view(7, 42, None, 43));
                views
            }
            ContainerViewIntent::Up { view_id: 3 } => vec![view(3, 10, None, 11)],
            ContainerViewIntent::Close { view_id } => self
                .views
                .views
                .iter()
                .filter(|v| v.view_id != view_id)
                .cloned()
                .map(Ok)
                .collect(),
            _ => return Ok(Some(ContainerViewOutcome::NotAContainer)),
        };
        self.views.views = next
            .into_iter()
            .collect::<Result<_, _>>()
            .map_err(|_| SessionError::SessionUnusable)?;
        self.events.push(SessionEvent::ContainerViews(AppliedDelta {
            server_sequence: self.revision + 1,
            base_revision: self.revision,
            new_revision: self.revision + 1,
            value: self.views.clone(),
        }));
        self.revision += 1;
        Ok(Some(
            if matches!(intent, ContainerViewIntent::Close { .. }) {
                ContainerViewOutcome::Closed
            } else {
                ContainerViewOutcome::Opened
            },
        ))
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
}
fn wait(
    link: &PlayLink,
    ready: impl Fn(&oteryn_client_ui::container_tree::ContainerPanel) -> bool,
) -> Result<(), String> {
    let until = Instant::now() + Duration::from_secs(3);
    loop {
        let panel = link.container_panel();
        if !panel.pending && ready(&panel) {
            return Ok(());
        }
        if Instant::now() >= until {
            return Err(format!("container browser timeout: {panel:?}"));
        }
        std::thread::yield_now();
    }
}
#[test]
fn synthetic_carried_nested_and_ground_container_scenario() -> Result<(), Box<dyn std::error::Error>>
{
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(scenario)?
        .join()
        .map_err(|_| "container scenario thread failed")??;
    Ok(())
}
fn scenario() -> Result<(), String> {
    let (link, commands, outbox) = play_channel();
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let task = runtime.spawn(run_session(
        ContainerSession {
            views: ContainerViews::default(),
            events: vec![],
            revision: 1,
        },
        commands,
        outbox,
    ));
    wait(&link, |p| p.views.is_some())?;
    assert!(link.inventory_select(item(10)?.handle));
    assert!(link.inventory_open_container());
    wait(&link, |p| {
        p.current().is_some_and(|v| v.container_handle.get() == 10)
    })?;
    assert!(link.container_panel().open);
    assert!(link.container_select(3, item(11)?.handle));
    assert!(link.container_open_selected());
    wait(&link, |p| {
        p.current().is_some_and(|v| v.container_handle.get() == 11)
    })?;
    assert_eq!(link.container_panel().rows(), &[item(12)?]);
    assert!(link.container_up());
    wait(&link, |p| {
        p.current().is_some_and(|v| v.container_handle.get() == 10)
    })?;
    // A selected live ground container's handle enters exactly the same production queue.
    assert!(link.container_open(item(42)?.handle));
    wait(&link, |p| p.active == Some(7))?;
    assert_eq!(link.container_panel().rows(), &[item(43)?]);
    assert!(!link.container_up());
    assert!(link.container_close());
    wait(&link, |p| {
        p.views.as_ref().is_some_and(|v| v.views.len() == 1)
    })?;
    assert_eq!(link.container_panel().active, Some(3));
    assert!(link.container_select(3, item(11)?.handle));
    // An ordinary non-container refusal leaves the current authoritative tree intact.
    assert!(link.container_open(item(99)?.handle));
    wait(&link, |p| p.feedback == Some("Not a container"))?;
    assert_eq!(link.container_panel().rows(), &[item(11)?]);
    assert!(link.container_close());
    wait(&link, |p| {
        p.views.as_ref().is_some_and(|v| v.views.is_empty())
    })?;
    link.request_logout();
    runtime.block_on(task).map_err(|e| e.to_string())?;
    assert!(link.container_panel().views.is_none());
    assert!(!link.container_open(item(10)?.handle));
    Ok(())
}
