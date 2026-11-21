use bevy::{
    prelude::*,
    text::{FontHinting, FontSize},
    ui::{Checked, InteractionDisabled, Pressed},
    ui_widgets::{Button, Checkbox},
};
use oteryn_client_ui_bevy as ui;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn native_button_projects_checked_pressed_release_and_disabled_without_hover_tint() -> TestResult {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, ui::GlobalUiPlugin));
    let mut images = Assets::<Image>::default();
    let mut source = Image::default();
    source.texture_descriptor.size.width = 98;
    source.texture_descriptor.size.height = 40;
    let up = images.add(source.clone());
    let down = images.add(source);
    app.insert_resource(images);
    let button = app
        .world_mut()
        .spawn(ui::button(
            ui::ButtonImages {
                up: up.clone(),
                down: down.clone(),
            },
            43.,
        ))
        .id();
    app.world_mut()
        .entity_mut(button)
        .insert(bevy::ui::ComputedNode {
            size: Vec2::new(43., 20.),
            border: BorderRect::all(1.),
            ..default()
        });
    let caption = app
        .world_mut()
        .spawn(ui::button_caption("Apply", Handle::default()))
        .id();
    let icon = app
        .world_mut()
        .spawn((ui::ButtonIcon::Left(6.), Node::default()))
        .id();
    app.world_mut()
        .entity_mut(button)
        .add_children(&[caption, icon]);
    assert!(app.world().get::<Button>(button).is_some());
    assert_eq!(
        app.world()
            .get::<TextFont>(caption)
            .ok_or("font")?
            .font_size,
        FontSize::Px(8.)
    );
    assert!(matches!(
        app.world().get::<FontHinting>(caption),
        Some(FontHinting::Enabled)
    ));
    for (pressed, checked, disabled) in [
        (false, false, false),
        (true, false, false),
        (false, true, false),
        (true, true, false),
        (false, true, true),
        (false, false, true),
    ] {
        let mut entity = app.world_mut().entity_mut(button);
        entity.remove::<(Pressed, Checked, InteractionDisabled)>();
        if pressed {
            entity.insert(Pressed);
        }
        if checked {
            entity.insert(Checked);
        }
        if disabled {
            entity.insert(InteractionDisabled);
        }
        // A previous hover tint must not survive projection.
        entity.get_mut::<ImageNode>().ok_or("image")?.color = Color::srgb(1.15, 1.15, 1.15);
        app.update();
        let image = app.world().get::<ImageNode>(button).ok_or("image")?;
        assert_eq!(
            image.image,
            if pressed || checked {
                down.clone()
            } else {
                up.clone()
            }
        );
        // The source holder is invisible; fixed-pixel child images draw the chrome.
        assert_eq!(image.color, Color::NONE);
        assert_eq!(
            app.world()
                .get::<ui::Frame>(button)
                .ok_or("native frame")?
                .insets,
            [1.; 4]
        );
        let painted = app
            .world()
            .get::<Children>(button)
            .ok_or("button children")?
            .iter()
            .filter_map(|e| app.world().get::<ImageNode>(e))
            .collect::<Vec<_>>();
        assert_eq!(painted.len(), 9);
        for piece in painted {
            assert_eq!(
                piece.image,
                if pressed || checked {
                    down.clone()
                } else {
                    up.clone()
                }
            );
            assert_eq!(piece.color, Color::WHITE);
            assert!(matches!(
                piece.image_mode,
                bevy::ui::widget::NodeImageMode::Tiled { .. }
            ));
        }
        let offset = if pressed || checked { 1. } else { 0. };
        let node = app.world().get::<Node>(caption).ok_or("caption geometry")?;
        assert_eq!((node.left, node.top), (px(offset), px(offset)));
        assert_eq!(
            app.world().get::<Node>(icon).ok_or("icon")?.left,
            px(6. + offset)
        );
        assert_eq!(
            app.world().get::<TextColor>(caption).ok_or("color")?.0,
            if disabled {
                ui::DISABLED_TEXT_COLOR
            } else {
                ui::TEXT_COLOR
            }
        );
    }
    Ok(())
}

fn pixels(value: Val) -> Result<f32, Box<dyn std::error::Error>> {
    match value {
        Val::Px(v) => Ok(v),
        _ => Err("expected source pixel dimension".into()),
    }
}

#[test]
fn expanded_options_groups_fit_one_outer_frame_with_no_accumulating_insets() -> TestResult {
    // Controls / Interface / Graphics / Sound / Misc. Only the active group expands.
    let counts = [4, 6, 2, 3, 4];
    for active in 0..counts.len() {
        let outer = ui::navigation_outer();
        let inset = pixels(outer.border.left)? + pixels(outer.padding.left)?;
        let available = pixels(outer.width)? - 2. * inset;
        let mut total_height = 0.;
        for (index, count) in counts.into_iter().enumerate() {
            let group = ui::navigation_group(if index == active { count } else { 1 });
            assert_eq!(pixels(group.width)?, available);
            let row_width = pixels(group.width)? - 2. * pixels(group.border.left)?;
            let mut world = World::new();
            let row = world
                .spawn(ui::button(
                    ui::ButtonImages {
                        up: Handle::default(),
                        down: Handle::default(),
                    },
                    row_width,
                ))
                .id();
            let row_node = world.get::<Node>(row).ok_or("native row")?;
            let rows = if index == active { count } else { 1 };
            assert_eq!(
                pixels(group.height)? - 2. * pixels(group.border.top)?,
                pixels(row_node.height)? * rows as f32
            );
            total_height += pixels(group.height)?;
        }
        total_height += (counts.len() - 1) as f32 * pixels(outer.row_gap)?;
        assert!(total_height <= pixels(outer.height)? - 2. * inset);
    }
    assert_eq!(
        ui::navigation_arrow(true, true, false, false),
        Some(ui::NavigationArrow::Down)
    );
    assert_eq!(ui::navigation_arrow(true, true, true, false), None);
    assert_eq!(
        ui::navigation_arrow(false, true, true, true),
        Some(ui::NavigationArrow::Right)
    );
    assert_eq!(ui::navigation_arrow(true, false, false, false), None);
    Ok(())
}

#[test]
fn console_option_keeps_native_checkbox_and_equal_indicator_insets() -> TestResult {
    let mut world = World::new();
    let row = world.spawn(ui::checkbox(ui::checkbox_row(true))).id();
    let indicator = world.spawn(ui::checkbox_indicator(Handle::default())).id();
    world.entity_mut(row).add_child(indicator);
    assert!(world.get::<Checkbox>(row).is_some());
    assert!(world.get::<Button>(row).is_none());
    let node = world.get::<Node>(row).ok_or("row")?;
    let check = world.get::<Node>(indicator).ok_or("indicator")?;
    let horizontal_inset = pixels(node.border.left)? + pixels(node.padding.left)?;
    let vertical_inset = (pixels(node.min_height)? - pixels(check.height)?) / 2.;
    assert_eq!(horizontal_inset, vertical_inset);
    assert_eq!(pixels(check.width)?, pixels(check.height)?);
    assert_eq!(horizontal_inset, 5.);
    Ok(())
}
