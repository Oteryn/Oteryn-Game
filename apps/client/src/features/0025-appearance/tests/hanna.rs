#[test]
fn srv7_actual_hanna_grant_and_rebootstrap_reach_native_wardrobe() -> Result<(), Error> {
    let capture: serde_json::Value = serde_json::from_str(include_str!("hanna-wire.json"))?;
    let decode = |row: &serde_json::Value| -> Result<Vec<u8>, Error> {
        let text = row["payload_hex"].as_str().ok_or("actual wardrobe bytes")?;
        if !text.len().is_multiple_of(2) {
            return Err("odd captured bytes".into());
        }
        text.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|p| Ok(u8::from_str_radix(std::str::from_utf8(p)?, 16)?))
            .collect()
    };
    let outfit = capture["award"]["outfit_key"]
        .as_str()
        .ok_or("actual award")?;
    for phase in ["grant_appearance", "reconnect_appearance"] {
        let records = capture[phase].as_array().ok_or("actual phase")?;
        let initial = records
            .iter()
            .find(|r| r["kind"] == "snapshot")
            .ok_or("actual snapshot")?;
        let peer = Peer {
            exchanges: Arc::new(Script {
                entries: Mutex::new(VecDeque::new()),
                release: tokio::sync::Semaphore::new(0),
            }),
            snapshot: decode(initial)?,
        };
        let (link, mut commands, outbox) = play_channel();
        outbox.properties_initialize(&peer);
        outbox.appearance_initialize(&peer);
        let mask = |panel: &oteryn_client_ui::appearance::Panel| {
            panel
                .catalogue
                .as_ref()
                .and_then(|c| c.outfits.iter().find(|o| o.key == outfit))
                .map(|o| o.permitted_addons)
                .unwrap_or(0)
        };
        if phase == "grant_appearance" {
            assert_eq!(mask(&link.appearance_panel()) & 2, 0);
        }
        for row in records
            .iter()
            .filter(|r| r["kind"] == "delta" && r["delta_type"] == 2)
        {
            let catalogue = wire::Catalogue::decode(&decode(row)?).checked()?;
            outbox.appearance_events(&[SessionEvent::Appearance(oteryn_session::AppliedDelta {
                server_sequence: row["server_sequence"].as_u64().ok_or("sequence")?,
                base_revision: row["base_revision"].as_u64().ok_or("base")?,
                new_revision: row["new_revision"].as_u64().ok_or("revision")?,
                value: oteryn_session::AppearanceUpdate::Catalogue(catalogue),
            })]);
        }
        let panel = link.appearance_panel();
        assert_eq!(mask(&panel) & 2, 2);
        assert!(panel.summary.contains(outfit));
        let before = panel.catalogue.as_ref().ok_or("native catalogue")?.clone();
        link.appearance_edit(0, outfit.into());
        link.appearance_edit(1, "2".into());
        link.appearance_edit(
            4,
            capture["content_revision"]
                .as_str()
                .ok_or("actual cosmetics revision")?
                .into(),
        );
        link.appearance_request(Intent::Apply);
        let crate::combat_input::PlayCommand::Feature(command) = commands.try_recv()? else {
            return Err("native wardrobe command".into());
        };
        assert_eq!((command.capability, command.kind), (25, 25));
        let change = wire::Change::decode(&command.payload).checked()?;
        assert_eq!(change.selection.addons, 2);
        assert_eq!(change.selection.outfit_key, outfit);
        assert_eq!(change.prior, before.selected);
        assert_eq!(change.expected_root_revision, before.root_revision);
        assert_eq!(change.context, before.context);
    }
    Ok(())
}
