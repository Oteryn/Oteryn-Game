use super::*;
use crate::play::{PlayView, play_channel};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
#[test]
fn minimap_markers_owner_preserves_persistence_and_close_retry_state() -> Result {
    let (link, _commands, _) = play_channel();
    link.use_shared_frontend();
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 200), -7, &[])?;
    view.use_shared_frontend();
    let root = std::env::temp_dir().join(format!("minimap-markers-editor-{}", std::process::id()));
    {
        let mut map = lock(&view.minimap);
        map.root = Some(root.clone());
        map.bind_world("11111111-1111-4111-8111-111111111111");
        map.binding = Some(([1; 32], [2; 32], 3));
    }
    assert!(view.minimap_editor_open(&link, true));
    let expected = view.minimap_editor_snapshot().ok_or("snapshot")?;
    let marker = Marker {
        x: 100,
        y: 200,
        floor: -7,
        label: "Persisted".into(),
        color: 2,
    };
    assert!(view.minimap_editor_bound(
        &link,
        &expected,
        editor::Action::Save(marker.clone()),
        true
    ));
    let path = lock(&view.minimap).path().ok_or("path")?;
    assert!(path.is_file());
    // Fail the existing owner's save path, then close while gameplay input is blocked.
    {
        let mut map = lock(&view.minimap);
        map.root = Some(path.clone());
    }
    let before = view.minimap_editor_snapshot().ok_or("before")?;
    let changed = Marker {
        label: "Retry".into(),
        ..marker
    };
    assert!(view.minimap_editor_bound(&link, &before, editor::Action::Save(changed), true));
    let failed = view.minimap_editor_snapshot().ok_or("failed")?;
    assert_ne!(failed.feedback, "Marker saved locally");
    assert!(lock(&view.minimap).dirty);
    crate::play::mailbox::lock(&link.mailbox).death_reset_commands = true;
    assert!(!view.minimap_editor_bound(&link, &failed, editor::Action::ZoomIn, true));
    assert!(view.minimap_editor_bound(&link, &expected, editor::Action::Close, false));
    assert!(lock(&view.minimap).dirty);
    crate::play::mailbox::lock(&link.mailbox).death_reset_commands = false;
    assert!(view.minimap_editor_open(&link, true));
    let reopened = view.minimap_editor_snapshot().ok_or("reopened")?;
    assert_eq!(failed.feedback, reopened.feedback);
    assert_eq!(failed.markers, reopened.markers);
    {
        let mut map = lock(&view.minimap);
        map.root = Some(root.clone());
        map.save()?;
        let mut restored = Minimap::default();
        restored.root = Some(root.clone());
        restored.world = map.world.clone();
        restored.binding = map.binding;
        restored.load()?;
        assert_eq!(restored.markers, map.markers);
    }
    std::fs::remove_dir_all(root)?;
    Ok(())
}
#[test]
fn minimap_markers_owner_rejects_changed_binding_target_and_closed_session() -> Result {
    let (link, commands, _) = play_channel();
    link.use_shared_frontend();
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 200), -7, &[])?;
    view.use_shared_frontend();
    assert!(view.minimap_editor_open(&link, true));
    let expected = view.minimap_editor_snapshot().ok_or("snapshot")?;
    let marker = Marker {
        x: 100,
        y: 200,
        floor: -7,
        label: "Old".into(),
        color: 1,
    };
    assert!(view.minimap_editor_bound(
        &link,
        &expected,
        editor::Action::Save(marker.clone()),
        true
    ));
    let target = view.minimap_editor_snapshot().ok_or("target")?;
    lock(&view.minimap).edit_marker(100, 200, -7, "Changed by owner".into(), 2)?;
    let changed = view.minimap_editor_snapshot().ok_or("changed")?;
    assert!(!view.minimap_editor_bound(&link, &target, editor::Action::Delete(marker), true));
    assert_eq!(view.minimap_editor_snapshot().as_ref(), Some(&changed));
    view.bind_minimap_world("22222222-2222-4222-8222-222222222222");
    let binding = view.minimap_editor_snapshot().ok_or("binding")?;
    assert!(!view.minimap_editor_bound(&link, &changed, editor::Action::ZoomOut, true));
    assert_eq!(view.minimap_editor_snapshot().as_ref(), Some(&binding));
    drop(commands);
    assert!(!view.minimap_editor_bound(&link, &binding, editor::Action::Close, true));
    assert!(!view.minimap_editor_open(&link, true));
    assert_eq!(view.minimap_editor_snapshot().as_ref(), Some(&binding));
    Ok(())
}
