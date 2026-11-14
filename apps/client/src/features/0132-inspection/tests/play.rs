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
fn cp02_0132_inspection_scenario() {
    let (link, mut rx, outbox) = crate::play::play_channel();
    link.use_shared_frontend();
    assert!(!link.inspect(oteryn_client_ui::inspection::Request::Catalogue(42)));
    outbox.discovery_initialize(&Selected);
    assert!(link.inspect(oteryn_client_ui::inspection::Request::Catalogue(42)));
    assert!(
        matches!(rx.try_recv(), Ok(crate::combat_input::PlayCommand::ServerMechanic(command)) if command.capability == 132 && command.command_type == 153)
    );
}

#[test]
fn cp02_inspection_native_session_scenario() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::server_mechanic::ServerMechanicCommand;
    use oteryn_client_ui::inspection::*;
    peer::scenario(|link, push, wait| {
        link.use_shared_frontend();
        let reply = Reply {
            status: 1,
            items: vec![InspectedItem {
                slot: 0,
                definition_ref: 42,
                count: 2,
                tier: 1,
                name: "Sword".into(),
                attributes: vec![("Attack".into(), "30".into())],
            }],
            requests: vec![],
            catalogue: None,
        };
        push(peer::Exchange {
            command: ServerMechanicCommand {
                capability: 132,
                command_type: 153,
                payload: encode_request(Request::Catalogue(42))
                    .map_err(|error| format!("{error:?}"))?,
            },
            payload: encode_reply(&reply).map_err(|error| format!("{error:?}"))?,
            status: oteryn_session::CommandStatus::Accepted,
        });
        assert!(link.discovery_action(132, 7, &["42".into()], 0));
        wait(&|| link.discovery_panel().inspection.reply.is_some());
        assert!(panel_text(&link.discovery_panel().inspection).contains("Sword"));
        assert!(panel_text(&link.discovery_panel().inspection).contains("Attack: 30"));
        push(peer::Exchange {
            command: ServerMechanicCommand {
                capability: 132,
                command_type: 153,
                payload: encode_request(Request::Catalogue(42))
                    .map_err(|error| format!("{error:?}"))?,
            },
            payload: vec![255],
            status: oteryn_session::CommandStatus::Accepted,
        });
        assert!(link.discovery_action(132, 8, &[], 0));
        wait(&|| !link.discovery_panel().inspection.pending);
        assert!(link.discovery_panel().inspection.reply.is_none());
        assert!(
            link.discovery_panel()
                .inspection
                .feedback
                .contains("invalid")
        );
        Ok(())
    })
}
