use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use bevy::{prelude::*, ui_widgets::Activate};
use oteryn_client_ui::prey::{Intent, PreyPanel, creature_label};
use oteryn_client_ui_bevy as source;
#[cfg(test)]
mod tests;
#[derive(Clone, Debug, PartialEq, Eq)]
enum Kind {
    Close,
    Refresh,
    Reroll(u8),
    Confirm,
    Cancel,
    Select(u8, u8),
    Send(Intent),
}
#[derive(Component)]
struct Action {
    generation: u64,
    panel: PreyPanel,
    kind: Kind,
}
#[derive(Resource, Default)]
struct State {
    seen: Option<(u64, PreyPanel, Option<u8>)>,
    root: Option<Entity>,
    confirmation: Option<u8>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>().add_systems(
        Update,
        (escape.after(crate::bevy_shell::input::route), project).chain(),
    );
}
fn escape(
    owner: NonSend<HostOwner>,
    guard: ModalGuard,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut state: ResMut<State>,
) {
    if !keys
        .as_ref()
        .is_some_and(|keys| keys.just_pressed(KeyCode::Escape))
        || owner.modal_active()
        || !state.root.is_some_and(|root| guard.allows(root))
        || !state
            .seen
            .as_ref()
            .is_some_and(|seen| seen.0 == owner.generation())
    {
        return;
    }
    if state.confirmation.take().is_none()
        && let Some(play) = &owner.play
    {
        play.link.prey_set_open(false);
    }
}
fn slot_title(state: u8) -> &'static str {
    match state {
        0 => "Locked",
        1 => "Inactive",
        _ => "Active",
    }
}
fn slot_actions(state: u8) -> Vec<Kind> {
    if state == 0 {
        return Vec::new();
    }
    let mut actions = vec![Kind::Reroll(1)];
    if state == 2 {
        actions.extend([
            Kind::Send(Intent::BonusReroll { slot: 1 }),
            Kind::Send(Intent::SetOption { slot: 1, option: 1 }),
            Kind::Send(Intent::SetOption { slot: 1, option: 2 }),
        ]);
    }
    actions
}
fn resolve(kind: Kind, confirmation: &mut Option<u8>) -> Option<Intent> {
    match kind {
        Kind::Reroll(slot) => {
            *confirmation = Some(slot);
            None
        }
        Kind::Confirm => confirmation.take().map(|slot| Intent::ListReroll { slot }),
        Kind::Cancel => {
            *confirmation = None;
            None
        }
        Kind::Send(intent) => Some(intent),
        Kind::Refresh => Some(Intent::Refresh),
        _ => None,
    }
}
fn activate(
    event: On<Activate>,
    actions: Query<&Action>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    guard: ModalGuard,
) {
    if !guard.allows(event.entity) {
        return;
    }
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    if action.generation != owner.generation() || action.panel != play.link.prey_panel() {
        return;
    }
    match action.kind {
        Kind::Close => {
            play.link.prey_set_open(false);
            state.confirmation = None;
        }
        Kind::Select(slot, index) => play.link.prey_select(slot, index),
        _ => {
            if let Some(intent) = resolve(action.kind.clone(), &mut state.confirmation) {
                play.link.prey_request(intent);
            }
        }
    }
}
fn label(commands: &mut Commands, skin: &Skins, parent: Entity, text: &str) {
    let child = commands
        .spawn(source::button_caption(text, skin.font.clone()))
        .id();
    commands.entity(parent).add_child(child);
}
fn button(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    generation: u64,
    panel: &PreyPanel,
    kind: Kind,
    text: &str,
) {
    let child = commands
        .spawn((
            source::button(skin.buttons.clone(), 220.),
            Action {
                generation,
                panel: panel.clone(),
                kind,
            },
        ))
        .observe(activate)
        .id();
    label(commands, skin, child, text);
    commands.entity(parent).add_child(child);
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skin: Res<Skins>,
    mut state: ResMut<State>,
) {
    let panel = owner
        .play
        .as_ref()
        .map(|play| play.link.prey_panel())
        .filter(|panel| panel.open);
    let Some(panel) = panel else {
        if let Some(root) = state.root.take() {
            commands.entity(root).despawn();
        }
        *state = State::default();
        return;
    };
    let generation = owner.generation();
    if state
        .seen
        .as_ref()
        .is_some_and(|(old, old_panel, _)| *old != generation || old_panel != &panel)
    {
        state.confirmation = None;
    }
    let seen = (generation, panel.clone(), state.confirmation);
    if state.seen.as_ref() == Some(&seen) {
        return;
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    let root = commands
        .spawn((
            GameModal,
            GlobalZIndex(158),
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            Node {
                position_type: PositionType::Absolute,
                left: percent(8.),
                top: percent(12.),
                width: percent(84.),
                max_height: percent(80.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                padding: px(24.).all(),
                row_gap: px(8.),
                ..default()
            },
            bevy::ui_widgets::ScrollArea,
            crate::bevy_shell::game_ui::install::RememberScroll("prey", 0),
        ))
        .id();
    label(&mut commands, &skin, root, "Prey Creatures");
    button(
        &mut commands,
        &skin,
        root,
        generation,
        &panel,
        Kind::Close,
        "Close",
    );
    if let Some(feedback) = panel.feedback {
        label(&mut commands, &skin, root, feedback);
    }
    if panel.pending.is_some() {
        label(&mut commands, &skin, root, "Waiting for the server…");
    } else if panel.selected {
        button(
            &mut commands,
            &skin,
            root,
            generation,
            &panel,
            Kind::Refresh,
            "Refresh",
        );
        if let Some(slot) = state.confirmation {
            label(
                &mut commands,
                &skin,
                root,
                &format!("Reroll the creature list in slot {slot}?"),
            );
            button(
                &mut commands,
                &skin,
                root,
                generation,
                &panel,
                Kind::Confirm,
                "Confirm reroll",
            );
            button(
                &mut commands,
                &skin,
                root,
                generation,
                &panel,
                Kind::Cancel,
                "Cancel",
            );
        } else if panel.confirmed.is_some() {
            for id in 1..=3 {
                let Some(slot) = panel.slot(id) else {
                    continue;
                };
                label(
                    &mut commands,
                    &skin,
                    root,
                    &format!("Slot {id} · {}", slot_title(slot.state)),
                );
                if slot.state == 2 {
                    let bonus = ["None", "Damage", "Defense", "Experience", "Loot"]
                        .get(usize::from(slot.bonus))
                        .copied()
                        .unwrap_or("Unknown");
                    label(
                        &mut commands,
                        &skin,
                        root,
                        &format!(
                            "{} · {bonus} +{}% · {}/10 · {:02}:{:02}",
                            creature_label(&slot.selected),
                            slot.percent,
                            slot.grade,
                            slot.hunting_seconds / 3600,
                            (slot.hunting_seconds / 60) % 60
                        ),
                    );
                }
                if slot.state == 1 {
                    for (index, creature) in slot.creatures.iter().enumerate() {
                        if let Ok(index) = u8::try_from(index + 1) {
                            button(
                                &mut commands,
                                &skin,
                                root,
                                generation,
                                &panel,
                                Kind::Select(id, index),
                                &format!("Select {}", creature_label(creature)),
                            );
                        }
                    }
                }
                for kind in slot_actions(slot.state) {
                    let (kind, text) = match kind {
                        Kind::Reroll(_) => (Kind::Reroll(id), "List reroll".to_owned()),
                        Kind::Send(Intent::BonusReroll { .. }) => (
                            Kind::Send(Intent::BonusReroll { slot: id }),
                            "Bonus reroll".to_owned(),
                        ),
                        Kind::Send(Intent::SetOption { option, .. }) => {
                            let text = format!(
                                "{} {}",
                                if slot.option == option { "[x]" } else { "[ ]" },
                                if option == 1 {
                                    "Auto-reroll"
                                } else {
                                    "Lock bonus"
                                }
                            );
                            (
                                Kind::Send(Intent::SetOption {
                                    slot: id,
                                    option: if slot.option == option { 0 } else { option },
                                }),
                                text,
                            )
                        }
                        _ => continue,
                    };
                    button(&mut commands, &skin, root, generation, &panel, kind, &text);
                }
            }
        } else {
            label(&mut commands, &skin, root, "Loading Prey…");
        }
    } else {
        label(
            &mut commands,
            &skin,
            root,
            "Prey is unavailable in this session.",
        );
    }
    state.root = Some(root);
    state.seen = Some(seen);
}
