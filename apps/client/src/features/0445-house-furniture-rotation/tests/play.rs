use oteryn_client_runtime::raw_feature::{
    CommandStatus, GatedCommandResult, house_rotation as wire,
};
use oteryn_client_ui::house_furniture_rotation::{Intent, Panel};
#[test]
fn native_rotation_decodes_named_catalogue_and_discards_stale_revision()
-> Result<(), Box<dyn std::error::Error>> {
    let mut panel = Panel {
        selected: true,
        ..Default::default()
    };
    let query = crate::play::feature_0445_house_furniture_rotation::protocol::command(
        &panel,
        Intent::Refresh,
    )?;
    assert_eq!((query.capability, query.kind), (445, 730));
    assert!(oteryn_client_ui::house_furniture_rotation::request(
        &mut panel,
        query.payload.clone()
    ));
    let catalogue = wire::Catalogue {
        page: 0,
        more: false,
        entries: vec![wire::Entry {
            label: "Wooden cupboard".into(),
            target: wire::Target::Item {
                item: [1; 16],
                expected_revision: 8,
            },
        }],
    };
    let result = GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: wire::Reply::Catalogue(catalogue.clone())
            .encode()
            .ok_or("encode")?,
    };
    crate::play::feature_0445_house_furniture_rotation::protocol::apply(
        &mut panel, &query, &result,
    );
    assert!(
        oteryn_client_ui::house_furniture_rotation::panel_text(&panel)
            .contains("Wooden cupboard — Rotate")
    );
    panel.choice = "1".into();
    let rotate = crate::play::feature_0445_house_furniture_rotation::protocol::command(
        &panel,
        Intent::Rotate,
    )?;
    assert_eq!(wire::Request::decode(&rotate.payload), catalogue.rotate(0));
    assert!(oteryn_client_ui::house_furniture_rotation::request(
        &mut panel,
        rotate.payload.clone()
    ));
    let result = GatedCommandResult {
        payload: wire::Reply::Applied(wire::UseDisposition::Committed)
            .encode()
            .ok_or("encode")?,
        ..result
    };
    crate::play::feature_0445_house_furniture_rotation::protocol::apply(
        &mut panel, &rotate, &result,
    );
    assert!(panel.catalogue.is_none());
    assert!(
        crate::play::feature_0445_house_furniture_rotation::protocol::command(
            &panel,
            Intent::Rotate
        )
        .is_err()
    );
    let query = crate::play::feature_0445_house_furniture_rotation::protocol::command(
        &panel,
        Intent::Refresh,
    )?;
    assert!(oteryn_client_ui::house_furniture_rotation::request(
        &mut panel,
        query.payload.clone()
    ));
    let malformed = GatedCommandResult {
        payload: vec![0, 0],
        ..result
    };
    crate::play::feature_0445_house_furniture_rotation::protocol::apply(
        &mut panel, &query, &malformed,
    );
    assert!(!panel.pending);
    assert!(panel.catalogue.is_none());
    assert!(panel.feedback.contains("Invalid furniture response"));
    Ok(())
}
