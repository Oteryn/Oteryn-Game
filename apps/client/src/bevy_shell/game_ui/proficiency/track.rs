//! Equipped-weapon progress and the shortcut's unspent-perk indicator.
use super::*;

use super::super::StatusBar;
use oteryn_client_ui::weapon_proficiency::{mastery_target, progress_level, progress_percent};

#[derive(Component)]
struct Track(u64, u32);
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
    mut badges: Query<&mut Node, With<Unused>>,
    mut captions: Query<&mut Text>,
    children: Query<&Children>,
) {
    let Some(bar) = bar else { return };
    let play = owner
        .play
        .as_ref()
        .filter(|_| owner.phase == Phase::Playing);
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
        if !panel.open && hands.clone().next().is_some() {
            play.link.weapon_proficiency_tick(std::time::Instant::now());
        }
        let page = panel.catalogue?;
        let entry = hands
            .filter_map(|hand| {
                page.entries
                    .iter()
                    .find(|e| e.definition == hand.item_definition_ref.get())
            })
            .next()?;
        let unused = entry
            .levels
            .iter()
            .take(progress_level(entry))
            .any(|level| level.selected.is_none());
        Some((
            entry.definition,
            format!(
                "{} · {} / {} XP ({}%)",
                entry.name,
                entry.experience,
                mastery_target(entry),
                progress_percent(entry)
            ),
            unused,
        ))
    });
    for mut node in &mut badges {
        node.display = if track.as_ref().is_some_and(|(_, _, unused)| *unused) {
            Display::Flex
        } else {
            Display::None
        };
    }
    let Some((definition, label, _)) = track else {
        for (entity, _) in &mut tracks {
            commands.entity(entity).despawn();
        }
        return;
    };
    if let Ok((entity, mut saved)) = tracks.single_mut() {
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
        commands.entity(button).add_child(text);
        commands.entity(bar.0).add_child(button);
    }
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
