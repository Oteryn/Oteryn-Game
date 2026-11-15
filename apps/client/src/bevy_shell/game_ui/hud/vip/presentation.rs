//! Local list preferences and actions over authoritative contacts and presence.
use super::*;
use bevy::{
    asset::RenderAssetUsages,
    picking::{events::PointerClick, pointer::PointerButton},
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use oteryn_session::VipPresenceStatus;

#[derive(Clone, Copy, Default)]
pub(super) enum Sort {
    Name,
    #[default]
    Status,
    Icon,
}
impl Sort {
    pub fn next(self) -> Self {
        match self {
            Self::Name => Self::Status,
            Self::Status => Self::Icon,
            Self::Icon => Self::Name,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Name => "Sort: Name",
            Self::Status => "Sort: Status",
            Self::Icon => "Sort: Icon",
        }
    }
}
#[derive(Resource, Default)]
pub(super) struct Preferences {
    pub sort: Sort,
    pub hide_offline: bool,
    pub context: Option<(u64, u64, u64)>,
    atlas: Handle<Image>,
}
#[derive(Component)]
pub(super) struct Icon(pub u64);
#[derive(Component)]
pub(super) struct Caption(bool);

// Original 5x7 pixel artwork: blank, heart, star, cross, shield, sword, flag,
// skull, crown, house, key, coin, diamond, sun, moon, tree, cup, book, bolt, eye.
const ART: [[u8; 7]; 20] = [
    [0; 7],
    [10, 31, 31, 31, 14, 14, 4],
    [4, 4, 31, 14, 14, 17, 0],
    [4, 4, 31, 4, 4, 4, 0],
    [31, 17, 17, 17, 10, 10, 4],
    [1, 3, 6, 28, 12, 10, 16],
    [16, 30, 31, 30, 16, 16, 16],
    [14, 31, 21, 31, 14, 14, 10],
    [17, 21, 31, 31, 31, 0, 0],
    [4, 14, 31, 17, 21, 21, 31],
    [12, 18, 18, 12, 4, 6, 4],
    [14, 17, 21, 21, 21, 17, 14],
    [4, 14, 31, 31, 14, 4, 0],
    [4, 21, 14, 31, 14, 21, 4],
    [6, 12, 24, 24, 24, 12, 6],
    [4, 14, 31, 14, 31, 4, 4],
    [31, 21, 21, 14, 4, 4, 14],
    [31, 21, 21, 31, 21, 21, 31],
    [6, 12, 8, 31, 2, 6, 12],
    [0, 14, 17, 21, 17, 14, 0],
];
pub(super) fn setup(
    mut commands: Commands,
    skins: Res<Skins>,
    regions: Res<super::super::super::GameRegions>,
    mut images: ResMut<Assets<Image>>,
    mut preferences: ResMut<Preferences>,
) {
    let mut pixels = vec![0; 240 * 12 * 4];
    for (icon, art) in ART.iter().enumerate() {
        for (y, bits) in art.iter().enumerate() {
            for x in 0..5 {
                if bits & (1 << (4 - x)) != 0 {
                    let offset = ((y + 2) * 240 + icon * 12 + x * 2 + 1) * 4;
                    pixels[offset..offset + 4].copy_from_slice(&[240, 200, 96, 255]);
                    pixels[offset + 4..offset + 8].copy_from_slice(&[240, 200, 96, 255]);
                }
            }
        }
    }
    preferences.atlas = images.add(Image::new(
        Extent3d {
            width: 240,
            height: 12,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ));
    let root = commands
        .spawn(Node {
            column_gap: px(3.),
            ..default()
        })
        .id();
    for (label, action, id, width, hide) in [
        ("Sort: Status", Action::Sort, "vip.sort", 80., false),
        (
            "Hide offline",
            Action::HideOffline,
            "vip.hide_offline",
            80.,
            true,
        ),
    ] {
        let entity = button(
            &mut commands,
            &skins,
            Control {
                generation: 0,
                revision: 0,
                command: action,
            },
            label,
            width,
            id,
        );
        let caption = commands
            .spawn((
                Caption(hide),
                source::button_caption(label, skins.font.clone()),
            ))
            .id();
        commands
            .entity(entity)
            .despawn_children()
            .add_child(caption);
        commands.entity(root).add_child(entity);
    }
    commands.entity(regions.vip).add_child(root);
}

pub(super) fn click(
    event: On<PointerClick>,
    guard: ModalGuard,
    controls: Query<&Control>,
    owner: NonSend<HostOwner>,
    mut preferences: ResMut<Preferences>,
) {
    if !guard.allows(event.entity) || !available(&owner) {
        return;
    }
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    let panel = play.link.vip_panel();
    if control.generation != owner.generation()
        || control.revision != panel.revision
        || panel.pending
    {
        return;
    }
    let Action::Edit(entry) = control.command else {
        return;
    };
    if event.button == PointerButton::Secondary {
        preferences.context = Some((owner.generation(), panel.revision, entry));
    }
}

// Independent ECS roles: list ordering, artwork, captions and the contextual action strip.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    regions: Res<super::super::super::GameRegions>,
    mut preferences: ResMut<Preferences>,
    mut rows: Query<(Entity, &Row, &mut Node)>,
    lists: Query<(Entity, &Children), With<List>>,
    mut icons: Query<(&Icon, &mut ImageNode)>,
    mut captions: Query<(&Caption, &mut Text)>,
    mut menu: Local<Option<((u64, u64, u64), Entity)>>,
) {
    let Some(play) = &owner.play else {
        preferences.context = None;
        if let Some((_, entity)) = menu.take() {
            commands.entity(entity).despawn();
        }
        return;
    };
    let panel = play.link.vip_panel();
    let snapshot = Snapshot::read(&panel).ok();
    let presence = play.link.vip_presence_panel();
    let status = |entry| {
        presence
            .presence
            .entries
            .iter()
            .find(|row| row.entry == entry)
            .map(|row| row.status)
    };
    let rank = |entry| match status(entry) {
        Some(VipPresenceStatus::Online) => 0,
        Some(VipPresenceStatus::Offline) => 2,
        _ => 1,
    };
    let mut order = Vec::new();
    for (entity, row, mut node) in &mut rows {
        let Some(contact) = snapshot
            .as_ref()
            .and_then(|s| s.entries.iter().find(|c| c.entry == row.entry))
        else {
            continue;
        };
        node.display =
            if preferences.hide_offline && status(row.entry) == Some(VipPresenceStatus::Offline) {
                Display::None
            } else {
                Display::Flex
            };
        order.push((entity, contact));
    }
    order.sort_by(|(_, a), (_, b)| {
        let primary = match preferences.sort {
            Sort::Name => std::cmp::Ordering::Equal,
            Sort::Status => rank(a.entry).cmp(&rank(b.entry)),
            Sort::Icon => b.icon.cmp(&a.icon),
        };
        primary
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.entry.cmp(&b.entry))
    });
    for (list, children) in &lists {
        let mut next: Vec<_> = children
            .iter()
            .filter(|entity| rows.get(*entity).is_err())
            .collect();
        next.extend(order.iter().map(|(entity, _)| *entity));
        if children.iter().ne(next.iter().copied()) {
            commands.entity(list).replace_children(&next);
        }
    }
    for (icon, mut image) in &mut icons {
        let id = snapshot
            .as_ref()
            .and_then(|s| s.entries.iter().find(|c| c.entry == icon.0))
            .and_then(|c| c.icon)
            .filter(|id| *id <= 19);
        image.image = preferences.atlas.clone();
        image.rect = id.map(|id| Rect::new(f32::from(id) * 12., 0., f32::from(id + 1) * 12., 12.));
        image.color = if id.is_some() {
            Color::WHITE
        } else {
            Color::NONE
        };
    }
    for (caption, mut text) in &mut captions {
        text.0 = if caption.0 {
            if preferences.hide_offline {
                "Show offline"
            } else {
                "Hide offline"
            }
        } else {
            preferences.sort.label()
        }
        .into();
    }
    if preferences
        .context
        .is_some_and(|(generation, revision, entry)| {
            generation != owner.generation()
                || revision != panel.revision
                || panel.pending
                || !available(&owner)
                || !snapshot
                    .as_ref()
                    .is_some_and(|s| s.entries.iter().any(|c| c.entry == entry))
        })
    {
        preferences.context = None;
    }
    if menu.as_ref().map(|(key, _)| *key) != preferences.context {
        if let Some((_, entity)) = menu.take() {
            commands.entity(entity).despawn();
        }
        if let Some((generation, revision, entry)) = preferences.context {
            let root = commands
                .spawn((
                    Name::new("vip.menu"),
                    Node {
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: px(3.),
                        ..default()
                    },
                ))
                .id();
            for (label, command) in [
                ("Message", Action::Message(entry)),
                ("Edit", Action::Edit(entry)),
                ("Remove", Action::Remove(entry)),
                ("Add", Action::Add),
                ("Close", Action::MenuClose),
            ] {
                let entity = button(
                    &mut commands,
                    &skins,
                    Control {
                        generation,
                        revision,
                        command,
                    },
                    label,
                    45.,
                    "vip.menu.action",
                );
                commands.entity(root).add_child(entity);
            }
            commands.entity(regions.vip).add_child(root);
            *menu = Some(((generation, revision, entry), root));
        }
    }
}
