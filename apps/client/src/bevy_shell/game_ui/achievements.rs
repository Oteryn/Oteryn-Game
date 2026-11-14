//! Account achievement browser projected from the retained owner.
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use bevy::{
    prelude::*,
    text::EditableText,
    ui_widgets::{Activate, ScrollArea, TextInput},
};
use oteryn_client_ui::account_achievements::{
    AchievementFilter as Filter, AchievementSort as Sort, AchievementsPanel, Intent, catalog_rows,
};
use oteryn_client_ui_bevy as source;
#[derive(Resource, Default)]
struct State {
    snapshot: Option<(u64, AchievementsPanel)>,
    overlay: Option<Entity>,
    selected: Option<String>,
}
#[derive(Component)]
struct Search;
#[derive(Component, Clone)]
struct Action {
    generation: u64,
    expected: AchievementsPanel,
    kind: Kind,
}
#[derive(Clone)]
enum Kind {
    Close,
    Refresh,
    Previous,
    Next,
    Filter,
    Sort,
    Search,
    Detail(String),
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>().add_systems(Update, project);
}
pub(in crate::bevy_shell) fn open(owner: &mut HostOwner) {
    if owner.modal_active() {
        return;
    }
    if let Some(play) = &owner.play {
        let panel = play.link.achievements_panel();
        let _ = play.link.achievements_open_bound(&panel);
    }
}
fn visible_rows(
    p: &AchievementsPanel,
) -> Vec<&oteryn_client_runtime::account_achievements::AccountAchievementRow> {
    if p.catalog.ready {
        catalog_rows(&p.catalog)
            .into_iter()
            .skip(p.catalog.page.saturating_mul(64))
            .take(64)
            .collect()
    } else {
        p.page
            .as_ref()
            .map_or_else(Vec::new, |page| page.rows.iter().take(64).collect())
    }
}
fn label(commands: &mut Commands, skins: &Skins, parent: Entity, text: impl Into<String>) {
    let child = commands
        .spawn(source::text(text, source::font(skins.font.clone(), false)))
        .id();
    commands.entity(parent).add_child(child);
}
fn button(commands: &mut Commands, skins: &Skins, parent: Entity, action: Action, text: &str) {
    let p = &action.expected;
    let enabled = match action.kind {
        Kind::Refresh => p.selected && p.pending.is_none(),
        Kind::Previous => {
            if p.catalog.ready {
                p.catalog.page > 0
            } else {
                !p.catalog.enabled && p.pending.is_none() && p.has_previous()
            }
        }
        Kind::Next => {
            if p.catalog.ready {
                p.catalog.page.saturating_add(1).saturating_mul(64) < catalog_rows(&p.catalog).len()
            } else {
                !p.catalog.enabled && p.pending.is_none() && p.has_next()
            }
        }
        _ => true,
    };
    let width = if matches!(action.kind, Kind::Detail(_)) {
        700.
    } else {
        170.
    };
    let mut button = commands.spawn((source::button(skins.buttons.clone(), width), action));
    if !enabled {
        button.insert(bevy::ui::InteractionDisabled);
    }
    let child = button.observe(activate).id();
    let caption = commands
        .spawn(source::button_caption(text, skins.font.clone()))
        .id();
    commands.entity(child).add_child(caption);
    commands.entity(parent).add_child(child);
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    guard: ModalGuard,
    mut focus: ResMut<bevy::input_focus::InputFocus>,
    fields: Query<(), With<Search>>,
) {
    let panel = owner
        .play
        .as_ref()
        .map(|p| p.link.achievements_panel())
        .filter(|p| p.open);
    let Some(panel) = panel else {
        if focus.get().is_some_and(|e| fields.contains(e)) {
            focus.clear();
        }
        if let Some(entity) = state.overlay.take() {
            commands.entity(entity).despawn();
        }
        state.snapshot = None;
        state.selected = None;
        return;
    };
    if !owner.modal_active()
        && state.overlay.is_some_and(|e| guard.allows(e))
        && let Some(play) = &owner.play
    {
        let _ = play.link.achievements_continue_bound(&panel);
    }
    if state
        .snapshot
        .as_ref()
        .is_some_and(|(g, p)| *g == owner.generation() && *p == panel)
    {
        return;
    }
    if state
        .snapshot
        .as_ref()
        .is_some_and(|(g, _)| *g != owner.generation())
    {
        state.selected = None;
    }
    if focus.get().is_some_and(|e| fields.contains(e)) {
        focus.clear();
    }
    if let Some(entity) = state.overlay.take() {
        commands.entity(entity).despawn();
    }
    let overlay = commands
        .spawn((
            GameModal,
            GlobalZIndex(165),
            bevy::input_focus::tab_navigation::TabGroup {
                order: 2,
                modal: true,
            },
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
    let body = commands
        .spawn((
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(800.),
                max_width: percent(95.),
                max_height: percent(95.),
                padding: UiRect::all(px(12.)),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollArea,
        ))
        .id();
    commands.entity(overlay).add_child(body);
    source::dialog_caption(
        &mut commands,
        body,
        "Account achievements",
        skins.font.clone(),
    );
    let points = if panel.catalog.ready {
        panel.catalog.points
    } else {
        panel.page.as_ref().map_or(0, |p| p.total_points)
    };
    let count = if panel.catalog.ready {
        panel.catalog.rows.len()
    } else {
        panel.page.as_ref().map_or(0, |p| p.fact_count as usize)
    };
    let matching = if panel.catalog.ready {
        catalog_rows(&panel.catalog).len()
    } else {
        panel.page.as_ref().map_or(0, |p| p.rows.len())
    };
    let page = if panel.catalog.ready {
        (panel.catalog.page as u64).saturating_add(1)
    } else {
        panel.page.as_ref().map_or(1, |p| u64::from(p.page) + 1)
    };
    let scope = if panel.catalog.ready {
        "matching"
    } else {
        "here; load all for filters"
    };
    label(
        &mut commands,
        &skins,
        body,
        format!("{points} points · {count} earned · {matching} {scope} · Page {page}"),
    );
    if panel.pending.is_some() || panel.catalog.collecting {
        label(&mut commands, &skins, body, "Loading achievements…");
    }
    if let Some(error) = panel.error {
        label(&mut commands, &skins, body, error);
    }
    let mut edit = EditableText::new(&panel.catalog.search);
    edit.max_characters = Some(128);
    let search = commands
        .spawn((
            Search,
            TextInput,
            edit,
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::no_wrap(),
            bevy::text::TextCursorStyle::default(),
            bevy::input_focus::tab_navigation::TabIndex(0),
            Node {
                height: px(26.),
                width: percent(100.),
                ..default()
            },
        ))
        .id();
    commands.entity(body).add_child(search);
    for (kind, caption) in [
        (Kind::Search, "Search".into()),
        (Kind::Filter, format!("Filter: {:?}", panel.catalog.filter)),
        (Kind::Sort, format!("Sort: {:?}", panel.catalog.sort)),
        (Kind::Refresh, "Load / refresh account".into()),
        (Kind::Previous, "Previous page".into()),
        (Kind::Next, "Next page".into()),
        (Kind::Close, "Close".into()),
    ] {
        button(
            &mut commands,
            &skins,
            body,
            Action {
                generation: owner.generation(),
                expected: panel.clone(),
                kind,
            },
            &caption,
        );
    }
    for row in visible_rows(&panel) {
        button(
            &mut commands,
            &skins,
            body,
            Action {
                generation: owner.generation(),
                expected: panel.clone(),
                kind: Kind::Detail(row.key.clone()),
            },
            &format!(
                "{} — Grade {} — {} points{}",
                row.name,
                row.grade,
                row.points,
                if row.secret { " — Secret" } else { "" }
            ),
        );
        if state.selected.as_deref() == Some(&row.key) {
            label(
                &mut commands,
                &skins,
                body,
                format!(
                    "{}\n{}\nEarned: {} (server Unix ms)",
                    row.name, row.description, row.earned_at
                ),
            );
        }
    }
    state.snapshot = Some((owner.generation(), panel.clone()));
    state.overlay = Some(overlay);
}
fn activate(
    event: On<Activate>,
    actions: Query<&Action>,
    search: Query<&EditableText, With<Search>>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    if !guard.allows(event.entity)
        || action.generation != owner.generation()
        || owner.modal_active()
    {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if play.link.achievements_panel() != action.expected {
        return;
    }
    let p = &action.expected;
    match &action.kind {
        Kind::Close => {
            let _ = play.link.achievements_dispatch_bound(p, None);
        }
        Kind::Refresh => {
            let _ = play
                .link
                .achievements_dispatch_bound(p, Some(Intent::Refresh));
        }
        Kind::Detail(key) => {
            state.selected = Some(key.clone());
            state.snapshot = None;
        }
        Kind::Previous | Kind::Next if !p.catalog.enabled => {
            let intent = if matches!(action.kind, Kind::Previous) {
                Intent::Previous
            } else {
                Intent::Next
            };
            let _ = play.link.achievements_dispatch_bound(p, Some(intent));
        }
        kind => {
            let mut filter = p.catalog.filter;
            let mut sort = p.catalog.sort;
            let mut delta = 0;
            match kind {
                Kind::Filter => {
                    filter = match filter {
                        Filter::All => Filter::Regular,
                        Filter::Regular => Filter::Secret,
                        Filter::Secret => Filter::All,
                    }
                }
                Kind::Sort => {
                    sort = match sort {
                        Sort::Name => Sort::Points,
                        Sort::Points => Sort::Grade,
                        Sort::Grade => Sort::Earned,
                        Sort::Earned => Sort::Name,
                    }
                }
                Kind::Previous => delta = -1,
                Kind::Next => delta = 1,
                Kind::Search => delta = 0,
                _ => return,
            }
            let value = search
                .iter()
                .next()
                .map_or_else(|| p.catalog.search.clone(), |s| s.value().to_string());
            let _ = play
                .link
                .achievements_browse_bound(p, &value, filter, sort, delta);
        }
    }
}

#[cfg(test)]
mod tests;
