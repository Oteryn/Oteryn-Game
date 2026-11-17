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
fn cp02_0232_team_finder_members_scenario() {
    let (link, mut rx, outbox) = crate::play::play_channel();
    assert!(!link.team_finder_browse(false, false));
    outbox.discovery_initialize(&Selected);
    assert!(link.team_finder_browse(false, false));
    assert!(
        matches!(rx.try_recv(), Ok(crate::combat_input::PlayCommand::ServerMechanic(command)) if command.capability == 232 && command.command_type == 444)
    );
}

#[test]
fn cp02_team_finder_native_session_scenario() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::server_mechanic::ServerMechanicCommand;
    use oteryn_client_ui::team_finder::*;
    use oteryn_client_ui::team_finder_leader::{LeaderListing, TeamTarget, encode_upsert};
    peer::scenario(|link, push, wait| {
        let listing = LeaderListing {
            min_level: 10,
            max_level: 100,
            vocation_mask: 15,
            slots: 4,
            free_slots: 3,
            party_members_join: false,
            timestamp: 0,
            target: TeamTarget::Boss(1),
        };
        push(peer::Exchange {
            command: ServerMechanicCommand {
                capability: 200,
                command_type: 390,
                payload: encode_upsert(listing).map_err(|error| format!("{error:?}"))?,
            },
            payload: vec![],
            status: oteryn_session::CommandStatus::Accepted,
        });
        assert!(link.discovery_action(
            232,
            9,
            &["10", "100", "15", "4", "3", "0", "1", "1", "0"].map(str::to_owned),
            0
        ));
        wait(&|| link.discovery_panel().leader.confirmed.is_some());
        let page = TeamFinderResult::Browse {
            listings: vec![BrowseListing {
                leader_handle: 5,
                name: "Leader".into(),
                listing,
                occupied_slots: 2,
                status: MemberStatus::None,
            }],
            next: Some(5),
        };
        push(peer::Exchange {
            command: ServerMechanicCommand {
                capability: 232,
                command_type: 444,
                payload: encode_query(TeamFinderQuery::Browse { after: None })
                    .map_err(|error| format!("{error:?}"))?,
            },
            payload: encode_result(&page).map_err(|error| format!("{error:?}"))?,
            status: oteryn_session::CommandStatus::Accepted,
        });
        assert!(link.discovery_action(232, 1, &[], 0));
        wait(&|| link.discovery_panel().teams.result.is_some());
        assert!(panel_text(&link.discovery_panel().teams).contains("2 free"));
        push(peer::Exchange {
            command: ServerMechanicCommand {
                capability: 232,
                command_type: 445,
                payload: encode_member_action(MemberAction::Request { leader: 5 })
                    .map_err(|error| format!("{error:?}"))?,
            },
            payload: vec![],
            status: oteryn_session::CommandStatus::Accepted,
        });
        assert!(link.discovery_action(232, 4, &[], 0));
        wait(&|| link.discovery_panel().teams.pending.is_none());
        assert!(link.discovery_panel().teams.result.is_none());
        assert!(!link.discovery_action(232, 4, &[], 0));
        let applicants = TeamFinderResult::Applicants {
            listing: Some(listing),
            occupied_slots: 1,
            members: vec![Applicant {
                handle: 9,
                name: "Applicant".into(),
                level: 20,
                vocation: 1,
                status: MemberStatus::Pending,
            }],
            next: None,
        };
        push(peer::Exchange {
            command: ServerMechanicCommand {
                capability: 232,
                command_type: 444,
                payload: encode_query(TeamFinderQuery::Applicants { after: None })
                    .map_err(|error| format!("{error:?}"))?,
            },
            payload: encode_result(&applicants).map_err(|error| format!("{error:?}"))?,
            status: oteryn_session::CommandStatus::Accepted,
        });
        assert!(link.discovery_action(232, 2, &[], 0));
        wait(&|| link.discovery_panel().teams.result.is_some());
        push(peer::Exchange {
            command: ServerMechanicCommand {
                capability: 232,
                command_type: 446,
                payload: encode_leader_action(LeaderAction::Accept { member: 9 })
                    .map_err(|error| format!("{error:?}"))?,
            },
            payload: vec![],
            status: oteryn_session::CommandStatus::Rejected,
        });
        assert!(link.discovery_action(232, 7, &[], 0));
        wait(&|| link.discovery_panel().teams.pending.is_none());
        assert!(link.discovery_panel().teams.feedback.contains("refused"));
        Ok(())
    })
}

include!("social.rs");
