//! Party combat rows consume capability 461; unknown/out-of-range vitals stay absent.
// Global behaviour: docs/reference/tibia-manual/interface.md §3.6.3.
// Structure reference: edubart/otclient@dab86e610991092fe8b2471f7de55a7c986b3cd2,
// modules/game_battle/battle.lua (filtering and health-driven list updates).
use super::*;
use oteryn_client_ui::party::{ListSort, party::PartyListEntity};
use std::collections::BTreeMap;
#[derive(Resource, Default)]
pub(super) struct Artwork {
    pub(super) assets: Option<super::super::outfit::preview::Assets>,
    pub(super) attempted: bool,
}
impl Artwork {
    pub(super) fn images(
        &mut self,
        panel: &PartyPanel,
        images: &mut Assets<Image>,
    ) -> BTreeMap<EntityRef, Handle<Image>> {
        if !self.attempted {
            self.attempted = true;
            self.assets = std::env::var_os(crate::map::ASSET_DIR_ENV).and_then(|root| {
                super::super::outfit::preview::Assets::load(std::path::Path::new(&root)).ok()
            });
        }
        let mut result = BTreeMap::new();
        for (_, row) in panel.list_rows() {
            let Some(oteryn_session::Look::Outfit {
                look_type, addons, ..
            }) = row.portrait.as_ref().map(|binding| &binding.look)
            else {
                continue;
            };
            if let Some(Ok(mut pixels)) = self.assets.as_mut().map(|assets| {
                assets.draw(&oteryn_client_ui::appearance::Preview {
                    look_type: *look_type,
                    addons: *addons,
                    colours: row.portrait_colours,
                })
            }) {
                // The shared native outfit compositor returns BGRA.
                for pixel in pixels.as_chunks_mut::<4>().0 {
                    pixel.swap(0, 2);
                    pixel[3] = 255;
                }
                result.insert(
                    row.entity,
                    images.add(Image::new(
                        bevy::render::render_resource::Extent3d {
                            width: 96,
                            height: 96,
                            depth_or_array_layers: 1,
                        },
                        bevy::render::render_resource::TextureDimension::D2,
                        pixels,
                        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                        bevy::asset::RenderAssetUsages::default(),
                    )),
                );
            }
        }
        result
    }
}
pub(super) fn controls(
    commands: &mut Commands,
    skin: &Skins,
    root: Entity,
    generation: u64,
    serial: u64,
    panel: &PartyPanel,
) {
    use super::widgets::{button, row};
    for (line_index, groups) in [
        "Players",
        "Knights",
        "Paladins",
        "Druids",
        "Sorcerers",
        "Monks",
        "Summons",
    ]
    .chunks(2)
    .enumerate()
    {
        let line = row(commands, root);
        for (index, title) in groups.iter().enumerate() {
            let group = line_index * 2 + index;
            let entry = button(
                commands,
                skin,
                line,
                Control {
                    generation,
                    serial,
                    kind: Kind::Filter(group),
                },
                title,
                78.,
                true,
            );
            if !panel.list_hidden[group] {
                commands.entity(entry).insert(bevy::ui::Checked);
            }
        }
    }
    let line = row(commands, root);
    for (sort, title) in [
        (ListSort::Time, "Time"),
        (ListSort::Distance, "Dist"),
        (ListSort::Health, "HP"),
        (ListSort::Name, "Name"),
    ] {
        let entry = button(
            commands,
            skin,
            line,
            Control {
                generation,
                serial,
                kind: Kind::Sort(sort),
            },
            title,
            38.,
            true,
        );
        if panel.list_sort == sort {
            commands.entity(entry).insert(bevy::ui::Checked);
        }
    }
}
pub(super) fn row(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    row: &PartyListEntity,
    image: Option<&Handle<Image>>,
    name: &str,
) -> Entity {
    use super::widgets::label;
    commands.entity(parent).insert((
        Name::new("party.combat-row"),
        Node {
            width: percent(100.),
            min_height: px(38.),
            flex_shrink: 0.,
            padding: px(2.).all(),
            column_gap: px(2.),
            overflow: Overflow::clip(),
            ..default()
        },
    ));
    if let Some(image) = image {
        let portrait = commands
            .spawn((
                Name::new("party.portrait"),
                ImageNode {
                    image: image.clone(),
                    rect: Some(Rect::new(32., 32., 96., 96.)),
                    ..default()
                },
                Node {
                    width: px(32.),
                    height: px(32.),
                    flex_shrink: 0.,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(parent).add_child(portrait);
    }
    let details = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.,
            min_width: px(0.),
            row_gap: px(2.),
            ..default()
        })
        .id();
    commands.entity(parent).add_child(details);
    let caption = label(
        commands,
        skin,
        details,
        &format!("{}{name}", if row.summon { "↳ " } else { "" }),
    );
    if row.distance <= 30 {
        for (value, name, colour) in [
            (
                row.health_percent,
                "party.health",
                Color::srgb_u8(0, 170, 0),
            ),
            (row.mana_percent, "party.mana", Color::srgb_u8(0, 100, 220)),
        ] {
            if let Some(value) = value {
                let bar = commands
                    .spawn((
                        Name::new(name),
                        Node {
                            width: percent(100.),
                            height: px(4.),
                            ..default()
                        },
                        BackgroundColor(Color::srgb_u8(48, 48, 48)),
                        Pickable::IGNORE,
                    ))
                    .id();
                let fill = commands
                    .spawn((
                        Name::new(format!("{name}.fill")),
                        Node {
                            width: percent(f32::from(value.min(100))),
                            height: percent(100.),
                            ..default()
                        },
                        BackgroundColor(colour),
                        Pickable::IGNORE,
                    ))
                    .id();
                commands.entity(bar).add_child(fill);
                commands.entity(details).add_child(bar);
            }
        }
        if let Some(skull) = row.skull.filter(|skull| *skull != 0) {
            let title = match skull {
                1 => "Yellow skull",
                2 => "Green skull",
                3 => "White skull",
                4 => "Red skull",
                5 => "Black skull",
                _ => "Orange skull",
            };
            let badge = label(commands, skin, details, title);
            commands.entity(badge).insert(Name::new("party.skull"));
        }
    }
    caption
}
