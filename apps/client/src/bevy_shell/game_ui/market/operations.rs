//! Item operations are local consent over the retained market owner.
mod widgets;
use super::*;
use oteryn_client_runtime::market::market::MarketRow;
use oteryn_client_ui::market::{Intent, MarketView};
#[derive(Clone)]
enum Review {
    Create {
        fields: [String; 2],
        sell: bool,
        anonymous: bool,
    },
    Cancel(MarketRow),
}
#[derive(Clone, Copy, Component)]
struct Control {
    revision: u64,
    kind: ControlKind,
}
#[derive(Clone, Copy)]
enum ControlKind {
    Close,
    Own,
    History,
    Previous,
    Next,
    Sell,
    Buy,
    Anonymous,
    Cancel(usize),
    Confirm,
    Dismiss,
    Coins,
}
#[derive(Component)]
struct Open(Key);
#[derive(Component)]
struct Field(usize);
#[derive(Resource, Default)]
struct Operations {
    generation: u64,
    binding: usize,
    panel: Option<MarketPanel>,
    root: Option<Entity>,
    revision: u64,
    fields: [String; 2],
    anonymous: bool,
    review: Option<Review>,
    dirty: bool,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<Operations>()
        .add_systems(Update, project.after(super::project));
}
pub(super) fn button(commands: &mut Commands, skin: &Skins, parent: Entity, key: Key) {
    let entity = commands
        .spawn((
            source::button(skin.buttons.clone(), 150.),
            Open(key),
            Name::new("market.operations.open"),
        ))
        .observe(open)
        .id();
    let caption = commands
        .spawn(source::button_caption(
            "Market operations…",
            skin.font.clone(),
        ))
        .id();
    commands.entity(entity).add_child(caption);
    commands.entity(parent).add_child(entity);
}
fn available(owner: &HostOwner) -> bool {
    super::purchase::available(owner)
}
fn valid(owner: &HostOwner, state: &Operations) -> bool {
    available(owner)
        && owner.generation() == state.generation
        && owner.play.as_ref().is_some_and(|play| {
            play.link.market_session_binding() == state.binding
                && state.panel.as_ref().is_some_and(|panel| {
                    panel.open && panel.selected && play.link.market_panel() == *panel
                })
        })
}
fn open(
    event: On<Activate>,
    controls: Query<&Open>,
    owner: NonSend<HostOwner>,
    market: Res<State>,
    mut state: ResMut<Operations>,
    guard: ModalGuard,
) {
    let Ok(Open(key)) = controls.get(event.entity) else {
        return;
    };
    if !guard.allows(event.entity)
        || !available(&owner)
        || market.key != Some(*key)
        || owner.generation() != key.generation
    {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    let panel = play.link.market_panel();
    if !panel.open || !panel.selected || market.seen.as_ref() != Some(&panel) {
        return;
    }
    state.generation = key.generation;
    state.binding = play.link.market_session_binding();
    state.panel = Some(panel);
    state.fields = ["1".into(), "1".into()];
    state.anonymous = false;
    state.review = None;
    state.dirty = true;
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    fields: Query<(&Field, &EditableText)>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<Operations>,
    guard: ModalGuard,
) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if !guard.allows(event.entity) || control.revision != state.revision || !valid(&owner, &state) {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    let Some(panel) = state.panel.clone() else {
        return;
    };
    if !matches!(control.kind, ControlKind::Close | ControlKind::Dismiss)
        && (panel.pending.is_some()
            || panel.details.pending.is_some()
            || panel.coins.pending.is_some())
    {
        return;
    }
    if matches!(
        control.kind,
        ControlKind::Sell | ControlKind::Buy | ControlKind::Anonymous
    ) {
        let mut values = state.fields.clone();
        for (field, text) in &fields {
            if super::purchase::editing(text) {
                return;
            }
            if field.0 < 2 {
                values[field.0] = text.value().to_string();
            }
        }
        state.fields = values;
    }
    match control.kind {
        ControlKind::Close => {
            state.panel = None;
            state.review = None;
        }
        ControlKind::Dismiss => state.review = None,
        ControlKind::Anonymous => state.anonymous = !state.anonymous,
        ControlKind::Own | ControlKind::History | ControlKind::Previous | ControlKind::Next => {
            let intent = match control.kind {
                ControlKind::Own => Intent::OwnOffers,
                ControlKind::History => Intent::History,
                ControlKind::Previous => Intent::PreviousPage,
                _ => Intent::NextPage,
            };
            play.link
                .market_request_bound(state.binding, &panel, intent);
        }
        ControlKind::Sell | ControlKind::Buy => {
            let sell = matches!(control.kind, ControlKind::Sell);
            let mut prepared = panel.clone();
            if prepared.definition == 0
                || !oteryn_client_ui::market::edit_details(
                    &mut prepared,
                    &panel.definition.to_string(),
                    &state.fields[0],
                    &state.fields[1],
                    sell,
                    state.anonymous,
                )
            {
                return;
            }
            state.review = Some(Review::Create {
                fields: state.fields.clone(),
                sell,
                anonymous: state.anonymous,
            });
        }
        ControlKind::Cancel(index) => {
            let Some(offer) = panel
                .rows
                .get(index)
                .filter(|row| panel.view == MarketView::Own && row.state == 1 && row.amount > 0)
            else {
                return;
            };
            state.review = Some(Review::Cancel(offer.clone()));
        }
        ControlKind::Confirm => {
            let Some(review) = state.review.as_ref() else {
                return;
            };
            match review {
                Review::Create {
                    fields,
                    sell,
                    anonymous,
                } => {
                    play.link.market_create_bound(
                        state.binding,
                        &panel,
                        [&fields[0], &fields[1]],
                        *sell,
                        *anonymous,
                    );
                }
                Review::Cancel(row) => {
                    play.link.market_cancel_bound(state.binding, &panel, row);
                }
            }
            state.review = None;
        }
        ControlKind::Coins => {
            if play.link.coin_market_open_bound(state.binding) {
                state.panel = None;
                state.review = None;
            }
        }
    }
    state.dirty = true;
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skin: Res<Skins>,
    mut state: ResMut<Operations>,
    mut focus: ResMut<InputFocus>,
    fields: Query<(), With<Field>>,
) {
    if state.panel.is_some() {
        let current = owner
            .play
            .as_ref()
            .filter(|play| {
                available(&owner)
                    && owner.generation() == state.generation
                    && play.link.market_session_binding() == state.binding
            })
            .map(|play| play.link.market_panel())
            .filter(|panel| panel.open && panel.selected);
        if current != state.panel {
            state.panel = current;
            state.review = None;
            state.dirty = true;
        }
    }
    if !state.dirty {
        return;
    }
    if focus.get().is_some_and(|entity| fields.contains(entity)) {
        focus.clear();
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    state.revision = state.revision.wrapping_add(1);
    state.dirty = false;
    if let Some(panel) = &state.panel {
        state.root = Some(widgets::build(&mut commands, &skin, &state, panel));
    }
}
