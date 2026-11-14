//! Portable preview of retained support evidence. Opening Help never queues a request.
use super::{
    super::{
        owner::{HostOwner, Phase},
        ui::Skins,
    },
    GameModal,
    modal::ModalGuard,
};
use crate::support_delivery::Evidence;
use bevy::{prelude::*, ui_widgets::Activate};
use oteryn_client_ui_bevy as source;
#[derive(Resource, Default)]
pub(in crate::bevy_shell) struct State {
    open: bool,
    generation: u64,
    overlay: Option<Entity>,
    rows: Vec<Evidence>,
}
#[derive(Component, Clone)]
struct Control {
    generation: u64,
    evidence: Option<Evidence>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_systems(Update, (keyboard, project).chain());
}
fn toggle(world: &mut State, generation: u64) {
    world.open = !world.open || world.generation != generation;
    world.generation = generation;
}
pub(in crate::bevy_shell) fn toggle_from_toolbar(state: &mut State, owner: &HostOwner) {
    if owner.phase == Phase::Playing && !owner.modal_active() {
        toggle(state, owner.generation());
    }
}
fn keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<State>,
    owner: NonSend<HostOwner>,
    modal: ModalGuard,
) {
    if keys.just_pressed(SHORTCUT)
        && (modal.allows_focus(None) || state.overlay.is_some_and(|entity| modal.allows(entity)))
        && owner.phase == Phase::Playing
        && !owner.modal_active()
    {
        toggle(&mut state, owner.generation());
    }
    if keys.just_pressed(KeyCode::Escape)
        && state.overlay.is_some_and(|entity| modal.allows(entity))
    {
        state.open = false;
    }
}
fn project(
    mut commands: Commands,
    skins: Res<Skins>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    let visible =
        state.open && state.generation == owner.generation() && owner.phase == Phase::Playing;
    let rows = if visible {
        owner
            .play
            .as_ref()
            .map(|play| play.link.support_evidence_rows())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    if !visible {
        if let Some(entity) = state.overlay.take() {
            commands.entity(entity).despawn();
        }
        state.open = false;
        state.rows.clear();
        return;
    }
    if state.overlay.is_some() && state.rows == rows {
        return;
    }
    if let Some(entity) = state.overlay.take() {
        commands.entity(entity).despawn();
    }
    let overlay = commands
        .spawn((
            GameModal,
            GlobalZIndex(165),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.45)),
        ))
        .id();
    let panel = commands
        .spawn((
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(700.),
                max_width: percent(95.),
                max_height: percent(95.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(16.)),
                row_gap: px(8.),
                ..default()
            },
        ))
        .id();
    commands.entity(overlay).add_child(panel);
    source::dialog_caption(
        &mut commands,
        panel,
        "Support-evidence delivery",
        skins.font.clone(),
    );
    for topic in [
        crate::play::HelpTopic::Questions,
        crate::play::HelpTopic::Manual,
        crate::play::HelpTopic::Client,
    ] {
        let text = commands
            .spawn(source::text(
                crate::play::help_text(topic),
                source::font(skins.font.clone(), false),
            ))
            .id();
        commands.entity(panel).add_child(text);
    }
    for row in &rows {
        let text = commands
            .spawn(source::text(
                &row.text,
                source::font(skins.font.clone(), false),
            ))
            .id();
        commands.entity(panel).add_child(text);
        if let Some(result) = &row.result {
            let text = commands
                .spawn(source::text(
                    format!("{:?}", result.status),
                    source::font(skins.font.clone(), false),
                ))
                .id();
            commands.entity(panel).add_child(text);
        } else if row.pending {
            let text = commands
                .spawn(source::text(
                    "Awaiting server reply",
                    source::font(skins.font.clone(), false),
                ))
                .id();
            commands.entity(panel).add_child(text);
        } else {
            button(
                &mut commands,
                &skins,
                panel,
                state.generation,
                Some(row.clone()),
                "I consent: send exactly this evidence",
            );
        }
    }
    button(
        &mut commands,
        &skins,
        panel,
        state.generation,
        None,
        "Close",
    );
    state.overlay = Some(overlay);
    state.rows = rows;
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    generation: u64,
    evidence: Option<Evidence>,
    label: &str,
) {
    let button = commands
        .spawn((
            source::button(skins.buttons.clone(), 360.),
            Control {
                generation,
                evidence,
            },
        ))
        .observe(activate)
        .id();
    let text = commands
        .spawn(source::button_caption(label, skins.font.clone()))
        .id();
    commands.entity(button).add_child(text);
    commands.entity(parent).add_child(button);
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    owner: NonSend<HostOwner>,
    modal: ModalGuard,
    mut state: ResMut<State>,
) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if !state.open
        || control.generation != owner.generation()
        || owner.phase != Phase::Playing
        || owner.modal_active()
        || !modal.allows(event.entity)
    {
        return;
    }
    if let Some(evidence) = &control.evidence {
        if let Some(play) = &owner.play {
            play.link.support_evidence_submit(
                evidence,
                true,
                play.view.gameplay_input_blocked(&play.link),
            );
        }
    } else {
        state.open = false;
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn support_evidence_window_registered() {
        let registry = include_str!("mod.rs");
        let toolbar = include_str!("../ui.rs");
        assert!(registry.contains("support_evidence::install(app)"));
        assert!(toolbar.contains("Support-evidence"));
        assert!(super::SHORTCUT == bevy::prelude::KeyCode::F8);
    }
}
const SHORTCUT: KeyCode = KeyCode::F8;
