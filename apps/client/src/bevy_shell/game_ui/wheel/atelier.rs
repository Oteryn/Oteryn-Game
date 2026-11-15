//! Bevy shares the native Gem Atelier's exact review and command owner.
use super::*;
use crate::play::GemControl;
use oteryn_client_ui::wheel_allocation::{self as model, Intent, gems};
use oteryn_client_ui_bevy as source;
#[derive(Resource, Default)]
struct State {
    open: bool,
    generation: u64,
    seen: Option<WheelPanel>,
    root: Option<Entity>,
}
#[derive(Clone, Copy)]
enum Kind {
    Open,
    Close,
    Action(GemControl),
}
#[derive(Component)]
struct Action {
    generation: u64,
    expected: WheelPanel,
    kind: Kind,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>().add_systems(Update, project);
}
pub(super) fn entry(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    key: Key,
    panel: &WheelPanel,
) {
    button(
        commands,
        skin,
        parent,
        key.generation,
        panel,
        Kind::Open,
        "Gem Atelier / Workshop",
    );
}
fn text(commands: &mut Commands, skin: &Skins, parent: Entity, value: impl Into<String>) {
    let label = commands
        .spawn(source::button_caption(value, skin.font.clone()))
        .id();
    commands.entity(parent).add_child(label);
}
fn button(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    generation: u64,
    expected: &WheelPanel,
    kind: Kind,
    label: &str,
) {
    let e = commands
        .spawn((
            source::button(skin.buttons.clone(), 400.),
            Action {
                generation,
                expected: expected.clone(),
                kind,
            },
        ))
        .observe(activate)
        .id();
    text(commands, skin, e, label);
    commands.entity(parent).add_child(e);
}
fn activate(
    event: On<Activate>,
    actions: Query<&Action>,
    owner: NonSend<HostOwner>,
    guard: ModalGuard,
    mut state: ResMut<State>,
) {
    if !guard.allows(event.entity) || owner.modal_active() {
        return;
    }
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    if action.generation != owner.generation()
        || play.view.gameplay_input_blocked(&play.link)
        || action.expected != play.link.wheel_panel()
    {
        return;
    }
    match action.kind {
        Kind::Open => {
            state.open = true;
            state.generation = action.generation;
            state.seen = None;
        }
        Kind::Close => {
            play.link
                .gem_control_bound(&action.expected, Some(GemControl::Cancel));
            state.open = false;
        }
        Kind::Action(action_kind) if state.open && state.generation == action.generation => {
            play.link
                .gem_control_bound(&action.expected, Some(action_kind));
        }
        _ => {}
    }
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skin: Res<Skins>,
    mut state: ResMut<State>,
) {
    let panel = owner.play.as_ref().map(|p| p.link.wheel_panel());
    if !state.open
        || state.generation != owner.generation()
        || panel.as_ref().is_none_or(|p| !p.open)
    {
        if let Some(root) = state.root.take() {
            commands.entity(root).despawn();
        }
        state.open = false;
        state.seen = None;
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if let Some(panel) = &panel {
        play.link.gem_control_bound(panel, None);
    }
    let panel = play.link.wheel_panel();
    if state.seen.as_ref() == Some(&panel) {
        return;
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    let root = commands
        .spawn((
            GameModal,
            GlobalZIndex(176),
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            Node {
                position_type: PositionType::Absolute,
                left: percent(18.),
                top: percent(8.),
                width: percent(65.),
                max_height: percent(84.),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                padding: px(24.).all(),
                row_gap: px(6.),
                ..default()
            },
            bevy::ui_widgets::ScrollArea,
        ))
        .id();
    text(
        &mut commands,
        &skin,
        root,
        "Gem Atelier and fragment workshop",
    );
    text(&mut commands, &skin, root, gems::view::details(&panel));
    let mut add = |kind, label: &str| {
        button(
            &mut commands,
            &skin,
            root,
            state.generation,
            &panel,
            kind,
            label,
        )
    };
    add(Kind::Close, "Back to Wheel");
    if panel.gems.review.is_some() {
        add(
            Kind::Action(GemControl::Confirm),
            "Confirm reviewed operation",
        );
        add(Kind::Action(GemControl::Cancel), "Cancel review");
    } else if panel.pending.is_none()
        && panel.gems.pending.is_none()
        && panel.diagram.pending.is_none()
    {
        if let Some(atelier) = &panel.confirmed_atelier {
            for gem in &atelier.gems {
                add(
                    Kind::Action(GemControl::Gem {
                        revision: atelier.revision,
                        id: gem.id,
                    }),
                    &model::gem_label(gem),
                );
            }
        }
        for q in 0..3 {
            add(
                Kind::Action(GemControl::Quality(q)),
                &format!(
                    "Quality {}{}",
                    q + 1,
                    if panel.quality == q { " selected" } else { "" }
                ),
            );
        }
        for vessel in 0..panel.vessel_slot_count() {
            add(
                Kind::Action(GemControl::Vessel(vessel)),
                &format!("Vessel {}", vessel + 1),
            );
        }
        if let Some(current) = gems::current(&panel) {
            for modifier in &current.modifiers {
                add(
                    Kind::Action(GemControl::Modifier {
                        nonce: current.request.nonce,
                        kind: modifier.kind,
                        id: modifier.id,
                    }),
                    &modifier.name,
                );
            }
        }
        for (action, label) in [
            (GemControl::Stage(Intent::Reveal), "Reveal gem"),
            (GemControl::Stage(Intent::Dismantle), "Dismantle gem"),
            (GemControl::Stage(Intent::Reshape), "Reshape modifier"),
            (
                GemControl::Stage(Intent::GradeUp),
                "Upgrade fragment modification",
            ),
            (GemControl::Lock, "Lock / unlock gem"),
            (GemControl::Socket, "Equip in vessel"),
            (GemControl::Clear, "Clear vessel"),
            (GemControl::Save, "Save vessels"),
            (GemControl::Retry, "Retry unresolved operation"),
            (GemControl::Refresh, "Refresh"),
        ] {
            add(Kind::Action(action), label);
        }
    }
    state.root = Some(root);
    state.seen = Some(panel);
}
