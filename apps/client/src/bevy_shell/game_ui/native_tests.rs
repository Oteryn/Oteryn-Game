//! Explicit native qualification fixture; never compiled into the shipped client.
use crate::bevy_shell::{
    self,
    owner::{ActivePlay, HostOwner, Phase},
};
use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, ScreenshotCaptured},
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Resource)]
struct Capture {
    path: PathBuf,
    started: Instant,
    requested: bool,
    result: Option<Result<(), String>>,
}
#[test]
fn native_game_shell_renders_retained_owners() -> Result<(), Box<dyn std::error::Error>> {
    let Some(assets) = std::env::var_os("OTERYN_BEVY_NATIVE_FIXTURE_ASSETS") else {
        return Ok(());
    };
    let output = std::env::var_os("OTERYN_BEVY_NATIVE_FIXTURE_OUTPUT")
        .ok_or("native fixture output path missing")?;
    let path = PathBuf::from(output);
    let mut owner = HostOwner::new(Some(path.with_extension("preferences.json")))?;
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.set_preference_character([7; 16]);
    view.use_shared_frontend();
    let (link, _commands, outbox) = crate::play::play_channel();
    link.use_shared_frontend();
    crate::play::feature_0007_chat::protocol::bevy_chat_fixture(
        &outbox,
        oteryn_session::ChatRoomSet::default(),
        Vec::new(),
    );
    outbox.bevy_hud_fixture(
        &[],
        Some(oteryn_session::ActorVitals {
            health: 42,
            max_health: 100,
            mana: 10,
            max_mana: 20,
            ..default()
        }),
    );
    outbox.bevy_hud_skills_fixture()?;
    crate::play::PlayLink::bevy_items_fixture(
        &outbox,
        Some(oteryn_session::CharacterInventory {
            main_backpack: Some(oteryn_session::ItemEntry {
                handle: 99.try_into()?,
                item_definition_ref: std::num::NonZeroU32::MIN,
                count: std::num::NonZeroU32::MIN,
                sub_type: 0,
            }),
            entries: vec![oteryn_session::ItemEntry {
                handle: 7.try_into()?,
                item_definition_ref: 3.try_into()?,
                count: 3.try_into()?,
                sub_type: 0,
            }],
            equipment: vec![],
        }),
        Some(Default::default()),
        true,
        true,
    );
    owner.play = Some(ActivePlay { view, link });
    owner.phase = Phase::Playing;
    let options = bevy_shell::Options {
        ui_assets: PathBuf::from(assets),
        preferences: None,
        login_settings: None,
        ui_audit: None,
    };
    let mut app = bevy_shell::build_app(options, owner);
    app.insert_resource(Capture {
        path: path.clone(),
        started: Instant::now(),
        requested: false,
        result: None,
    })
    .add_systems(PostUpdate, capture.after(bevy::ui::UiSystems::PostLayout));
    let result = app.run();
    if !matches!(result, AppExit::Success) {
        return Err("native game shell failed".into());
    }
    // The screenshot observer writes before sending AppExit. A native timeout never creates it.
    if !path.is_file() {
        return Err("native game shell capture missing".into());
    }
    Ok(())
}
fn capture(
    mut commands: Commands,
    mut capture: ResMut<Capture>,
    viewport: Res<super::viewport::WorldViewport>,
    text: Query<&bevy::text::TextLayoutInfo>,
    assets: Res<AssetServer>,
    skins: Res<crate::bevy_shell::ui::Skins>,
    mut exit: MessageWriter<AppExit>,
) {
    if capture.started.elapsed() > Duration::from_secs(45) {
        capture.result = Some(Err("native layout or capture timed out".into()));
        exit.write(AppExit::error());
        return;
    }
    if capture.requested
        || viewport.size.x == 0
        || !assets.is_loaded_with_dependencies(skins.font.id())
        || !assets.is_loaded_with_dependencies(skins.frame.id())
        || !assets.is_loaded_with_dependencies(skins.inset.id())
        || !text.iter().any(|layout| !layout.glyphs.is_empty())
    {
        return;
    }
    capture.requested = true;
    commands.spawn(Screenshot::primary_window()).observe(
        |event: On<ScreenshotCaptured>,
         mut capture: ResMut<Capture>,
         mut exit: MessageWriter<AppExit>| {
            let result = event
                .image
                .clone()
                .try_into_dynamic()
                .map_err(|error| error.to_string())
                .and_then(|image| image.save(&capture.path).map_err(|error| error.to_string()));
            let succeeded = result.is_ok();
            capture.result = Some(result);
            exit.write(if succeeded {
                AppExit::Success
            } else {
                AppExit::error()
            });
        },
    );
}
