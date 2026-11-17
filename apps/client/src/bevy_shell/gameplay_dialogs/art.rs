//! Named Tibia Global art slots; a short caption stands in while the extracted file is absent.
use bevy::prelude::*;

/// One icon from the Tibia Global client sheets, loaded from the UI asset directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::bevy_shell) enum ArtSlot {
    ChatSay,
    ChatWhisper,
    ChatYell,
    ActionBarLocked,
    ActionBarUnlocked,
}
impl ArtSlot {
    /// The file under the UI asset directory.
    pub(in crate::bevy_shell) fn path(self) -> &'static str {
        match self {
            Self::ChatSay => "images/game/console/mode-say.png",
            Self::ChatWhisper => "images/game/console/mode-whisper.png",
            Self::ChatYell => "images/game/console/mode-yell.png",
            Self::ActionBarLocked => "images/game/actionbar/lock-locked.png",
            Self::ActionBarUnlocked => "images/game/actionbar/lock-unlocked.png",
        }
    }
    /// The caption shown while the file is absent.
    pub(in crate::bevy_shell) fn fallback(self) -> &'static str {
        match self {
            Self::ChatSay => "S",
            Self::ChatWhisper => "W",
            Self::ChatYell => "Y",
            Self::ActionBarLocked => "L",
            Self::ActionBarUnlocked => "U",
        }
    }
}

/// A node that shows `0` as an image, or its fallback caption until the image has loaded.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::bevy_shell) struct ArtIcon(pub(in crate::bevy_shell) ArtSlot);
#[derive(Component)]
pub(in crate::bevy_shell) struct ArtFallback;

/// An icon node of `size` logical pixels for `slot`, with its fallback caption.
pub(in crate::bevy_shell) fn icon(
    commands: &mut Commands,
    slot: ArtSlot,
    font: Handle<Font>,
    size: Vec2,
) -> Entity {
    let fallback = commands
        .spawn((
            ArtFallback,
            oteryn_client_ui_bevy::text(slot.fallback(), oteryn_client_ui_bevy::font(font, true)),
        ))
        .id();
    let root = commands
        .spawn((
            ArtIcon(slot),
            ImageNode {
                color: Color::NONE,
                ..default()
            },
            Pickable::IGNORE,
            Node {
                width: px(size.x),
                height: px(size.y),
                flex_shrink: 0.,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(fallback);
    root
}

/// Shows each icon's file once it has loaded, and its caption while the file is absent.
pub(in crate::bevy_shell) fn project(
    assets: Option<Res<AssetServer>>,
    mut handles: Local<std::collections::HashMap<ArtSlot, Handle<Image>>>,
    mut icons: Query<(&ArtIcon, &mut ImageNode, &Children)>,
    mut captions: Query<(&mut Text, &mut Visibility), With<ArtFallback>>,
) {
    for (icon, mut image, children) in &mut icons {
        let loaded = assets.as_ref().and_then(|assets| {
            let handle = handles
                .entry(icon.0)
                .or_insert_with(|| assets.load(icon.0.path()));
            assets.is_loaded(handle.id()).then(|| handle.clone())
        });
        let shown = loaded.is_some();
        if let Some(handle) = loaded {
            if image.image != handle {
                image.image = handle;
            }
            if image.color != Color::WHITE {
                image.color = Color::WHITE;
            }
        } else if image.color != Color::NONE {
            image.color = Color::NONE;
        }
        for child in children {
            if let Ok((mut text, mut visibility)) = captions.get_mut(*child) {
                if text.0 != icon.0.fallback() {
                    text.0 = icon.0.fallback().into();
                }
                let wanted = if shown {
                    Visibility::Hidden
                } else {
                    Visibility::Inherited
                };
                if *visibility != wanted {
                    *visibility = wanted;
                }
            }
        }
    }
}
