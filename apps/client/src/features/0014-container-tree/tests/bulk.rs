#[test]
fn managed_container_moves_all_through_live_handles_and_stops_on_refusal()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::container_tree::{ContainerView, ContainerViews};
    use oteryn_session::{ItemEntry, ItemMoveDestination, ItemMoveOutcome};
    let item = |handle: u64| -> Result<ItemEntry, Box<dyn std::error::Error>> {
        Ok(ItemEntry {
            handle: handle.try_into()?,
            item_definition_ref: 42.try_into()?,
            count: 7.try_into()?,
            sub_type: 0,
        })
    };
    let (link, mut commands, outbox) = play_channel();
    outbox.container_initialize(Some(ContainerViews {
        views: vec![
            ContainerView {
                view_id: 0,
                container_handle: 10.try_into()?,
                parent_handle: None,
                capacity: 20,
                entries: vec![item(11)?, item(12)?],
                browse: None,
            },
            ContainerView {
                view_id: 1,
                container_handle: 20.try_into()?,
                parent_handle: None,
                capacity: 20,
                entries: vec![],
                browse: None,
            },
        ],
    }));
    assert!(link.container_activate(0));
    assert!(!link.container_move_all(0));
    assert!(commands.try_recv().is_err());
    assert!(link.container_move_all(1));
    let crate::combat_input::PlayCommand::MoveContents(requests) = commands.try_recv()? else {
        return Err("move contents command".into());
    };
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].source, item(11)?.handle);
    assert_eq!(requests[1].source, item(12)?.handle);
    assert_eq!(
        requests[0].destination,
        ItemMoveDestination::Container(20.try_into()?)
    );
    assert_eq!(requests[0].count, None);
    assert!(!link.container_move_all(1));
    assert!(
        outbox.container_bulk_result(Some(oteryn_client_runtime::inventory::MoveResponse {
            outcome: ItemMoveOutcome::Moved,
            detail: None
        }))
    );
    assert!(
        !outbox.container_bulk_result(Some(oteryn_client_runtime::inventory::MoveResponse {
            outcome: ItemMoveOutcome::NoRoom,
            detail: None
        }))
    );
    let panel = link.container_panel();
    assert!(!panel.bulk.pending);
    assert_eq!(panel.bulk.completed, 1);
    assert_eq!(panel.bulk.total, 2);
    assert!(oteryn_client_ui::container_tree::view::panel_text(&panel).contains("1 / 2 moved"));
    assert!(oteryn_client_ui::container_tree::view::panel_text(&panel).contains("room"));
    outbox.container_end();
    assert!(!link.container_move_all(1));
    Ok(())
}

#[test]
fn managed_container_menu_opens_the_selected_live_inventory_container()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    outbox.container_initialize(Some(Default::default()));
    let backpack = oteryn_session::ItemEntry {
        handle: 10.try_into()?,
        item_definition_ref: 42.try_into()?,
        count: 1.try_into()?,
        sub_type: 0,
    };
    outbox.inventory_initialize(
        Some(oteryn_session::CharacterInventory {
            main_backpack: Some(backpack),
            ..Default::default()
        }),
        false,
    );
    assert!(link.inventory_select(backpack.handle));
    assert!(!link.discovery_action(98, 9, &[], 0));
    assert!(link.discovery_action(98, 7, &[], 0));
    assert!(link.discovery_action(98, 9, &[], 0));
    assert_eq!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::ContainerView(
            oteryn_session::container_tree::ContainerViewIntent::Open {
                handle: backpack.handle,
                replace_view: None
            }
        )
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
use crate::{
    combat_input::{CombatCommand, CombatReport},
    play::{Stepper, play_channel, run_session},
};
use oteryn_client_runtime::{
    container_tree::{ContainerView, ContainerViews},
    inventory::{MoveRequest, MoveResponse},
};
use oteryn_session::{
    ItemMoveOutcome, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::{
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};
type Error = Box<dyn std::error::Error + Send + Sync>;

fn views() -> Result<ContainerViews, Error> {
    Ok(ContainerViews {
        views: vec![
            ContainerView {
                view_id: 1,
                container_handle: 1_u64.try_into()?,
                parent_handle: None,
                capacity: 20,
                browse: None,
                entries: (10_u64..13)
                    .map(|id| {
                        Ok(oteryn_session::ItemEntry {
                            handle: id.try_into()?,
                            item_definition_ref: 42_u32.try_into()?,
                            count: 7_u32.try_into()?,
                            sub_type: 0,
                        })
                    })
                    .collect::<Result<_, Error>>()?,
            },
            ContainerView {
                view_id: 2,
                container_handle: 2_u64.try_into()?,
                parent_handle: None,
                capacity: 20,
                browse: None,
                entries: vec![],
            },
        ],
    })
}
struct Peer {
    refuse_second: bool,
    views: ContainerViews,
    moved: Arc<Mutex<Vec<MoveRequest>>>,
}
impl Stepper for Peer {
    fn container_views(&self) -> Option<ContainerViews> {
        Some(self.views.clone())
    }
    async fn move_item(
        &mut self,
        request: MoveRequest,
    ) -> Result<Option<MoveResponse>, SessionError> {
        let mut moved = self.moved.lock().unwrap_or_else(PoisonError::into_inner);
        moved.push(request);
        Ok(Some(MoveResponse {
            outcome: if self.refuse_second && moved.len() == 2 {
                ItemMoveOutcome::NoRoom
            } else {
                ItemMoveOutcome::Moved
            },
            detail: None,
        }))
    }
    async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
        tokio::time::sleep(duration).await;
        Ok(())
    }
    fn drain(&mut self) -> Vec<SessionEvent> {
        vec![]
    }
    async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        Ok(None)
    }
}
#[test]
fn move_all_uses_live_container_handles_and_stops_at_first_server_refusal()
-> Result<(), Box<dyn std::error::Error>> {
    scenario(true)
}
#[test]
fn move_all_moves_every_displayed_stack_when_the_server_accepts()
-> Result<(), Box<dyn std::error::Error>> {
    scenario(false)
}
fn scenario(refuse_second: bool) -> Result<(), Box<dyn std::error::Error>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            let run = || -> Result<(), Error> {
                let runtime = oteryn_client_runtime::ClientRuntime::new()?;
                runtime.block_on(async {
                    let moved = Arc::new(Mutex::new(Vec::new()));
                    let (link, incoming, outbox) = play_channel();
                    let task = tokio::spawn(run_session(
                        Peer {
                            refuse_second,
                            views: views()?,
                            moved: moved.clone(),
                        },
                        incoming,
                        outbox,
                    ));
                    tokio::time::timeout(Duration::from_secs(3), async {
                        while link.container_panel().views.is_none() {
                            tokio::task::yield_now().await;
                        }
                    })
                    .await?;
                    assert!(link.container_activate(1));
                    assert!(!link.container_move_all_to(1));
                    assert!(!link.container_move_all_to(99));
                    assert!(link.container_move_all_to(2));
                    tokio::time::timeout(Duration::from_secs(3), async {
                        while link.item_movement().pending {
                            tokio::task::yield_now().await;
                        }
                    })
                    .await?;
                    let requests = moved.lock().unwrap_or_else(PoisonError::into_inner).clone();
                    assert_eq!(requests.len(), if refuse_second { 2 } else { 3 });
                    assert_eq!(requests[0].source, 10_u64.try_into()?);
                    assert_eq!(requests[1].source, 11_u64.try_into()?);
                    assert_eq!(
                        requests[0].destination,
                        oteryn_session::ItemMoveDestination::Container(2_u64.try_into()?)
                    );
                    assert_eq!(requests[0].count, None);
                    assert_eq!(
                        link.item_movement().feedback,
                        Some(if refuse_second {
                            "There is not enough room"
                        } else {
                            "Item moved"
                        })
                    );
                    drop(link);
                    task.await?;
                    Ok::<_, Error>(())
                })??;
                Ok(())
            };
            run().map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "bulk movement test thread failed")?
        .map_err(Into::into)
}
