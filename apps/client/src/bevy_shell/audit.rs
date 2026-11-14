//! Opt-in, bounded public UI observation for OS-driven acceptance checks.
use super::{
    login::{LoginControl, LoginField},
    owner::{GraphicsDraft, HostOwner, Phase},
    selection::PickerControl,
    ui::Control,
};
use bevy::{prelude::*, window::PrimaryWindow};
use serde::Serialize;
use std::path::PathBuf;

mod named;

/// In the explicit UI audit mode, F12 captures the rendered window through Bevy.
/// GPU capture also works on native test displays without a desktop compositor.
pub(super) fn capture_on_key(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    output: Res<AuditOutput>,
) {
    if keys.just_pressed(KeyCode::F12)
        && let Some(path) = &output.path
    {
        use bevy::render::view::screenshot::{Screenshot, save_to_disk};
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.with_extension("png")));
    }
}
#[derive(Resource)]
pub(super) struct AuditOutput {
    path: Option<PathBuf>,
    last: Vec<u8>,
    error_reported: bool,
    update: u64,
}
impl AuditOutput {
    pub fn new(path: Option<PathBuf>) -> Self {
        Self {
            path,
            last: Vec::new(),
            error_reported: false,
            update: 0,
        }
    }
}
#[derive(Serialize)]
struct Bounds {
    id: String,
    bounds_physical: [f32; 4],
    bounds_logical: [f32; 4],
    enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    text_metrics: Option<TextMetrics>,
}
#[derive(Serialize)]
struct TextMetrics {
    focused: bool,
    characters: usize,
    glyphs: usize,
    visible: bool,
}
#[derive(Serialize)]
struct PlayObservation {
    generation: u64,
    map_center: (i32, i32, i16),
    last_step: Option<crate::play::StepReceipt>,
}
#[derive(Serialize)]
struct Snapshot<'a> {
    schema: &'static str,
    update: u64,
    lifecycle: Phase,
    generation: u64,
    login_visible: bool,
    login_pending: bool,
    login_feedback: &'a str,
    selection_pending: bool,
    character_scoped: bool,
    play_observation: Option<PlayObservation>,
    accepted: GraphicsDraft,
    draft: Option<GraphicsDraft>,
    feedback: &'a str,
    status: &'a str,
    window_mode: String,
    present_mode: String,
    window_physical: [u32; 2],
    scale_factor: f32,
    controls: Vec<Bounds>,
}
// ECS component queries expose bounds only, never connection-field contents.
#[allow(clippy::type_complexity)]
pub(super) fn write(
    owner: NonSend<HostOwner>,
    mut output: ResMut<AuditOutput>,
    window: Query<&Window, With<PrimaryWindow>>,
    controls: Query<(
        &Control,
        &ComputedNode,
        &UiGlobalTransform,
        Has<bevy::ui::InteractionDisabled>,
    )>,
    login_controls: Query<
        (
            Entity,
            Option<&LoginField>,
            Option<&LoginControl>,
            &ComputedNode,
            &UiGlobalTransform,
            Has<bevy::ui::InteractionDisabled>,
            Option<&bevy::text::EditableText>,
            Option<&bevy::text::TextLayoutInfo>,
            Option<&InheritedVisibility>,
        ),
        Or<(With<LoginField>, With<LoginControl>)>,
    >,
    picker_controls: Query<(
        &PickerControl,
        &ComputedNode,
        &UiGlobalTransform,
        Has<bevy::ui::InteractionDisabled>,
    )>,
    named: named::NamedControls,
) {
    // Native drivers wait for picking/layout updates. Disabled without --ui-audit.
    if output.path.is_some() {
        output.update = output.update.saturating_add(1);
    }
    let Some(path) = &output.path else {
        return;
    };
    let Ok(window) = window.single() else {
        return;
    };
    let mut bounds = named.bounds(window.scale_factor());
    for (control, node, transform, disabled) in &controls {
        let dialog_control = matches!(
            control,
            Control::Fullscreen
                | Control::Vsync
                | Control::IntegerScaling
                | Control::Apply
                | Control::Ok
                | Control::Cancel
        );
        if dialog_control && named.full_options() {
            continue;
        }
        if dialog_control != owner.draft().is_some() || node.size().min_element() <= 0. {
            continue;
        }
        let min = transform.translation - node.size() / 2.;
        let physical = [min.x, min.y, node.size().x, node.size().y];
        bounds.push(Bounds {
            id: control.id().into(),
            bounds_physical: physical,
            bounds_logical: physical.map(|v| v / window.scale_factor()),
            enabled: !disabled,
            text_metrics: None,
        });
    }
    if owner.login.visible && owner.selection.pending().is_none() {
        for (entity, field, control, node, transform, disabled, editable, layout, visibility) in
            &login_controls
        {
            let id = field
                .map(LoginField::id)
                .or_else(|| control.map(|control| control.id()));
            if let Some(id) = id
                && node.size().min_element() > 0.
            {
                let min = transform.translation - node.size() / 2.;
                let physical = [min.x, min.y, node.size().x, node.size().y];
                bounds.push(Bounds {
                    id: id.into(),
                    bounds_physical: physical,
                    bounds_logical: physical.map(|v| v / window.scale_factor()),
                    enabled: !disabled,
                    // Public connection fields only: expose counts, never field contents.
                    text_metrics: editable.map(|text| TextMetrics {
                        focused: named.focus() == Some(entity),
                        characters: text.value().chars().count(),
                        glyphs: layout.map_or(0, |layout| layout.glyphs.len()),
                        visible: visibility.is_some_and(|visibility| visibility.get()),
                    }),
                });
            }
        }
    }
    if let Some(request) = owner.selection.pending() {
        for (control, node, transform, disabled) in &picker_controls {
            if control.request_id == request.id && node.size().min_element() > 0. {
                let min = transform.translation - node.size() / 2.;
                let physical = [min.x, min.y, node.size().x, node.size().y];
                bounds.push(Bounds {
                    id: control.id.clone(),
                    bounds_physical: physical,
                    bounds_logical: physical.map(|v| v / window.scale_factor()),
                    enabled: !disabled,
                    text_metrics: None,
                });
            }
        }
    }
    // Stable sorting keeps source-named controls ahead of any legacy duplicate.
    bounds.sort_by(|a, b| a.id.cmp(&b.id));
    bounds.dedup_by(|a, b| a.id == b.id);
    let snapshot = Snapshot {
        schema: "oteryn.bevy-ui-audit.v1",
        update: output.update,
        lifecycle: owner.phase,
        generation: owner.generation(),
        login_visible: owner.login.visible,
        login_pending: owner.login.form.pending(),
        login_feedback: owner.login.form.message(),
        selection_pending: owner.selection.pending().is_some(),
        character_scoped: owner.settings.character_scope().is_some(),
        play_observation: owner.play.as_ref().map(|play| PlayObservation {
            generation: owner.generation(),
            map_center: play.view.map_center(),
            last_step: play.view.step_receipt(),
        }),
        accepted: owner.accepted(),
        draft: owner.draft(),
        feedback: &owner.settings.feedback,
        status: &owner.status,
        window_mode: format!("{:?}", window.mode),
        present_mode: format!("{:?}", window.present_mode),
        window_physical: [window.physical_width(), window.physical_height()],
        scale_factor: window.scale_factor(),
        controls: bounds,
    };
    let Ok(bytes) = serde_json::to_vec_pretty(&snapshot) else {
        return;
    };
    if bytes == output.last {
        return;
    }
    let result = (|| -> std::io::Result<()> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension("audit-next");
        std::fs::write(&temporary, &bytes)?;
        std::fs::rename(temporary, path)
    })();
    match result {
        Ok(()) => {
            output.last = bytes;
            output.error_reported = false;
        }
        Err(error) if !output.error_reported => {
            eprintln!("UI audit could not be written: {error}");
            output.error_reported = true;
        }
        Err(_) => {}
    }
}

#[cfg(test)]
mod tests;
