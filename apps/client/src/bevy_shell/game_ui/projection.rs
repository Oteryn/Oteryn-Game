//! Projection of retained regions and server-bound item appearances.
use super::*;

pub(in crate::bevy_shell) fn project(
    owner: NonSend<HostOwner>,
    mut roots: Query<&mut Node, With<GameRoot>>,
) {
    let display = if owner.play.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    for mut root in &mut roots {
        if root.display != display {
            root.display = display;
        }
    }
}

#[derive(Component)]
pub(super) struct RenderedItem {
    item: oteryn_session::ItemEntry,
    atlas_size: UVec2,
    available: bool,
    draws: Vec<crate::map::Draw>,
}

pub(super) fn prepare_item_appearances(
    owner: Option<NonSendMut<HostOwner>>,
    icons: Query<&ItemIcon>,
) {
    if let Some(mut owner) = owner
        && let Some(play) = &mut owner.play
    {
        let _ = play
            .view
            .prepare_item_icons(icons.iter().map(|icon| &icon.0));
    }
}

pub(super) fn item_appearances(
    mut commands: Commands,
    skins: Res<Skins>,
    owner: Option<NonSend<HostOwner>>,
    cache: Option<Res<super::super::scene::SceneCache>>,
    mut icons: Query<(Entity, &ItemIcon, &mut ImageNode, Option<&RenderedItem>)>,
) {
    let scene = owner
        .as_ref()
        .and_then(|owner| owner.play.as_ref())
        .map(|play| play.view.scene());
    let atlas_image = cache.as_ref().and_then(|cache| cache.atlas_image());
    for (entity, item, mut image, previous) in &mut icons {
        let draws = scene.and_then(|scene| scene.item_icon(&item.0));
        let source = draws.zip(scene).zip(atlas_image);
        let size = scene.map_or(UVec2::ZERO, |scene| {
            UVec2::new(scene.atlas().width(), scene.atlas().height())
        });
        if previous.is_some_and(|previous| {
            previous.item == item.0
                && previous.atlas_size == size
                && previous.available == source.is_some()
                && previous.draws.as_slice() == draws.unwrap_or(&[])
        }) {
            continue;
        }
        let next = RenderedItem {
            item: item.0,
            atlas_size: size,
            available: source.is_some(),
            draws: draws.unwrap_or(&[]).to_vec(),
        };
        commands.entity(entity).despawn_children();
        image.color = Color::NONE;
        if let Some(((draws, scene), handle)) = source {
            let min = draws.iter().fold([0, 0], |min, draw| {
                [min[0].min(draw.offset[0]), min[1].min(draw.offset[1])]
            });
            let max = draws.iter().fold([32, 32], |max, draw| {
                [
                    max[0].max(draw.offset[0] + 32),
                    max[1].max(draw.offset[1] + 32),
                ]
            });
            let scale = 32. / ((max[0] - min[0]).max(max[1] - min[1]) as f32);
            for (index, draw) in draws.iter().enumerate() {
                let Ok([u0, v0, u1, v1]) = scene.atlas().uv_rect(draw.cell) else {
                    continue;
                };
                let rect = Rect::new(
                    u0 * scene.atlas().width() as f32,
                    v0 * scene.atlas().height() as f32,
                    u1 * scene.atlas().width() as f32,
                    v1 * scene.atlas().height() as f32,
                );
                let node = ImageNode {
                    image: handle.clone(),
                    rect: Some(rect),
                    ..default()
                };
                // Single-cell icons use the root image, with extra cells composed as children.
                if draws.len() == 1 {
                    *image = node;
                } else {
                    let child = commands
                        .spawn((
                            node,
                            Node {
                                position_type: PositionType::Absolute,
                                left: px((draw.offset[0] - min[0]) as f32 * scale),
                                top: px((draw.offset[1] - min[1]) as f32 * scale),
                                width: px(32. * scale),
                                height: px(32. * scale),
                                ..default()
                            },
                            ZIndex(index as i32),
                            Pickable::IGNORE,
                        ))
                        .id();
                    commands.entity(entity).add_child(child);
                }
            }
            commands.entity(entity).insert(Name::new(format!(
                "Item appearance {}",
                item.0.appearance_id
            )));
        } else {
            let label = commands
                .spawn((
                    source::text("?", source::font(skins.font.clone(), false)),
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(10.),
                        top: px(7.),
                        ..default()
                    },
                ))
                .id();
            commands
                .entity(entity)
                .insert(Name::new(format!(
                    "Appearance unavailable, {} items",
                    item.0.count
                )))
                .add_child(label);
        }
        if item.0.count.get() > 1 {
            let label = commands
                .spawn((
                    source::text(
                        item.0.count.to_string(),
                        source::font(skins.font.clone(), false),
                    ),
                    Node {
                        position_type: PositionType::Absolute,
                        right: px(0.),
                        bottom: px(0.),
                        ..default()
                    },
                    ZIndex(100),
                ))
                .id();
            commands.entity(entity).add_child(label);
        }
        commands.entity(entity).insert(next);
    }
}

pub(super) fn project_regions(
    owner: NonSend<HostOwner>,
    regions: Res<GameRegions>,
    mut nodes: Query<&mut Node>,
) {
    for (entity, id) in [
        (regions.battle, "battle"),
        (regions.chat, "chat"),
        (regions.equipment, "inventory"),
        (regions.containers, "inventory"),
        (regions.minimap, "minimap"),
        (regions.vip, "vip"),
    ] {
        if let Ok(mut node) = nodes.get_mut(entity) {
            let visible = owner.settings.preferences.panel_visible(id)
                || id == "vip"
                    && owner
                        .play
                        .as_ref()
                        .is_some_and(|play| play.link.vip_is_open());
            node.display = if visible {
                Display::Flex
            } else {
                Display::None
            };
        }
    }
    if let Ok(mut node) = nodes.get_mut(regions.vitals) {
        node.display = if owner.settings.preferences.status_panel_visible {
            Display::Flex
        } else {
            Display::None
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_item_appearance_builds_one_visible_source_label()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new();
        app.insert_resource(Skins {
            buttons: source::ButtonImages {
                up: default(),
                down: default(),
            },
            checked: default(),
            unchecked: default(),
            frame: default(),
            inset: default(),
            font: default(),
        })
        .add_systems(Update, item_appearances);
        let icon = app
            .world_mut()
            .spawn((
                ItemIcon(oteryn_session::ItemEntry {
                    appearance_id: 0,
                    handle: 7.try_into()?,
                    item_definition_ref: std::num::NonZeroU32::MIN,
                    count: 3.try_into()?,
                    sub_type: 0,
                }),
                ImageNode::default(),
            ))
            .id();
        app.update();
        let icon = app.world().entity(icon);
        assert_eq!(
            icon.get::<ImageNode>().ok_or("icon image")?.color,
            Color::NONE
        );
        assert_eq!(icon.get::<Children>().ok_or("label children")?.len(), 2);
        assert_eq!(
            icon.get::<Name>().ok_or("appearance diagnostic")?.as_str(),
            "Appearance unavailable, 3 items"
        );
        let mut labels = app.world_mut().query::<&Text>();
        let labels: Vec<_> = labels
            .iter(app.world())
            .map(|text| text.0.as_str())
            .collect();
        assert_eq!(labels, ["?", "3"]);
        Ok(())
    }
    #[test]
    fn known_item_appearance_uses_scene_atlas_without_question_mark()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::bevy_shell::{game_ui::viewport::WorldViewport, owner::ActivePlay, scene};
        let item = oteryn_session::ItemEntry {
            appearance_id: 3031,
            handle: 7.try_into()?,
            item_definition_ref: 17.try_into()?,
            count: 3.try_into()?,
            sub_type: 0,
        };
        let mut view =
            crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 0, &[])?;
        view.prime_icon_assets(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
        view.prepare_item_icons(std::iter::once(&item))?;
        let gold = view
            .scene()
            .item_icon(&item)
            .and_then(|draws| draws.first())
            .ok_or("real gold sprite")?;
        let atlas = view.scene().atlas();
        let [u0, v0, u1, v1] = atlas.uv_rect(gold.cell)?;
        let expected_rect = Rect::new(
            u0 * atlas.width() as f32,
            v0 * atlas.height() as f32,
            u1 * atlas.width() as f32,
            v1 * atlas.height() as f32,
        );
        let mut owner = HostOwner::new(None)?;
        let (link, _, _) = crate::play::play_channel();
        owner.play = Some(ActivePlay { view, link });
        let mut app = App::new();
        app.insert_non_send(owner)
            .init_resource::<scene::SceneCache>()
            .init_resource::<Assets<Image>>()
            .init_resource::<Time>()
            .init_resource::<WorldViewport>()
            .insert_resource(Skins {
                buttons: source::ButtonImages {
                    up: default(),
                    down: default(),
                },
                checked: default(),
                unchecked: default(),
                frame: default(),
                inset: default(),
                font: default(),
            })
            .add_systems(
                Update,
                (prepare_item_appearances, scene::project, item_appearances).chain(),
            );
        app.world_mut()
            .spawn((Window::default(), bevy::window::PrimaryWindow));
        let icon = app
            .world_mut()
            .spawn((ItemIcon(item), ImageNode::default()))
            .id();
        app.update();
        let image = app
            .world()
            .entity(icon)
            .get::<ImageNode>()
            .ok_or("icon image")?;
        let atlas = app
            .world()
            .resource::<scene::SceneCache>()
            .atlas_image()
            .ok_or("atlas image")?;
        assert_eq!(&image.image, atlas);
        assert_eq!(image.color, Color::WHITE);
        assert_eq!(image.rect, Some(expected_rect));
        let labels: Vec<_> = app
            .world_mut()
            .query::<&Text>()
            .iter(app.world())
            .map(|text| text.0.as_str())
            .collect();
        assert_eq!(labels, ["3"]);
        Ok(())
    }
}
