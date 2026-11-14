//! Guild and Guild Bank presentation over the retained session owners.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{
        owner::{HostOwner, Phase},
        ui::Skins,
    },
    modal::ModalGuard,
};
use crate::play::{
    feature_0040_guild::view::{GuildWindowAction, GuildWindowView, needs_confirmation, permitted},
    feature_0055_guild_bank::view::{GuildBankControl, GuildBankView},
};
use bevy::{
    input_focus::InputFocus,
    prelude::*,
    text::{EditableText, TextCursorStyle},
    ui_widgets::Activate,
};
use oteryn_client_ui::{guild::Intent, guild_bank::Intent as BankIntent};

#[derive(Clone, PartialEq)]
struct Snapshot {
    generation: u64,
    guild: GuildWindowView,
    bank: GuildBankView,
}
#[derive(Clone, PartialEq, Eq, Debug)]
enum Action {
    Close,
    Refresh,
    Invitations,
    Roster,
    More,
    Invite,
    Join([u8; 16]),
    Leave,
    Kick([u8; 16]),
    Rank([u8; 16], u8),
    Bank(BankIntent),
    Confirm,
    Cancel,
}
#[derive(Component, Clone)]
struct Control {
    serial: u64,
    action: Action,
}
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Field {
    Name,
    Guild,
    Amount,
}
#[derive(Clone, Default, PartialEq, Eq)]
struct Draft {
    name: String,
    fields: [String; 2],
}
#[derive(Clone)]
struct Confirmation {
    action: Action,
    draft: Draft,
    snapshot: Snapshot,
}
#[derive(Resource, Default)]
struct State {
    snapshot: Option<Snapshot>,
    root: Option<Entity>,
    serial: u64,
    confirmation: Option<Confirmation>,
    draft: Draft,
    dirty: bool,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>().add_systems(Update, project);
}
pub(in crate::bevy_shell) fn open(owner: &mut HostOwner, bank: bool) {
    if !available(owner) {
        return;
    }
    if let Some(play) = &owner.play {
        if bank {
            play.link.guild_bank_window_open();
        } else {
            play.link.guild_window_open();
        }
    }
}
fn available(owner: &HostOwner) -> bool {
    owner.phase == Phase::Playing
        && !owner.modal_active()
        && owner.play.as_ref().is_some_and(|play| {
            play.link.guild_window_available() && !play.view.gameplay_input_blocked(&play.link)
        })
}
fn snapshot(owner: &HostOwner) -> Option<Snapshot> {
    if owner.phase != Phase::Playing {
        return None;
    }
    let play = owner.play.as_ref()?;
    Some(Snapshot {
        generation: owner.generation(),
        guild: play.link.guild_window_view(),
        bank: play.link.guild_bank_window_view(),
    })
}
fn drafts(inputs: &Query<(&Field, &EditableText)>, fallback: &Draft) -> Option<Draft> {
    let mut draft = fallback.clone();
    for (field, text) in inputs {
        // A destructive edit/paste is not yet the value the player will see. Retry the click
        // after the editor has finished, instead of confirming an earlier amount or name.
        if text.pending_paste.is_some()
            || text
                .pending_edits
                .iter()
                .any(bevy::text::TextEdit::is_destructive)
        {
            return None;
        }
        let value = text.value().to_string();
        match field {
            Field::Name => draft.name = value,
            Field::Guild => draft.fields[0] = value,
            Field::Amount => draft.fields[1] = value,
        }
    }
    Some(draft)
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    inputs: Query<(&Field, &EditableText)>,
    mut focus: ResMut<InputFocus>,
) {
    let current = snapshot(&owner).filter(|s| s.guild.panel.open || s.bank.panel.open);
    let changed = state.snapshot != current;
    if !changed && !state.dirty {
        return;
    }
    if let Some(draft) = drafts(&inputs, &state.draft) {
        state.draft = draft;
    }
    if changed {
        state.confirmation = None;
        if let Some(current) = &current {
            if state.snapshot.as_ref().is_none_or(|old| {
                old.generation != current.generation || !old.guild.same_session(&current.guild)
            }) {
                state.draft = Draft::default();
            }
            if state
                .snapshot
                .as_ref()
                .is_none_or(|old| old.bank.panel.fields != current.bank.panel.fields)
            {
                state.draft.fields = current.bank.panel.fields.clone();
            }
        }
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
        focus.clear();
    }
    state.snapshot = current;
    state.dirty = false;
    state.serial = state.serial.saturating_add(1);
    if state.snapshot.is_some() {
        state.root = Some(widgets::build(&mut commands, &skins, &state));
    }
}
fn guild_intent(action: &Action, draft: &Draft) -> Option<Intent> {
    Some(match action {
        Action::Refresh => Intent::RefreshOwn,
        Action::Invitations => Intent::RefreshInvitations,
        Action::Roster => Intent::RefreshRoster,
        Action::More => Intent::MoreRoster,
        Action::Invite => Intent::Invite(draft.name.clone()),
        Action::Join(guild) => Intent::Join(*guild),
        Action::Leave => Intent::Leave,
        Action::Kick(character) => Intent::Kick(*character),
        Action::Rank(character, level) => Intent::SetRank {
            character: *character,
            level: *level,
        },
        _ => return None,
    })
}
fn dispatch(
    owner: &HostOwner,
    expected: &Snapshot,
    action: &Action,
    draft: &Draft,
    confirmed: bool,
) -> bool {
    let Some(play) = &owner.play else {
        return false;
    };
    if let Action::Bank(intent) = action {
        play.link.guild_bank_window_dispatch(
            &expected.bank,
            GuildBankControl::Request {
                intent: *intent,
                fields: draft.fields.clone(),
            },
            confirmed,
        )
    } else if *action == Action::Close {
        if expected.bank.panel.open {
            play.link
                .guild_bank_window_dispatch(&expected.bank, GuildBankControl::Close, false)
        } else {
            play.link
                .guild_window_dispatch(&expected.guild, GuildWindowAction::Close, false)
        }
    } else if let Some(intent) = guild_intent(action, draft) {
        play.link.guild_window_dispatch(
            &expected.guild,
            GuildWindowAction::Request(intent),
            confirmed,
        )
    } else {
        false
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    guard: ModalGuard,
    inputs: Query<(&Field, &EditableText)>,
) {
    if !guard.allows(event.entity) || !available(&owner) {
        return;
    }
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    let Some(expected) = state.snapshot.clone() else {
        return;
    };
    if control.serial != state.serial || snapshot(&owner).as_ref() != Some(&expected) {
        return;
    }
    let Some(draft) = drafts(&inputs, &state.draft) else {
        return;
    };
    state.draft = draft.clone();
    if control.action == Action::Cancel {
        state.confirmation = None;
        state.dirty = true;
        return;
    }
    if control.action == Action::Confirm {
        if let Some(confirmation) = state.confirmation.take() {
            if confirmation.snapshot == expected && confirmation.draft == draft {
                dispatch(&owner, &expected, &confirmation.action, &draft, true);
            }
            state.dirty = true;
        }
        return;
    }
    if state.confirmation.is_some() && control.action != Action::Close {
        return;
    }
    let confirm = match &control.action {
        Action::Bank(intent) => matches!(
            intent,
            BankIntent::Deposit | BankIntent::Withdraw | BankIntent::Claim
        ),
        action => guild_intent(action, &draft).is_some_and(|intent| {
            permitted(&expected.guild.panel, &intent) && needs_confirmation(&intent)
        }),
    };
    if confirm {
        // The owner revalidates every selector and amount under its lock on Confirm.
        state.confirmation = Some(Confirmation {
            action: control.action.clone(),
            draft,
            snapshot: expected,
        });
        state.dirty = true;
    } else {
        dispatch(&owner, &expected, &control.action, &draft, false);
    }
}
