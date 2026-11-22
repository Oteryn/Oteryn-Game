//! Public item encyclopedia; inventory/player inspection remains a separate window.
mod equipment;
mod widgets;
use super::{
    super::{
        owner::{HostOwner, Phase},
        ui::Skins,
    },
    GameModal,
    modal::ModalGuard,
};
use bevy::{
    input_focus::InputFocus,
    prelude::*,
    text::EditableText,
    ui::InteractionDisabled,
    ui_widgets::{Activate, ScrollArea, TextInput},
};
use equipment::Filters;
use oteryn_client_ui::inspection::{Panel, Request, catalogue};
use oteryn_client_ui_bevy as source;
use widgets::build;

use oteryn_client_ui::quick_loot::{Action as LootAction, QuickLootFilterMode as Mode};
#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    Track,
    ExcludeSale,
    OwnValue,
    MarketValue,
    MerchantValue,
    Hands,
    Class,
    Level,
    Vocation,
    ManageLoot,
    LootFilter(Mode),
    Browse,
    Next,
    Category(bool),
    Select(u32),
    Details,
    DropSources,
    NextDropSources,
    Prices,
    Close,
}
#[derive(Component, Clone, Copy)]
struct Control {
    generation: u64,
    serial: u64,
    action: Action,
}
#[derive(Copy, Clone, Default, PartialEq, Eq)]
struct LootControls {
    analyser: bool,
    tracked: bool,
    market: Option<u64>,
    merchant: Option<u64>,
    value: Option<u64>,
    source: Option<oteryn_client_ui::analyser::PriceSource>,
    excluded: bool,
    available: bool,
    ready: bool,
    skipped: bool,
    accepted: bool,
}
impl LootControls {
    fn read(link: &crate::play::PlayLink, panel: &Panel) -> Self {
        let loot = link.quick_loot_panel();
        let id = panel.catalogue_definition.parse::<u32>().ok();
        let (analyser, tracked, excluded) = link.catalogue_loot_options(id);
        let (market, merchant, value, source) =
            id.map(|id| link.catalogue_price(id)).unwrap_or_default();
        Self {
            market,
            merchant,
            value,
            source,
            analyser,
            tracked,
            excluded,
            available: loot.selected,
            ready: loot.selected && loot.pending.is_none(),
            skipped: id.is_some_and(|id| loot.skipped.contains(&id)),
            accepted: id.is_some_and(|id| loot.accepted.contains(&id)),
        }
    }
}
#[derive(Component)]
struct Search;
#[derive(Component)]
struct OwnPrice;
#[derive(Resource, Default)]
struct State {
    overlay: Option<Entity>,
    snapshot: Option<(u64, Panel, String, LootControls)>,
    serial: u64,
    filters: Filters,
    drop_owner: Option<(u64, u32)>,
    pending: Option<(Entity, Control)>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(
            Update,
            (
                keyboard,
                finish.after(bevy::text::EditableTextSystems),
                project,
            )
                .chain(),
        );
}
fn keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    owner: NonSend<HostOwner>,
    state: Res<State>,
    guard: ModalGuard,
) {
    if owner.phase != Phase::Playing || owner.modal_active() {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if let Some(overlay) = state.overlay
        && guard.allows(overlay)
        && keys.just_pressed(KeyCode::Escape)
    {
        play.link.item_catalogue_set_open(false);
    }
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skin: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
    parents: Query<&ChildOf>,
    search: Query<&EditableText, With<Search>>,
) {
    let generation = owner.generation();
    let snapshot = owner
        .play
        .as_ref()
        .filter(|_| owner.phase == Phase::Playing)
        .and_then(|play| {
            let mut panel = play.link.inspection_panel();
            if state
                .snapshot
                .as_ref()
                .is_none_or(|(old, ..)| *old != generation)
            {
                state.filters = Default::default();
            }
            state.filters.project(&mut panel, &play.link);
            if let Some(Request::CatalogueQuery(catalogue::Query::Drops { reference, .. })) =
                panel.last_request
            {
                state.drop_owner = Some((generation, reference));
            } else if panel.catalogue_drop_next.is_none() {
                state.drop_owner = None;
            }
            panel.catalogue_open.then(|| {
                let loot = LootControls::read(&play.link, &panel);
                {
                    let details = play.link.item_catalogue_projected_text(&panel);
                    (generation, panel, details, loot)
                }
            })
        });
    if state.snapshot == snapshot {
        return;
    }
    if let Some(overlay) = state.overlay.take() {
        let mut current = focus.get();
        while let Some(entity) = current {
            if entity == overlay {
                focus.clear();
                break;
            }
            current = parents.get(entity).ok().map(ChildOf::parent);
        }
        commands.entity(overlay).despawn();
    }
    let search_value = snapshot
        .as_ref()
        .map(|(generation, panel, _, _)| {
            if state
                .snapshot
                .as_ref()
                .is_some_and(|(previous, _, _, _)| previous == generation)
            {
                search
                    .single()
                    .map(|text| text.value().to_string())
                    .unwrap_or_else(|_| panel.search.clone())
            } else {
                panel.search.clone()
            }
        })
        .unwrap_or_default();
    state.snapshot = snapshot;
    let Some((generation, panel, text, loot)) = &state.snapshot else {
        return;
    };
    let Some(serial) = state.serial.checked_add(1) else {
        return;
    };
    let control = Control {
        generation: *generation,
        serial,
        action: Action::Close,
    };
    let continuation = drop_query(panel, *generation, state.drop_owner);
    let overlay = build(
        &mut commands,
        &skin,
        panel,
        text,
        &search_value,
        control,
        (continuation.is_some(), *loot, state.filters),
    );
    state.serial = serial;
    state.overlay = Some(overlay);
}
fn activate(
    event: On<Activate>,
    guard: ModalGuard,
    controls: Query<&Control>,
    mut state: ResMut<State>,
) {
    if guard.allows(event.entity)
        && let Ok(control) = controls.get(event.entity)
    {
        state.pending = Some((event.entity, *control));
    }
}
fn drop_query(
    panel: &Panel,
    generation: u64,
    owner: Option<(u64, u32)>,
) -> Option<catalogue::Query> {
    let reference = panel.catalogue_definition.parse().ok()?;
    let after = panel.catalogue_drop_next?;
    (owner == Some((generation, reference)) && panel.catalogue.contains_key(&reference))
        .then_some(catalogue::Query::Drops { reference, after })
}

fn finish(
    guard: ModalGuard,
    own: Query<&EditableText, With<OwnPrice>>,
    search: Query<&EditableText, With<Search>>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    let Some((entity, control)) = state.pending.take() else {
        return;
    };
    if !guard.allows(entity) || owner.modal_active() || owner.phase != Phase::Playing {
        return;
    }
    if control.generation != owner.generation() || control.serial != state.serial {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    let mut panel = play.link.inspection_panel();
    state.filters.project(&mut panel, &play.link);
    if !panel.catalogue_open
        || state
            .snapshot
            .as_ref()
            .is_none_or(|(_, previous, _, _)| previous != &panel)
    {
        return;
    }
    if control.action == Action::Close {
        play.link.item_catalogue_set_open(false);
        return;
    }
    if panel.pending || !panel.selected || play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    match control.action {
        Action::OwnValue | Action::MarketValue | Action::MerchantValue => {
            use oteryn_client_ui::analyser::PriceSource;
            let Ok(id) = panel.catalogue_definition.parse() else {
                return;
            };
            let Some((_, _, _, shown)) = &state.snapshot else {
                return;
            };
            let (source, price) = match control.action {
                Action::MarketValue => (PriceSource::Market, shown.market),
                Action::MerchantValue => (PriceSource::Merchant, shown.merchant),
                Action::OwnValue => {
                    let Ok(editor) = own.single() else { return };
                    if editor.pending_paste.is_some()
                        || editor
                            .pending_edits
                            .iter()
                            .any(bevy::text::TextEdit::is_destructive)
                    {
                        state.pending = Some((entity, control));
                        return;
                    }
                    (
                        PriceSource::Own,
                        editor.value().to_string().trim().parse().ok(),
                    )
                }
                _ => return,
            };
            if let Some(price) = price {
                play.link.catalogue_choose_price(id, source, price);
            }
        }
        Action::Track | Action::ExcludeSale => {
            let Ok(id) = panel.catalogue_definition.parse::<u32>() else {
                return;
            };
            let loot = LootControls::read(&play.link, &panel);
            if state
                .snapshot
                .as_ref()
                .is_none_or(|(_, _, _, shown)| shown != &loot)
            {
                return;
            }
            let tracking = control.action == Action::Track;
            play.link.catalogue_loot_preference(
                id,
                tracking,
                if tracking {
                    !loot.tracked
                } else {
                    !loot.excluded
                },
            );
        }
        Action::Hands | Action::Class | Action::Level | Action::Vocation => {
            match control.action {
                Action::Hands => state.filters.hands = (state.filters.hands + 1) % 3,
                Action::Class => {
                    state.filters.class = match state.filters.class {
                        None => Some(0),
                        Some(4) => None,
                        Some(c) => Some(c + 1),
                    }
                }
                Action::Level => state.filters.level = !state.filters.level,
                Action::Vocation => state.filters.vocation = !state.filters.vocation,
                _ => return,
            }
            let character = play.link.cyclopedia_character_panel();
            if state.filters.level && character.general.is_none()
                || state.filters.vocation && character.vocation_id.is_none()
            {
                play.link.cyclopedia_character_request(
                    oteryn_client_ui::cyclopedia_character::Intent::SelectTab(
                        oteryn_client_runtime::cyclopedia_character::CharacterTab::GeneralStats,
                    ),
                );
            }
            if let Some(snapshot) = &mut state.snapshot {
                snapshot.2.clear();
            }
        }
        Action::ManageLoot | Action::LootFilter(_) => {
            let Ok(definition) = panel.catalogue_definition.parse::<u32>() else {
                return;
            };
            if !panel.catalogue.contains_key(&definition) {
                return;
            }
            let loot = LootControls::read(&play.link, &panel);
            if !loot.available {
                return;
            }
            if let Action::LootFilter(mode) = control.action {
                if !loot.ready
                    || state
                        .snapshot
                        .as_ref()
                        .is_none_or(|(_, _, _, shown)| shown != &loot)
                {
                    return;
                }
                let listed = match mode {
                    Mode::Skip => loot.skipped,
                    Mode::Accept => loot.accepted,
                };
                play.link.quick_loot_action(if listed {
                    LootAction::Remove { mode, definition }
                } else {
                    LootAction::Add { mode, definition }
                });
            } else {
                play.link.item_catalogue_set_open(false);
                play.link.discovery_open(98);
            }
        }
        Action::Select(id) => {
            play.link.item_catalogue_select(id);
        }
        Action::Details => play.link.item_catalogue_request(6),
        Action::DropSources => play.link.item_catalogue_request(7),
        Action::NextDropSources => {
            if let Some(catalogue::Query::Drops { reference, after }) =
                drop_query(&panel, control.generation, state.drop_owner)
            {
                play.link.item_catalogue_continue_drops(reference, after);
            }
        }
        Action::Prices => {
            play.link.item_catalogue_request(9);
            if let Ok(definition) = panel.catalogue_definition.parse() {
                play.link
                    .market_query(oteryn_client_ui::market::details::Query::Statistics {
                        definition,
                    });
            }
        }
        Action::Browse | Action::Next | Action::Category(_) => {
            let mut changed = false;
            if let Ok(search) = search.single() {
                if search.pending_paste.is_some()
                    || search
                        .pending_edits
                        .iter()
                        .any(bevy::text::TextEdit::is_destructive)
                {
                    state.pending = Some((entity, control));
                    return;
                }
                let value = search.value().to_string();
                let value = value.trim();
                if catalogue::items(0, 0, value).is_err() {
                    return;
                }
                changed = value != panel.search.trim();
                play.link.item_catalogue_filter(value, &panel.category);
            }
            let action = match control.action {
                Action::Next if !changed => 1,
                Action::Category(true) => 2,
                Action::Category(false) => 3,
                _ => 0,
            };
            play.link.item_catalogue_request(action);
        }
        Action::Close => {}
    }
}
#[cfg(test)]
mod tests;
