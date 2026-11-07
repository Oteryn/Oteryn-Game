#[test]
fn srv7_expired_rental_catalogue_keeps_honest_prior_and_clears_native_draft() -> Result<(), Error> {
    let mut snapshot = wire::Snapshot::decode(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/protocol-oteryn/tests/fixtures/appearance_snapshot_v1.bin"
    )))
    .checked()?;
    let rented = "oteryn:content.mount.rented_horse";
    snapshot.catalogue.palette = vec![[0, 0, 0], [255, 255, 255]];
    let selected = wire::Selected {
        selection: wire::Selection {
            outfit_key: snapshot
                .catalogue
                .outfits
                .first()
                .ok_or("fixture outfit")?
                .key
                .clone(),
            colours: [0; 4],
            addons: 0,
            mount_key: Some(rented.into()),
        },
        revision: snapshot.catalogue.root_revision,
        provenance: "rental-ui-regression".into(),
    };
    snapshot.catalogue.selected = Some(selected.clone());
    snapshot.catalogue.mounts = vec![rented.into()];
    let peer = Peer {
        exchanges: Arc::new(Script {
            entries: Mutex::new(VecDeque::new()),
            release: tokio::sync::Semaphore::new(0),
        }),
        snapshot: snapshot.encode().checked()?,
    };
    let (link, mut commands, outbox) = play_channel();
    outbox.properties_initialize(&peer);
    outbox.appearance_initialize(&peer);
    assert_eq!(link.appearance_panel().fields[2], rented);
    let mut expired = snapshot;
    expired.catalogue.mounts.clear();
    // An unavailable stored selection is still valid wire data. Never rewrite
    // that historical before-state merely to make the new draft acceptable.
    let encoded = expired.catalogue.encode().checked()?;
    let decoded = wire::Catalogue::decode(&encoded).checked()?;
    assert_eq!(decoded.selected, Some(selected.clone()));
    outbox.appearance_events(&[SessionEvent::Appearance(oteryn_session::AppliedDelta {
        server_sequence: 2,
        base_revision: 1,
        new_revision: 2,
        value: oteryn_session::AppearanceUpdate::Catalogue(decoded),
    })]);
    let panel = link.appearance_panel();
    assert_eq!(panel.confirmed, Some(selected.clone()));
    assert_eq!(
        panel.catalogue.as_ref().and_then(|c| c.selected.clone()),
        Some(selected.clone())
    );
    assert_eq!(
        panel.fields[2], "",
        "expired mount must be removed only from the new editable draft"
    );
    assert!(panel.summary.contains(rented));
    assert!(panel.summary.contains("unavailable"));
    link.appearance_edit(3, "1,0,0,0".into());
    // The existing chooser requires this explicit revision field; the actual
    // rental capture supplies its real value for the final integration play.
    link.appearance_edit(4, "r1".into());
    link.appearance_request(Intent::Apply);
    let crate::combat_input::PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("native appearance change command".into());
    };
    assert_eq!((command.capability, command.kind), (25, 25));
    let change = wire::Change::decode(&command.payload).checked()?;
    assert_eq!(change.prior, Some(selected.clone()));
    assert_eq!(change.selection.mount_key, None);
    assert_eq!(change.selection.colours, [1, 0, 0, 0]);
    assert_eq!(
        change.expected_root_revision,
        expired.catalogue.root_revision
    );
    let fresh_peer = Peer {
        snapshot: expired.encode().checked()?,
        exchanges: peer.exchanges,
    };
    let (fresh, _, outbox) = play_channel();
    outbox.properties_initialize(&fresh_peer);
    outbox.appearance_initialize(&fresh_peer);
    assert_eq!(fresh.appearance_panel().confirmed, Some(selected));
    assert_eq!(
        fresh.appearance_panel().fields[2],
        "",
        "reconnect must not repopulate an expired mount draft"
    );
    Ok(())
}
