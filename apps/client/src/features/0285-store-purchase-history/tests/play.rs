use super::feature_0098_quick_loot::peer;
struct Selected;
impl crate::play::Stepper for Selected {
    async fn step(
        &mut self,
        _: oteryn_session::StepDirection,
    ) -> Result<oteryn_session::StepOutcome, oteryn_session::SessionError> {
        Err(oteryn_session::SessionError::SessionUnusable)
    }
    async fn serve(&mut self, _: std::time::Duration) -> Result<(), oteryn_session::SessionError> {
        Ok(())
    }
    fn drain(&mut self) -> Vec<oteryn_session::SessionEvent> {
        Vec::new()
    }
    async fn combat(
        &mut self,
        _: crate::combat_input::CombatCommand,
    ) -> Result<crate::combat_input::CombatReport, oteryn_session::SessionError> {
        Err(oteryn_session::SessionError::SessionUnusable)
    }
    async fn logout(
        &mut self,
    ) -> Result<Option<oteryn_session::LogoutResult>, oteryn_session::SessionError> {
        Ok(None)
    }

    fn server_mechanic_selected(&self, _: u32) -> bool {
        true
    }
}
#[test]
fn cp02_0285_store_purchase_history_scenario() {
    let (link, mut rx, outbox) = crate::play::play_channel();
    assert!(!link.purchase_history(false));
    outbox.discovery_initialize(&Selected);
    assert!(link.purchase_history(false));
    assert!(
        matches!(rx.try_recv(), Ok(crate::combat_input::PlayCommand::ServerMechanic(command)) if command.capability == 285 && command.command_type == 501)
    );
}

#[test]
fn cp02_store_history_native_session_scenario() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::discovery::store::encode_history_page;
    use oteryn_client_runtime::server_mechanic::ServerMechanicCommand;
    use oteryn_client_ui::purchase_history::*;
    peer::scenario(|link, push, wait| {
        assert!(!link.purchase_history(true));
        let page = StoreHistoryPage {
            entries: vec![],
            next_page_cursor: "opaque-server-cursor".into(),
        };
        push(peer::Exchange {
            command: ServerMechanicCommand {
                capability: 285,
                command_type: 501,
                payload: encode_history_request(&StoreHistoryRequest {
                    page_cursor: String::new(),
                })
                .map_err(|error| format!("{error:?}"))?,
            },
            payload: encode_history_page(&page).map_err(|error| format!("{error:?}"))?,
            status: oteryn_session::CommandStatus::Accepted,
        });
        assert!(link.discovery_action(285, 1, &[], 0));
        wait(&|| link.discovery_panel().history.page.is_some());
        assert!(panel_text(&link.discovery_panel().history).contains("No purchases"));
        push(peer::Exchange {
            command: ServerMechanicCommand {
                capability: 285,
                command_type: 501,
                payload: encode_history_request(&StoreHistoryRequest {
                    page_cursor: "opaque-server-cursor".into(),
                })
                .map_err(|error| format!("{error:?}"))?,
            },
            payload: encode_history_page(&StoreHistoryPage {
                entries: vec![],
                next_page_cursor: String::new(),
            })
            .map_err(|error| format!("{error:?}"))?,
            status: oteryn_session::CommandStatus::Accepted,
        });
        assert!(link.discovery_action(285, 2, &[], 0));
        wait(&|| !link.discovery_panel().history.pending);
        assert!(!link.discovery_action(285, 2, &[], 0));
        Ok(())
    })
}
