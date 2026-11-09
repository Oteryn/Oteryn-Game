use oteryn_client_runtime::inventory::{MoveRequest, MoveResponse};
use oteryn_session::{ItemMoveOutcome, SessionError};

struct BulkSession {
    views: oteryn_session::container_tree::ContainerViews,
    moved: std::sync::mpsc::Sender<MoveRequest>,
    responses: std::collections::VecDeque<ItemMoveOutcome>,
}
impl crate::play::Stepper for BulkSession {
    fn container_views(&self) -> Option<oteryn_session::container_tree::ContainerViews> {
        Some(self.views.clone())
    }
    async fn move_item(
        &mut self,
        request: MoveRequest,
    ) -> Result<Option<MoveResponse>, SessionError> {
        self.moved
            .send(request)
            .map_err(|_| SessionError::SessionUnusable)?;
        Ok(Some(MoveResponse {
            outcome: self
                .responses
                .pop_front()
                .ok_or(SessionError::SessionUnusable)?,
            detail: None,
        }))
    }
    async fn step(
        &mut self,
        _: oteryn_session::StepDirection,
    ) -> Result<oteryn_session::StepOutcome, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn serve(&mut self, duration: std::time::Duration) -> Result<(), SessionError> {
        tokio::time::sleep(duration).await;
        Ok(())
    }
    fn drain(&mut self) -> Vec<oteryn_session::SessionEvent> {
        Vec::new()
    }
    async fn combat(
        &mut self,
        _: crate::combat_input::CombatCommand,
    ) -> Result<crate::combat_input::CombatReport, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn logout(&mut self) -> Result<Option<oteryn_session::LogoutResult>, SessionError> {
        Ok(None)
    }
}
#[test]
fn managed_container_session_sends_in_order_and_keeps_remaining_items_after_refusal()
-> Result<(), String> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            use oteryn_session::container_tree::{ContainerView, ContainerViews};
            let handle = |value| std::num::NonZeroU64::new(value).ok_or("handle");
            let entries = (11..=13)
                .map(|id| {
                    Ok(oteryn_session::ItemEntry {
                        handle: handle(id)?,
                        item_definition_ref: std::num::NonZeroU32::MIN,
                        count: std::num::NonZeroU32::MIN,
                        sub_type: 0,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            let (sent, received) = std::sync::mpsc::channel();
            let session = BulkSession {
                views: ContainerViews {
                    views: vec![
                        ContainerView {
                            view_id: 0,
                            container_handle: handle(10)?,
                            parent_handle: None,
                            capacity: 20,
                            entries,
                            browse: None,
                        },
                        ContainerView {
                            view_id: 1,
                            container_handle: handle(20)?,
                            parent_handle: None,
                            capacity: 20,
                            entries: vec![],
                            browse: None,
                        },
                    ],
                },
                moved: sent,
                responses: [ItemMoveOutcome::Moved, ItemMoveOutcome::NoRoom].into(),
            };
            let (link, commands, outbox) = crate::play::play_channel();
            let runtime = oteryn_client_runtime::ClientRuntime::new().map_err(|e| e.to_string())?;
            runtime
                .spawn(crate::play::run_session(session, commands, outbox))
                .map_err(|e| e.to_string())?;
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
            while link.container_panel().views.is_none() {
                if std::time::Instant::now() >= deadline {
                    return Err("container initialization deadline".into());
                }
                std::thread::yield_now();
            }
            assert!(link.container_activate(0));
            assert!(link.container_move_all(1));
            assert_eq!(
                received
                    .recv_timeout(std::time::Duration::from_secs(3))
                    .map_err(|e| e.to_string())?
                    .source,
                handle(11)?
            );
            assert_eq!(
                received
                    .recv_timeout(std::time::Duration::from_secs(3))
                    .map_err(|e| e.to_string())?
                    .source,
                handle(12)?
            );
            while link.container_panel().bulk.pending {
                if std::time::Instant::now() >= deadline {
                    return Err("bulk response deadline".into());
                }
                std::thread::yield_now();
            }
            assert_eq!(link.container_panel().bulk.completed, 1);
            assert!(received.try_recv().is_err());
            assert!(!link.item_movement().pending);
            drop(link);
            runtime.shutdown(std::time::Duration::from_secs(1));
            Ok(())
        })
        .map_err(|e| e.to_string())?
        .join()
        .map_err(|_| "bulk scenario thread failed".to_owned())?
}
