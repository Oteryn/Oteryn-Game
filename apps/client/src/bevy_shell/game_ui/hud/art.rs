//! Named art slots for the Tibia Global sidebar icons. Each slot names one image under
//! `images/game/sidebar/` in the `--ui-assets` directory and is loaded through the asset server
//! the same way as the shared skin files; until that image has loaded (or when the file is
//! absent) the slot shows its short text fallback instead.
use bevy::prelude::*;
use oteryn_client_ui_bevy as source;

/// The directory, relative to the UI asset root, that holds every sidebar art slot.
pub(in crate::bevy_shell) const DIRECTORY: &str = "images/game/sidebar/";

/// One icon of the sidebar art: the image key and the text shown while the image is absent.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub(in crate::bevy_shell) struct ArtSlot {
    pub key: String,
    handle: Option<Handle<Image>>,
}
impl ArtSlot {
    /// The image this slot loads, relative to the UI asset root.
    pub(in crate::bevy_shell) fn path(&self) -> String {
        format!("{DIRECTORY}{}.png", self.key)
    }
}
/// The text a slot shows until its image has loaded.
#[derive(Component)]
pub(in crate::bevy_shell) struct ArtFallback;

/// The fixed slots of the sidebar chrome; condition and control-button slots add their own
/// keys (`condition-*`, `control-*`).
pub(in crate::bevy_shell) const SLOTS: &[(&str, &str)] = &[
    ("battle-filter-monsters", "M"),
    ("battle-filter-players", "P"),
    ("battle-sort", "≡"),
    ("combat-balanced", "Ba"),
    ("combat-chase", "Ch"),
    ("combat-defensive", "De"),
    ("combat-offensive", "Of"),
    ("combat-pvp-dove", "Dv"),
    ("combat-pvp-redfist", "Rf"),
    ("combat-pvp-whitehand", "Wh"),
    ("combat-pvp-yellowhand", "Yh"),
    ("combat-secure", "Sc"),
    ("combat-stand", "St"),
    ("inventory-blessings", "Bless"),
    ("inventory-maximise", "+"),
    ("inventory-minimise", "−"),
    ("inventory-purse", "Purse"),
    ("minimap-centre", "◎"),
    ("minimap-floor-down", "▼"),
    ("minimap-floor-up", "▲"),
    ("minimap-full-map", "Map"),
    ("minimap-rose", "✛"),
    ("minimap-zoom-in", "+"),
    ("minimap-zoom-out", "−"),
    ("sidebar-add", "‹"),
    ("sidebar-remove", "›"),
    ("slot-ammo", "Am"),
    ("slot-armor", "Ar"),
    ("slot-backpack", "Bp"),
    ("slot-feet", "Ft"),
    ("slot-head", "Hd"),
    ("slot-left-hand", "Lh"),
    ("slot-legs", "Lg"),
    ("slot-necklace", "Nk"),
    ("slot-right-hand", "Rh"),
    ("slot-ring", "Rg"),
    ("status-harmony", "Hm"),
    ("status-health", "♥"),
    ("status-mana", "♦"),
    ("status-serene", "Sr"),
    ("widget-close", "×"),
    ("widget-maximise", "+"),
    ("widget-minimise", "−"),
];

/// The text fallback of a fixed slot.
pub(in crate::bevy_shell) fn fallback(key: &str) -> &'static str {
    SLOTS
        .iter()
        .find(|(slot, _)| *slot == key)
        .map_or("?", |(_, text)| *text)
}

/// Spawns a `width` x `height` art slot whose text fallback is `text`.
pub(in crate::bevy_shell) fn spawn(
    commands: &mut Commands,
    font: &Handle<Font>,
    key: impl Into<String>,
    text: &str,
    size: Vec2,
) -> Entity {
    let key = key.into();
    let slot = commands
        .spawn((
            Name::new(key.clone()),
            ArtSlot { key, handle: None },
            ImageNode {
                color: Color::NONE,
                ..default()
            },
            Node {
                width: px(size.x),
                height: px(size.y),
                flex_shrink: 0.,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                overflow: Overflow::clip(),
                ..default()
            },
        ))
        .id();
    let caption = commands
        .spawn((
            ArtFallback,
            source::text(text, source::font(font.clone(), false)),
        ))
        .id();
    commands.entity(slot).add_child(caption);
    slot
}
/// Spawns a fixed slot from [`SLOTS`].
pub(in crate::bevy_shell) fn fixed(
    commands: &mut Commands,
    font: &Handle<Font>,
    key: &'static str,
    size: Vec2,
) -> Entity {
    spawn(commands, font, key, fallback(key), size)
}

pub(in crate::bevy_shell) fn install(app: &mut App) {
    app.add_systems(PostUpdate, project.before(bevy::ui::UiSystems::Prepare));
}
/// Loads every slot's image once and swaps the fallback text for it as soon as it is loaded.
fn project(
    assets: Option<Res<AssetServer>>,
    mut slots: Query<(&mut ArtSlot, &mut ImageNode, Option<&Children>)>,
    mut captions: Query<&mut Node, With<ArtFallback>>,
) {
    for (mut slot, mut image, children) in &mut slots {
        if slot.handle.is_none()
            && let Some(assets) = &assets
        {
            let path = slot.path();
            slot.handle = Some(assets.load(path));
        }
        let loaded = match (&slot.handle, &assets) {
            (Some(handle), Some(assets)) => assets.is_loaded_with_dependencies(handle.id()),
            _ => false,
        };
        if loaded && let Some(handle) = &slot.handle {
            if image.image != *handle {
                image.image = handle.clone();
            }
            if image.color != Color::WHITE {
                image.color = Color::WHITE;
            }
        }
        let display = if loaded { Display::None } else { Display::Flex };
        for child in children.into_iter().flatten() {
            if let Ok(mut node) = captions.get_mut(*child)
                && node.display != display
            {
                node.display = display;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_art_slots_have_unique_sorted_keys_and_a_text_fallback() {
        assert!(SLOTS.windows(2).all(|pair| pair[0].0 < pair[1].0));
        assert!(SLOTS.iter().all(|(key, text)| {
            !text.is_empty()
                && key
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        }));
    }
    #[test]
    fn a_slot_without_its_image_shows_the_text_fallback() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut app = App::new();
        install(&mut app);
        let mut commands = app.world_mut().commands();
        let slot = fixed(&mut commands, &default(), "widget-close", Vec2::splat(12.));
        app.world_mut().flush();
        app.update();
        let art = app
            .world()
            .entity(slot)
            .get::<ArtSlot>()
            .ok_or("art slot component")?;
        assert_eq!(art.path(), "images/game/sidebar/widget-close.png");
        let mut captions = app
            .world_mut()
            .query_filtered::<(&Text, &Node), With<ArtFallback>>();
        let shown: Vec<_> = captions
            .iter(app.world())
            .map(|(text, node)| (text.0.clone(), node.display))
            .collect();
        assert_eq!(shown, [("×".to_owned(), Display::Flex)]);
        Ok(())
    }
}
