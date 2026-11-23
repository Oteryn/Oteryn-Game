//! Bevy shares the native Gem Atelier's exact review and command owner.
use super::*;
use crate::play::GemControl;
use oteryn_client_ui::wheel_allocation::{self as model, Intent, gems};
use oteryn_client_ui_bevy as source;
use std::sync::Arc;
#[derive(Resource, Default)]
struct State {
    open: bool,
    query: String,
    workshop_kind: Option<u8>,
    in_vessel: bool,
    quality: Option<u8>,
    locked: bool,
    generation: u64,
    seen: Option<Arc<WheelPanel>>,
    root: Option<Entity>,
}
impl State {
    fn shows_gem(&self, panel: &WheelPanel, gem: &model::atelier::GemView, search: &str) -> bool {
        self.quality.is_none_or(|q| gem.quality == q)
            && (!self.locked || gem.locked)
            && (search.is_empty()
                || gem.modifiers.iter().enumerate().any(|(index, id)| {
                    let kind = u8::from(index == 2);
                    gems::current(panel)
                        .and_then(|r| r.modifiers.iter().find(|m| m.kind == kind && m.id == *id))
                        .is_some_and(|m| {
                            m.name.to_lowercase().contains(search)
                                || m.effects.iter().any(|e| {
                                    e.name.to_lowercase().contains(search)
                                        || e.target.to_lowercase().contains(search)
                                })
                        })
                }))
    }
}
#[derive(Clone, Copy)]
enum Kind {
    Open,
    OpenGem { revision: u64, id: [u8; 16] },
    Close,
    FilterText,
    WorkshopFilter,
    FilterEquipped,
    FilterQuality,
    FilterLocked,
    Action(GemControl),
}
#[derive(Component)]
struct Action {
    generation: u64,
    expected: Arc<WheelPanel>,
    kind: Kind,
}
#[derive(Component)]
pub(super) struct ModifierSearch;
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
    let shared = Arc::new(panel.clone());
    if let Some(atelier) = &panel.confirmed_atelier {
        for gem in atelier.gems.iter().filter(|gem| gem.vessel) {
            button(
                commands,
                skin,
                parent,
                key.generation,
                &shared,
                Kind::OpenGem {
                    revision: atelier.revision,
                    id: gem.id,
                },
                &format!("Inspect gem in vessel {}", u16::from(gem.domain) + 1),
            );
        }
    }
    button(
        commands,
        skin,
        parent,
        key.generation,
        &shared,
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
    expected: &Arc<WheelPanel>,
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
    if matches!(kind, Kind::OpenGem { .. })
        && (expected.pending.is_some()
            || expected.gems.pending.is_some()
            || expected.gems.review.is_some()
            || expected.retry.is_some())
    {
        commands.entity(e).insert(bevy::ui::InteractionDisabled);
    }
    text(commands, skin, e, label);
    commands.entity(parent).add_child(e);
}
fn activate(
    event: On<Activate>,
    actions: Query<&Action>,
    fields: Query<&bevy::text::EditableText, With<ModifierSearch>>,
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
        || *action.expected != play.link.wheel_panel()
    {
        return;
    }
    match action.kind {
        Kind::Open | Kind::OpenGem { .. } => {
            if let Kind::OpenGem { revision, id } = action.kind {
                play.link
                    .gem_control_bound(&action.expected, Some(GemControl::Gem { revision, id }));
                if play.link.wheel_panel().gem != Some(id) {
                    return;
                }
            }
            state.open = true;
            state.query.clear();
            state.workshop_kind = None;
            state.in_vessel = false;
            state.quality = None;
            state.locked = false;
            state.generation = action.generation;
            state.seen = None;
        }
        Kind::FilterText if state.open => {
            if let Ok(field) = fields.single() {
                state.query = field.value().to_string().trim().to_string();
                state.seen = None;
            }
        }
        Kind::WorkshopFilter if state.open => {
            state.workshop_kind = match state.workshop_kind {
                None => Some(0),
                Some(0) => Some(1),
                Some(_) => None,
            };
            state.seen = None;
        }
        Kind::FilterEquipped if state.open => {
            state.in_vessel = !state.in_vessel;
            state.seen = None;
        }
        Kind::FilterQuality if state.open => {
            state.quality = match state.quality {
                None => Some(0),
                Some(0) => Some(1),
                Some(1) => Some(2),
                Some(_) => None,
            };
            state.seen = None;
        }
        Kind::FilterLocked if state.open => {
            state.locked = !state.locked;
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
    if state.open
        && matches!(
            action.kind,
            Kind::FilterText | Kind::FilterQuality | Kind::FilterLocked
        )
    {
        let panel = &action.expected;
        let search = state.query.to_lowercase();
        if panel
            .selected_gem()
            .is_none_or(|gem| !state.shows_gem(panel, gem, &search))
            && let Some(atelier) = &panel.confirmed_atelier
        {
            let selection = atelier
                .gems
                .iter()
                .find(|gem| state.shows_gem(panel, gem, &search))
                .map_or(GemControl::ClearSelection, |gem| GemControl::Gem {
                    revision: atelier.revision,
                    id: gem.id,
                });
            play.link.gem_control_bound(panel, Some(selection));
        }
    }
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skin: Res<Skins>,
    fields: Query<&bevy::text::EditableText, With<ModifierSearch>>,
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
    if state.seen.as_deref() == Some(&panel) {
        return;
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    let root = commands
        .spawn((
            GameModal,
            crate::bevy_shell::ui::window_layout::WindowLayer("gem_atelier"),
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
    let panel = Arc::new(panel);
    let search = state.query.to_lowercase();
    if panel.gems.review.is_none() {
        text(
            &mut commands,
            &skin,
            root,
            "Search gems and workshop by modifier",
        );
        let field = commands
            .spawn((
                ModifierSearch,
                bevy::ui_widgets::TextInput,
                bevy::text::EditableText::new(
                    fields
                        .single()
                        .map_or_else(|_| state.query.clone(), |f| f.value().to_string()),
                ),
                source::font(skin.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                bevy::text::TextCursorStyle::default(),
                bevy::input_focus::tab_navigation::TabIndex(0),
                Node {
                    min_height: px(26.),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
            ))
            .id();
        commands.entity(root).add_child(field);
    }
    let mut visible_modifier = panel.modifier.is_none();
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
        add(Kind::FilterText, "Apply modifier search");
        if let Some(atelier) = &panel.confirmed_atelier {
            let quality = match state.quality {
                Some(0) => "Lesser",
                Some(1) => "Regular",
                Some(2) => "Greater",
                _ => "All",
            };
            add(Kind::FilterQuality, &format!("Quality filter: {quality}"));
            add(
                Kind::FilterLocked,
                if state.locked {
                    "Show locked and unlocked gems"
                } else {
                    "Show only locked gems"
                },
            );
            for gem in atelier
                .gems
                .iter()
                .filter(|gem| state.shows_gem(&panel, gem, &search))
            {
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
            if let Some(gem) = panel
                .selected_gem()
                .filter(|gem| state.shows_gem(&panel, gem, &search))
            {
                for (position, id) in gem.modifiers.iter().enumerate() {
                    let kind = u8::from(position == 2);
                    if let Some(modifier) = current
                        .modifiers
                        .iter()
                        .find(|m| m.kind == kind && m.id == *id)
                    {
                        add(
                            Kind::Action(GemControl::Modifier {
                                nonce: current.request.nonce,
                                kind,
                                id: *id,
                            }),
                            &format!("Workshop: {}", modifier.name),
                        );
                    }
                }
            }
            let kind = match state.workshop_kind {
                Some(0) => "Basic",
                Some(1) => "Supreme",
                _ => "All",
            };
            add(Kind::WorkshopFilter, &format!("Workshop kind: {kind}"));
            add(
                Kind::FilterEquipped,
                if state.in_vessel {
                    "Show all workshop modifiers"
                } else {
                    "Show only in-vessel modifiers"
                },
            );
            for modifier in current.modifiers.iter().filter(|m| {
                state.workshop_kind.is_none_or(|kind| m.kind == kind)
                    && m.name.to_lowercase().contains(&search)
                    && (!state.in_vessel
                        || panel.confirmed_atelier.as_ref().is_some_and(|a| {
                            a.gems.iter().any(|g| {
                                g.vessel
                                    && g.modifiers
                                        .iter()
                                        .enumerate()
                                        .any(|(i, id)| u8::from(i == 2) == m.kind && *id == m.id)
                            })
                        }))
            }) {
                visible_modifier |= panel.modifier == Some((modifier.kind, modifier.id));
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
            if matches!(
                action,
                GemControl::Stage(Intent::Dismantle | Intent::Reshape)
                    | GemControl::Lock
                    | GemControl::Socket
            ) && panel
                .selected_gem()
                .is_none_or(|gem| !state.shows_gem(&panel, gem, &search))
            {
                continue;
            }
            if matches!(action, GemControl::Stage(Intent::GradeUp)) && !visible_modifier {
                continue;
            }
            add(Kind::Action(action), label);
        }
    }
    state.root = Some(root);
    state.seen = Some(panel);
}

#[cfg(test)]
pub(super) fn shared_button_catalogue(app: &mut App) -> bool {
    use std::borrow::Borrow;
    let mut actions = app.world_mut().query::<&Action>();
    let mut panels = actions
        .iter(app.world())
        .filter(|action| matches!(action.kind, Kind::Action(_)))
        .map(|action| -> &WheelPanel { action.expected.borrow() });
    panels
        .next()
        .is_some_and(|first| panels.all(|other| std::ptr::eq(first, other)))
}
