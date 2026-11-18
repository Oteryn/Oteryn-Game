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
                channel: None,
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
    // List actions sit in the list's right-click menu: open it the way a player does.
    let in_menu = app
        .world()
        .get::<ChildOf>(entity)
        .is_some_and(|parent| app.world().get::<view::ListMenu>(parent.parent()).is_some());
    if in_menu {
        let list = app
            .world_mut()
            .query_filtered::<Entity, With<List>>()
            .single(app.world())?;
        app.world_mut().trigger(secondary_click(list));
        app.update();
    }
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
pub(super) fn secondary_click(entity: Entity) -> bevy::picking::events::PointerClick {
    bevy::picking::events::PointerClick {
        entity,
        pointer: bevy::picking::events::Pointer::new(
            bevy::picking::pointer::PointerId::Mouse,
            bevy::picking::pointer::Location {
                target: bevy::camera::NormalizedRenderTarget::None {
                    width: 100,
                    height: 100,
                },
                position: Vec2::ZERO,
            },
        ),
        button: bevy::picking::pointer::PointerButton::Secondary,
        hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0., None, None),
        duration: std::time::Duration::ZERO,
        count: 1,
    }
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
    let row = control(&mut app, |action| matches!(action, Action::Edit(73)))?;
    publish(&outbox, CONTACTS, 5, VipPresenceStatus::Offline);
    app.update();
    assert_eq!(
        control(&mut app, |action| matches!(action, Action::Edit(73)))?,
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
    click(&mut app, |action| matches!(action, Action::Edit(73)))?;
    assert!(control(&mut app, |action| matches!(action, Action::Notify(_))).is_err());
    click(&mut app, |action| matches!(action, Action::Save(_)))?;
    assert!(commands.try_recv().is_err());
    click(&mut app, |action| matches!(action, Action::Remove(73)))?;
    assert_eq!(request(&mut commands)?.number, 73);
    Ok(())
}

include!("presentation_cases.rs");
