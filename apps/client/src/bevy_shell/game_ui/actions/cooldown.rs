//! Server-observed cooldowns above the console. No timer starts on local activation.
//! Reference: edubart/otclient dab86e610991092fe8b2471f7de55a7c986b3cd2,
//! modules/game_cooldown/{cooldown.lua,cooldown.otui}; Global manual §3.8.
use super::*;
use bevy::{
    picking::{Pickable, hover::Hovered},
    ui::widget::ImageNode,
};
use oteryn_client_ui::spell_action_bar::CooldownIcon;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct Key(bool, u32);
#[derive(Component)]
struct Remaining;
#[derive(Component)]
struct Caption;

#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct Projection<'w, 's> {
    // This filter proves the root and fill Node queries are disjoint to Bevy.
    #[allow(clippy::type_complexity)]
    roots: Query<'w, 's, (Entity, &'static mut Node), (With<CooldownLine>, Without<Remaining>)>,
    icons: Query<'w, 's, (Entity, &'static Key, &'static Children, &'static Hovered)>,
    images: Query<'w, 's, &'static mut ImageNode>,
    fills: Query<'w, 's, &'static mut Node, With<Remaining>>,
    captions: Query<'w, 's, &'static mut Text, (With<Caption>, Without<ActionHint>)>,
    hints: Query<'w, 's, &'static mut Text, (With<ActionHint>, Without<Caption>)>,
}

pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    assets: Res<AssetServer>,
    loaded_images: Res<Assets<Image>>,
    mut ui: Projection,
) {
    let Some((root, mut container)) = ui.roots.iter_mut().next() else {
        return;
    };
    let icons = owner
        .play
        .as_ref()
        .map_or_else(Vec::new, |play| play.link.cooldown_bar_icons());
    container.display = if icons.is_empty() {
        Display::None
    } else {
        Display::Flex
    };
    for (entity, key, _, _) in &ui.icons {
        if !icons.iter().any(|icon| *key == Key(icon.group, icon.id)) {
            commands.entity(entity).despawn();
        }
    }
    if let Some(icon) = ui.icons.iter().find_map(|(_, key, _, hovered)| {
        hovered
            .get()
            .then(|| icons.iter().find(|icon| *key == Key(icon.group, icon.id)))
            .flatten()
    }) {
        for mut hint in &mut ui.hints {
            hint.0 = if icon.remaining_ms == 0 {
                format!("{} — no active cooldown observed", icon.label)
            } else {
                format!(
                    "{} — {:.1}s remaining",
                    icon.label,
                    icon.remaining_ms as f64 / 1000.
                )
            };
            if !icon.tooltip.is_empty() {
                hint.0.push_str(&format!("\n{}", icon.tooltip));
            }
        }
    }
    for icon in &icons {
        let key = Key(icon.group, icon.id);
        if let Some((_, _, children, _)) = ui
            .icons
            .iter()
            .find(|(_, candidate, _, _)| **candidate == key)
        {
            let artwork = children
                .iter()
                .filter_map(|child| ui.images.get(child).ok())
                .any(|image| loaded_images.contains(image.image.id()));
            for child in children.iter() {
                if let Ok(mut image) = ui.images.get_mut(child) {
                    image.rect = group_rect(icon);
                }
                if let Ok(mut node) = ui.fills.get_mut(child) {
                    node.width = percent(f32::from(icon.overlay_per_mille.min(1000)) / 10.);
                }
                if let Ok(mut text) = ui.captions.get_mut(child) {
                    text.0 = caption(icon, artwork);
                }
            }
        } else {
            spawn(&mut commands, &skins, &assets, root, icon);
        }
    }
}

// The external UI sheet has verified mappings for the four classic groups only.
// A cast index is never used as a spell image index. Other icons retain their real label.
fn group_rect(icon: &CooldownIcon) -> Option<Rect> {
    (icon.group && (1..=4).contains(&icon.id)).then(|| {
        let x = (icon.id - 1) as f32 * 20.;
        let y = if icon.remaining_ms > 0 { 20. } else { 0. };
        Rect::new(x, y, x + 20., y + 20.)
    })
}

fn caption(icon: &CooldownIcon, artwork: bool) -> String {
    if artwork {
        if icon.remaining_ms == 0 {
            String::new()
        } else {
            format!("{:.1}", icon.remaining_ms as f64 / 1000.)
        }
    } else if icon.remaining_ms == 0 {
        icon.label.clone()
    } else {
        format!("{} {:.1}s", icon.label, icon.remaining_ms as f64 / 1000.)
    }
}

fn spawn(
    commands: &mut Commands,
    skins: &Skins,
    assets: &AssetServer,
    root: Entity,
    icon: &CooldownIcon,
) {
    let entity = commands
        .spawn((
            Key(icon.group, icon.id),
            Name::new(format!(
                "Cooldown {} {}",
                if icon.group { "group" } else { "spell" },
                icon.id
            )),
            Node {
                height: px(24.),
                min_width: px(24.),
                flex_shrink: 0.,
                align_items: AlignItems::Center,
                column_gap: px(2.),
                padding: UiRect::horizontal(px(2.)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.12, 0.12)),
            Hovered::default(),
        ))
        .id();
    if let Some(rect) = group_rect(icon) {
        let image = commands
            .spawn((
                ImageNode {
                    image: assets.load("images/game/spells/cooldowns.png"),
                    rect: Some(rect),
                    ..default()
                },
                Node {
                    width: px(20.),
                    height: px(20.),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(entity).add_child(image);
    }
    let label = commands
        .spawn((
            Caption,
            source::text(
                &caption(icon, false),
                source::font(skins.font.clone(), true),
            ),
            Node {
                max_width: px(120.),
                max_height: px(20.),
                overflow: Overflow::clip(),
                ..default()
            },
        ))
        .id();
    let fill = commands
        .spawn((
            Remaining,
            Name::new("Cooldown remaining"),
            Node {
                position_type: PositionType::Absolute,
                bottom: px(0.),
                left: px(0.),
                height: px(2.),
                width: percent(f32::from(icon.overlay_per_mille.min(1000)) / 10.),
                ..default()
            },
            BackgroundColor(Color::WHITE),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(entity).add_children(&[label, fill]);
    commands.entity(root).add_child(entity);
}
