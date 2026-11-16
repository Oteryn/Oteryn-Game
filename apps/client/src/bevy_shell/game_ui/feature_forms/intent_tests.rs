//! Each form's buttons and number fields reach the session through the real Bevy form route:
//! the open key goes through the shell router, the fields are typed into the form's widgets
//! and a button is activated, so the submit path the player triggers decides every intent.
use super::*;
use oteryn_session::{CommandStatus, GatedCommandResult};

/// Types `value` into field `index` of the shown form, once the form shows the latest state.
fn set(app: &mut App, index: usize, value: &str) {
    app.update();
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if field.0 == index {
            *text = EditableText::new(value);
        }
    }
}

/// The shown form's summary text once the last submit has been projected.
fn summary(app: &mut App) -> String {
    app.update();
    app.world_mut()
        .query_filtered::<&Text, With<Summary>>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect()
}

fn with_link<T>(
    app: &App,
    read: impl FnOnce(&crate::play::PlayLink) -> T,
) -> Result<T, Box<dyn std::error::Error>> {
    let owner = app.world().non_send::<HostOwner>();
    Ok(read(&owner.play.as_ref().ok_or("play")?.link))
}

/// One key press through the feature's open-key handler the shell router calls, then one
/// frame; used where the default key bindings claim the chord before the feature handlers.
fn open_key(
    app: &mut App,
    handler: fn(&crate::play::PlayLink, &[NormalizedInputEvent]) -> bool,
    code: u16,
    modifiers: Modifiers,
) -> TestResult {
    let event = NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(code)?,
        state: ButtonState::Pressed,
        modifiers,
        repeat: false,
    };
    assert!(with_link(app, |link| handler(link, &[event]))?);
    app.update();
    Ok(())
}

fn drain(commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>) {
    while commands.try_recv().is_ok() {}
}

fn accepted(payload: Vec<u8>) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload,
    }
}

fn ctrl() -> Modifiers {
    Modifiers::one(Modifier::Control)
}

fn ctrl_shift() -> Modifiers {
    ctrl().with(Modifier::Shift)
}

/// The form for the test, its capabilities selected.
fn capable() -> Result<Fixture, Box<dyn std::error::Error>> {
    let (app, commands, outbox) = fixture()?;
    outbox.bevy_feature_form_capabilities();
    Ok((app, commands, outbox))
}

/// A typed channel that is neither the own channel nor an invitation is refused; the earlier
/// pick is never joined, left or written to in its place.
#[test]
fn private_channels_refuse_a_typed_channel_that_is_not_listed() -> TestResult {
    let (mut app, mut commands, outbox) = capable()?;
    with_link(&app, |link| link.chat_private_open())?;
    drain(&mut commands);
    outbox.bevy_private_channels_listed(7, 9);
    app.update();
    assert!(form_open(&app, "Private channels")?);
    set(&mut app, 0, "9");
    click(&mut app, Some(2))?;
    assert!(commands.try_recv().is_ok(), "the invitation is joined");
    assert_eq!(
        with_link(&app, |link| link.chat_private_panel())?.channel,
        Some(9)
    );
    outbox.bevy_private_channels_answered();
    app.update();
    for action in [2, 3, 6] {
        set(&mut app, 0, "5");
        set(&mut app, 2, "hello");
        click(&mut app, Some(action))?;
        assert!(
            commands.try_recv().is_err(),
            "button {action} used the earlier channel"
        );
        app.update();
        assert!(summary(&mut app).contains("Select your channel or an invitation"));
        assert_eq!(
            with_link(&app, |link| link.chat_private_panel())?.channel,
            Some(9)
        );
    }
    Ok(())
}

/// An unknown room number is refused with feedback; Open, Close and Send use the typed row.
#[test]
fn public_channels_map_the_room_row_and_refuse_an_unknown_room() -> TestResult {
    use oteryn_client_runtime::chat_public_channels::ChatRoom;
    use oteryn_session::ChatIntent;
    let (mut app, mut commands, outbox) = capable()?;
    press(&mut app, 6, ctrl())?;
    assert!(form_open(&app, "Public Channels")?);
    drain(&mut commands);
    for (action, row, expected) in [
        (0, "2", ChatIntent::OpenRoom(ChatRoom::ALL[1])),
        (1, "3", ChatIntent::CloseRoom(ChatRoom::ALL[2])),
        (
            2,
            "1",
            ChatIntent::Room {
                room: ChatRoom::ALL[0],
                text: "hi".into(),
            },
        ),
    ] {
        set(&mut app, 0, row);
        set(&mut app, 1, "hi");
        click(&mut app, Some(action))?;
        assert_eq!(commands.try_recv()?, PlayCommand::ChatPublic(expected));
        outbox.bevy_feature_forms_answered();
        app.update();
    }
    for row in ["99", "0", ""] {
        set(&mut app, 0, row);
        click(&mut app, Some(0))?;
        assert!(commands.try_recv().is_err(), "room {row:?} was opened");
        app.update();
        assert!(summary(&mut app).contains("Enter a room number from the list"));
        assert_eq!(
            with_link(&app, |link| link.chat_public_panel())?.room,
            ChatRoom::ALL[0]
        );
    }
    Ok(())
}

/// A refused Exiva read leaves Refresh usable; the other buttons say why they do nothing.
#[test]
fn exiva_refresh_stays_enabled_after_a_refused_read() -> TestResult {
    let (mut app, mut commands, outbox) = capable()?;
    open_key(&mut app, crate::play::PlayLink::exiva_input, 8, ctrl())?;
    assert!(form_open(&app, "Exiva permissions")?);
    assert_eq!(commands.try_recv()?, PlayCommand::Exiva(vec![5]));
    let mut refused = accepted(Vec::new());
    refused.status = CommandStatus::Rejected;
    outbox.bevy_exiva_result(&refused);
    app.update();
    let disabled = app
        .world_mut()
        .query::<(&Control, Has<InteractionDisabled>)>()
        .iter(app.world())
        .any(|(control, disabled)| control.action == Some(2) && disabled);
    assert!(!disabled, "Refresh is disabled after a refused read");
    click(&mut app, Some(0))?;
    assert!(commands.try_recv().is_err());
    assert!(summary(&mut app).contains("Exiva request refused; refresh the list"));
    click(&mut app, Some(2))?;
    assert_eq!(commands.try_recv()?, PlayCommand::Exiva(vec![5]));
    Ok(())
}

/// Add, Remove and each permission button send the payload the native window sent.
#[test]
fn exiva_buttons_map_to_their_requests() -> TestResult {
    use oteryn_protocol_oteryn::exiva::PlayerList;
    let (mut app, mut commands, outbox) = capable()?;
    open_key(&mut app, crate::play::PlayLink::exiva_input, 8, ctrl())?;
    drain(&mut commands);
    let list = accepted(
        PlayerList {
            modes: 2,
            players: vec![([3; 16], "Al".into())],
        }
        .encode()
        .ok_or("list")?,
    );
    outbox.bevy_exiva_result(&list);
    app.update();
    set(&mut app, 0, "Bob");
    click(&mut app, Some(0))?;
    assert_eq!(
        commands.try_recv()?,
        PlayCommand::Exiva(b"\x06Bob".to_vec())
    );
    outbox.bevy_exiva_result(&list);
    app.update();
    set(&mut app, 1, "1");
    click(&mut app, Some(1))?;
    let mut remove = vec![2];
    remove.extend([3; 16]);
    assert_eq!(commands.try_recv()?, PlayCommand::Exiva(remove));
    outbox.bevy_exiva_result(&list);
    app.update();
    for row in ["5", "0", "x"] {
        set(&mut app, 1, row);
        click(&mut app, Some(1))?;
        assert!(commands.try_recv().is_err(), "row {row:?} was removed");
        assert!(summary(&mut app).contains("Enter a player row from the list"));
    }
    for (action, bit) in (3..=7).zip(0..) {
        click(&mut app, Some(action))?;
        assert_eq!(
            commands.try_recv()?,
            PlayCommand::Exiva(vec![0, 2 ^ (1 << bit)])
        );
        outbox.bevy_exiva_result(&list);
        app.update();
    }
    Ok(())
}

/// Preview artwork resolved by the server is drawn in the Bevy podium window.
#[test]
fn podium_preview_artwork_is_shown_in_the_podium_window() -> TestResult {
    use oteryn_session::property_services::house_podium as wire;
    let (mut app, mut commands, outbox) = capable()?;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    app.insert_resource(podium::PodiumArtwork {
        loaded: Some(
            crate::play::feature_0150_house_podium::view::preview_assets::Assets::load(&root),
        ),
    });
    press(&mut app, 19, ctrl().with(Modifier::Alt))?;
    assert!(form_open(&app, "House podium")?);
    drain(&mut commands);
    for (index, value) in [(1, "1"), (2, "1"), (3, "citizen")] {
        set(&mut app, index, value);
    }
    click(&mut app, Some(7))?;
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("preview request missing".into());
    };
    assert_eq!(command.kind, wire::preview::COMMAND_PREVIEW);
    let images = |app: &mut App| {
        app.world_mut()
            .query_filtered::<&ImageNode, With<podium::PodiumImage>>()
            .iter(app.world())
            .count()
    };
    assert_eq!(images(&mut app), 0);
    let artwork = wire::preview::Artwork {
        object: false,
        direction: 1,
        looks: [128, 136],
    };
    outbox.bevy_properties_result(
        &command,
        &accepted(wire::preview::Artwork::encode(Some(artwork)).ok_or("preview reply")?),
    );
    app.update();
    assert_eq!(
        with_link(&app, |link| link.house_podium_panel())?.artwork,
        Some(artwork)
    );
    assert!(has_text(&mut app, "Podium artwork preview"));
    assert_eq!(
        images(&mut app),
        2,
        "the outfit and its addon look are drawn"
    );
    click(&mut app, None)?;
    app.update();
    assert_eq!(images(&mut app), 0, "the preview closes with its window");
    Ok(())
}

/// A key must be typed: an empty or invalid key never clears or overwrites F1, and a bad row
/// or an unassigned key says why nothing happened.
#[test]
fn item_hotkeys_require_an_explicit_key_and_report_refusals() -> TestResult {
    let (mut app, mut commands, _outbox) = capable()?;
    press(&mut app, 8, ctrl_shift())?;
    assert!(form_open(&app, "Item hotkeys — Ctrl+Shift+E")?);
    drain(&mut commands);
    let f1 = |app: &App| -> Result<Option<u32>, Box<dyn std::error::Error>> {
        Ok(with_link(app, |link| link.hotkey_equip_panel())?.bindings[0].map(|item| item.get()))
    };
    set(&mut app, 0, "1");
    set(&mut app, 1, "1");
    click(&mut app, Some(0))?;
    assert_eq!(f1(&app)?, Some(3031));
    for (key, action) in [("", 0), ("", 1), ("13", 0), ("0", 1)] {
        set(&mut app, 0, key);
        set(&mut app, 1, "2");
        click(&mut app, Some(action))?;
        assert_eq!(f1(&app)?, Some(3031), "key {key:?} changed F1");
        assert!(summary(&mut app).contains("Enter a key number from 1 to 12"));
    }
    set(&mut app, 0, "");
    click(&mut app, Some(2))?;
    assert!(commands.try_recv().is_err());
    set(&mut app, 0, "1");
    set(&mut app, 1, "9");
    click(&mut app, Some(0))?;
    assert_eq!(f1(&app)?, Some(3031));
    assert!(summary(&mut app).contains("Enter an item row from the list"));
    set(&mut app, 0, "2");
    click(&mut app, Some(2))?;
    assert!(commands.try_recv().is_err());
    assert!(summary(&mut app).contains("No item is assigned to this key"));
    Ok(())
}

/// Assign, Use, Smart equip, Equip definition and Clear act on the typed key and rows.
#[test]
fn item_hotkey_buttons_map_to_their_actions() -> TestResult {
    let (mut app, mut commands, _outbox) = capable()?;
    press(&mut app, 8, ctrl_shift())?;
    drain(&mut commands);
    set(&mut app, 0, "3");
    set(&mut app, 1, "2");
    click(&mut app, Some(0))?;
    let panel = with_link(&app, |link| link.hotkey_equip_panel())?;
    assert_eq!(panel.bindings[2].map(|item| item.get()), Some(3035));
    let smart = panel.smart;
    click(&mut app, Some(3))?;
    assert_eq!(
        with_link(&app, |link| link.hotkey_equip_panel())?.smart,
        !smart
    );
    set(&mut app, 0, "3");
    click(&mut app, Some(2))?;
    let PlayCommand::ServerMechanic(_) = commands.try_recv()? else {
        return Err("use key request missing".into());
    };
    assert_eq!(
        with_link(&app, |link| link.hotkey_equip_panel())?.item_definition,
        3035
    );
    Ok(())
}

/// Clear removes only the typed key.
#[test]
fn item_hotkey_clear_removes_the_typed_key() -> TestResult {
    let (mut app, mut commands, _outbox) = capable()?;
    press(&mut app, 8, ctrl_shift())?;
    drain(&mut commands);
    for key in ["1", "2"] {
        set(&mut app, 0, key);
        set(&mut app, 1, "1");
        click(&mut app, Some(0))?;
    }
    set(&mut app, 0, "2");
    click(&mut app, Some(1))?;
    let bindings = with_link(&app, |link| link.hotkey_equip_panel())?.bindings;
    assert_eq!(bindings[0].map(|item| item.get()), Some(3031));
    assert_eq!(bindings[1], None);
    Ok(())
}

/// The Drome buttons send controls 200-204, Buy sends the typed reward and the reward
/// buttons send the typed rank; out-of-range numbers are refused with feedback, never a panic.
#[test]
fn arena_buttons_map_to_drome_and_reward_actions() -> TestResult {
    use oteryn_protocol_oteryn::arena::{ArenaAction, DromeAction, encode_match_request};
    let (mut app, mut commands, outbox) = capable()?;
    press(&mut app, 29, ctrl_shift())?;
    assert!(form_open(&app, "Arenas — Ctrl+Shift+Z")?);
    drain(&mut commands);
    outbox.bevy_arena_answered();
    app.update();
    let mut expect = |app: &mut App, action: usize, wanted: ArenaAction| -> TestResult {
        click(app, Some(action))?;
        let PlayCommand::Feature(command) = commands.try_recv()? else {
            return Err(format!("button {action} sent nothing").into());
        };
        assert_eq!(
            command.payload,
            encode_match_request(wanted).map_err(|error| format!("{error:?}"))?,
            "button {action}"
        );
        outbox.bevy_arena_answered();
        app.update();
        Ok(())
    };
    for (action, drome) in (11..).zip([
        DromeAction::Status,
        DromeAction::Enter,
        DromeAction::Leave,
        DromeAction::LowerStart,
        DromeAction::ClaimPotion,
    ]) {
        expect(&mut app, action, ArenaAction::Drome(drome))?;
    }
    set(&mut app, 1, "18");
    expect(&mut app, 16, ArenaAction::Drome(DromeAction::Buy(18)))?;
    set(&mut app, 0, "2");
    expect(
        &mut app,
        8,
        ArenaAction::ClaimReward {
            rank: 2,
            category: 3,
            choice: 2,
        },
    )?;
    for reward in ["0", "19", "", "-9223372036854775808"] {
        set(&mut app, 1, reward);
        click(&mut app, Some(16))?;
        assert!(commands.try_recv().is_err(), "reward {reward:?} was bought");
        assert!(summary(&mut app).contains("Enter a Drome reward number from 1 to 18"));
    }
    for rank in ["", "4", "0"] {
        set(&mut app, 0, rank);
        click(&mut app, Some(5))?;
        assert!(commands.try_recv().is_err(), "rank {rank:?} was claimed");
        assert!(summary(&mut app).contains("Enter a reward rank from 1 to 3"));
    }
    Ok(())
}

/// Rows count members first, then invitations; an invalid row says so and sends nothing.
#[test]
fn guild_rows_map_to_members_and_invitations() -> TestResult {
    use oteryn_client_runtime::guild::{GuildCommand, guild::GuildIntent};
    let (mut app, mut commands, outbox) = capable()?;
    press(&mut app, 10, ctrl())?;
    assert!(form_open(&app, "Guild")?);
    drain(&mut commands);
    outbox.bevy_guild_listed();
    app.update();
    let own = [1; 16];
    for (action, row, level, wanted) in [
        (4, "3", "", GuildIntent::Join { guild: [4; 16] }),
        (
            6,
            "1",
            "",
            GuildIntent::Kick {
                guild: own,
                character: [2; 16],
            },
        ),
        (
            7,
            "2",
            "",
            GuildIntent::RevokeInvite {
                guild: own,
                character: [3; 16],
            },
        ),
        (
            10,
            "2",
            "1",
            GuildIntent::SetRank {
                guild: own,
                character: [3; 16],
                level: 1,
                expected_revision: 3,
            },
        ),
    ] {
        set(&mut app, 1, level);
        set(&mut app, 2, row);
        click(&mut app, Some(action))?;
        assert_eq!(
            commands.try_recv()?,
            PlayCommand::Guild(GuildCommand::Intent(wanted))
        );
        outbox.bevy_guild_answered();
        app.update();
    }
    for (action, row, level, message) in [
        (4, "1", "", "Enter a row number from the list"),
        (4, "9", "", "Enter a row number from the list"),
        (6, "3", "", "Enter a row number from the list"),
        (7, "", "", "Enter a row number from the list"),
        (10, "1", "", "Enter a member row and a rank level"),
        (10, "9", "1", "Enter a member row and a rank level"),
    ] {
        set(&mut app, 1, level);
        set(&mut app, 2, row);
        click(&mut app, Some(action))?;
        assert!(commands.try_recv().is_err(), "button {action} row {row:?}");
        assert!(summary(&mut app).contains(message), "button {action}");
    }
    Ok(())
}

/// The Tibia Global guild window's Guild chat button opens the guild room in the chat window.
#[test]
fn guild_chat_button_opens_the_guild_room() -> TestResult {
    let (mut app, mut commands, outbox) = capable()?;
    press(&mut app, 10, ctrl())?;
    drain(&mut commands);
    outbox.bevy_guild_listed();
    app.update();
    assert!(has_text(&mut app, "Guild chat"));
    click(&mut app, Some(11))?;
    assert!(!with_link(&app, |link| link.chat_panel())?.guild_open);
    assert!(summary(&mut app).contains("Guild chat unavailable or waiting for the server"));
    outbox.bevy_chat_guild_member();
    click(&mut app, Some(11))?;
    let chat = with_link(&app, |link| link.chat_panel())?;
    assert!(chat.guild_open && chat.open);
    assert!(commands.try_recv().is_err());
    Ok(())
}

/// Load all, Previous and Next ask for the server pages the native window asked for.
#[test]
fn account_achievements_buttons_page_through_the_server_list() -> TestResult {
    use oteryn_client_runtime::account_achievements::{AccountAchievementsResult, Response};
    let (mut app, mut commands, outbox) = capable()?;
    open_key(
        &mut app,
        crate::play::PlayLink::achievements_input,
        4,
        ctrl_shift(),
    )?;
    assert!(form_open(&app, "Account achievements — Ctrl+Shift+A")?);
    assert_eq!(commands.try_recv()?, PlayCommand::AccountAchievements(0));
    let page = |page: u32, has_more: bool| {
        Response::Page(AccountAchievementsResult {
            total_points: 0,
            fact_count: 0,
            page,
            has_more,
            rows: Vec::new(),
        })
    };
    outbox.bevy_achievements_result(&page(0, true));
    app.update();
    click(&mut app, Some(1))?;
    assert!(commands.try_recv().is_err(), "no page before the first");
    click(&mut app, Some(2))?;
    assert_eq!(commands.try_recv()?, PlayCommand::AccountAchievements(1));
    outbox.bevy_achievements_result(&page(1, false));
    app.update();
    click(&mut app, Some(2))?;
    assert!(commands.try_recv().is_err(), "no page after the last");
    click(&mut app, Some(1))?;
    assert_eq!(commands.try_recv()?, PlayCommand::AccountAchievements(0));
    outbox.bevy_achievements_result(&page(0, true));
    app.update();
    click(&mut app, Some(0))?;
    assert_eq!(commands.try_recv()?, PlayCommand::AccountAchievements(0));
    Ok(())
}

/// Open, Preview, Start and Cancel send the typed boss, context and difficulty.
#[test]
fn boss_difficulty_buttons_map_to_their_requests() -> TestResult {
    use oteryn_client_runtime::boss_difficulty::{Request, encode_request};
    let (mut app, mut commands, outbox) = capable()?;
    open_key(
        &mut app,
        crate::play::PlayLink::boss_difficulty_input,
        28,
        ctrl(),
    )?;
    assert!(form_open(&app, "Boss difficulty")?);
    drain(&mut commands);
    let context = [1; 16];
    for (action, wanted) in [
        (
            0,
            Request::OpenWithProgress {
                boss_key: "oteryn:the_lion".into(),
            },
        ),
        (
            1,
            Request::Preview {
                context,
                difficulty: 3,
            },
        ),
        (
            2,
            Request::Start {
                context,
                difficulty: 3,
            },
        ),
        (3, Request::Cancel { context }),
    ] {
        set(&mut app, 0, "oteryn:the_lion");
        set(&mut app, 1, &"01".repeat(16));
        set(&mut app, 2, "3");
        click(&mut app, Some(action))?;
        let PlayCommand::Feature(command) = commands.try_recv()? else {
            return Err(format!("button {action} sent nothing").into());
        };
        assert_eq!(
            (command.capability, command.kind, command.payload),
            (
                225,
                431,
                encode_request(&wanted).map_err(|error| format!("{error:?}"))?
            ),
            "button {action}"
        );
        outbox.bevy_feature_forms_answered();
        app.update();
    }
    Ok(())
}

/// Select refuses an unknown hireling; the configuration buttons review store actions
/// 131-136 and Confirm buys the reviewed one.
#[test]
fn hireling_management_buttons_map_to_configuration_purchases() -> TestResult {
    use oteryn_client_runtime::cui3::store;
    let (mut app, mut commands, outbox) = capable()?;
    press(&mut app, 11, ctrl_shift())?;
    assert!(form_open(&app, "Hireling management")?);
    drain(&mut commands);
    outbox.bevy_hireling_configuration_priced();
    app.update();
    for row in ["9", "0", ""] {
        set(&mut app, 0, row);
        click(&mut app, Some(0))?;
        assert!(summary(&mut app).contains("Enter a hireling number from the list"));
        assert_eq!(
            with_link(&app, |link| link.hireling_management_panel())?.identity,
            None
        );
    }
    set(&mut app, 0, "1");
    click(&mut app, Some(0))?;
    assert_eq!(
        with_link(&app, |link| link.hireling_management_panel())?.identity,
        Some(crate::play::Outbox::BEVY_HIRELING)
    );
    for (action, offer) in (5..).zip(["25438", "25437", "1001", "1002", "1003", "1004"]) {
        set(&mut app, 6, "Bella");
        click(&mut app, Some(action))?;
        assert!(
            commands.try_recv().is_err(),
            "button {action} bought unreviewed"
        );
        click(&mut app, Some(11))?;
        let PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
            return Err(format!("button {action} purchase missing").into());
        };
        let (quote, _) = store::decode_purchase_request_with_hireling(&command.payload)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(quote.offer_key, format!("store:hireling:{offer}"));
        outbox.bevy_feature_forms_answered();
        app.update();
    }
    Ok(())
}
