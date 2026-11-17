use oteryn_client_runtime::{
    house::{Action, Page, Row},
    raw_feature::{CommandStatus, GatedCommandResult},
};
use oteryn_client_ui::house::{Panel, request};
fn reply(payload: Vec<u8>) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload,
    }
}
#[test]
fn gap_house_revision_refusal_paging_and_cleanup() -> Result<(), &'static str> {
    let mut w = [1; 16];
    w[6] = 0x70;
    w[8] = 0x80;
    let mut p = Panel {
        selected: true,
        world: w,
        town: "oteryn:content.area.city.thais".into(),
        page: Page {
            rows: vec![Row {
                house: "a".into(),
                revision: 7,
                acl_revision: 9,
                ..Row::default()
            }],
            cursor: Some("a".into()),
        },
        ..Panel::default()
    };
    let q = request(&mut p, None, true)?;
    assert!(q.payload.ends_with(&[26, 1, b'a']));
    assert!(request(&mut p, None, false).is_err());
    let old = p.page.clone();
    p.apply(&q, &reply(vec![8, 1, 26, 0]));
    assert_eq!(p.page, old);
    let a = request(&mut p, Some(&Action::Guests(vec![])), false)?;
    assert!(a.payload.ends_with(&[34, 4, 8, 1, 16, 9]));
    p.apply(&a, &reply(vec![8, 3, 16, 0]));
    assert_eq!(p.page, old);
    assert!(p.feedback.contains("refused"));
    let a = request(&mut p, Some(&Action::Guests(vec![])), false)?;
    p.page.rows.push(Row {
        house: "b".into(),
        acl_revision: 99,
        ..Row::default()
    });
    p.index = 1;
    p.apply(&a, &reply(vec![8, 1, 16, 10]));
    assert_eq!(p.page.rows[1].acl_revision, 99);
    p.index = 0;
    assert_eq!(p.page.rows[0].acl_revision, 10);
    assert_eq!(p.page.rows[0].revision, 7);
    let a = request(&mut p, Some(&Action::Bid(20)), false)?;
    p.apply(&a, &reply(vec![8, 1, 16, 50]));
    assert_eq!(p.page.rows[0].acl_revision, 10);
    let q = request(&mut p, None, false)?;
    let mut refused = reply(vec![8, 1]);
    refused.status = CommandStatus::Rejected;
    p.apply(&q, &refused);
    assert!(p.feedback.contains("refused"));
    let q = request(&mut p, None, false)?;
    p.ended();
    p.apply(&q, &reply(vec![8, 1]));
    assert_eq!(p, Panel::default());
    assert!(request(&mut p, None, false).is_err());
    Ok(())
}

#[test]
fn gap_house_adjacent_capabilities_do_not_admit_house() {
    let mut panel = Panel::default();
    for capabilities in [&[99][..], &[59][..], &[86][..], &[99, 59, 86][..]] {
        panel.admitted([0; 16], capabilities);
        assert!(!panel.selected);
    }
    panel.admitted([0; 16], &[49]);
    assert!(panel.selected);
}

#[test]
fn house_bid_reports_insufficient_funds() -> Result<(), &'static str> {
    let mut world = [1; 16];
    world[6] = 0x70;
    world[8] = 0x80;
    let mut panel = Panel {
        selected: true,
        world,
        ..Panel::default()
    };
    panel.page.rows.push(Row {
        house: "a".into(),
        ..Row::default()
    });
    let command = request(&mut panel, Some(&Action::Bid(20)), false)?;
    panel.apply(&command, &reply(vec![8, 8, 16, 0]));
    assert_eq!(
        panel.feedback,
        "The server refused the action: insufficient funds"
    );
    assert!(panel.pending.is_none());
    Ok(())
}

#[test]
fn house_catalogue_shows_auction_identity_and_navigation() {
    let mut panel = Panel::default();
    panel.page.rows.push(Row {
        house: "a".into(),
        kind: 2,
        owned: true,
        ends: Some(123),
        maximum: Some(456),
        ..Row::default()
    });
    panel.page.cursor = Some("a".into());
    let text = oteryn_client_ui::house::panel_text(&panel);
    assert!(text.contains("1. a [shop; owned by you]"));
    assert!(text.contains("Auction ends (Unix ms): 123; your maximum bid: 456"));
    assert!(text.contains("Next page available"));
}
