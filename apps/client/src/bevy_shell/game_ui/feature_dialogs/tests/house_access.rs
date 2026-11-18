use super::super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn house_list_window_shortcut_save_and_session_cleanup() -> TestResult {
    house_window_action("Save guests", 4, "02020202-0202-7202-8202-020202020202")
}
#[test]
fn house_browser_shortcut_bid_and_session_cleanup() -> TestResult {
    house_window_action("Bid", 3, "1250")
}
#[test]
fn house_browser_opens_owned_and_incoming_catalogue() -> TestResult {
    house_window_action("Owned / incoming", 3, "")
}
#[test]
fn house_browser_reviews_and_confirms_transfer() -> TestResult {
    house_window_action("Review transfer", 3, "200")
}
#[test]
fn house_browser_clears_ownership_error_on_reopen() -> TestResult {
    house_window_action("Review move-out", 2, "invalid")
}
#[test]
fn house_acl_reply_replaces_pending_member_draft() -> TestResult {
    house_window_action("Load guests", 4, "")
}
#[test]
fn house_acl_reply_restores_requested_house_after_map() -> TestResult {
    house_window_action_with_map("Load guests", 4, "", ReplyTiming::AfterMapProjection)
}
#[test]
fn house_acl_refused_door_read_restores_requested_house_after_map() -> TestResult {
    refused_door_read(ReplyTiming::AfterMapProjection)
}
#[test]
fn house_acl_refused_door_read_routes_before_map_projection() -> TestResult {
    refused_door_read(ReplyTiming::BeforeMapProjection)
}

fn refused_door_read(timing: ReplyTiming) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            house_window_action_with_map("Load door", 5, "101 110 -7", timing)
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "house ACL scenario panicked")??;
    Ok(())
}

struct MissingDoor;

impl crate::play::Stepper for MissingDoor {
    fn feature_selected(&self, capability: u32) -> bool {
        capability == 49
    }

    async fn feature(
        &mut self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
    ) -> Result<oteryn_session::GatedCommandResult, oteryn_session::SessionError> {
        use oteryn_session::property_services::house;
        assert_eq!((command.capability, command.kind), (49, 57));
        Ok(oteryn_session::GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_session::CommandStatus::Accepted,
            payload: house::encode_house_result(house::HouseDisposition::Rejected, 0),
        })
    }

    async fn serve(&mut self, _: std::time::Duration) -> Result<(), oteryn_session::SessionError> {
        // Let the test advance the real session exactly once between Bevy frames.
        tokio::task::yield_now().await;
        Ok(())
    }

    fn drain(&mut self) -> Vec<oteryn_session::SessionEvent> {
        Vec::new()
    }

    async fn step(
        &mut self,
        _: oteryn_session::StepDirection,
    ) -> Result<oteryn_session::StepOutcome, oteryn_session::SessionError> {
        Err(oteryn_session::SessionError::SessionUnusable)
    }

    async fn combat(
        &mut self,
        _: crate::combat_input::CombatCommand,
    ) -> Result<crate::combat_input::CombatReport, oteryn_session::SessionError> {
        Err(oteryn_session::SessionError::SessionUnusable)
    }

    async fn logout(
        &mut self,
    ) -> Result<Option<oteryn_session::LogoutResult>, oteryn_session::SessionError> {
        Ok(None)
    }
}

#[test]
fn house_acl_reply_preserves_unrelated_draft_and_pending_paste() -> TestResult {
    house_window_action("Load subowners", 4, "")
}
#[test]
fn house_handoff_clears_unsent_access_drafts() -> TestResult {
    house_window_action("Handoff", 4, "old member")
}
#[test]
fn rejected_house_member_edit_sends_nothing() -> TestResult {
    house_window_action("Save guests", 4, &"é".repeat(4097))
}
fn house_window_action(caption: &str, field_index: usize, value: &str) -> TestResult {
    house_window_action_with_map(caption, field_index, value, ReplyTiming::WithoutMap)
}

#[derive(PartialEq)]
enum ReplyTiming {
    WithoutMap,
    AfterMapProjection,
    BeforeMapProjection,
}

fn house_window_action_with_map(
    caption: &str,
    field_index: usize,
    value: &str,
    timing: ReplyTiming,
) -> TestResult {
    let map_during_read = timing != ReplyTiming::WithoutMap;
    let reply_before_map_projection = timing == ReplyTiming::BeforeMapProjection;
    use crate::{
        combat_input::PlayCommand,
        play::{PlayView, play_channel},
    };
    use bevy::input::keyboard::{Key as LogicalKey, KeyboardInput};
    let (link, mut receiver, outbox) = play_channel();
    link.use_shared_frontend();
    let advance_session = |session: std::pin::Pin<&mut _>| {
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(std::future::Future::poll(session, &mut context).is_pending());
    };
    let routed = if caption == "Load door" {
        let (send, incoming) = tokio::sync::mpsc::unbounded_channel();
        let mut session = Box::pin(crate::play::run_session(
            MissingDoor,
            incoming,
            outbox.clone(),
        ));
        advance_session(session.as_mut());
        Some((send, session))
    } else {
        None
    };
    if map_during_read {
        link.bevy_house_map_fixture();
    } else {
        link.bevy_house_list_fixture();
    }
    if caption == "Handoff" {
        link.house_edit(2, String::new());
    }
    if caption == "Review transfer" {
        use oteryn_client_runtime::{
            house::house_sale as wire,
            raw_feature::{CommandStatus, GatedCommandResult},
        };
        link.house_ownership_open();
        let PlayCommand::Feature(query) = receiver.try_recv()? else {
            return Err("query".into());
        };
        let reply = wire::HouseSaleReply {
            now_unix_ms: 1_800_000_000_000,
            has_more: false,
            rows: vec![wire::HouseSaleRow {
                house: "oteryn:content.house.harbour_home".into(),
                name: "Harbour Home".into(),
                revision: 7,
                owner: "Elara Vale".into(),
                recipient: String::new(),
                transfer_state: 0,
                due_at_unix_ms: 0,
                price: 0,
                rent: 50,
                allowed: 1 << 3,
                move_out_state: 0,
            }],
        };
        link.bevy_house_ownership_result(
            &query,
            &GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: CommandStatus::Accepted,
                payload: wire::encode_reply(&reply).map_err(|e| format!("{e:?}"))?,
            },
        );
    }

    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .init_resource::<InputFocus>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_message::<KeyboardInput>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::super::modal::install(&mut app);
    super::super::houses::install(&mut app);
    app.update();
    let press_h = |app: &mut App| {
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::KeyH,
            logical_key: LogicalKey::Character("H".into()),
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
        app.update();
    };
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::ControlLeft);
        keys.press(KeyCode::ShiftLeft);
        keys.press(KeyCode::AltLeft);
    }
    // Ctrl+Alt+Shift+H belongs to the cast chat binding; the house window stays closed.
    press_h(&mut app);
    assert!(
        !app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "Enter one member UUID per line."),
        "Ctrl+Alt+Shift+H opened the house window"
    );
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::AltLeft);
    press_h(&mut app);
    for expected in [
        if map_during_read {
            "Edit House List — Home"
        } else {
            "Edit House List — oteryn:house.test"
        },
        "Enter one member UUID per line.",
    ] {
        assert!(
            app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .any(|text| text.0 == expected),
            "missing house list descriptor: {expected}"
        );
    }
    let button = |app: &mut App, caption: &str| {
        app.world_mut()
            .query::<(&Text, &ChildOf)>()
            .iter(app.world())
            .find_map(|(text, parent)| (text.0 == caption).then_some(parent.parent()))
            .ok_or("house button")
    };
    let save = button(
        &mut app,
        if caption == "Handoff" {
            "Query"
        } else {
            caption
        },
    )?;
    for (field, mut text) in app
        .world_mut()
        .query::<(&super::super::houses::Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if caption == "Handoff" && field.0 == 2 {
            text.editor_mut().set_text("3");
        }
        if field.0 == field_index {
            text.editor_mut().set_text(value);
        }
        if caption == "Review transfer" {
            if field.0 == 2 {
                text.editor_mut().set_text("1");
            }
            if field.0 == 4 {
                text.editor_mut().set_text("Mira Vale");
            }
            if field.0 == 5 {
                text.editor_mut().set_text("invalid date");
            }
        }
        if field.0 == 6 {
            text.editor_mut().set_text("CONFIRM");
        }
    }
    if caption == "Handoff" {
        use oteryn_session::{
            AppliedDelta, GatedSnapshot, SessionEvent,
            activity_scope::{ActivityScope, ActivityScopeSnapshot, encode_activity_scope},
        };
        let mut id = [1; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        let mut successor = id;
        successor[15] = 2;
        for generation in 1..=2 {
            if generation == 2 {
                for (field, mut text) in app
                    .world_mut()
                    .query::<(&super::super::houses::Field, &mut EditableText)>()
                    .iter_mut(app.world_mut())
                {
                    if matches!(field.0, 2 | 4 | 6) {
                        text.editor_mut().set_text("old draft");
                    }
                }
                id = successor;
                successor[15] += 1;
            }
            let payload = encode_activity_scope(&ActivityScopeSnapshot {
                handoff: id,
                source_session: id,
                session: successor,
                scope: ActivityScope::Instance {
                    world: id,
                    instance: id,
                },
                scope_generation: generation,
                connection_generation: 1,
                lease_generation: generation + 1,
                selection: vec![49, 117],
                requires_ack: false,
            })
            .map_err(|e| format!("{e:?}"))?;
            outbox.push(vec![SessionEvent::GatedSnapshotReplaced(AppliedDelta {
                server_sequence: generation,
                base_revision: generation - 1,
                new_revision: generation,
                value: GatedSnapshot {
                    domain_id: 99,
                    snapshot_type: 1,
                    revision: generation,
                    payload,
                },
            })]);
            app.update();
            let mut cleared = Vec::new();
            for (field, text) in app
                .world_mut()
                .query::<(&super::super::houses::Field, &EditableText)>()
                .iter(app.world())
            {
                if matches!(field.0, 2 | 4 | 6) {
                    assert_eq!(text.value(), "");
                    cleared.push(field.0);
                }
            }
            cleared.sort_unstable();
            assert_eq!(cleared, [2, 4, 6]);
        }
        return Ok(());
    }
    app.world_mut().trigger(Activate { entity: save });
    app.update();
    if value.len() > 8192 {
        assert!(receiver.try_recv().is_err());
        return Ok(());
    }
    if caption == "Review move-out" {
        let has_error = |app: &mut App| {
            app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .any(|text| text.0.contains("Select an ownership row"))
        };
        app.update();
        assert!(has_error(&mut app));
        let entity = button(&mut app, "Close")?;
        app.world_mut().trigger(Activate { entity });
        app.update();
        press_h(&mut app);
        let entity = button(&mut app, caption)?;
        assert!(!has_error(&mut app));

        for (field, mut text) in app
            .world_mut()
            .query::<(&super::super::houses::Field, &mut EditableText)>()
            .iter_mut(app.world_mut())
        {
            if field.0 == 2 {
                text.editor_mut().set_text("invalid");
            }
        }
        app.world_mut().trigger(Activate { entity });
        app.update();
        app.update();
        assert!(has_error(&mut app));
        app.world_mut().non_send_mut::<HostOwner>().close();
        app.update();
        let (link, _commands, _outbox) = play_channel();
        link.use_shared_frontend();
        link.bevy_house_list_fixture();
        let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
        view.use_shared_frontend();
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.phase = crate::bevy_shell::owner::Phase::Playing;
        owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
        press_h(&mut app);
        button(&mut app, caption)?;
        assert!(!has_error(&mut app));
        return Ok(());
    }
    if caption == "Review transfer" {
        app.update();
        for (field, mut text) in app
            .world_mut()
            .query::<(&super::super::houses::Field, &mut EditableText)>()
            .iter_mut(app.world_mut())
        {
            match field.0 {
                2 => assert_eq!(text.value(), "1"),
                3 => assert_eq!(text.value(), "200"),
                4 => assert_eq!(text.value(), "Mira Vale"),
                5 => {
                    assert_eq!(text.value(), "invalid date");
                    text.editor_mut().set_text("2027-01-20 12:00:00 UTC");
                }
                _ => {}
            }
        }
        let retry = button(&mut app, caption)?;
        app.world_mut().trigger(Activate { entity: retry });
        app.update();
        assert!(receiver.try_recv().is_err(), "review sends no mutation");
        app.update();
        assert!(
            app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .any(|text| text.0.contains("Offer Harbour Home to Mira Vale")
                    && text.0.contains("2027-01-20 12:00:00 UTC")
                    && text.0.contains("for 200 gold"))
        );
        let confirm = app
            .world_mut()
            .query::<(&Text, &ChildOf)>()
            .iter(app.world())
            .find_map(|(text, parent)| {
                (text.0 == "Confirm reviewed action").then_some(parent.parent())
            })
            .ok_or("confirm button")?;
        app.world_mut().trigger(Activate { entity: confirm });
        app.update();
    }
    let PlayCommand::Feature(command) = receiver.try_recv()? else {
        return Err("house intent".into());
    };
    if matches!(caption, "Load guests" | "Load subowners" | "Load door") {
        use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
        use oteryn_session::property_services::house;
        use std::sync::{Arc, Mutex};
        let clipboard = Arc::new(Mutex::new(None));
        app.update();
        let mut door_editor = None;
        for (entity, field, mut text) in app
            .world_mut()
            .query::<(Entity, &super::super::houses::Field, &mut EditableText)>()
            .iter_mut(app.world_mut())
        {
            if field.0 == 2 {
                text.editor_mut().set_text("2");
            }
            if field.0 == 4 && !reply_before_map_projection {
                text.editor_mut().set_text("intervening edit");
            }
            if field.0 == 5 && caption == "Load subowners" {
                text.editor_mut().set_text("2027-01-20");
                text.pending_paste = Some(bevy::clipboard::ClipboardRead::Pending(Arc::clone(
                    &clipboard,
                )));
                door_editor = Some(entity);
            }
        }
        if map_during_read {
            let entity = button(&mut app, "Show entrance map")?;
            app.world_mut().trigger(Activate { entity });
            app.update();
            if !reply_before_map_projection {
                app.update();
                assert!(
                    app.world_mut()
                        .query::<&Text>()
                        .iter(app.world())
                        .any(|text| text.0 == "◆ 2: Harbour Home\n120,130,7")
                );
            }
        }
        let door_refused = caption == "Load door";
        if let Some((send, mut session)) = routed {
            let query =
                house::decode_house_query(&command.payload).map_err(|e| format!("{e:?}"))?;
            assert_eq!(
                query.acl,
                Some(house::HouseAclRead {
                    house: "oteryn:house.test".into(),
                    selector: house::HouseAclSelector::Door {
                        x: 101,
                        y: 110,
                        floor: -7,
                    },
                })
            );
            let owner = app.world().non_send::<HostOwner>();
            let link = &owner.play.as_ref().ok_or("active play")?.link;
            let selected = link.house_panel().model;
            assert_eq!(selected.index, 1);
            assert_eq!(selected.pending.as_ref(), Some(&command));
            send.send(PlayCommand::Feature(command))?;
            advance_session(session.as_mut());
            let restored = link.house_panel().model;
            assert!(
                !restored.pending_acl_read,
                "the session must route the door refusal"
            );
            assert!(restored.pending.is_none());
            assert!(restored.acl_snapshot.is_none());
            assert_eq!(restored.index, 0);
        } else {
            let payload = house::encode_house_acl_snapshot(&house::HouseAclSnapshot {
                revision: 8,
                members: vec![],
            })
            .map_err(|e| format!("{e:?}"))?;
            outbox.bevy_properties_result(
                &command,
                &GatedCommandResult {
                    command_id: 1,
                    server_sequence: 1,
                    status: CommandStatus::Accepted,
                    payload,
                },
            );
        }
        app.update();
        assert!(
            app.world_mut()
                .query::<(&super::super::houses::Field, &EditableText)>()
                .iter(app.world())
                .any(|(field, text)| field.0 == 4 && text.value() == "")
        );
        if !reply_before_map_projection {
            assert!(
                app.world_mut()
                    .query::<(&super::super::houses::Field, &EditableText)>()
                    .iter(app.world())
                    .any(|(field, text)| field.0 == 2 && text.value() == "1")
            );
        }
        if caption == "Load subowners" {
            let entity = door_editor.ok_or("door editor")?;
            let draft = app
                .world()
                .get::<EditableText>(entity)
                .ok_or("door editor was replaced")?;
            assert_eq!(draft.value(), "2027-01-20");
            assert!(matches!(&draft.pending_paste,
                Some(bevy::clipboard::ClipboardRead::Pending(pending))
                    if Arc::ptr_eq(pending, &clipboard)
            ));
        }
        if map_during_read {
            let title = app
                .world_mut()
                .query::<&Text>()
                .iter(app.world())
                .find(|text| text.0.starts_with("Edit House List — "))
                .ok_or("house title")?
                .0
                .clone();
            let close = button(&mut app, "Close map")?;
            app.world_mut().trigger(Activate { entity: close });
            app.update();
            for (field, mut text) in app
                .world_mut()
                .query::<(&super::super::houses::Field, &mut EditableText)>()
                .iter_mut(app.world_mut())
            {
                if field.0 == 6 {
                    text.editor_mut().set_text("CONFIRM");
                }
            }
            let entity = button(&mut app, "Save guests")?;
            app.world_mut().trigger(Activate { entity });
            app.update();
            let PlayCommand::Feature(save) = receiver.try_recv()? else {
                return Err("save guests".into());
            };
            let intent = oteryn_protocol_oteryn::house::decode_house_intent(&save.payload)
                .map_err(|error| format!("{error:?}"))?;
            assert_eq!(intent.house, "oteryn:house.test");
            assert_eq!(
                title, "Edit House List — Home",
                "title for {}",
                intent.house
            );
            let expected_revision = if door_refused { 7 } else { 8 };
            assert!(matches!(
                intent.action,
                oteryn_protocol_oteryn::house::HouseAction::Acl {
                    subowners: false,
                    revision,
                    ..
                } if revision == expected_revision
            ));
        }
        return Ok(());
    }
    if matches!(caption, "Bid" | "Save guests") {
        app.update();
        assert!(
            app.world_mut()
                .query::<(&super::super::houses::Field, &EditableText)>()
                .iter(app.world())
                .any(|(field, text)| field.0 == 6 && text.value() == "")
        );
    }
    if caption == "Bid" {
        use std::sync::{Arc, Mutex};
        app.init_resource::<bevy::text::FontCx>()
            .init_resource::<bevy::text::LayoutCx>()
            .init_resource::<bevy::clipboard::Clipboard>()
            .add_systems(PostUpdate, bevy::text::apply_text_edits);
        let clipboard = Arc::new(Mutex::new(None));
        for (field, mut text) in app
            .world_mut()
            .query::<(&super::super::houses::Field, &mut EditableText)>()
            .iter_mut(app.world_mut())
        {
            if field.0 == 4 {
                text.pending_paste = Some(bevy::clipboard::ClipboardRead::Pending(Arc::clone(
                    &clipboard,
                )));
            }
            if matches!(field.0, 2 | 4 | 6) {
                text.editor_mut().set_text("new draft");
            }
        }
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .bevy_house_ownership_result(
                &command,
                &oteryn_session::GatedCommandResult {
                    command_id: 1,
                    server_sequence: 1,
                    status: oteryn_session::CommandStatus::Rejected,
                    payload: vec![],
                },
            );
        app.update();
        for (field, text) in app
            .world_mut()
            .query::<(&super::super::houses::Field, &EditableText)>()
            .iter(app.world())
        {
            if matches!(field.0, 2 | 4 | 6) {
                if field.0 == 4 {
                    assert!(text.pending_paste.is_some());
                }
                assert_eq!(text.value(), "new draft");
            }
        }
        *clipboard.lock().map_err(|_| "clipboard lock")? = Some(Ok("pasted member".into()));
        app.update();
        app.update();
        assert!(
            app.world_mut()
                .query::<(&super::super::houses::Field, &EditableText)>()
                .iter(app.world())
                .any(|(field, text)| field.0 == 4
                    && text.value().to_string().contains("pasted member"))
        );
    }
    let expected_kind = if matches!(caption, "Owned / incoming" | "Review transfer") {
        oteryn_client_runtime::house::house_sale::COMMAND
    } else {
        56
    };
    assert_eq!((command.capability, command.kind), (49, expected_kind));
    if caption == "Review transfer" {
        let intent = oteryn_client_runtime::house::house_sale::decode_request(&command.payload)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            (intent.action, intent.price, intent.expected_revision),
            (3, 200, 7)
        );
        assert_eq!(intent.recipient, "Mira Vale");
        assert_eq!(intent.house, "oteryn:content.house.harbour_home");
    }
    app.world_mut().non_send_mut::<HostOwner>().close();
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&super::super::houses::Field>()
            .iter(app.world())
            .count(),
        0
    );
    assert!(receiver.try_recv().is_err());
    Ok(())
}

#[test]
fn entrance_map_keeps_the_edit_row_bound_while_a_bid_is_pending() -> TestResult {
    use crate::{combat_input::PlayCommand, play::play_channel};

    let (link, mut commands, _) = play_channel();
    link.bevy_house_map_fixture();
    link.house_edit(3, "1250".into());
    link.house_edit(6, "CONFIRM".into());
    link.house_request(2);
    assert!(matches!(commands.try_recv()?, PlayCommand::Feature(_)));
    let pending = link.house_panel();
    assert!(pending.model.pending.is_some());
    let mut drafts = pending.fields.clone();
    drafts[2] = "2".into();

    link.house_map(pending.activity_revision, &pending.model, drafts);
    let selected = link.house_panel();
    assert_eq!(
        selected.model.index + 1,
        selected.fields[2].parse::<usize>()?
    );
    assert_eq!(selected.model.index, 0);
    Ok(())
}
