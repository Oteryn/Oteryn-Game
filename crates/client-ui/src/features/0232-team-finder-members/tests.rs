use super::*;
use crate::team_finder_leader::{LeaderListing, TeamTarget};
fn listing() -> LeaderListing {
    LeaderListing {
        min_level: 10,
        max_level: 100,
        vocation_mask: 3,
        slots: 4,
        free_slots: 3,
        party_members_join: false,
        timestamp: 77,
        target: TeamTarget::Hunt { kind: 1, area: 2 },
    }
}
fn browse(next: Option<u64>) -> Panel {
    Panel {
        selected: true,
        result: Some(TeamFinderResult::Browse {
            listings: vec![
                BrowseListing {
                    leader_handle: 5,
                    name: "Full team".into(),
                    listing: listing(),
                    occupied_slots: 4,
                    status: MemberStatus::None,
                },
                BrowseListing {
                    leader_handle: 9,
                    name: "River scouts".into(),
                    listing: listing(),
                    occupied_slots: 2,
                    status: MemberStatus::Pending,
                },
            ],
            next,
        }),
        ..Panel::default()
    }
}

#[test]
fn t029_team_finder_list_layout_shows_every_row_with_level_range_free_slots_and_status() {
    let panel = browse(Some(9));
    let text = panel_text(&panel);
    assert!(text.starts_with("Social — Join team"));
    assert!(text.contains("2 of 2 teams on this page; more teams available"));
    assert!(text.contains("Full team: level 10–100, 0 free, None"));
    assert!(text.contains("River scouts: level 10–100, 2 free, Pending"));
    let names = panel.visible_names();
    assert_eq!(names.len(), 2);
    assert!(names[1].starts_with("River scouts — 10–100; 2 free;"));
    assert!(!panel_text(&browse(None)).contains("more teams available"));
    let empty = Panel {
        filter: TeamFilter {
            only_open: true,
            name: "zzz".into(),
            ..TeamFilter::default()
        },
        ..browse(None)
    };
    assert!(panel_text(&empty).contains("No teams match."));
    assert!(
        panel_text(&Panel::default()).contains("unavailable"),
        "an unselected panel never lists teams"
    );
}

#[test]
fn t029_social_composition_assemble_tab_lists_advertisement_occupancy_and_applicants() {
    let panel = Panel {
        selected: true,
        applicants: true,
        result: Some(TeamFinderResult::Applicants {
            listing: Some(listing()),
            occupied_slots: 1,
            members: vec![Applicant {
                handle: 3,
                name: "Mira".into(),
                level: 55,
                vocation: 2,
                status: MemberStatus::Invited,
            }],
            next: None,
        }),
        ..Panel::default()
    };
    let text = panel_text(&panel);
    assert!(text.starts_with("Social — Assemble team"));
    assert!(text.contains("Advertisement: level 10–100, vocations 3, 4 slots"));
    assert!(text.contains("Occupied: 1"));
    assert!(text.contains("Mira: level 55, vocation 2, Invited"));
    assert_eq!(panel.visible_names().len(), 1);
    assert_eq!(panel.handle(), Some(3));
    let none = Panel {
        result: Some(TeamFinderResult::Applicants {
            listing: None,
            occupied_slots: 0,
            members: Vec::new(),
            next: None,
        }),
        ..panel
    };
    assert!(panel_text(&none).contains("No active advertisement."));
}
