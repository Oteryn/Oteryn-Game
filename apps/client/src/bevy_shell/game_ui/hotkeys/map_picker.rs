//! Modal map object selection in the current preference draft.
use super::super::{GameModal, viewport::WorldViewport};
use super::*;
use bevy::picking::{events::PointerClick, pointer::PointerButton};
#[derive(Component)]
pub(super) struct Overlay;
pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    editors: Query<(Entity, &Editor)>,
    parents: Query<&ChildOf>,
    overlays: Query<Entity, With<Overlay>>,
    mut dialogs: Query<&mut Visibility, With<GameModal>>,
    mut hidden: Local<Option<Entity>>,
) {
    let editor = editors.iter().find_map(|(entity, editor)| {
        (owner.preferences_draft().is_some()
            && editor.map_pick
            && editor.generation == owner.generation())
        .then_some(entity)
    });
    if let Some(editor) = editor
        && overlays.is_empty()
    {
        *hidden = parents
            .iter_ancestors(editor)
            .find(|entity| dialogs.contains(*entity));
        if let Some(entity) = *hidden
            && let Ok(mut dialog) = dialogs.get_mut(entity)
        {
            *dialog = Visibility::Hidden;
        }
        commands
            .spawn((
                Overlay,
                GameModal,
                GlobalZIndex(199),
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100.),
                    height: percent(100.),
                    ..default()
                },
            ))
            .observe(pick);
    } else if editor.is_none() && !overlays.is_empty() {
        for overlay in &overlays {
            commands.entity(overlay).despawn();
        }
        if let Some(entity) = hidden.take()
            && let Ok(mut dialog) = dialogs.get_mut(entity)
        {
            *dialog = Visibility::Inherited;
        }
    }
}
fn pick(
    event: On<PointerClick>,
    owner: NonSend<HostOwner>,
    guard: super::super::modal::ModalGuard,
    viewport: Res<WorldViewport>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut editors: Query<&mut Editor>,
    mut fields: Query<(&Field, &mut EditableText)>,
) {
    let Ok(mut editor) = editors.single_mut() else {
        return;
    };
    if !editor.map_pick
        || editor.generation != owner.generation()
        || owner.preferences_draft().is_none()
        || !guard.allows(event.entity)
    {
        return;
    }
    editor.map_pick = false;
    if event.button != PointerButton::Primary {
        return;
    }
    let Some(play) = &owner.play else { return };
    let (w, h) = play.view.scene().view().viewport_px();
    let scale = windows.single().map_or(1., |window| window.scale_factor());
    let item = viewport
        .scene_point(event.pointer.position * scale, UVec2::new(w, h))
        .and_then(|point| play.view.click_tile(f64::from(point.x), f64::from(point.y)))
        .and_then(|tile| play.view.shortcut_object_at(tile));
    let Some((definition, subtype)) = item else {
        editor.feedback = "No object at that position".into();
        return;
    };
    for (field, mut text) in &mut fields {
        let value = match field.0 {
            2 => definition,
            3 => subtype,
            _ => continue,
        };
        text.pending_edits.clear();
        text.pending_paste = None;
        text.editor_mut().set_text(&value.to_string());
    }
    if !(1..=5).contains(&editor.mode) {
        editor.mode = 4;
    }
    editor.feedback = "Object selected; Add / Save to assign".into();
}
