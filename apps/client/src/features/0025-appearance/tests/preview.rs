use super::*;
#[test]
fn appearance_preview_selectors_reach_the_wire_command() -> Result<(), Error> {
    let snapshot = wire::Snapshot::decode(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/protocol-oteryn/tests/fixtures/appearance_snapshot_v1.bin"
    )))
    .checked()?;
    let (link, mut commands, _) = play_channel();
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        let panel = &mut mailbox.properties.appearance;
        panel.selected = true;
        panel.catalogue = Some(snapshot.catalogue.clone());
        panel.catalogue.as_mut().ok_or("catalogue")?.palette = vec![[1, 2, 3], [4, 5, 6]];
        panel.catalogue.as_mut().ok_or("catalogue")?.outfits[0].permitted_addons = 3;
        panel.fields = [
            snapshot.catalogue.outfits[0].key.clone(),
            "0".into(),
            String::new(),
            "0,0,0,0".into(),
            "preview-test".into(),
        ];
    }
    assert!(link.appearance_colour(0));
    assert!(link.appearance_addon(1));
    let preview = link.appearance_panel().preview().ok_or("preview")?;
    assert_eq!(preview.colours[0], [4, 5, 6]);
    assert_eq!(preview.addons, 1);
    assert_eq!(preview.look_type, snapshot.catalogue.outfits[0].look_type);
    assert!(link.appearance_panel().confirmed.is_none());
    link.appearance_request(Intent::Apply);
    let crate::combat_input::PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("wrong command".into());
    };
    let change = wire::Change::decode(&command.payload).checked()?;
    assert_eq!(change.selection.colours, [1, 0, 0, 0]);
    assert_eq!(change.selection.addons, 1);
    assert!(!link.appearance_colour(1));
    assert!(!link.appearance_addon(2));
    Ok(())
}
