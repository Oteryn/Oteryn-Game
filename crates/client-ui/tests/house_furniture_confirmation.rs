use oteryn_client_runtime::raw_feature::house_wrap::{Action, Request, encode_catalogue_query};
use oteryn_client_ui::house_furniture_wrap::{Panel, request};

#[test]
fn furniture_requires_review_of_exact_item_and_destination() -> Result<(), &'static str> {
    let mut panel = Panel {
        selected: true,
        ..Panel::default()
    };
    let action = Request {
        action: Action::Unwrap,
        item: [1; 16],
        expected_revision: 2,
        acl_revision: 3,
        position: [10, 20, -7],
    };
    assert!(!request(&mut panel, action.encode()));
    assert!(!panel.pending);
    assert!(panel.feedback.contains("10, 20, floor -7"));
    let changed = Request {
        position: [11, 20, -7],
        ..action
    };
    assert!(!request(&mut panel, changed.encode()));
    assert!(request(&mut panel, changed.encode()));
    panel.pending = false;
    let query = encode_catalogue_query(0).ok_or("query")?;
    assert!(request(&mut panel, query));
    panel.pending = false;
    assert!(!request(&mut panel, changed.encode()));
    Ok(())
}
