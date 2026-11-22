//! Equipped-weapon progress and the shortcut's unspent-perk indicator.
use super::*;

use super::super::StatusBar;
use oteryn_client_ui::weapon_proficiency::{mastery_target, progress_level, progress_percent};

#[derive(Component)]
struct Track(u64, u32);
#[derive(Component)]
struct TrackFill;
use crate::bevy_shell::ui::toolbar::grid::UnusedPerk as Unused;

pub(super) fn install(app: &mut App) {
    app.add_systems(Update, project);
}
#[allow(clippy::too_many_arguments)]
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    bar: Option<Res<StatusBar>>,
    skin: Res<Skins>,
    mut tracks: Query<(Entity, &mut Track)>,
    mut badges: Query<&mut Node, (With<Unused>, Without<TrackFill>)>,
    mut fills: Query<&mut Node, With<TrackFill>>,
    mut captions: Query<&mut Text>,
    children: Query<&Children>,
) {
    let Some(bar) = bar else { return };
    let play = owner
        .play
        .as_ref()
        .filter(|_| owner.phase == Phase::Playing);
    let mut unused = false;
    let track = play.and_then(|play| {
        let slots = play.link.equipment_slots();
        use oteryn_client_ui::equipment::{EquipmentSlot as E, ItemMoveDestination as D};
        let hands = slots
            .iter()
            .filter(|s| {
                s.available && matches!(s.destination, D::Equipment(E::LeftHand | E::RightHand))
            })
            .filter_map(|s| s.item);
        let panel = play.link.weapon_proficiency_panel();
        if !panel.selected {
            return None;
        }
        if !panel.open {
            play.link.weapon_proficiency_tick(std::time::Instant::now());
        }
        let page = panel.catalogue?;
        unused = !panel.open
            && page.entries.iter().any(|entry| {
                entry.definition != 0
                    && entry
                        .levels
                        .iter()
                        .take(progress_level(entry))
                        .any(|level| level.selected.is_none())
            });
        let entry = hands
            .filter_map(|hand| {
                page.entries
                    .iter()
                    .find(|e| e.definition == hand.item_definition_ref.get())
            })
            .next()?;
        Some((
            entry.definition,
            format!(
                "{} · {} / {} XP ({}%)",
                entry.name,
                entry.experience,
                mastery_target(entry),
                progress_percent(entry)
            ),
            next_level_percent(entry),
        ))
    });
    for mut node in &mut badges {
        node.display = if unused { Display::Flex } else { Display::None };
    }
    let Some((definition, label, progress)) = track else {
        for (entity, _) in &mut tracks {
            commands.entity(entity).despawn();
        }
        return;
    };
    if let Ok((entity, mut saved)) = tracks.single_mut() {
        for mut fill in &mut fills {
            fill.width = percent(progress as f32);
        }
        *saved = Track(owner.generation(), definition);
        if let Ok(children) = children.get(entity)
            && let Some(child) = children.first()
            && let Ok(mut text) = captions.get_mut(*child)
            && text.0 != label
        {
            text.0 = label;
        }
    } else {
        let button = commands
            .spawn((
                Track(owner.generation(), definition),
                oteryn_client_ui_bevy::button(skin.buttons.clone(), 240.),
            ))
            .observe(open)
            .id();
        let text = commands
            .spawn(oteryn_client_ui_bevy::button_caption(
                label,
                skin.font.clone(),
            ))
            .id();
        let fill = commands
            .spawn((
                TrackFill,
                Name::new("Progress to next weapon level"),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0.),
                    bottom: px(0.),
                    height: px(3.),
                    width: percent(progress as f32),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.2, 0.6, 0.3)),
                bevy::picking::Pickable::IGNORE,
            ))
            .id();
        commands.entity(button).add_children(&[text, fill]);
        commands.entity(bar.0).add_child(button);
    }
}
fn next_level_percent(
    entry: &oteryn_client_runtime::weapon_proficiency::ProficiencyCatalogueEntry,
) -> u32 {
    let mut previous = 0;
    for &threshold in entry.thresholds.iter().take(entry.levels.len() + 2) {
        if entry.experience < threshold {
            return ((u64::from(entry.experience - previous) * 100)
                / u64::from(threshold - previous)) as u32;
        }
        previous = threshold;
    }
    100
}
fn open(event: On<Activate>, owner: NonSend<HostOwner>, guard: ModalGuard, tracks: Query<&Track>) {
    let Some(play) = &owner.play else { return };
    let Ok(track) = tracks.get(event.entity) else {
        return;
    };
    if track.0 == owner.generation()
        && guard.allows(event.entity)
        && !owner.modal_active()
        && !play.view.gameplay_input_blocked(&play.link)
    {
        play.link.open_equipped_proficiency(track.1);
    }
}
