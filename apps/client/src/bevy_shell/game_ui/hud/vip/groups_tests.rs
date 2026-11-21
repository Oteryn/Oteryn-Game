use super::tests::{CONTACTS, click, control, field, publish, request, set_text};
use super::*;
use oteryn_client_runtime::vip::vip as wire;
use oteryn_session::VipPresenceStatus;
type TestResult = Result<(), Box<dyn std::error::Error>>;
#[test]
fn vip_groups_add_rename_delete_use_the_existing_canonical_actions_without_optimistic_readback()
-> TestResult {
    let (mut app, mut commands, outbox) = super::super::tests::app()?;
    publish(&outbox, CONTACTS, 4, VipPresenceStatus::Online);
    app.update();
    let original = control(&mut app, |action| matches!(action, Action::GroupRename(1)))?;
    click(&mut app, |action| matches!(action, Action::GroupAdd))?;
    set_text(&mut app, "Hunters")?;
    click(&mut app, |action| matches!(action, Action::Save(_)))?;
    let add = request(&mut commands)?;
    assert_eq!(add.operation, wire::Operation::GroupAdd);
    assert_eq!(add.text, "Hunters");
    assert!(control(&mut app, |action| matches!(action, Action::GroupRename(2))).is_err());
    assert_eq!(
        control(&mut app, |action| matches!(action, Action::GroupRename(1)))?,
        original
    );
    let added = CONTACTS.replace(
        r#""groups":[{"group":1,"name":"Friends"}]"#,
        r#""groups":[{"group":1,"name":"Friends"},{"group":2,"name":"Hunters"}]"#,
    );
    publish(&outbox, &added, 5, VipPresenceStatus::Online);
    app.update();
    assert!(control(&mut app, |action| matches!(action, Action::GroupRename(2))).is_ok());
    assert_eq!(
        control(&mut app, |action| matches!(action, Action::GroupRename(1)))?,
        original
    );
    click(&mut app, |action| matches!(action, Action::GroupRename(1)))?;
    set_text(&mut app, "Hunting friends")?;
    click(&mut app, |action| matches!(action, Action::Save(_)))?;
    let rename = request(&mut commands)?;
    assert_eq!(rename.operation, wire::Operation::GroupRename);
    assert_eq!(rename.number, 1);
    assert_eq!(rename.text, "Hunting friends");
    let renamed = added.replace("Friends", "Hunting friends");
    publish(&outbox, &renamed, 6, VipPresenceStatus::Online);
    app.update();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "Hunting friends")
    );
    click(&mut app, |action| matches!(action, Action::GroupRename(1)))?;
    click(&mut app, |action| matches!(action, Action::GroupRemove(1)))?;
    let delete = request(&mut commands)?;
    assert_eq!(delete.operation, wire::Operation::GroupRemove);
    assert_eq!(delete.number, 1);
    let owner = app.world().non_send::<HostOwner>();
    let panel = owner.play.as_ref().ok_or("play")?.link.vip_panel();
    let snapshot = Snapshot::read(&panel)?;
    assert_eq!(snapshot.groups[0].name, "Hunting friends");
    assert_eq!(snapshot.entries[0].groups.as_deref(), Some(&[1][..]));
    let deleted = renamed
        .replace(r#""groups":[1]"#, r#""groups":[]"#)
        .replace(r#"{"group":1,"name":"Hunting friends"},"#, "");
    publish(&outbox, &deleted, 7, VipPresenceStatus::Online);
    app.update();
    assert!(control(&mut app, |action| matches!(action, Action::GroupRename(1))).is_err());
    assert!(control(&mut app, |action| matches!(action, Action::GroupRename(2))).is_ok());
    let readback = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .vip_panel();
    assert!(
        Snapshot::read(&readback)?.entries[0]
            .groups
            .as_ref()
            .is_some_and(Vec::is_empty)
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn vip_group_refusal_preserves_contacts_memberships_catalog_and_preferences() -> TestResult {
    let (mut app, mut commands, outbox) = super::super::tests::app()?;
    publish(&outbox, CONTACTS, 4, VipPresenceStatus::Online);
    app.update();
    let preferences = app
        .world()
        .non_send::<HostOwner>()
        .settings
        .preferences
        .clone();
    click(&mut app, |action| matches!(action, Action::GroupRename(1)))?;
    set_text(&mut app, "Duplicate")?;
    click(&mut app, |action| matches!(action, Action::Save(_)))?;
    assert_eq!(
        request(&mut commands)?.operation,
        wire::Operation::GroupRename
    );
    outbox.bevy_vip_refusal_fixture(6);
    app.update();
    let owner = app.world().non_send::<HostOwner>();
    let panel = owner.play.as_ref().ok_or("play")?.link.vip_panel();
    assert!(!panel.pending);
    assert_eq!(panel.view.as_deref(), Some(CONTACTS));
    assert_eq!(owner.settings.preferences, preferences);
    assert_eq!(
        Snapshot::read(&panel)?.entries[0].groups.as_deref(),
        Some(&[1][..])
    );
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "That group name is taken")
    );
    Ok(())
}
#[test]
fn vip_group_editor_rejects_changed_revision_stale_generation_pending_modal_and_unknown_catalog()
-> TestResult {
    let (mut app, mut commands, outbox) = super::super::tests::app()?;
    publish(&outbox, CONTACTS, 4, VipPresenceStatus::Online);
    app.update();
    let add = control(&mut app, |action| matches!(action, Action::GroupAdd))?;
    app.world_mut()
        .get_mut::<Control>(add)
        .ok_or("add")?
        .generation = 999;
    app.world_mut().trigger(Activate { entity: add });
    assert!(app.world().resource::<State>().draft.is_none());
    app.update();
    click(&mut app, |action| matches!(action, Action::GroupRename(1)))?;
    let save = control(&mut app, |action| matches!(action, Action::Save(_)))?;
    publish(&outbox, CONTACTS, 5, VipPresenceStatus::Online);
    app.world_mut().trigger(Activate { entity: save });
    app.update();
    assert!(commands.try_recv().is_err());
    click(&mut app, |action| matches!(action, Action::GroupAdd))?;
    set_text(&mut app, "Hunters")?;
    let modal = app
        .world_mut()
        .spawn((
            super::super::super::GameModal,
            Node::default(),
            GlobalZIndex(200),
        ))
        .id();
    click(&mut app, |action| matches!(action, Action::Save(_)))?;
    assert!(commands.try_recv().is_err());
    app.world_mut().entity_mut(modal).despawn();
    click(&mut app, |action| matches!(action, Action::Cancel(_)))?;
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .vip_request(Intent::Query(0));
    assert_eq!(request(&mut commands)?.operation, wire::Operation::Query);
    click(&mut app, |action| matches!(action, Action::GroupAdd))?;
    assert!(app.world().resource::<State>().draft.is_none());
    publish(
        &outbox,
        r#"{"entries":[],"groups":[{"group":99,"name":"Unknown"}]}"#,
        6,
        VipPresenceStatus::Online,
    );
    app.update();
    click(&mut app, |action| matches!(action, Action::GroupAdd))?;
    assert!(app.world().resource::<State>().draft.is_none());
    assert!(control(&mut app, |action| matches!(action, Action::GroupRename(99))).is_err());
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn vip_group_editor_waits_for_real_clipboard_and_cancel_discards_a_late_value() -> TestResult {
    use std::sync::{Arc, Mutex};
    let (mut app, mut commands, outbox) = super::tests::native_app()?;
    click(&mut app, |action| matches!(action, Action::GroupAdd))?;
    let editor = field(&mut app)?;
    assert_eq!(
        app.world()
            .get::<EditableText>(editor)
            .ok_or("editor")?
            .max_characters,
        Some(25)
    );
    let pending = Arc::new(Mutex::new(None));
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .ok_or("editor")?
        .pending_paste = Some(bevy::clipboard::ClipboardRead::Pending(Arc::clone(
        &pending,
    )));
    click(&mut app, |action| matches!(action, Action::Save(_)))?;
    assert!(commands.try_recv().is_err());
    *pending.lock().map_err(|_| "clipboard lock")? = Some(Ok("Hunters".into()));
    app.update();
    let add = request(&mut commands)?;
    assert_eq!(add.operation, wire::Operation::GroupAdd);
    assert_eq!(add.text, "Hunters");
    publish(&outbox, CONTACTS, 5, VipPresenceStatus::Online);
    app.update();
    click(&mut app, |action| matches!(action, Action::GroupRename(1)))?;
    let editor = field(&mut app)?;
    let late = Arc::new(Mutex::new(None));
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .ok_or("editor")?
        .pending_paste = Some(bevy::clipboard::ClipboardRead::Pending(Arc::clone(&late)));
    click(&mut app, |action| matches!(action, Action::Save(_)))?;
    click(&mut app, |action| matches!(action, Action::Cancel(_)))?;
    *late.lock().map_err(|_| "clipboard lock")? = Some(Ok("Late group".into()));
    app.update();
    assert!(commands.try_recv().is_err());
    Ok(())
}
