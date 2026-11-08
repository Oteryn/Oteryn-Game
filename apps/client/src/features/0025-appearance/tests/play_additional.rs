use super::*;

#[test]
fn nx5_item_unlock_delta_decodes_into_native_catalogue_and_feedback() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), Error> {
            let runtime = oteryn_client_runtime::ClientRuntime::new().checked()?;
            runtime
                .block_on(async {
                    let mut snapshot = wire::Snapshot::decode(include_bytes!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../../crates/protocol-oteryn/tests/fixtures/appearance_snapshot_v1.bin"
                    )))
                    .checked()?;
                    snapshot.catalogue.outfits.truncate(1);
                    snapshot.catalogue.outfits[0].key =
                        "oteryn:content.outfit.citizen_outfits".into();
                    snapshot.catalogue.outfits[0].permitted_addons = 0;
                    snapshot.catalogue.mounts.clear();
                    let exchanges = Arc::new(Script {
                        entries: Mutex::new(VecDeque::new()),
                        release: tokio::sync::Semaphore::new(0),
                    });
                    let (link, commands, outbox) = play_channel();
                    let task = tokio::spawn(run_session(
                        Peer {
                            exchanges,
                            snapshot: snapshot.encode().checked()?,
                        },
                        commands,
                        outbox.clone(),
                    ));
                    ready(&link).await.checked()?;
                    tokio::time::timeout(Duration::from_secs(3), async {
                        while link.appearance_panel().catalogue.is_none() {
                            tokio::task::yield_now().await;
                        }
                    })
                    .await
                    .checked()?;
                    let draft = link.appearance_panel().fields;
                    let mut next = snapshot.catalogue;
                    next.outfits.push(wire::OutfitOption {
                        key: "oteryn:content.outfit.soil_guardian_outfits".into(),
                        look_type: 516,
                        permitted_addons: 3,
                    });
                    next.mounts
                        .push("oteryn:content.mount.singeing_steed".into());
                    // The exact server catalogue codec feeds the same event the Session emits.
                    let decoded = wire::Catalogue::decode(&next.encode().checked()?).checked()?;
                    outbox.push(vec![SessionEvent::Appearance(
                        oteryn_session::AppliedDelta {
                            server_sequence: 42,
                            base_revision: 0,
                            new_revision: 1,
                            value: oteryn_session::AppearanceUpdate::Catalogue(decoded),
                        },
                    )]);
                    let panel = link.appearance_panel();
                    let text = oteryn_client_ui::appearance::panel_text(&panel);
                    assert!(text.contains("soil_guardian_outfits; allowed addon mask: 3"));
                    assert!(text.contains("singeing_steed"));
                    assert!(
                        text.contains(
                            "Unlocked outfit: oteryn:content.outfit.soil_guardian_outfits"
                        )
                    );
                    assert!(text.contains("Unlocked mount: oteryn:content.mount.singeing_steed"));
                    assert_eq!(
                        panel.fields, draft,
                        "an unlock preserves the current selection draft"
                    );
                    // Repeated catalogue delivery is silent rather than a second success notice.
                    outbox.push(vec![SessionEvent::Appearance(
                        oteryn_session::AppliedDelta {
                            server_sequence: 43,
                            base_revision: 1,
                            new_revision: 2,
                            value: oteryn_session::AppearanceUpdate::Catalogue(next),
                        },
                    )]);
                    assert_eq!(link.appearance_panel().fields, draft);
                    drop(link);
                    task.await.checked()?;
                    Ok::<(), Error>(())
                })
                .checked()??;
            Ok(())
        })?
        .join()
        .map_err(|_| "NX-5 appearance play thread")?
}
