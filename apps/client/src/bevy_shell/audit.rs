//! Opt-in, bounded public UI observation for OS-driven acceptance checks.
use super::{
    owner::{GraphicsDraft, HostOwner, Phase},
    ui::Control,
};
use bevy::{prelude::*, window::PrimaryWindow};
use serde::Serialize;
use std::path::PathBuf;
#[derive(Resource)]
pub(super) struct AuditOutput {
    path: Option<PathBuf>,
    last: Vec<u8>,
    error_reported: bool,
}
impl AuditOutput {
    pub fn new(path: Option<PathBuf>) -> Self {
        Self {
            path,
            last: Vec::new(),
            error_reported: false,
        }
    }
}
#[derive(Serialize)]
struct Bounds {
    id: &'static str,
    bounds_physical: [f32; 4],
    bounds_logical: [f32; 4],
    enabled: bool,
}
#[derive(Serialize)]
struct Snapshot<'a> {
    schema: &'static str,
    lifecycle: Phase,
    generation: u64,
    character_scoped: bool,
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
) {
    let Some(path) = &output.path else {
        return;
    };
    let Ok(window) = window.single() else {
        return;
    };
    let mut bounds = Vec::new();
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
        if dialog_control != owner.draft().is_some() || node.size().min_element() <= 0. {
            continue;
        }
        let min = transform.translation - node.size() / 2.;
        let physical = [min.x, min.y, node.size().x, node.size().y];
        bounds.push(Bounds {
            id: control.id(),
            bounds_physical: physical,
            bounds_logical: physical.map(|v| v / window.scale_factor()),
            enabled: !disabled,
        });
    }
    bounds.sort_by_key(|b| b.id);
    let snapshot = Snapshot {
        schema: "oteryn.bevy-ui-audit.v1",
        lifecycle: owner.phase,
        generation: owner.generation(),
        character_scoped: owner.settings.character_scope().is_some(),
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
