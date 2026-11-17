//! The states bar of the status widget: one 9x9 condition icon per active state (Tibia
//! Global StatesBar), read from the admitted play link's confirmed view models.
use super::{HostOwner, Skins, source};
use bevy::prelude::*;
use oteryn_client_ui::condition_status::icon_key;
use std::time::Duration;

#[derive(PartialEq, Eq)]
struct Icon {
    key: String,
    label: String,
    count: usize,
}
impl Icon {
    fn new(key: &str, label: &str, count: usize) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            count,
        }
    }
}

// Only the rendered labels/counts are retained, never an independent gameplay state.
#[derive(Component, Default)]
pub(super) struct StatusRow(Vec<Icon>);

/// The size of one condition icon.
const ICON: f32 = 9.;
/// The cell of a condition in the Tibia `player-state-flags` strip, by its player-state bit
/// (Poison 1 -> 0 ... Feared 1048576 -> 20; the new mana shield, Agony and Powerless sit at
/// 26-28). Conditions without a Tibia state bit have no cell.
fn state_cell(key: &str) -> Option<u8> {
    Some(match key {
        "Poison" => 0,
        "Burning" => 1,
        "Energy" => 2,
        "Drunk" => 3,
        "Paralysed" => 5,
        "Haste" => 6,
        "Drowning" => 8,
        "Freezing" => 9,
        "Dazzled" => 10,
        "Cursed" => 11,
        "Bleeding" => 15,
        "Intense Hex" => 17,
        "Rooted" => 19,
        "Feared" => 20,
        "Mana Shield" => 26,
        "Agony" => 27,
        "Powerless" => 28,
        _ => return None,
    })
}
/// The art slot key of a state: its `player-state-flags-<cell>` strip cell, else `condition-`
/// and the lowercase key with every other character turned into a dash.
fn slot_key(key: &str) -> String {
    if let Some(cell) = state_cell(key) {
        return format!("player-state-flags-{cell}");
    }
    let slug: String = key
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    format!("condition-{}", slug.trim_matches('-'))
}

pub(super) fn setup(commands: &mut Commands, vitals: Entity) {
    let row = commands
        .spawn((
            Name::new("Status icons"),
            StatusRow::default(),
            Node {
                width: percent(100.),
                min_height: px(13.),
                display: Display::None,
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Center,
                column_gap: px(2.),
                row_gap: px(2.),
                padding: UiRect::horizontal(px(2.)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(vitals).add_child(row);
}

pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    time: Res<Time<Real>>,
    mut refreshed: Local<Option<(u64, Duration)>>,
    mut rows: Query<(Entity, &mut StatusRow, &mut Node)>,
) {
    let mut next = Vec::new();
    if let Some(play) = &owner.play {
        let conditions = play.link.condition_status_panel();
        let rest = play.link.rest_status_panel();
        let battle = play.link.battle_instinct_status_panel();
        let mechanics = play.link.mechanics_status_panel();
        let now = time.elapsed();
        if (conditions.selected || rest.selected || battle.selected || mechanics.selected)
            && refreshed.is_none_or(|(generation, last)| {
                generation != owner.generation()
                    || now.saturating_sub(last) >= Duration::from_secs(2)
            })
        {
            // Each existing request method owns capability selection and its pending guard.
            play.link.condition_status_request();
            play.link.rest_status_request();
            play.link.battle_instinct_status_request();
            play.link.mechanics_status_request();
            *refreshed = Some((owner.generation(), now));
        }
        if conditions.selected
            && conditions.error.is_none()
            && let Some(status) = &conditions.snapshot
        {
            // Tibia shows the strip in player-state bit order; conditions without a state bit
            // follow in the closed wire catalogue (1..=27) order. Sub-IDs identify
            // simultaneous instances of skill/regeneration conditions.
            let mut ids: Vec<u8> = (1..=27).collect();
            ids.sort_by_key(|id| (state_cell(icon_key(*id)).unwrap_or(u8::MAX), *id));
            for id in ids {
                let count = status
                    .conditions
                    .iter()
                    .filter(|condition| {
                        condition.icon == id
                            && (condition.remaining_ms == -1 || condition.remaining_ms > 0)
                    })
                    .count();
                if count > 0 && !conditions.hidden.contains(&id) {
                    let label = icon_key(id);
                    next.push(Icon::new(label, label, count));
                }
            }
        }
        if rest.selected
            && rest.error.is_none()
            && let Some(status) = rest.snapshot
        {
            if status.inside_resting_area {
                next.push(Icon::new("Resting Area", "Rest", 1));
            }
            if status.food_remaining_ms == 0 {
                next.push(Icon::new("Hungry", "Hungry", 1));
            }
        }
        if let Some(taints) = play.link.boss_taints() {
            if taints.soul_war > 0 {
                next.push(Icon::new(
                    "Soul War",
                    &lfmt!("Soul War {}", taints.soul_war),
                    1,
                ));
            }
            if taints.rotten_blood > 0 || taints.rotten_blood_final {
                next.push(Icon::new(
                    "Rotten Blood",
                    &lfmt!("Rotten Blood {}", taints.rotten_blood),
                    1,
                ));
            }
        }
        if battle.selected
            && battle.error.is_none()
            && let Some(status) = battle.snapshot
            && status.active
        {
            let label = if status.melee == 0 && status.shielding == 0 {
                "Instinct ready"
            } else {
                "Instinct"
            };
            next.push(Icon::new("Battle Instinct", label, 1));
        }
        if mechanics.selected
            && mechanics.error.is_none()
            && let Some(status) = &mechanics.snapshot
        {
            // These timers belong to the player; deaths, dust and party-member
            // observations are not active conditions on the local character.
            let mut timers: Vec<_> = status
                .timers
                .iter()
                .filter(|timer| timer.remaining_ms > 0)
                .collect();
            timers.sort_by_key(|timer| &timer.key);
            for timer in timers {
                let label = match timer.key.as_str() {
                    "oteryn:item.tibia.i7439" => "Berserk",
                    "oteryn:item.tibia.i7440" => "Mastermind",
                    "oteryn:item.tibia.i7443" => "Bullseye",
                    "oteryn:item.tibia.i49271" => "Transcendence",
                    "oteryn:item.tibia.i35563" => "Shield potion",
                    "wealth_duplex" => "Wealth Duplex",
                    key => key,
                };
                next.push(Icon::new(&timer.key, label, 1));
            }
        }
        // The status bar honours the HUD page's Status Bar column; the HUD column belongs to
        // the Conditions HUD pane (`PlayView::hud_conditions_text`).
        let prefs = play.view.preferences();
        if !prefs.special_condition_order.is_empty() || !prefs.condition_order.is_empty() {
            let order = prefs.ordered_condition_keys();
            next.sort_by_key(|icon| {
                order
                    .iter()
                    .position(|key| key == &icon.key)
                    .unwrap_or(usize::MAX)
            });
        }
        next.retain(|icon| {
            !play
                .view
                .preferences()
                .condition_status_hidden
                .contains(&icon.key)
        });
    } else {
        *refreshed = None;
    }
    let Ok((entity, mut shown, mut node)) = rows.single_mut() else {
        return;
    };
    if shown.0 == next {
        return;
    }
    commands.entity(entity).despawn_children();
    node.display = if next.is_empty() {
        Display::None
    } else {
        Display::Flex
    };
    for icon in &next {
        let container = commands
            .spawn((
                Name::new(icon.key.clone()),
                Node {
                    align_items: AlignItems::Center,
                    column_gap: px(1.),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let fallback: String = icon.label.chars().take(2).collect();
        let slot = super::super::art::spawn(
            &mut commands,
            &skins.font,
            slot_key(&icon.key),
            &fallback,
            Vec2::splat(ICON),
        );
        commands.entity(container).add_child(slot);
        if icon.count > 1 {
            let badge = commands
                .spawn((
                    Name::new("Status count"),
                    source::text(
                        icon.count.to_string(),
                        source::font(skins.font.clone(), true),
                    ),
                    Node {
                        padding: UiRect::horizontal(px(2.)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb_u8(90, 65, 15)),
                ))
                .id();
            commands.entity(container).add_child(badge);
        }
        commands.entity(entity).add_child(container);
    }
    shown.0 = next;
}

#[cfg(test)]
#[path = "status_icons/tests.rs"]
mod tests;
