use super::*;

#[test]
fn pc01_charm_progression_scenario() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), Error> {
            let runtime = oteryn_client_runtime::ClientRuntime::new().checked()?;
            runtime
                .block_on(async {
                    let bytes = Vec::new();
                    let exchanges = Arc::new(Script {
                        entries: Mutex::new(VecDeque::new()),
                        release: tokio::sync::Semaphore::new(0),
                    });
                    let (link, mut incoming, outbox) = play_channel();
                    link.charm_progression_request(Intent::Query);
                    assert!(!link.charm_progression_panel().pending);
                    assert!(incoming.try_recv().is_err());
                    let task = tokio::spawn(run_session(
                        Peer {
                            exchanges: Arc::clone(&exchanges),
                            snapshot: bytes.clone(),
                        },
                        incoming,
                        outbox.clone(),
                    ));
                    ready(&link).await.checked()?;
                    let event = oteryn_input_actions::NormalizedInputEvent::Key {
                        code: oteryn_input_actions::KeyCode::new(6).checked()?,
                        state: oteryn_input_actions::ButtonState::Pressed,
                        modifiers: oteryn_input_actions::Modifiers::one(
                            oteryn_input_actions::Modifier::Control,
                        )
                        .with(oteryn_input_actions::Modifier::Alt),
                        repeat: false,
                    };
                    assert!(link.charm_progression_input(&[event]));
                    assert!(link.charm_progression_panel().open);
                    use wire::{
                        bestiary::BestiaryRaceProgress,
                        charm::{CharmKind, CharmState, CharmView},
                    };
                    let one = std::num::NonZeroU32::new(1).ok_or("index")?;
                    let mut reply = wire::CharmProgressionReply {
                        outcome: wire::CharmProgressionOutcome::Query,
                        fee_gold: 0,
                        bestiary: vec![BestiaryRaceProgress {
                            race: one,
                            kill_count: 30,
                            kill_thresholds: [10, 20, 30],
                        }],
                        charms: CharmView {
                            charms: vec![CharmState {
                                charm: one,
                                kind: CharmKind::Major,
                                unlocked_stage: 0,
                                assigned_race: None,
                                next_stage_cost: 100,
                                effect_active: false,
                            }],
                            charm_points_available: 200,
                            minor_charm_echoes_available: 0,
                            assignment_slot_limit: Some(one),
                        },
                    };
                    send(
                        &link,
                        &exchanges,
                        Intent::Query,
                        84,
                        wire::encode_charm_progression_request(
                            &wire::CharmProgressionAction::Query,
                        )
                        .checked()?,
                        CommandStatus::Accepted,
                        wire::encode_charm_progression_reply(&reply).checked()?,
                    )
                    .await
                    .checked()?;
                    assert!(link.charm_progression_panel().summary.contains("30 kills"));
                    link.charm_progression_edit(0, "1".into());
                    link.charm_progression_edit(1, "1".into());
                    let cases = [
                        (
                            Intent::Unlock,
                            wire::CharmProgressionAction::Unlock {
                                charm: one,
                                expected_stage: 0,
                            },
                            wire::CharmProgressionOutcome::Unlock(
                                wire::charm::CharmUnlockDisposition::Unlocked,
                            ),
                        ),
                        (
                            Intent::Assign,
                            wire::CharmProgressionAction::Assign {
                                charm: one,
                                race: one,
                            },
                            wire::CharmProgressionOutcome::Assign(
                                wire::charm::CharmAssignDisposition::Assigned,
                            ),
                        ),
                        (
                            Intent::Remove,
                            wire::CharmProgressionAction::Remove { charm: one },
                            wire::CharmProgressionOutcome::Remove(
                                wire::CharmRemoveDisposition::Removed,
                            ),
                        ),
                        (
                            Intent::ResetAll,
                            wire::CharmProgressionAction::ResetAll,
                            wire::CharmProgressionOutcome::ResetAll(
                                wire::CharmRemoveDisposition::Removed,
                            ),
                        ),
                    ];
                    for (intent, action, outcome) in cases {
                        reply.outcome = outcome;
                        reply.fee_gold = if matches!(intent, Intent::Remove | Intent::ResetAll) {
                            100
                        } else {
                            0
                        };
                        reply.charms.charms[0].unlocked_stage = 1;
                        send(
                            &link,
                            &exchanges,
                            intent,
                            84,
                            wire::encode_charm_progression_request(&action).checked()?,
                            CommandStatus::Accepted,
                            wire::encode_charm_progression_reply(&reply).checked()?,
                        )
                        .await
                        .checked()?;
                        assert!(
                            link.charm_progression_panel()
                                .feedback
                                .contains("Completed")
                        );
                    }
                    let intent = Intent::ResetAll;
                    let payload = wire::encode_charm_progression_request(
                        &wire::CharmProgressionAction::ResetAll,
                    )
                    .checked()?;
                    reply.outcome = wire::CharmProgressionOutcome::ResetAll(
                        wire::CharmRemoveDisposition::NotEnoughGold,
                    );
                    reply.fee_gold = 0;
                    let refused = wire::encode_charm_progression_reply(&reply).checked()?;

                    send(
                        &link,
                        &exchanges,
                        intent,
                        84,
                        payload.clone(),
                        CommandStatus::Accepted,
                        refused,
                    )
                    .await
                    .checked()?;
                    assert!(
                        link.charm_progression_panel()
                            .feedback
                            .to_lowercase()
                            .contains("refused")
                    );
                    send(
                        &link,
                        &exchanges,
                        intent,
                        84,
                        payload.clone(),
                        CommandStatus::Rejected,
                        vec![],
                    )
                    .await
                    .checked()?;
                    assert!(
                        link.charm_progression_panel()
                            .feedback
                            .to_lowercase()
                            .contains("refused")
                    );
                    send(
                        &link,
                        &exchanges,
                        intent,
                        84,
                        payload,
                        CommandStatus::Accepted,
                        vec![255],
                    )
                    .await
                    .checked()?;
                    assert!(link.charm_progression_panel().feedback.contains("Invalid"));
                    link.charm_progression_edit(0, "invalid".into());
                    link.charm_progression_request(Intent::Assign);
                    assert!(!link.charm_progression_panel().pending);
                    // Ending with a request in flight clears pending, selection, drafts and confirmed state.
                    link.charm_progression_request(Intent::Query);
                    assert!(link.charm_progression_panel().pending);
                    exchanges.release.add_permits(1);
                    task.await.checked()?;
                    let ended = link.charm_progression_panel();
                    assert!(!ended.pending);
                    assert!(!ended.selected);
                    assert!(!ended.open);
                    assert!(ended.summary.is_empty());
                    assert!(ended.fields.iter().all(String::is_empty));
                    assert!(ended.feedback.contains("ended"));
                    // Re-admit on this mailbox with a fresh peer. No old selection/confirmation survives.
                    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
                    let reconnect = tokio::spawn(run_session(
                        Peer {
                            exchanges: Arc::clone(&exchanges),
                            snapshot: bytes,
                        },
                        receiver,
                        outbox.clone(),
                    ));
                    ready(&link).await.checked()?;
                    assert!(!link.charm_progression_panel().open);
                    assert!(!link.charm_progression_panel().pending);
                    assert!(link.charm_progression_panel().reply.is_none());
                    drop(sender);
                    reconnect.await.checked()?;
                    Ok::<(), Error>(())
                })
                .checked()?
                .checked()?;
            Ok(())
        })
        .checked()?
        .join()
        .map_err(|_| "scenario thread failed")
        .checked()?
}

#[test]
fn srx2_charm_consumers_server_reply_reaches_native_panel() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), Error> {
            oteryn_client_runtime::ClientRuntime::new()
                .checked()?
                .block_on(async {
                    let catalogue: serde_json::Value =
                        serde_json::from_slice(include_bytes!(concat!(
                            env!("CARGO_MANIFEST_DIR"),
                            "/../../content/charms/charms-00000-00024.json"
                        )))
                        .checked()?;
                    let records = catalogue["records"].as_array().ok_or("Charm records")?;
                    let names = [
                        "adrenaline_burst",
                        "bless",
                        "carnage",
                        "cleanse",
                        "cripple",
                        "fatal_hold",
                        "numb",
                        "scavenge",
                        "void_inversion",
                    ];
                    let race = std::num::NonZeroU32::new(1).ok_or("race")?;
                    let mut charms = Vec::new();
                    for name in names {
                        let key = format!("oteryn:charm.{name}");
                        let (index, row) = records
                            .iter()
                            .enumerate()
                            .find(|(_, row)| {
                                row["definition"]["identity"]["key"].as_str() == Some(key.as_str())
                            })
                            .ok_or("Charm definition")?;
                        charms.push(wire::charm::CharmState {
                            charm: std::num::NonZeroU32::new(u32::try_from(index + 1).checked()?)
                                .ok_or("Charm index")?,
                            kind: if row["definition"]["category"].as_str() == Some("major") {
                                wire::charm::CharmKind::Major
                            } else {
                                wire::charm::CharmKind::Minor
                            },
                            unlocked_stage: 3,
                            assigned_race: Some(race),
                            next_stage_cost: 0,
                            effect_active: true,
                        });
                    }
                    let reply = wire::CharmProgressionReply {
                        outcome: wire::CharmProgressionOutcome::Query,
                        fee_gold: 0,
                        bestiary: vec![],
                        charms: wire::charm::CharmView {
                            charms,
                            charm_points_available: 0,
                            minor_charm_echoes_available: 0,
                            assignment_slot_limit: None,
                        },
                    };
                    let exchanges = Arc::new(Script {
                        entries: Mutex::new(VecDeque::new()),
                        release: tokio::sync::Semaphore::new(0),
                    });
                    let (link, incoming, outbox) = play_channel();
                    let task = tokio::spawn(run_session(
                        Peer {
                            exchanges: Arc::clone(&exchanges),
                            snapshot: vec![],
                        },
                        incoming,
                        outbox,
                    ));
                    ready(&link).await?;
                    link.charm_progression_open();
                    send(
                        &link,
                        &exchanges,
                        Intent::Query,
                        84,
                        wire::encode_charm_progression_request(
                            &wire::CharmProgressionAction::Query,
                        )
                        .checked()?,
                        CommandStatus::Accepted,
                        wire::encode_charm_progression_reply(&reply).checked()?,
                    )
                    .await?;
                    let panel = link.charm_progression_panel();
                    assert!(panel.open);
                    assert_eq!(panel.reply.as_ref(), Some(&reply));
                    let native_text = oteryn_client_ui::charm_progression::panel_text(&panel);
                    for charm in &reply.charms.charms {
                        assert!(native_text.contains(&format!("Charm {}:", charm.charm)));
                    }
                    assert_eq!(native_text.matches("active true").count(), names.len());
                    assert_eq!(native_text.matches("stage 3").count(), names.len());
                    drop(link);
                    task.await.checked()?;
                    Ok::<(), Error>(())
                })
                .checked()?
        })
        .checked()?
        .join()
        .map_err(|_| "Charm client play thread failed")
        .checked()?
}
