use super::tests::{laid_out_app_with, rect};
use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn scaled_context_menu_keeps_its_last_entry_inside_the_window() -> TestResult {
    check_scaled_context_menus(bevy::text::DEFAULT_FONT_DATA, None)
}

#[test]
fn scaled_context_menus_fit_production_skin_font_or_wider_fixture() -> TestResult {
    if let Some(root) = std::env::var_os("OTERYN_UI_ASSETS") {
        let font = std::fs::read(std::path::Path::new(&root).join("fonts/verdanab.ttf"))?;
        check_scaled_context_menus(&font, None)
    } else {
        // At 13 px Bevy's bundled Fira Mono is wider on these menu captions than
        // the production 11 px Verdana Bold, including "Communication Lists".
        check_scaled_context_menus(bevy::text::DEFAULT_FONT_DATA, Some(13.))
    }
}

fn check_scaled_context_menus(font_bytes: &[u8], fixture_size: Option<f32>) -> TestResult {
    use super::super::{actions, chat};
    use crate::bevy_shell::gameplay_dialogs::popup;
    use bevy::ui::UiSystems;
    for (pointer, forced, speech) in [
        (Vec2::new(1190., 715.), false, true),
        (Vec2::new(1190., 715.), true, false),
        (Vec2::new(150., 90.), false, false),
    ] {
        let mut app = laid_out_app_with(1200, 720, |app| {
            app.add_plugins(bevy::text::TextPlugin)
                .add_message::<bevy::input::keyboard::KeyboardInput>()
                .add_systems(
                    PostUpdate,
                    bevy::ui::widget::measure_text_system
                        .after(bevy::text::detect_text_needs_rerender)
                        .in_set(UiSystems::Content),
                );
            actions::install(app);
            chat::install(app);
        })?;
        app.insert_resource(UiScale(1.5))
            .add_systems(Update, popup::layout);
        let font = app.world().resource::<Skins>().font.clone();
        assert_eq!(
            font.path().map(|path| path.path()),
            Some(std::path::Path::new("fonts/verdanab.ttf"))
        );
        app.world_mut()
            .resource_mut::<Assets<Font>>()
            .insert(font.id(), Font::from_bytes(font_bytes.to_vec()))?;
        let target = if speech {
            let owner = app.world().non_send::<HostOwner>();
            crate::play::feature_0007_chat::protocol::edit_chat_fixture(
                &owner.play.as_ref().ok_or("play")?.link,
                |panel| {
                    panel.selected = true;
                    panel.lines.push(oteryn_session::ChatLine::Private {
                        speaker_name: "Alice".into(),
                        text: "bounds fixture".into(),
                    });
                },
            );
            app.update();
            app.update();
            app.world_mut()
                .query::<(Entity, &bevy::text::EditableText)>()
                .iter(app.world())
                .find_map(|(entity, text)| {
                    text.value()
                        .to_string()
                        .contains("bounds fixture")
                        .then_some(entity)
                })
                .ok_or("transcript row")?
        } else {
            actions::tests::slot(&mut app, 0)?
        };
        let mut click = actions::tests::secondary_click(target);
        click.pointer = bevy::picking::events::Pointer::new(
            bevy::picking::pointer::PointerId::Mouse,
            bevy::picking::pointer::Location {
                target: bevy::camera::NormalizedRenderTarget::None {
                    width: 1200,
                    height: 720,
                },
                position: pointer,
            },
        );
        app.world_mut().trigger(click);
        app.world_mut().flush();
        let last = if speech {
            "Mirror This Tab"
        } else {
            "Reset Zoom"
        };
        let (caption, entry) = app
            .world_mut()
            .query::<(Entity, &Text, &ChildOf)>()
            .iter(app.world())
            .find_map(|(entity, text, parent)| {
                (text.0 == last
                    && app
                        .world()
                        .get::<popup::PopupEntry>(parent.parent())
                        .is_some())
                .then_some((entity, parent.parent()))
            })
            .ok_or("final menu entry")?;
        let frame = app
            .world()
            .get::<ChildOf>(entry)
            .ok_or("popup frame")?
            .parent();
        let scrim = app
            .world_mut()
            .query_filtered::<Entity, With<popup::PopupScrim>>()
            .single(app.world())?;
        if let Some(size) = fixture_size {
            let mut labels = app.world_mut().query::<(Entity, &ChildOf, &TextFont)>();
            let labels: Vec<_> = labels
                .iter(app.world())
                .filter(|(_, parent, _)| {
                    app.world()
                        .get::<popup::PopupEntry>(parent.parent())
                        .is_some()
                })
                .map(|(entity, _, _)| entity)
                .collect();
            for label in labels {
                app.world_mut()
                    .get_mut::<TextFont>(label)
                    .ok_or("menu label font")?
                    .font_size = bevy::text::FontSize::Px(size);
            }
        }
        if forced {
            app.world_mut().get_mut::<Node>(frame).ok_or("frame")?.width = px(150.);
        }
        app.update();
        let caption_font = &app
            .world()
            .get::<TextFont>(caption)
            .ok_or("caption font")?
            .font;
        let bevy::text::FontSource::Handle(handle) = caption_font else {
            return Err("popup caption must use the skin font handle".into());
        };
        assert_eq!(handle.id(), font.id());
        let fonts = app.world().resource::<Assets<Font>>();
        assert!(
            fonts.get(handle).ok_or("loaded popup font")?.data.as_ref() == font_bytes,
            "popup caption must be measured with the supplied font bytes"
        );
        let menu = rect(&app, frame)?;
        assert!(menu.width() > 0. && menu.height() > 0.);
        let last = rect(&app, entry)?;
        assert!(menu.contains(last.min) && menu.contains(last.max));
        let mut labels = app
            .world_mut()
            .query::<(Entity, &ChildOf, &bevy::text::ComputedTextBlock)>();
        for (label, parent, block) in labels.iter(app.world()) {
            if app
                .world()
                .get::<popup::PopupEntry>(parent.parent())
                .is_some()
            {
                let intrinsic = block.buffer().calculate_content_widths().max;
                let bounds = rect(&app, label)?;
                assert!(
                    intrinsic > 0. && bounds.width() + 1. >= intrinsic,
                    "{intrinsic} in {bounds:?}"
                );
                assert!(menu.contains(bounds.min) && menu.contains(bounds.max));
            }
        }
        assert!(rect(&app, caption)?.width() > 40.);
        assert!(menu.max.x <= 1200. && menu.max.y <= 720., "{menu:?}");
        assert_eq!(rect(&app, scrim)?, Rect::new(0., 0., 1200., 720.));
        assert_eq!(menu.min, pointer.min(Vec2::new(1200., 720.) - menu.size()));
        if forced {
            assert!(menu.width() >= 225.);
        }
    }
    Ok(())
}
