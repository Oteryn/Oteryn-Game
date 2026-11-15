//! Canonical custom group catalog; membership changes only through server readback.
use super::*;
#[derive(Clone)]
pub(super) enum Draft {
    Add,
    Rename(model::Group),
}
#[derive(Component)]
pub(super) struct Catalog;
#[derive(Component)]
pub(super) struct GroupRow {
    generation: u64,
    group: model::Group,
}
fn catalog(snapshot: &Snapshot) -> Option<&[model::Group]> {
    (snapshot.groups.len() <= 8
        && snapshot
            .groups
            .iter()
            .all(|group| (1..=8).contains(&group.group) && valid_name(&group.name)))
    .then_some(snapshot.groups.as_slice())
}
fn valid_name(name: &str) -> bool {
    !name.trim().is_empty()
        && name.chars().count() <= 25
        && name.len() <= 100
        && !name.chars().any(char::is_control)
}
pub(super) fn current(panel: &oteryn_client_ui::vip::VipPanel, draft: Option<&Draft>) -> bool {
    let Some(draft) = draft else { return true };
    let Ok(snapshot) = Snapshot::read(panel) else {
        return false;
    };
    catalog(&snapshot).is_some_and(|groups| match draft {
        Draft::Add => true,
        Draft::Rename(expected) => groups.contains(expected),
    })
}
pub(super) fn setup(commands: &mut Commands, skins: &Skins, list: Entity) {
    let header = commands
        .spawn(Node {
            width: percent(100.),
            align_items: AlignItems::Center,
            ..default()
        })
        .id();
    let title = commands
        .spawn((
            source::text("Groups", source::font(skins.font.clone(), false)),
            Node {
                flex_grow: 1.,
                ..default()
            },
        ))
        .id();
    let add = button(
        commands,
        skins,
        Control {
            generation: 0,
            revision: 0,
            command: Action::GroupAdd,
        },
        "Add",
        35.,
        "vip.groups.add",
    );
    commands.entity(header).add_children(&[title, add]);
    let catalog = commands
        .spawn((
            Catalog,
            Node {
                width: percent(100.),
                flex_direction: FlexDirection::Column,
                row_gap: px(3.),
                ..default()
            },
        ))
        .id();
    commands.entity(list).add_children(&[header, catalog]);
}
pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    catalogs: Query<Entity, With<Catalog>>,
    rows: Query<(Entity, &GroupRow)>,
    mut controls: Query<(Entity, &mut Control)>,
) {
    let generation = owner.generation();
    let panel = owner.play.as_ref().map(|play| play.link.vip_panel());
    let snapshot = panel.as_ref().and_then(|panel| Snapshot::read(panel).ok());
    let groups = snapshot.as_ref().and_then(catalog).unwrap_or(&[]);
    let mut retained = std::collections::HashSet::new();
    let mut retired = std::collections::HashSet::new();
    for (entity, row) in &rows {
        if row.generation != generation || !groups.contains(&row.group) {
            retired.insert(entity);
            commands.entity(entity).despawn();
        } else {
            retained.insert(row.group.group);
        }
    }
    if let Ok(parent) = catalogs.single() {
        for group in groups
            .iter()
            .filter(|group| !retained.contains(&group.group))
        {
            let row = button(
                &mut commands,
                &skins,
                Control {
                    generation,
                    revision: panel.as_ref().map_or(0, |panel| panel.revision),
                    command: Action::GroupRename(group.group),
                },
                &group.name,
                130.,
                "vip.groups.row",
            );
            commands.entity(row).insert((
                GroupRow {
                    generation,
                    group: group.clone(),
                },
                Node {
                    width: percent(100.),
                    min_width: px(0.),
                    height: px(20.),
                    border: UiRect::all(px(1.)),
                    padding: UiRect::horizontal(px(4.)),
                    overflow: Overflow::clip(),
                    flex_shrink: 0.,
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::Center,
                    ..default()
                },
            ));
            commands.entity(parent).add_child(row);
        }
    }
    for (entity, mut control) in &mut controls {
        if retired.contains(&entity) {
            continue;
        }
        if !matches!(control.command, Action::GroupAdd | Action::GroupRename(_)) {
            continue;
        }
        if let Action::GroupRename(id) = control.command {
            if control.generation != generation || !groups.iter().any(|group| group.group == id) {
                continue;
            }
        } else {
            control.generation = generation;
        }
        control.revision = panel.as_ref().map_or(0, |panel| panel.revision);
        let blocked = !available(&owner)
            || panel
                .as_ref()
                .is_none_or(|panel| !panel.selected || panel.pending)
            || snapshot.as_ref().and_then(catalog).is_none();
        if blocked {
            commands.entity(entity).insert(InteractionDisabled);
        } else {
            commands.entity(entity).remove::<InteractionDisabled>();
        }
    }
}
pub(super) fn activate(action: Action, owner: &HostOwner, state: &mut State) {
    let Some(play) = &owner.play else { return };
    let panel = play.link.vip_panel();
    let Ok(snapshot) = Snapshot::read(&panel) else {
        return;
    };
    let Some(catalog) = catalog(&snapshot) else {
        return;
    };
    match action {
        Action::GroupAdd | Action::GroupRename(_) => {
            let group = match action {
                Action::GroupRename(id) => {
                    let Some(group) = catalog.iter().find(|group| group.group == id) else {
                        return;
                    };
                    Draft::Rename(group.clone())
                }
                _ => Draft::Add,
            };
            let text = match &group {
                Draft::Add => String::new(),
                Draft::Rename(group) => group.name.clone(),
            };
            state.serial = state.serial.saturating_add(1);
            state.pending = None;
            state.draft = Some(super::Draft {
                generation: owner.generation(),
                revision: panel.revision,
                serial: state.serial,
                contact: None,
                group: Some(group),
                text,
                icon: None,
                notify: None,
                groups: None,
                catalog: snapshot.groups,
                feedback: String::new(),
            });
        }
        Action::GroupRemove(id)
            if state.draft.as_ref().is_some_and(|draft| {
                super::current(owner, draft)
                    && matches!(&draft.group,Some(Draft::Rename(group))if group.group==id)
            }) =>
        {
            play.link.vip_request(Intent::GroupRemove(u64::from(id)));
            if play.link.vip_panel().pending {
                state.draft = None;
                state.pending = None;
            }
        }
        _ => {}
    }
}
pub(super) fn intent(draft: &mut super::Draft) -> Option<Intent> {
    if !valid_name(&draft.text) {
        draft.feedback = "Enter a group name of 1–25 characters.".into();
        return None;
    }
    match draft.group.as_ref()? {
        Draft::Add => Some(Intent::GroupAdd(draft.text.clone())),
        Draft::Rename(group) => Some(Intent::GroupRename(
            u64::from(group.group),
            draft.text.clone(),
        )),
    }
}
