use super::*;
use crate::combat_input::PlayCommand;
use oteryn_client_runtime::vip::vip as wire;
use oteryn_session::{VipPresenceBatch, VipPresenceEntry, VipPresenceStatus};
type TestResult = Result<(), Box<dyn std::error::Error>>;
pub(super) const CONTACTS: &str = r#"{"entries":[{"entry":73,"name":"Alice","description":"Friend","icon":2,"notify":true,"groups":[1]},{"entry":91,"name":"Bob","description":"","icon":0,"notify":false,"groups":[]}],"groups":[{"group":1,"name":"Friends"}],"next_entry":0}"#;
pub(super) fn publish(
    outbox: &crate::play::Outbox,
    raw: &str,
    revision: u64,
    status: VipPresenceStatus,
) {
    outbox.bevy_vip_fixture(
        raw,
        revision,
        VipPresenceBatch {
            entries: vec![VipPresenceEntry {
                entry: 73,
                name: "Alice".into(),
                status,
                announce: false,
                confirmed_for_ms: 1000,
            }],
        },
    );
}
pub(super) fn control(
    app: &mut App,
    predicate: impl Fn(Action) -> bool,
) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| predicate(control.command).then_some(entity))
        .ok_or_else(|| "control".into())
}
pub(super) fn click(app: &mut App, predicate: impl Fn(Action) -> bool) -> TestResult {
    let entity = control(app, predicate)?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
pub(super) fn field(app: &mut App) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query_filtered::<Entity, With<Field>>()
        .single(app.world())
        .map_err(Into::into)
}
pub(super) fn set_text(app: &mut App, value: &str) -> TestResult {
    let field = field(app)?;
    *app.world_mut()
        .get_mut::<EditableText>(field)
        .ok_or("editor")? = EditableText::new(value);
    Ok(())
}
pub(super) fn request(
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
) -> Result<wire::Request, Box<dyn std::error::Error>> {
    match commands.try_recv()? {
        PlayCommand::Vip(command) => {
            Ok(wire::decode_request(command.kind, &command.payload)
                .map_err(|_| "VIP wire decode")?)
        }
        _ => Err("unexpected command".into()),
    }
}
#[test]
fn vip_add_and_edit_use_actual_canonical_action_queue_without_local_echo() -> TestResult {
    let (mut app, mut commands, outbox) = super::super::tests::app()?;
    publish(&outbox, CONTACTS, 4, VipPresenceStatus::Online);
    app.update();
    click(&mut app, |action| matches!(action, Action::Add))?;
    set_text(&mut app, "Charlie")?;
    click(&mut app, |action| matches!(action, Action::Save(_)))?;
    let add = request(&mut commands)?;
    assert_eq!(add.operation, wire::Operation::Add);
    assert_eq!(add.text, "Charlie");
    assert_eq!(
        app.world_mut().query::<&Row>().iter(app.world()).count(),
        2,
        "no fabricated contact before server refresh"
    );
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .vip_panel()
            .pending
    );
    publish(&outbox, CONTACTS, 5, VipPresenceStatus::Online);
    app.update();
    click(&mut app, |action| matches!(action, Action::Contact(73)))?;
    click(&mut app, |action| matches!(action, Action::Edit(73)))?;
    set_text(&mut app, "Hunting partner")?;
    click(&mut app, |action| matches!(action, Action::Notify(_)))?;
    click(&mut app, |action| matches!(action, Action::Icon(_, 1)))?;
    click(&mut app, |action| matches!(action, Action::Group(_, 1)))?;
    click(&mut app, |action| matches!(action, Action::Save(_)))?;
    let edit = request(&mut commands)?;
    assert_eq!(edit.operation, wire::Operation::Edit);
    assert_eq!(edit.number, 73);
    assert_eq!(edit.text, "Hunting partner");
    assert_eq!(edit.icon, 3);
    assert!(!edit.notify);
    assert!(edit.groups.is_empty());
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn vip_remove_keeps_the_authoritative_contact_until_the_server_refreshes() -> TestResult {
    let (mut app, mut commands, outbox) = super::super::tests::app()?;
    publish(&outbox, CONTACTS, 4, VipPresenceStatus::Online);
    app.update();
    click(&mut app, |action| matches!(action, Action::Contact(73)))?;
    click(&mut app, |action| matches!(action, Action::Edit(73)))?;
    click(&mut app, |action| matches!(action, Action::Remove(73)))?;
    let remove = request(&mut commands)?;
    assert_eq!(remove.operation, wire::Operation::Remove);
    assert_eq!(remove.number, 73);
    assert!(
        app.world_mut()
            .query::<&Row>()
            .iter(app.world())
            .any(|row| row.entry == 73)
    );
    Ok(())
}
#[test]
fn vip_rows_are_retained_and_presence_is_authoritative_and_malformed_reply_is_closed() -> TestResult
{
    let (mut app, _commands, outbox) = super::super::tests::app()?;
    publish(&outbox, CONTACTS, 4, VipPresenceStatus::Online);
    app.update();
    app.update();
    let row = control(&mut app, |action| matches!(action, Action::Contact(73)))?;
    publish(&outbox, CONTACTS, 5, VipPresenceStatus::Offline);
    app.update();
    assert_eq!(
        control(&mut app, |action| matches!(action, Action::Contact(73)))?,
        row
    );
    assert!(
        app.world_mut()
            .query::<(&Text, &TextColor)>()
            .iter(app.world())
            .any(|(text, color)| text.0 == "Alice" && color.0 == Color::srgb_u8(248, 96, 96))
    );
    publish(
        &outbox,
        r#"{"entries":[{"entry":73}],"groups":[]}"#,
        6,
        VipPresenceStatus::Online,
    );
    app.update();
    assert_eq!(app.world_mut().query::<&Row>().iter(app.world()).count(), 0);
    app.world_mut().trigger(Activate { entity: row });
    assert!(app.world().resource::<State>().draft.is_none());
    Ok(())
}
#[test]
fn vip_stale_generation_changed_revision_pending_and_foreground_modal_fence_actions() -> TestResult
{
    let (mut app, mut commands, outbox) = super::super::tests::app()?;
    publish(&outbox, CONTACTS, 4, VipPresenceStatus::Online);
    app.update();
    let add = control(&mut app, |action| matches!(action, Action::Add))?;
    app.world_mut()
        .get_mut::<Control>(add)
        .ok_or("control")?
        .generation = 999;
    app.world_mut().trigger(Activate { entity: add });
    assert!(app.world().resource::<State>().draft.is_none());
    app.update();
    click(&mut app, |action| matches!(action, Action::Add))?;
    set_text(&mut app, "Charlie")?;
    let save = control(&mut app, |action| matches!(action, Action::Save(_)))?;
    publish(&outbox, CONTACTS, 5, VipPresenceStatus::Online);
    app.world_mut().trigger(Activate { entity: save });
    app.update();
    assert!(commands.try_recv().is_err());
    click(&mut app, |action| matches!(action, Action::Add))?;
    set_text(&mut app, "Charlie")?;
    let modal = app
        .world_mut()
        .spawn((
            super::super::super::GameModal,
            GlobalZIndex(200),
            Node::default(),
        ))
        .id();
    click(&mut app, |action| matches!(action, Action::Save(_)))?;
    assert!(commands.try_recv().is_err());
    app.world_mut().entity_mut(modal).despawn();
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .vip_request(Intent::Query(0));
    assert_eq!(request(&mut commands)?.operation, wire::Operation::Query);
    click(&mut app, |action| matches!(action, Action::Save(_)))?;
    assert!(commands.try_recv().is_err());
    Ok(())
}

type NativeFixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    crate::play::Outbox,
);
pub(super) fn native_app() -> Result<NativeFixture, Box<dyn std::error::Error>> {
    let (mut app, commands, outbox) = super::super::tests::app()?;
    app.add_plugins((
        bevy::input::InputPlugin,
        bevy::input_focus::InputDispatchPlugin,
        bevy::window::WindowPlugin::default(),
        bevy::text::TextPlugin,
        bevy::ui_widgets::TextInputPlugin,
    ))
    .init_resource::<bevy::ui::UiScale>()
    .init_resource::<bevy::picking::events::PointerState>()
    .add_message::<bevy::picking::events::PointerRelease>()
    .add_systems(
        PostUpdate,
        bevy::ui::widget::update_editable_text_styles.before(bevy::text::EditableTextSystems),
    );
    let font = app
        .world_mut()
        .resource_mut::<Assets<Font>>()
        .add(Font::from_bytes(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/assets/FiraMono-subset.ttf"
            ))
            .to_vec(),
        ));
    app.world_mut().resource_mut::<Skins>().font = font;
    publish(&outbox, CONTACTS, 4, VipPresenceStatus::Online);
    app.update();
    Ok((app, commands, outbox))
}
#[test]
fn vip_save_waits_for_the_actual_upstream_async_clipboard_value() -> TestResult {
    use std::sync::{Arc, Mutex};
    let (mut app, mut commands, _outbox) = native_app()?;
    click(&mut app, |action| matches!(action, Action::Add))?;
    let field = field(&mut app)?;
    let pending = Arc::new(Mutex::new(None));
    app.world_mut()
        .get_mut::<EditableText>(field)
        .ok_or("editor")?
        .pending_paste = Some(bevy::clipboard::ClipboardRead::Pending(Arc::clone(
        &pending,
    )));
    click(&mut app, |action| matches!(action, Action::Save(_)))?;
    assert!(
        commands.try_recv().is_err(),
        "unresolved clipboard must not submit the previous value"
    );
    *pending.lock().map_err(|_| "clipboard fixture lock")? = Some(Ok("Charlie".into()));
    app.update();
    let add = request(&mut commands)?;
    assert_eq!(add.operation, wire::Operation::Add);
    assert_eq!(add.text, "Charlie");
    app.update();
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn vip_cancelled_clipboard_submission_cannot_reach_a_replacement_editor() -> TestResult {
    use std::sync::{Arc, Mutex};
    let (mut app, mut commands, _outbox) = native_app()?;
    click(&mut app, |action| matches!(action, Action::Add))?;
    let field = field(&mut app)?;
    let pending = Arc::new(Mutex::new(None));
    app.world_mut()
        .get_mut::<EditableText>(field)
        .ok_or("editor")?
        .pending_paste = Some(bevy::clipboard::ClipboardRead::Pending(Arc::clone(
        &pending,
    )));
    click(&mut app, |action| matches!(action, Action::Save(_)))?;
    click(&mut app, |action| matches!(action, Action::Cancel(_)))?;
    click(&mut app, |action| matches!(action, Action::Contact(73)))?;
    click(&mut app, |action| matches!(action, Action::Edit(73)))?;
    *pending.lock().map_err(|_| "clipboard fixture lock")? = Some(Ok("Late value".into()));
    app.update();
    assert!(commands.try_recv().is_err());
    assert_eq!(
        app.world()
            .resource::<State>()
            .draft
            .as_ref()
            .ok_or("draft")?
            .text,
        "Friend"
    );
    Ok(())
}
#[test]
fn vip_native_checkbox_uses_typed_value_change_and_missing_edit_metadata_is_not_defaulted()
-> TestResult {
    let (mut app, mut commands, outbox) = super::super::tests::app()?;
    publish(&outbox, CONTACTS, 4, VipPresenceStatus::Online);
    app.update();
    click(&mut app, |action| matches!(action, Action::Contact(73)))?;
    click(&mut app, |action| matches!(action, Action::Edit(73)))?;
    let notify = control(&mut app, |action| matches!(action, Action::Notify(_)))?;
    assert!(
        app.world()
            .entity(notify)
            .contains::<bevy::ui_widgets::Checkbox>()
    );
    app.world_mut().trigger(bevy::ui_widgets::ValueChange {
        source: notify,
        value: false,
        is_final: true,
    });
    let group = control(&mut app, |action| matches!(action, Action::Group(_, 1)))?;
    app.world_mut().trigger(bevy::ui_widgets::ValueChange {
        source: group,
        value: false,
        is_final: true,
    });
    click(&mut app, |action| matches!(action, Action::Save(_)))?;
    let edit = request(&mut commands)?;
    assert!(!edit.notify);
    assert!(edit.groups.is_empty());
    publish(
        &outbox,
        r#"{"entries":[{"entry":73,"name":"Alice"}],"groups":[]}"#,
        5,
        VipPresenceStatus::Unconfirmed,
    );
    app.update();
    click(&mut app, |action| matches!(action, Action::Contact(73)))?;
    click(&mut app, |action| matches!(action, Action::Edit(73)))?;
    assert!(control(&mut app, |action| matches!(action, Action::Notify(_))).is_err());
    click(&mut app, |action| matches!(action, Action::Save(_)))?;
    assert!(commands.try_recv().is_err());
    click(&mut app, |action| matches!(action, Action::Remove(73)))?;
    assert_eq!(request(&mut commands)?.number, 73);
    Ok(())
}

#[test]
fn vip_default_hidden_sidebar_is_reachable_through_real_host_ctrl_v_and_region_projection()
-> TestResult {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let (mut app, mut commands, outbox) = super::super::tests::app()?;
    app.add_systems(
        Update,
        crate::bevy_shell::game_ui::projection::project_regions,
    );
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .use_shared_frontend();
    publish(&outbox, CONTACTS, 4, VipPresenceStatus::Online);
    app.update();
    let vip = app
        .world()
        .resource::<crate::bevy_shell::game_ui::GameRegions>()
        .vip;
    assert_eq!(
        app.world().get::<Node>(vip).ok_or("VIP region")?.display,
        Display::None
    );
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .vip_is_open()
    );
    app.world_mut().non_send_mut::<HostOwner>().route_input(
        &[NormalizedInputEvent::Key {
            code: KeyCode::new(25)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(Modifier::Control),
            repeat: false,
        }],
        false,
    );
    assert_eq!(request(&mut commands)?.operation, wire::Operation::Query);
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .vip_is_open()
    );
    publish(&outbox, CONTACTS, 5, VipPresenceStatus::Online);
    app.update();
    assert_eq!(
        app.world().get::<Node>(vip).ok_or("VIP region")?.display,
        Display::Flex
    );
    assert_eq!(app.world_mut().query::<&Row>().iter(app.world()).count(), 2);
    click(&mut app, |action| matches!(action, Action::Add))?;
    assert!(app.world().resource::<State>().draft.is_some());
    Ok(())
}

#[test]
fn vip_close_hides_a_saved_visible_sidebar_and_preserves_a_pending_canonical_query() -> TestResult {
    let (mut app, mut commands, outbox) = super::super::tests::app()?;
    app.add_systems(
        Update,
        crate::bevy_shell::game_ui::projection::project_regions,
    );
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .settings
        .preferences
        .visible_panels
        .push("vip".into());
    publish(&outbox, CONTACTS, 4, VipPresenceStatus::Online);
    app.update();
    let region = app
        .world()
        .resource::<crate::bevy_shell::game_ui::GameRegions>()
        .vip;
    assert_eq!(
        app.world().get::<Node>(region).ok_or("VIP region")?.display,
        Display::Flex
    );
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .vip_request(Intent::Query(0));
    assert_eq!(request(&mut commands)?.operation, wire::Operation::Query);
    click(&mut app, |action| matches!(action, Action::Close))?;
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .settings
            .preferences
            .panel_visible("vip")
    );
    assert_eq!(
        app.world().get::<Node>(region).ok_or("VIP region")?.display,
        Display::None
    );
    let owner = app.world().non_send::<HostOwner>();
    let link = &owner.play.as_ref().ok_or("play")?.link;
    assert!(!link.vip_is_open());
    assert!(link.vip_panel().pending);
    assert!(
        link.vip_panel()
            .view
            .as_ref()
            .is_some_and(|raw| raw == CONTACTS)
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn vip_message_control_opens_existing_private_conversation() -> TestResult {
    let (mut app, _commands, outbox) = super::super::tests::app()?;
    publish(&outbox, CONTACTS, 4, VipPresenceStatus::Online);
    app.update();
    click(&mut app, |action| matches!(action, Action::Contact(73)))?;
    let message = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(entity, name)| (name.as_str() == "vip.message").then_some(entity))
        .ok_or("VIP Message action")?;
    app.world_mut().trigger(Activate { entity: message });
    let panel = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .chat_panel();
    assert_eq!(
        panel.console.current,
        Some(oteryn_client_ui::chat::ChannelTab::Conversation(
            "Alice".into()
        ))
    );
    assert!(app.world().resource::<State>().draft.is_none());
    Ok(())
}

#[test]
fn vip_projects_saved_icon_description_and_membership_section() -> TestResult {
    let (mut app, _commands, outbox) = super::super::tests::app()?;
    publish(&outbox, CONTACTS, 4, VipPresenceStatus::Online);
    app.update();
    let names: Vec<_> = app
        .world_mut()
        .query::<&Name>()
        .iter(app.world())
        .map(|name| name.as_str().to_owned())
        .collect();
    assert!(names.iter().any(|name| name == "vip.contacts.group.1"));
    assert!(names.iter().any(|name| name == "vip.icon.2"));
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "Friend")
    );
    Ok(())
}

#[test]
fn vip_offline_filter_control_hides_confirmed_offline_contacts() -> TestResult {
    let (mut app, _commands, outbox) = super::super::tests::app()?;
    publish(&outbox, CONTACTS, 4, VipPresenceStatus::Offline);
    app.update();
    let filter = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(entity, name)| (name.as_str() == "vip.hide_offline").then_some(entity))
        .ok_or("offline filter")?;
    app.world_mut().trigger(Activate { entity: filter });
    app.update();
    assert!(
        !app.world_mut()
            .query::<&Row>()
            .iter(app.world())
            .any(|row| row.entry == 73)
    );
    assert!(
        app.world_mut()
            .query::<&Row>()
            .iter(app.world())
            .any(|row| row.entry == 91)
    );
    click(&mut app, |action| matches!(action, Action::Sort(1)))?;
    let preferences = &app.world().non_send::<HostOwner>().settings.preferences;
    let saved: crate::play::ReferencePreferences =
        serde_json::from_str(&serde_json::to_string(preferences)?)?;
    assert_eq!(saved.vip_sort, 1);
    assert!(saved.vip_hide_offline);
    Ok(())
}

#[test]
fn vip_sorting_uses_name_type_and_confirmed_status() {
    let alice = Contact {
        entry: 73,
        name: "Alice".into(),
        description: None,
        icon: Some(2),
        notify: None,
        groups: None,
    };
    let bob = Contact {
        entry: 91,
        name: "Bob".into(),
        icon: Some(0),
        ..alice.clone()
    };
    let presence = [VipPresenceEntry {
        entry: 73,
        name: "Alice".into(),
        status: VipPresenceStatus::Online,
        announce: false,
        confirmed_for_ms: 1000,
    }];
    assert!(
        view::contact_sort_key(&alice, 0, &presence) < view::contact_sort_key(&bob, 0, &presence)
    );
    assert!(
        view::contact_sort_key(&alice, 1, &presence) > view::contact_sort_key(&bob, 1, &presence)
    );
    assert!(
        view::contact_sort_key(&alice, 2, &presence) < view::contact_sort_key(&bob, 2, &presence)
    );
}

#[test]
fn vip_double_click_opens_conversation_after_first_activation() -> TestResult {
    use bevy::picking::{
        backend::HitData,
        events::{Pointer, PointerClick},
        pointer::{Location, PointerButton, PointerId},
    };
    let (mut app, _commands, outbox) = super::super::tests::app()?;
    publish(&outbox, CONTACTS, 4, VipPresenceStatus::Online);
    app.update();
    let row = control(&mut app, |action| matches!(action, Action::Contact(73)))?;
    app.world_mut().trigger(Activate { entity: row });
    app.update();
    app.world_mut().trigger(PointerClick {
        entity: row,
        pointer: Pointer::new(
            PointerId::Mouse,
            Location {
                target: bevy::camera::NormalizedRenderTarget::None {
                    width: 100,
                    height: 100,
                },
                position: Vec2::ZERO,
            },
        ),
        button: PointerButton::Primary,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
        duration: std::time::Duration::ZERO,
        count: 2,
    });
    let panel = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .chat_panel();
    assert_eq!(
        panel.console.current,
        Some(oteryn_client_ui::chat::ChannelTab::Conversation(
            "Alice".into()
        ))
    );
    assert!(app.world().resource::<State>().draft.is_none());
    Ok(())
}
