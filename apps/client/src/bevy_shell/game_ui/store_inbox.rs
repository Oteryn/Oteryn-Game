//! Remote Store Inbox claim actions and the character's custody summary.
#[cfg(test)]
mod tests;
use super::{
    super::{
        owner::{HostOwner, Phase},
        ui::Skins,
    },
    GameModal,
    modal::ModalGuard,
};
use bevy::{
    input_focus::{InputFocus, tab_navigation::TabGroup},
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::{Activate, ScrollArea},
};
use oteryn_client_runtime::cui3::store as wire;
use oteryn_client_ui::cyclopedia_character::{CyclopediaCharacterPanel, Intent};
use oteryn_client_ui::store::StorePanel;
use oteryn_client_ui_bevy as source;
#[derive(Resource, Default)]
struct State {
    generation: u64,
    revision: u64,
    root: Option<Entity>,
    snapshot: Option<CyclopediaCharacterPanel>,
    store: Option<StorePanel>,
}
#[derive(Component, Clone, Copy)]
struct Control {
    generation: u64,
    revision: u64,
    kind: Kind,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Close,
    Refresh,
    Previous,
    Next,
    Claim(usize),
    FirstClaimPage,
    NextClaims,
    PlaceLamp,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
fn perform(owner: &HostOwner, state: &State, control: Control) {
    if owner.phase != Phase::Playing
        || owner.modal_active()
        || control.generation != owner.generation()
        || control.revision != state.revision
    {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    let panel = play.link.cyclopedia_character_panel();
    if !panel.items.open || state.snapshot.as_ref() != Some(&panel) {
        return;
    }
    if control.kind == Kind::Close {
        play.link.store_inbox_set_open(false);
        return;
    }
    if play.view.gameplay_input_blocked(&play.link) || panel.target.is_some() {
        return;
    }
    let store = play.link.store_panel();
    if state.store.as_ref() != Some(&store) {
        return;
    }
    if control.kind == Kind::FirstClaimPage {
        if store.inbox_page_cursor.is_some() && !store.pending {
            play.link.store_inbox_refresh();
        }
        return;
    }
    if control.kind == Kind::NextClaims {
        play.link.store_inbox_next_claim_page();
        return;
    }
    if control.kind == Kind::PlaceLamp {
        play.link.store_inbox_place_lamp();
        return;
    }
    if let Kind::Claim(index) = control.kind {
        if store.selected
            && !store.pending
            && !store.inbox.claims_require_channel
            && let Some(entry) = store.inbox.entries.get(index)
            && let Ok(payload) = wire::encode_inbox_claim_request(&wire::StoreInboxClaimRequest {
                fulfilment_unit_id: entry.claim_key(),
            })
        {
            play.link
                .store_request(wire::COMMAND_TYPE_STORE_INBOX_CLAIM, payload);
        }
        return;
    }
    if control.kind == Kind::Refresh && store.selected && !store.pending {
        play.link.store_inbox_refresh();
    }
    if !panel.selected || panel.pending || panel.blessings.pending {
        return;
    }
    play.link.cyclopedia_character_request(match control.kind {
        Kind::Refresh => Intent::Items,
        Kind::Previous => Intent::ItemsPrevious,
        Kind::Next => Intent::ItemsNext,
        Kind::Close
        | Kind::Claim(_)
        | Kind::FirstClaimPage
        | Kind::NextClaims
        | Kind::PlaceLamp => return,
    });
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    state: Res<State>,
) {
    if guard.allows(event.entity)
        && let Ok(control) = controls.get(event.entity)
    {
        perform(&owner, &state, *control);
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    state: Res<State>,
) {
    let Some(root) = state.root else {
        events.clear();
        return;
    };
    if !guard.allows(root) {
        events.clear();
        return;
    }
    if events.read().any(|event| {
        event.key_code == KeyCode::Escape
            && event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
    }) {
        perform(
            &owner,
            &state,
            Control {
                generation: state.generation,
                revision: state.revision,
                kind: Kind::Close,
            },
        );
    }
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
    parents: Query<&ChildOf>,
) {
    let panel = owner
        .play
        .as_ref()
        .filter(|_| owner.phase == Phase::Playing)
        .map(|play| play.link.cyclopedia_character_panel())
        .filter(|p| p.items.open);
    let store = owner
        .play
        .as_ref()
        .filter(|_| panel.is_some())
        .map(|play| play.link.store_panel());
    if panel == state.snapshot && store == state.store && state.generation == owner.generation() {
        return;
    }
    if let Some(root) = state.root.take() {
        let mut ancestor = focus.get();
        while let Some(entity) = ancestor {
            if entity == root {
                focus.clear();
                break;
            }
            ancestor = parents.get(entity).ok().map(ChildOf::parent);
        }
        commands.entity(root).despawn();
    }
    state.generation = owner.generation();
    state.revision = state.revision.wrapping_add(1);
    state.snapshot = panel;
    state.store = store;
    let (Some(panel), Some(store)) = (&state.snapshot, &state.store) else {
        return;
    };
    let root = commands
        .spawn((
            GameModal,
            crate::bevy_shell::ui::window_layout::WindowLayer("store_inbox"),
            GlobalZIndex(175),
            TabGroup {
                order: 4,
                modal: true,
            },
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .id();
    let window = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            ScrollArea,
            Node {
                width: px(620.),
                max_width: percent(96.),
                max_height: percent(92.),
                padding: UiRect {
                    top: px(24.),
                    bottom: px(12.),
                    left: px(12.),
                    right: px(12.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(window);
    source::dialog_caption(&mut commands, window, "Store Inbox", skins.font.clone());
    let mut lines =
        vec!["Character inbox summary — includes Store and other deliveries.".to_string()];
    if store.selected
        && matches!(
            store.pending_action,
            wire::COMMAND_TYPE_STORE_INBOX_CLAIM
                | oteryn_client_runtime::world_interaction::world_object::COMMAND_TYPE_USE_INTENT
        )
        && let Some(feedback) = &store.feedback
    {
        lines.push(feedback.clone());
    }
    if store.inbox.claims_require_channel && !store.inbox.entries.is_empty() {
        lines.push("Return to the Channel to claim Store items into your backpack.".into());
    }
    let busy = panel.pending || panel.blessings.pending;
    let disabled = busy || !panel.selected || panel.target.is_some();
    if !panel.selected {
        lines.push("Inbox summary is unavailable in this session.".into());
    } else if panel.target.is_some() {
        lines.push("Close the other character's profile before refreshing your inbox.".into());
    } else {
        if busy {
            lines.push("Waiting for the server…".into());
        }
        if let Some(feedback) = panel.feedback {
            lines.push(feedback.into());
        }
        if let Some(page) = &panel.items.page {
            lines.push(format!(
                "Summary entries {}–{} of {} (all locations)",
                if page.total == 0 { 0 } else { page.offset + 1 },
                usize::from(page.offset) + page.rows.len(),
                page.total
            ));
            let mut found = false;
            for row in &page.rows {
                if row.quantities[3] > 0 {
                    lines.push(format!("{} × {}", row.definition, row.quantities[3]));
                    found = true;
                }
            }
            if !found {
                lines.push("No inbox items on this summary page.".into());
            }
        }
    }
    for line in lines {
        let text = commands
            .spawn(source::text(line, source::font(skins.font.clone(), false)))
            .id();
        commands.entity(window).add_child(text);
    }
    let previous = panel.items.page.as_ref().is_some_and(|p| p.offset > 0);
    let next = panel
        .items
        .page
        .as_ref()
        .is_some_and(|p| usize::from(p.offset) + p.rows.len() < usize::from(p.total));
    let store_disabled = !store.selected || store.pending || panel.target.is_some();
    let mut buttons = Vec::new();
    for (index, entry) in store.inbox.entries.iter().enumerate() {
        let title = store.browser.page.as_ref().and_then(|page| {
            page.products.iter().find(|product| {
                product.artwork.kind == 1 && product.artwork.looks[0] == u32::from(entry.item_id)
            })
        });
        let caption = title.map_or_else(
            || format!("Claim item {}", entry.item_id),
            |product| format!("Claim {}", product.offer.title),
        );
        buttons.push((
            Kind::Claim(index),
            caption,
            store_disabled || store.inbox.claims_require_channel,
        ));
    }
    if store.inbox.lamp_definition.is_some() {
        buttons.push((Kind::PlaceLamp, "Place Hireling".into(), store_disabled));
    }
    if store.inbox_page_cursor.is_some() {
        buttons.push((
            Kind::FirstClaimPage,
            "First claim page".into(),
            store_disabled,
        ));
    }
    if store.inbox.next.is_some() {
        buttons.push((Kind::NextClaims, "Next claim page".into(), store_disabled));
    }
    buttons.extend([
        (Kind::Refresh, "Refresh".into(), disabled && store_disabled),
        (
            Kind::Previous,
            "Previous summary page".into(),
            disabled || !previous,
        ),
        (Kind::Next, "Next summary page".into(), disabled || !next),
        (Kind::Close, "Close".into(), false),
    ]);
    for (kind, caption, disabled) in buttons {
        let button = commands
            .spawn((
                source::button(skins.buttons.clone(), 220.),
                Control {
                    generation: state.generation,
                    revision: state.revision,
                    kind,
                },
            ))
            .observe(activate)
            .id();
        if disabled {
            commands.entity(button).insert(InteractionDisabled);
        }
        let caption = commands
            .spawn(source::button_caption(caption, skins.font.clone()))
            .id();
        commands.entity(button).add_child(caption);
        commands.entity(window).add_child(button);
    }
    state.root = Some(root);
}
