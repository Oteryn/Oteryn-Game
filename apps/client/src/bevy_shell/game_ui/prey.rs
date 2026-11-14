//! Prey presentation over the existing session owner.
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use bevy::{prelude::*, ui_widgets::Activate};
use oteryn_client_ui::prey::{Intent, PreyPanel, panel_text};
use oteryn_client_ui_bevy as source;
#[derive(Resource, Default)]
struct State {
    shown: Option<(u64, PreyPanel)>,
    root: Option<Entity>,
    confirmation: Option<Intent>,
}
#[derive(Component, Clone)]
struct Control {
    generation: u64,
    expected: PreyPanel,
    intent: Option<Intent>,
    confirm: bool,
    candidate: Option<(u8, u8)>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
) {
    let panel = owner
        .play
        .as_ref()
        .map(|p| p.link.prey_panel())
        .filter(|p| p.open);
    let next = panel.map(|p| (owner.generation(), p));
    if state.shown == next && (next.is_some() || state.root.is_none()) {
        return;
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    state.confirmation = None;
    state.shown = next.clone();
    let Some((generation, panel)) = next else {
        return;
    };
    let root = commands
        .spawn((
            GameModal,
            GlobalZIndex(165),
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                position_type: PositionType::Absolute,
                left: percent(10.),
                top: percent(10.),
                width: percent(80.),
                max_height: percent(80.),
                overflow: Overflow::scroll_y(),
                padding: UiRect::all(px(24.)),
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                ..default()
            },
            bevy::ui_widgets::ScrollArea,
        ))
        .id();
    state.root = Some(root);
    source::dialog_caption(&mut commands, root, "Prey", skins.font.clone());
    let text = commands
        .spawn(source::text(
            panel_text(&panel),
            source::font(skins.font.clone(), false),
        ))
        .id();
    commands.entity(root).add_child(text);
    if let Some(slot) = panel.offered_slot {
        for row in &panel.offered_today {
            let quote = oteryn_client_ui::prey::PickConfirmation {
                slot,
                creature: row.creature.clone(),
                price_gold: row.price_gold,
            };
            button(
                &mut commands,
                &skins,
                root,
                &format!(
                    "{}: {} gold",
                    oteryn_client_ui::prey::creature_label(&row.creature),
                    row.price_gold
                ),
                Control {
                    generation,
                    expected: panel.clone(),
                    intent: Some(Intent::ConfirmPickAgain(quote)),
                    confirm: true,
                    candidate: None,
                },
            );
        }
        if !panel.offered_next.is_empty() {
            button(
                &mut commands,
                &skins,
                root,
                "Next offers",
                Control {
                    generation,
                    expected: panel.clone(),
                    intent: Some(Intent::OfferedToday {
                        slot,
                        after: panel.offered_next.clone(),
                    }),
                    confirm: false,
                    candidate: None,
                },
            );
        }
    }
    for slot in panel.confirmed.iter().flat_map(|p| &p.slots) {
        button(
            &mut commands,
            &skins,
            root,
            &format!("Slot {}: offered today", slot.slot),
            Control {
                generation,
                expected: panel.clone(),
                intent: Some(Intent::OfferedToday {
                    slot: slot.slot,
                    after: String::new(),
                }),
                confirm: false,
                candidate: None,
            },
        );

        for (index, (label, _, enabled)) in oteryn_client_ui::prey::tier_choices(&panel, slot.slot)
            .iter()
            .enumerate()
        {
            if *enabled {
                button(
                    &mut commands,
                    &skins,
                    root,
                    label,
                    Control {
                        generation,
                        expected: panel.clone(),
                        intent: Some(Intent::SetTier {
                            slot: slot.slot,
                            tier: index as u8 + 1,
                        }),
                        confirm: false,
                        candidate: None,
                    },
                );
            }
        }

        let status = match slot.state {
            0 => "Locked",
            1 => "Inactive",
            2 => "Active",
            _ => "Unknown",
        };
        let option = match slot.option {
            0 => "Manual",
            1 => "Auto reroll",
            2 => "Lock bonus",
            _ => "Unknown",
        };
        let text = commands
            .spawn(source::text(
                format!(
                    "Slot {}: {status}; {} seconds remaining; {option}",
                    slot.slot, slot.hunting_seconds
                ),
                source::font(skins.font.clone(), false),
            ))
            .id();
        commands.entity(root).add_child(text);

        for (index, creature) in slot.creatures.iter().enumerate() {
            button(
                &mut commands,
                &skins,
                root,
                &format!(
                    "Slot {}: {}",
                    slot.slot,
                    oteryn_client_ui::prey::creature_label(creature)
                ),
                Control {
                    generation,
                    expected: panel.clone(),
                    intent: None,
                    confirm: false,
                    candidate: Some((slot.slot, index as u8 + 1)),
                },
            );
        }
        button(
            &mut commands,
            &skins,
            root,
            "Select candidate",
            Control {
                generation,
                expected: panel.clone(),
                intent: Some(Intent::Select { slot: slot.slot }),
                confirm: false,
                candidate: None,
            },
        );

        let mut editable = bevy::text::EditableText::new("oteryn:");
        editable.max_characters = Some(256);
        let editor = commands
            .spawn((
                CreatureField(slot.slot),
                bevy::ui_widgets::TextInput,
                editable,
                source::font(skins.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                bevy::text::TextCursorStyle::default(),
                bevy::input_focus::tab_navigation::TabIndex(0),
                Node {
                    min_height: px(25.),
                    width: percent(100.),
                    ..default()
                },
            ))
            .id();
        commands.entity(root).add_child(editor);
        button(
            &mut commands,
            &skins,
            root,
            "Select any (5 cards)",
            Control {
                generation,
                expected: panel.clone(),
                intent: Some(Intent::SelectAny {
                    slot: slot.slot,
                    creature: String::new(),
                }),
                confirm: true,
                candidate: None,
            },
        );
        for (label, intent, confirm) in [
            (
                "Reroll list (server determines charge)",
                Intent::ListReroll { slot: slot.slot },
                true,
            ),
            (
                "Reroll bonus",
                Intent::BonusReroll { slot: slot.slot },
                true,
            ),
            (
                "Auto reroll",
                Intent::SetOption {
                    slot: slot.slot,
                    option: 1,
                },
                true,
            ),
            (
                "Lock bonus",
                Intent::SetOption {
                    slot: slot.slot,
                    option: 2,
                },
                true,
            ),
            (
                "No automatic action",
                Intent::SetOption {
                    slot: slot.slot,
                    option: 0,
                },
                false,
            ),
        ] {
            button(
                &mut commands,
                &skins,
                root,
                label,
                Control {
                    generation,
                    expected: panel.clone(),
                    intent: Some(intent),
                    confirm,
                    candidate: None,
                },
            );
        }
    }
    button(
        &mut commands,
        &skins,
        root,
        "Refresh",
        Control {
            generation,
            expected: panel.clone(),
            intent: Some(Intent::Refresh),
            confirm: false,
            candidate: None,
        },
    );
    button(
        &mut commands,
        &skins,
        root,
        "Close",
        Control {
            generation,
            expected: panel,
            intent: None,
            confirm: false,
            candidate: None,
        },
    );
}
fn button(commands: &mut Commands, skins: &Skins, parent: Entity, label: &str, control: Control) {
    let entity = commands
        .spawn((source::button(skins.buttons.clone(), 290.), control))
        .observe(activate)
        .id();
    let caption = commands
        .spawn(source::button_caption(label, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(caption);
    commands.entity(parent).add_child(entity);
}
// The observer needs the slot editor as well as the existing modal and owner guards.
#[allow(clippy::too_many_arguments)]
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    editors: Query<(&CreatureField, &bevy::text::EditableText)>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    mut commands: Commands,
    skins: Res<Skins>,
) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if !guard.allows(event.entity)
        || owner.modal_active()
        || owner.generation() != control.generation
    {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if control.intent.is_none() && control.candidate.is_none() {
        let _ = play.link.prey_dispatch_bound(&control.expected, None);
        return;
    }
    let mut control = control.clone();
    if let Some(Intent::SelectAny { slot, creature }) = &mut control.intent
        && creature.is_empty()
    {
        let Some((_, editable)) = editors.iter().find(|(field, _)| field.0 == *slot) else {
            return;
        };
        *creature = editable.value().to_string();
        if creature.is_empty() {
            return;
        }
    }
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    if play.link.prey_panel() != control.expected {
        return;
    }
    if let Some((slot, index)) = control.candidate {
        if state.confirmation.is_none() {
            let _ = play.link.prey_choose_bound(&control.expected, slot, index);
        }
        return;
    }
    if control.expected.pending.is_some() || !control.expected.selected {
        return;
    }
    let slot = match &control.intent {
        Some(
            Intent::SetTier { slot, .. }
            | Intent::ListReroll { slot }
            | Intent::Select { slot }
            | Intent::SelectAny { slot, .. }
            | Intent::BonusReroll { slot }
            | Intent::SetOption { slot, .. }
            | Intent::OfferedToday { slot, .. },
        ) => Some(*slot),
        Some(Intent::ConfirmPickAgain(quote)) => Some(quote.slot),
        Some(Intent::Refresh) | None => None,
    };
    if slot.is_some_and(|slot| !control.expected.unlocked(slot)) {
        return;
    }
    if control.confirm {
        if state.confirmation.is_some() {
            return;
        }
        state.confirmation = control.intent.clone();
        if let Some(root) = state.root {
            let cancel = commands
                .spawn((
                    source::button(skins.buttons.clone(), 100.),
                    CancelConfirmation(control.clone()),
                ))
                .observe(cancel_confirmation)
                .id();
            let label = commands
                .spawn(source::button_caption("Cancel", skins.font.clone()))
                .id();
            commands.entity(cancel).add_child(label);
            commands.entity(root).add_child(cancel);
            let mut confirmed = control.clone();
            confirmed.confirm = false;
            button(
                &mut commands,
                &skins,
                root,
                "Confirm Prey action",
                confirmed,
            );
        }
        return;
    }
    if matches!(
        &control.intent,
        Some(
            Intent::ListReroll { .. }
                | Intent::BonusReroll { .. }
                | Intent::SetOption { option: 1..=2, .. }
                | Intent::SelectAny { .. }
                | Intent::ConfirmPickAgain(_)
        )
    ) && state.confirmation != control.intent
    {
        return;
    }
    if control.intent.is_some()
        && state.confirmation.is_some()
        && state.confirmation != control.intent
    {
        return;
    }
    if play
        .link
        .prey_dispatch_bound(&control.expected, control.intent.clone())
    {
        state.confirmation = None;
        state.shown = None;
    }
}

#[cfg(test)]
mod tests;

#[derive(Component)]
struct CreatureField(u8);
#[derive(Component)]
struct CancelConfirmation(Control);
fn cancel_confirmation(
    event: On<Activate>,
    controls: Query<&CancelConfirmation>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    if !guard.allows(event.entity) || owner.modal_active() {
        return;
    }
    let Ok(CancelConfirmation(control)) = controls.get(event.entity) else {
        return;
    };
    if owner.generation() != control.generation
        || owner
            .play
            .as_ref()
            .is_none_or(|p| p.link.prey_panel() != control.expected)
    {
        return;
    }
    state.confirmation = None;
    state.shown = None;
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    state: Res<State>,
    controls: Query<&Control>,
) {
    let Some(root) = state.root else {
        events.clear();
        return;
    };
    if !guard.allows(root) || owner.modal_active() {
        events.clear();
        return;
    }
    if events.read().any(|e| {
        e.key_code == KeyCode::Escape && e.state == bevy::input::ButtonState::Pressed && !e.repeat
    }) && let Some(control) = controls
        .iter()
        .find(|control| control.intent.is_none() && control.candidate.is_none())
        && control.generation == owner.generation()
        && let Some(play) = &owner.play
    {
        let _ = play.link.prey_dispatch_bound(&control.expected, None);
    }
}
pub(in crate::bevy_shell) fn open(owner: &mut HostOwner) {
    if !owner.modal_active()
        && let Some(play) = &owner.play
        && !play.view.gameplay_input_blocked(&play.link)
    {
        let _ = play.link.prey_open_bound(&play.link.prey_panel());
    }
}
