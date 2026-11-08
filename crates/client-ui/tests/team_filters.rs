use oteryn_client_ui::{
    team_finder::*,
    team_finder_leader::{LeaderListing, TeamTarget},
};
#[test]
fn filtered_team_selection_keeps_the_server_handle() -> Result<(), Box<dyn std::error::Error>> {
    let listing = LeaderListing {
        min_level: 20,
        max_level: 100,
        vocation_mask: 3,
        slots: 4,
        free_slots: 3,
        party_members_join: false,
        timestamp: 0,
        target: TeamTarget::Boss(9),
    };
    let mut panel = Panel {
        selected: true,
        result: Some(TeamFinderResult::Browse {
            listings: vec![
                BrowseListing {
                    leader_handle: 10,
                    name: "Full".into(),
                    listing,
                    occupied_slots: 4,
                    status: MemberStatus::None,
                },
                BrowseListing {
                    leader_handle: 20,
                    name: "Alice".into(),
                    listing,
                    occupied_slots: 1,
                    status: MemberStatus::None,
                },
            ],
            next: Some(100),
        }),
        ..Default::default()
    };
    panel.filter.only_open = true;
    panel.filter.level = Some(50);
    panel.filter.name = "ali".into();
    assert_eq!(panel.visible_indices(), vec![1]);
    assert_eq!(panel.handle(), Some(20));
    let command = member(&mut panel, false).ok_or("no join command")?;
    assert_eq!(
        decode_member_action(&command.payload).map_err(|_| "invalid join")?,
        MemberAction::Request { leader: 20 }
    );
    panel.filter.level = Some(101);
    assert_eq!(panel.handle(), None);
    Ok(())
}
