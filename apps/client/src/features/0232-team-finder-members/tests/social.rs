#[test]
fn social_team_filters_join_the_visible_leader() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::server_mechanic::ServerMechanicCommand;
    use oteryn_client_ui::team_finder::*;
    use oteryn_client_ui::team_finder_leader::{LeaderListing, TeamTarget};
    peer::scenario(|link, push, wait| {
        let listing = LeaderListing {
            min_level: 10,
            max_level: 100,
            vocation_mask: 3,
            slots: 4,
            free_slots: 3,
            party_members_join: false,
            timestamp: 0,
            target: TeamTarget::Hunt { kind: 1, area: 2 },
        };
        let page = TeamFinderResult::Browse {
            listings: vec![
                BrowseListing {
                    leader_handle: 5,
                    name: "Full team".into(),
                    listing,
                    occupied_slots: 4,
                    status: MemberStatus::None,
                },
                BrowseListing {
                    leader_handle: 9,
                    name: "River scouts".into(),
                    listing,
                    occupied_slots: 2,
                    status: MemberStatus::None,
                },
            ],
            next: Some(9),
        };
        push(peer::Exchange {
            command: ServerMechanicCommand {
                capability: 232,
                command_type: 444,
                payload: encode_query(TeamFinderQuery::Browse { after: None })
                    .map_err(|e| format!("{e:?}"))?,
            },
            payload: encode_result(&page).map_err(|e| format!("{e:?}"))?,
            status: oteryn_session::CommandStatus::Accepted,
        });
        assert!(link.discovery_action(232, 1, &[], 0));
        wait(&|| link.discovery_panel().teams.result.is_some());
        let mut fields = vec![String::new(); 15];
        for (index, value) in [(10, "river"), (11, "50"), (12, "2"), (13, "2"), (14, "1")] {
            fields[index] = value.into();
        }
        assert!(link.discovery_action(232, 11, &fields, 0));
        let state = link.discovery_panel();
        assert_eq!(state.teams.visible_names().len(), 1);
        assert!(panel_text(&state.teams).contains("Social — Join team"));
        assert!(panel_text(&state.teams).contains("1 of 2 teams on this page"));
        push(peer::Exchange {
            command: ServerMechanicCommand {
                capability: 232,
                command_type: 445,
                payload: encode_member_action(MemberAction::Request { leader: 9 })
                    .map_err(|e| format!("{e:?}"))?,
            },
            payload: vec![],
            status: oteryn_session::CommandStatus::Accepted,
        });
        assert!(link.discovery_action(232, 4, &[], 0));
        wait(&|| link.discovery_panel().teams.pending.is_none());
        Ok(())
    })
}
