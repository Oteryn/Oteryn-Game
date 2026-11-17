//! Server-confirmed Bosstiary catalogue, progress and hazard level, independent of slots and trackers.
use super::super::{
    owner::{HostOwner, Phase},
    ui::Skins,
};
use bevy::{prelude::*, ui_widgets::Activate};
use oteryn_client_ui::boss_encounter::{BossCategory, Intent, catalogue_cards, slot_text};

#[derive(Component)]
struct Window;
#[derive(Component, Clone, Copy)]
struct Action {
    generation: u64,
    kind: Kind,
}
/// One Bosstiary control: a server request, or a retained selection the next request uses.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Request(Intent),
    Boss(bool),
    Slot,
    SlotBoss(bool),
}

/// The "Track kills" toggle of one boss entry, by the server's boss key.
#[derive(Component, Clone)]
struct TrackToggle {
    generation: u64,
    key: String,
    track: bool,
}
/// The Bosstiary tracker as the entry toggles show it: tracked boss keys, and whether the
/// tracker can take a change now.
#[derive(Clone, PartialEq, Eq)]
struct Tracking {
    keys: Vec<String>,
    ready: bool,
    feedback: Option<&'static str>,
}

#[derive(Resource, Default)]
struct Snapshot(
    Option<(
        u64,
        oteryn_client_ui::boss_encounter::BossEncounterPanel,
        Tracking,
    )>,
);
pub(super) fn install(app: &mut App) {
    app.init_resource::<Snapshot>().add_systems(Update, project);
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    roots: Query<Entity, With<Window>>,
    mut snapshot: ResMut<Snapshot>,
) {
    let panel = owner
        .play
        .as_ref()
        .filter(|_| {
            owner.phase == Phase::Playing && owner.settings.preferences.panel_visible("bosstiary")
        })
        .map(|play| {
            let tracker = play.link.cyclopedia_tracker_panel();
            let tracking = Tracking {
                keys: tracker
                    .bosstiary_snapshot
                    .iter()
                    .flat_map(|tracked| &tracked.entries)
                    .map(|entry| entry.creature_key.clone())
                    .collect(),
                ready: tracker.selected && !tracker.tracker_busy(true),
                feedback: tracker.widget_feedback[1],
            };
            (
                owner.generation(),
                play.link.boss_encounter_panel(),
                tracking,
            )
        });
    if snapshot.0 == panel {
        return;
    }
    snapshot.0 = panel.clone();
    for root in &roots {
        commands.entity(root).despawn();
    }
    let Some((generation, panel, tracking)) = panel else {
        return;
    };
    let root = commands
        .spawn((
            Window,
            crate::bevy_shell::ui::window_layout::WindowLayer("bosstiary"),
            bevy::ui_widgets::ScrollArea,
            GlobalZIndex(150),
            Node {
                position_type: PositionType::Absolute,
                left: px(220.),
                top: px(90.),
                width: px(650.),
                max_height: percent(75.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(12.)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.08, 0.09, 0.12)),
        ))
        .id();
    let mut text = String::from("Bosstiary\n");
    if let Some(page) = &panel.catalogue {
        text.push_str(&format!("Boss points: {}\n", page.points));
    }
    if let Some((unlocked, selected)) = panel.hazard {
        text.push_str(&format!("Hazard level: {selected} (unlocked {unlocked})\n"));
    }
    for entry in catalogue_cards(&panel) {
        text.push_str(&format!(
            "{} — {} — {} kills\n",
            entry.name,
            entry.category_name(),
            entry.kills
        ));
        for (i, stage) in ["Prowess", "Expertise", "Mastery"].iter().enumerate() {
            text.push_str(&format!(
                "  {stage}: {}/{} ({} points)\n",
                entry.kills.min(entry.thresholds[i]),
                entry.thresholds[i],
                entry.points[i]
            ));
        }
    }
    if !panel.fields[0].is_empty() {
        let name = panel
            .boss_names
            .get(&panel.fields[0])
            .map_or(panel.fields[0].as_str(), String::as_str);
        text.push_str(&format!("Selected boss: {name}\n"));
    }
    text.push_str(&slot_text(&panel));
    if !panel.summary.is_empty() {
        text.push_str(&panel.summary);
        text.push('\n');
    }
    if let Some(feedback) = &panel.feedback {
        text.push_str(feedback);
    }
    let label = commands
        .spawn((
            Text::new(text),
            TextFont {
                font: skins.font.clone().into(),
                font_size: 15.0.into(),
                ..default()
            },
        ))
        .id();

    let row = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(6.),
            row_gap: px(4.),
            ..default()
        })
        .id();
    for (label, kind) in [
        ("Refresh", Kind::Request(Intent::CatalogueRefresh)),
        ("Next page", Kind::Request(Intent::CatalogueNext)),
        ("Hazard", Kind::Request(Intent::HazardQuery)),
        ("Lower hazard", Kind::Request(Intent::HazardLower)),
        ("Raise hazard", Kind::Request(Intent::HazardRaise)),
        ("Previous boss", Kind::Boss(true)),
        ("Next boss", Kind::Boss(false)),
        ("Enter boss room", Kind::Request(Intent::Enter)),
        ("Leave: defeated", Kind::Request(Intent::Defeated)),
        ("Leave: time out", Kind::Request(Intent::Timeout)),
        ("Boss slots", Kind::Request(Intent::SlotsDetailed)),
        ("Next slot", Kind::Slot),
        ("Previous slot boss", Kind::SlotBoss(true)),
        ("Next slot boss", Kind::SlotBoss(false)),
        ("Equip slot", Kind::Request(Intent::SlotEquipSelected)),
        ("Remove slot", Kind::Request(Intent::SlotRemoveSelected)),
    ] {
        let button = commands
            .spawn((
                oteryn_client_ui_bevy::button(skins.buttons.clone(), 120.),
                Action {
                    generation: owner.generation(),
                    kind,
                },
            ))
            .observe(activate)
            .id();
        let caption = commands
            .spawn((
                Text::new(label),
                TextFont {
                    font: skins.font.clone().into(),
                    font_size: 15.0.into(),
                    ..default()
                },
            ))
            .id();
        commands.entity(button).add_child(caption);
        commands.entity(row).add_child(button);
    }
    commands.entity(root).add_child(row);
    commands.entity(root).add_child(label);
    track_toggles(&mut commands, &skins, root, generation, &panel, &tracking);
}
/// Tibia's "Track kills" checkbox on each boss entry; the server takes a Bosstiary tracker
/// change only after the boss's first credited kill.
fn track_toggles(
    commands: &mut Commands,
    skins: &Skins,
    root: Entity,
    generation: u64,
    panel: &oteryn_client_ui::boss_encounter::BossEncounterPanel,
    tracking: &Tracking,
) {
    if let Some(feedback) = tracking.feedback {
        let line = commands
            .spawn((
                Text::new(feedback),
                TextFont {
                    font: skins.font.clone().into(),
                    font_size: 15.0.into(),
                    ..default()
                },
            ))
            .id();
        commands.entity(root).add_child(line);
    }
    for entry in catalogue_cards(panel) {
        let tracked = tracking.keys.contains(&entry.key);
        let row = commands
            .spawn(Node {
                column_gap: px(6.),
                align_items: AlignItems::Center,
                ..default()
            })
            .id();
        let name = commands
            .spawn((
                Text::new(entry.name.clone()),
                TextFont {
                    font: skins.font.clone().into(),
                    font_size: 15.0.into(),
                    ..default()
                },
            ))
            .id();
        let button = commands
            .spawn((
                oteryn_client_ui_bevy::button(skins.buttons.clone(), 140.),
                TrackToggle {
                    generation,
                    key: entry.key.clone(),
                    track: !tracked,
                },
            ))
            .observe(track)
            .id();
        if !tracking.ready || entry.kills == 0 {
            commands
                .entity(button)
                .insert(bevy::ui::InteractionDisabled);
        }
        let caption = commands
            .spawn((
                Text::new(if tracked {
                    tr!("Stop tracking kills")
                } else {
                    tr!("Track kills")
                }),
                TextFont {
                    font: skins.font.clone().into(),
                    font_size: 15.0.into(),
                    ..default()
                },
            ))
            .id();
        commands.entity(button).add_child(caption);
        commands.entity(row).add_children(&[name, button]);
        commands.entity(root).add_child(row);
    }
}
/// Bosstiary is an ordinary sidebar window. Its controls can sit above an
/// existing dialog without turning the window itself into a gameplay fence.
#[derive(bevy::ecs::system::SystemParam)]
struct WindowAccess<'w, 's> {
    guard: super::modal::ModalGuard<'w, 's>,
    windows: Query<'w, 's, &'static GlobalZIndex, With<Window>>,
    parents: Query<'w, 's, &'static ChildOf>,
    modals: Query<'w, 's, (Entity, &'static GlobalZIndex), With<super::GameModal>>,
    nodes: Query<'w, 's, (&'static Node, Option<&'static Visibility>)>,
}

impl WindowAccess<'_, '_> {
    fn allows(&self, entity: Entity) -> bool {
        if self.guard.allows(entity) {
            return true;
        }
        for ancestor in std::iter::once(entity).chain(self.parents.iter_ancestors(entity)) {
            if self.nodes.get(ancestor).is_ok_and(|(node, visibility)| {
                node.display == Display::None || visibility == Some(&Visibility::Hidden)
            }) {
                return false;
            }
            if let Ok(window) = self.windows.get(ancestor) {
                return self
                    .modals
                    .iter()
                    .any(|(modal, depth)| depth.0 < window.0 && self.guard.allows(modal));
            }
        }
        false
    }
}

fn track(
    event: On<Activate>,
    toggles: Query<&TrackToggle>,
    owner: NonSend<HostOwner>,
    guard: WindowAccess,
) {
    let Ok(toggle) = toggles.get(event.entity) else {
        return;
    };
    if guard.allows(event.entity)
        && owner.phase == Phase::Playing
        && owner.generation() == toggle.generation
        && !owner.modal_active()
        && let Some(play) = &owner.play
    {
        let intent = if toggle.track {
            oteryn_client_ui::cyclopedia_tracker::Intent::Add
        } else {
            oteryn_client_ui::cyclopedia_tracker::Intent::Remove
        };
        play.link
            .cyclopedia_tracker_change(true, intent, &toggle.key);
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&Action>,
    owner: NonSend<HostOwner>,
    guard: WindowAccess,
) {
    let Ok(action) = controls.get(event.entity) else {
        return;
    };
    if guard.allows(event.entity)
        && owner.phase == Phase::Playing
        && owner.generation() == action.generation
        && !owner.modal_active()
        && let Some(play) = &owner.play
    {
        match action.kind {
            Kind::Request(intent) => play.link.boss_encounter_request(intent),
            Kind::Boss(previous) => {
                play.link
                    .boss_encounter_browse("", BossCategory::All, previous)
            }
            Kind::Slot => play.link.boss_slot_select_slot(),
            Kind::SlotBoss(previous) => play.link.boss_slot_select_boss(previous),
        }
    }
}

#[cfg(test)]
mod tests;
