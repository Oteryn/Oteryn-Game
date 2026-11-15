use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use bevy::{
    prelude::*,
    text::EditableText,
    ui_widgets::{Activate, TextInput},
};
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
    ChooseAny(u8),
    ConfirmAny(Intent),
}
#[derive(Component)]
struct Wildcard(u8);
#[derive(Component)]
struct FreeReroll(u64);
#[derive(Component)]
struct Action {
    generation: u64,
    panel: PreyPanel,
    kind: Kind,
}
#[derive(Resource, Default)]
struct State {
    seen: Option<(u64, PreyPanel, Option<u8>, Option<Intent>)>,
    root: Option<Entity>,
    confirmation: Option<u8>,
    wildcard: Option<Intent>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_systems(Update, (project, refresh_rerolls).chain());
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
    fields: Query<(&Wildcard, &EditableText)>,
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
    if owner.modal_active()
        || action.generation != owner.generation()
        || action.panel != play.link.prey_panel()
    {
        return;
    }
    match action.kind {
        Kind::Close => {
            play.link.prey_set_open(false);
            state.confirmation = None;
            state.wildcard = None;
        }
        Kind::ChooseAny(slot) => {
            let Some((_, text)) = fields.iter().find(|(field, _)| field.0 == slot) else {
                return;
            };
            if text.pending_paste.is_some()
                || text
                    .pending_edits
                    .iter()
                    .any(bevy::text::TextEdit::is_destructive)
            {
                return;
            }
            let entered = text.value().to_string();
            let creature = action
                .panel
                .confirmed
                .iter()
                .flat_map(|reply| &reply.slots)
                .flat_map(|slot| slot.creatures.iter().chain(std::iter::once(&slot.selected)))
                .chain(
                    action
                        .panel
                        .offered_today
                        .iter()
                        .map(|offer| &offer.creature),
                )
                .find(|key| {
                    key.eq_ignore_ascii_case(entered.trim())
                        || creature_label(key).eq_ignore_ascii_case(entered.trim())
                })
                .cloned()
                .unwrap_or_else(|| entered.trim().to_owned());
            let request = oteryn_client_ui::prey::prey::Request::SelectAny {
                slot,
                creature: creature.clone(),
            };
            if !action.panel.unlocked(slot)
                || action.panel.pending.is_some()
                || oteryn_client_ui::prey::prey::encode_request(&request).is_err()
            {
                return;
            }
            state.confirmation = None;
            state.wildcard = Some(Intent::SelectAny { slot, creature });
        }
        Kind::ConfirmAny(ref expected) => {
            if state.wildcard.as_ref() == Some(expected)
                && let Some(intent) = state.wildcard.take()
            {
                play.link.prey_request(intent);
            }
        }
        Kind::Cancel => {
            state.confirmation = None;
            state.wildcard = None;
        }
        Kind::Select(slot, index) => play.link.prey_select(slot, index),
        _ => {
            if let Some(intent) = resolve(action.kind.clone(), &mut state.confirmation) {
                play.link.prey_request(intent);
            }
        }
    }
}
fn label(commands: &mut Commands, skin: &Skins, parent: Entity, text: &str) -> Entity {
    let child = commands
        .spawn(source::button_caption(text, skin.font.clone()))
        .id();
    commands.entity(parent).add_child(child);
    child
}
fn button(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    generation: u64,
    panel: &PreyPanel,
    kind: Kind,
    text: &str,
) -> Entity {
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
    child
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
        .is_some_and(|(old, old_panel, _, _)| *old != generation || old_panel != &panel)
    {
        state.confirmation = None;
        state.wildcard = None;
    }
    let seen = (
        generation,
        panel.clone(),
        state.confirmation,
        state.wildcard.clone(),
    );
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
    if let Some(confirmed) = &panel.confirmed {
        label(
            &mut commands,
            &skin,
            root,
            &format!("Wildcards: {}", confirmed.wildcard_balance),
        );
    }
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
        if let Some(Intent::SelectAny { slot, creature }) = &state.wildcard {
            label(
                &mut commands,
                &skin,
                root,
                &format!(
                    "Use a wildcard for {} in slot {slot}?",
                    creature_label(creature)
                ),
            );
            let confirm = button(
                &mut commands,
                &skin,
                root,
                generation,
                &panel,
                Kind::ConfirmAny(Intent::SelectAny {
                    slot: *slot,
                    creature: creature.clone(),
                }),
                "Confirm wildcard selection",
            );
            commands
                .entity(confirm)
                .insert(Name::new("prey.wildcard.confirm"));
            button(
                &mut commands,
                &skin,
                root,
                generation,
                &panel,
                Kind::Cancel,
                "Cancel",
            );
        } else if let Some(slot) = state.confirmation {
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
                let availability = label(&mut commands, &skin, root, "");
                commands
                    .entity(availability)
                    .insert(FreeReroll(slot.free_reroll_at));
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
                if slot.state != 0 {
                    label(&mut commands, &skin, root, "Wildcard creature name");
                    let mut editor = EditableText::new("");
                    editor.max_characters = Some(256);
                    let field = commands
                        .spawn((
                            Wildcard(id),
                            TextInput,
                            editor,
                            Name::new(format!("prey.wildcard.creature.{id}")),
                            source::font(skin.font.clone(), false),
                            TextColor(source::TEXT_COLOR),
                            bevy::text::TextCursorStyle::default(),
                            bevy::input_focus::tab_navigation::TabIndex(0),
                            source::frame(skin.inset.clone(), [1.; 4]),
                            Node {
                                width: px(300.),
                                min_height: px(25.),
                                ..default()
                            },
                        ))
                        .id();
                    commands.entity(root).add_child(field);
                    let choose = button(
                        &mut commands,
                        &skin,
                        root,
                        generation,
                        &panel,
                        Kind::ChooseAny(id),
                        "Choose creature",
                    );
                    commands
                        .entity(choose)
                        .insert(Name::new(format!("prey.wildcard.choose.{id}")));
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

fn refresh_rerolls(mut labels: Query<(&FreeReroll, &mut Text)>) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    for (deadline, mut text) in &mut labels {
        let value = if deadline.0 <= now {
            "Free list reroll available".into()
        } else {
            format!("Free list reroll in {} seconds", deadline.0 - now)
        };
        if text.0 != value {
            text.0 = value;
        }
    }
}
