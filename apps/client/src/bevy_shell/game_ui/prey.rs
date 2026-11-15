use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use bevy::{
    prelude::*,
    text::EditableText,
    ui::InteractionDisabled,
    ui_widgets::{Activate, TextInput},
};
use oteryn_client_ui::prey::{Intent, PreyPanel, creature_label};
use oteryn_client_ui_bevy as source;
#[cfg(test)]
mod tests;
mod view;
use view::project;
#[derive(Clone, Debug, PartialEq, Eq)]
enum Kind {
    Close,
    Refresh,
    Store,
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
    app.init_resource::<State>().add_systems(
        Update,
        (
            escape.after(crate::bevy_shell::input::route),
            project,
            refresh_rerolls,
        )
            .chain(),
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
    // A pending slot or wildcard confirmation is cancelled first; otherwise the window closes.
    let pending = state.confirmation.take().is_some() | state.wildcard.take().is_some();
    if !pending && let Some(play) = &owner.play {
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
// These fixed Wildcard costs are the admitted Prey rules; gold quotes are not on the wire.
fn affordable(panel: &PreyPanel, kind: &Kind) -> bool {
    let cost = match kind {
        Kind::Send(Intent::BonusReroll { .. } | Intent::SetOption { option: 1, .. }) => 1,
        Kind::ChooseAny(_)
        | Kind::ConfirmAny(_)
        | Kind::Send(Intent::SetOption { option: 2, .. }) => 5,
        _ => 0,
    };
    cost == 0
        || panel
            .confirmed
            .as_ref()
            .is_some_and(|reply| reply.wildcard_balance >= cost)
}
fn reroll_time(free_at: u64, now: u64) -> String {
    let minutes = free_at.saturating_sub(now).div_ceil(60);
    if minutes == 0 {
        "Free list reroll available".into()
    } else {
        format!(
            "Next free list reroll in {:02}:{:02} (estimated)",
            minutes / 60,
            minutes % 60
        )
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
        || !affordable(&action.panel, &action.kind)
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
            if entered.trim().is_empty() {
                return;
            }
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
                .unwrap_or_else(|| {
                    let name = entered.trim();
                    if name.starts_with("oteryn:") {
                        name.to_owned()
                    } else {
                        let slug = name
                            .to_ascii_lowercase()
                            .split(|c: char| !c.is_ascii_alphanumeric())
                            .filter(|part| !part.is_empty())
                            .collect::<Vec<_>>()
                            .join("_");
                        format!("oteryn:creature.{slug}")
                    }
                });
            let request = oteryn_client_ui::prey::prey::Request::SelectAny {
                slot,
                creature: creature.clone(),
            };
            if action.panel.slot(slot).is_none_or(|slot| slot.state != 1)
                || !action.panel.unlocked(slot)
                || action.panel.pending.is_some()
                || oteryn_client_ui::prey::prey::encode_request(&request).is_err()
            {
                return;
            }
            state.confirmation = None;
            state.wildcard = Some(Intent::SelectAny { slot, creature });
        }
        Kind::ConfirmAny(ref expected) => {
            if matches!(expected, Intent::SelectAny { slot, .. } if action.panel.slot(*slot).is_some_and(|slot| slot.state == 1))
                && state.wildcard.as_ref() == Some(expected)
                && let Some(intent) = state.wildcard.take()
            {
                play.link.prey_request(intent);
            }
        }
        Kind::Cancel => {
            state.confirmation = None;
            state.wildcard = None;
        }
        Kind::Store => {
            play.link.prey_set_open(false);
            state.confirmation = None;
            play.link.store_set_open(true);
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
    let disabled = !affordable(panel, &kind);
    let child = commands
        .spawn((
            source::button(skin.buttons.clone(), 320.),
            Action {
                generation,
                panel: panel.clone(),
                kind,
            },
        ))
        .observe(activate)
        .id();
    if disabled {
        commands.entity(child).insert(InteractionDisabled);
    }
    label(commands, skin, child, text);
    commands.entity(parent).add_child(child);
    child
}

fn refresh_rerolls(mut labels: Query<(&FreeReroll, &mut Text)>) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    for (deadline, mut text) in &mut labels {
        let value = reroll_time(deadline.0, now);
        if text.0 != value {
            text.0 = value;
        }
    }
}
