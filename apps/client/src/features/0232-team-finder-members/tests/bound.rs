use super::*;
use crate::play::feature_0232_team_finder_members::input::{
    TeamFinderAction as Action, TeamFinderSnapshot,
};
use oteryn_client_ui::team_finder::*;

fn snapshot(link: &crate::play::PlayLink) -> TeamFinderSnapshot {
    link.team_finder_snapshot()
}
#[test]
fn team_finder_actions_are_bound_and_confirmed() {
    let (link, mut rx, outbox) = crate::play::play_channel();
    outbox.discovery_initialize(&Selected);
    link.discovery_open(232);
    let before = snapshot(&link);
    assert!(link.team_finder_dispatch_bound(
        &before,
        Action::Browse {
            applicants: false,
            next: false
        },
        false
    ));
    assert!(
        matches!(rx.try_recv(), Ok(crate::combat_input::PlayCommand::ServerMechanic(command)) if command.command_type == 444)
    );
    assert!(!link.team_finder_dispatch_bound(
        &before,
        Action::Browse {
            applicants: false,
            next: false
        },
        false
    ));
    assert!(rx.try_recv().is_err());
    let listing = oteryn_client_ui::team_finder_leader::LeaderListing {
        min_level: 10,
        max_level: 100,
        vocation_mask: 3,
        slots: 4,
        free_slots: 3,
        party_members_join: false,
        timestamp: 0,
        target: oteryn_client_ui::team_finder_leader::TeamTarget::Boss(1),
    };
    outbox.discovery_result(
        232,
        &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_session::CommandStatus::Accepted,
            payload: encode_result(&TeamFinderResult::Browse {
                listings: vec![BrowseListing {
                    leader_handle: 9,
                    name: "Server leader".into(),
                    listing,
                    occupied_slots: 2,
                    status: MemberStatus::Pending,
                }],
                next: None,
            })
            .unwrap_or_default(),
        },
    );
    let before = snapshot(&link);
    assert!(!link.team_finder_dispatch_bound(&before, Action::Member { cancel: true }, false));
    assert_eq!(snapshot(&link), before);
    assert!(link.team_finder_dispatch_bound(&before, Action::Member { cancel: true }, true));
    assert_eq!(snapshot(&link).teams.result, before.teams.result);
    assert!(
        matches!(rx.try_recv(), Ok(crate::combat_input::PlayCommand::ServerMechanic(command)) if command.command_type == 445)
    );
    assert!(rx.try_recv().is_err());
    // All remaining network actions use the existing owner encoders, one request each.
    for (applicants, action, command_type) in [
        (false, Action::Member { cancel: false }, 445),
        (true, Action::Decide(2), 446),
        (true, Action::Decide(3), 446),
        (true, Action::Decide(4), 446),
        (
            false,
            Action::Browse {
                applicants: false,
                next: true,
            },
            444,
        ),
        (
            true,
            Action::Browse {
                applicants: true,
                next: true,
            },
            444,
        ),
        (true, Action::Advertise(Some(listing)), 390),
        (true, Action::Advertise(None), 391),
    ] {
        let result = if applicants {
            TeamFinderResult::Applicants {
                listing: Some(listing),
                occupied_slots: 2,
                members: vec![Applicant {
                    handle: 17,
                    name: "Applicant".into(),
                    level: 33,
                    vocation: 2,
                    status: MemberStatus::Pending,
                }],
                next: Some(17),
            }
        } else {
            TeamFinderResult::Browse {
                listings: vec![BrowseListing {
                    leader_handle: 9,
                    name: "Server leader".into(),
                    listing,
                    occupied_slots: 2,
                    status: MemberStatus::Pending,
                }],
                next: Some(9),
            }
        };
        outbox.bevy_team_finder_state(
            Panel {
                selected: true,
                open: true,
                applicants,
                result: Some(result),
                ..Default::default()
            },
            oteryn_client_ui::team_finder_leader::Panel {
                selected: true,
                confirmed: Some(listing),
                ..Default::default()
            },
        );
        let before = snapshot(&link);
        if action.needs_confirmation() {
            assert!(!link.team_finder_dispatch_bound(&before, action.clone(), false));
            assert_eq!(snapshot(&link), before);
        }
        assert!(link.team_finder_dispatch_bound(&before, action.clone(), true));
        assert!(
            matches!(rx.try_recv(),Ok(crate::combat_input::PlayCommand::ServerMechanic(command)) if command.command_type == command_type)
        );
        assert!(!link.team_finder_dispatch_bound(&before, action, true));
        assert!(rx.try_recv().is_err());
        assert_eq!(snapshot(&link).teams.result, before.teams.result);
        assert_eq!(snapshot(&link).leader.confirmed, before.leader.confirmed);
        outbox.discovery_result(
            if command_type < 400 { 200 } else { 232 },
            &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
                command_id: 1,
                server_sequence: 1,
                status: oteryn_session::CommandStatus::Rejected,
                payload: vec![],
            },
        );
        assert_eq!(snapshot(&link).leader.confirmed, before.leader.confirmed);
    }
    outbox.bevy_team_finder_state(
        Panel {
            selected: true,
            open: true,
            ..Default::default()
        },
        oteryn_client_ui::team_finder_leader::Panel {
            selected: true,
            ..Default::default()
        },
    );
    assert!(link.team_finder_dispatch_bound(
        &snapshot(&link),
        Action::Advertise(Some(listing)),
        false
    ));
    assert!(snapshot(&link).leader.confirmed.is_none());
    assert!(rx.try_recv().is_ok());
    outbox.discovery_result(
        200,
        &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 2,
            server_sequence: 2,
            status: oteryn_session::CommandStatus::Accepted,
            payload: vec![],
        },
    );
    assert_eq!(snapshot(&link).leader.confirmed, Some(listing));
    assert!(link.team_finder_dispatch_bound(&snapshot(&link), Action::Advertise(None), true));
    assert_eq!(snapshot(&link).leader.confirmed, Some(listing));
    assert!(rx.try_recv().is_ok());
    outbox.discovery_result(
        200,
        &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 3,
            server_sequence: 3,
            status: oteryn_session::CommandStatus::Accepted,
            payload: vec![],
        },
    );
    assert!(snapshot(&link).leader.confirmed.is_none());
    assert!(rx.try_recv().is_err());
}
#[test]
fn team_finder_rejected_when_session_replaced_or_target_changed() {
    let (link, mut rx, outbox) = crate::play::play_channel();
    outbox.discovery_initialize(&Selected);
    link.discovery_open(232);
    let before = snapshot(&link);
    let modal = link.bevy_server_modal_fixture();
    assert!(!link.team_finder_dispatch_bound(
        &before,
        Action::Browse {
            applicants: false,
            next: false
        },
        false
    ));
    assert_eq!(snapshot(&link), before);
    assert!(rx.try_recv().is_err());
    drop(modal);
    let (other, _, other_outbox) = crate::play::play_channel();
    other_outbox.discovery_initialize(&Selected);
    other.discovery_open(232);
    let query = Action::Browse {
        applicants: false,
        next: false,
    };
    assert!(!other.team_finder_dispatch_bound(&before, query.clone(), false));
    assert!(link.team_finder_dispatch_bound(
        &before,
        Action::Filter(TeamFilter {
            name: "changed".into(),
            ..Default::default()
        }),
        false
    ));
    let changed = snapshot(&link);
    assert!(!link.team_finder_dispatch_bound(&before, query.clone(), false));
    assert_eq!(snapshot(&link), changed);
    assert!(link.team_finder_dispatch_bound(&changed, Action::Close, false));
    let window_closed = snapshot(&link);
    assert!(!link.team_finder_dispatch_bound(&changed, query.clone(), false));
    assert_eq!(snapshot(&link), window_closed);
    link.discovery_open(232);
    let listing = oteryn_client_ui::team_finder_leader::LeaderListing {
        min_level: 1,
        max_level: 100,
        vocation_mask: 3,
        slots: 4,
        free_slots: 3,
        party_members_join: false,
        timestamp: 0,
        target: oteryn_client_ui::team_finder_leader::TeamTarget::Boss(1),
    };
    outbox.bevy_team_finder_state(
        Panel {
            selected: true,
            open: true,
            result: Some(TeamFinderResult::Browse {
                listings: [(9, "First"), (17, "Second")]
                    .into_iter()
                    .map(|(leader_handle, name)| BrowseListing {
                        leader_handle,
                        name: name.into(),
                        listing,
                        occupied_slots: 1,
                        status: MemberStatus::None,
                    })
                    .collect(),
                next: None,
            }),
            ..Default::default()
        },
        Default::default(),
    );
    let target_before = snapshot(&link);
    link.team_finder_select(1);
    let target_after = snapshot(&link);
    assert_ne!(target_before.teams.handle(), target_after.teams.handle());
    assert!(!link.team_finder_dispatch_bound(
        &target_before,
        Action::Member { cancel: false },
        false
    ));
    assert_eq!(snapshot(&link), target_after);
    outbox.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    let closed = snapshot(&link);
    assert!(!link.team_finder_dispatch_bound(&changed, query, false));
    assert_eq!(snapshot(&link), closed);
    assert!(rx.try_recv().is_err());
}
