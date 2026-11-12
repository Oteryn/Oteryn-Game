//! Opt-in Bevy desktop host over the production client owners.
mod audit;
mod controls;
mod gameplay_dialogs;
mod input;
mod login;
mod owner;
mod scene;
mod selection;
mod ui;

use bevy::{
    asset::AssetPlugin,
    image::ImagePlugin,
    prelude::*,
    window::{PresentMode, PrimaryWindow, WindowMode},
};
use owner::HostOwner;
use std::{ffi::OsString, path::PathBuf};

pub struct Options {
    pub ui_assets: PathBuf,
    pub preferences: Option<PathBuf>,
    pub ui_audit: Option<PathBuf>,
    pub login_settings: Option<PathBuf>,
}
impl Options {
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self, String> {
        let mut args = args.into_iter();
        let mut ui_assets = None;
        let mut preferences = crate::play::PreferenceStore::path();
        let mut ui_audit = None;
        let mut login_settings = crate::native_login::settings_path().ok();
        while let Some(argument) = args.next() {
            let target =
                match argument.to_str() {
                    Some("--ui-assets") => &mut ui_assets,
                    Some("--preferences") => &mut preferences,
                    Some("--ui-audit") => &mut ui_audit,
                    Some("--login-settings") => &mut login_settings,
                    _ => return Err(
                        "Use --ui-assets <directory> [--preferences <path>] [--ui-audit <path>] [--login-settings <path>]"
                            .into(),
                    ),
                };
            let value = args
                .next()
                .filter(|v| !v.is_empty())
                .ok_or("Missing option path")?;
            *target = Some(PathBuf::from(value));
        }
        let ui_assets = ui_assets
            .ok_or("--ui-assets <directory> is required; source skins/fonts are external")?
            .canonicalize()
            .map_err(|_| "Cannot open --ui-assets directory")?;
        ui::validate_assets(&ui_assets)?;
        Ok(Self {
            ui_assets,
            preferences,
            ui_audit,
            login_settings,
        })
    }
}

/// Called only by the executable. Headless tests never install WinitPlugin or run this loop.
pub fn run(args: impl IntoIterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let options = Options::parse(args)?;
    let mut owner = HostOwner::new(options.preferences)?;
    owner.configure_login(options.login_settings, |name| std::env::var(name).ok());
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: options.ui_assets.to_string_lossy().into_owned(),
                ..default()
            })
            .set(ImagePlugin::default_nearest())
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Oteryn — Bevy host (opt-in)".into(),
                    resolution: (1024, 720).into(),
                    ..default()
                }),
                ..default()
            }),
    )
    .add_plugins((
        oteryn_client_ui_bevy::GlobalUiPlugin,
        bevy::input_focus::tab_navigation::TabNavigationPlugin,
    ))
    .insert_resource(ClearColor(Color::BLACK))
    .insert_non_send(owner)
    .insert_resource(audit::AuditOutput::new(options.ui_audit))
    .init_resource::<scene::SceneCache>()
    .add_systems(Startup, (ui::setup, login::setup, selection::setup).chain())
    .add_systems(PreUpdate, selection::poll.before(poll_owner))
    .add_systems(PreUpdate, poll_owner)
    .add_systems(
        Update,
        (
            login::keyboard,
            login::project,
            selection::keyboard,
            selection::project,
            ui::keyboard,
            ui::project,
            project_window,
            scene::project,
        )
            .chain(),
    )
    .add_systems(PostUpdate, audit::write.after(bevy::ui::UiSystems::Layout));
    app.init_non_send::<input::NativeInput>().add_systems(
        Update,
        input::route
            .before(login::keyboard)
            .before(selection::keyboard)
            .before(ui::keyboard)
            .before(ui::project),
    );
    gameplay_dialogs::install(&mut app);
    // App::run transfers the app into its runner. HostOwner::drop owns shutdown there.
    match app.run() {
        AppExit::Success => Ok(()),
        AppExit::Error(code) => Err(format!("Bevy event loop exited with {code}").into()),
    }
}
fn poll_owner(mut owner: NonSendMut<HostOwner>, window: Query<&Window, With<PrimaryWindow>>) {
    if let Ok(window) = window.single() {
        owner.poll([window.physical_width(), window.physical_height()]);
    }
}
fn project_window(
    owner: NonSend<HostOwner>,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
    mut scale: ResMut<UiScale>,
) {
    if let Ok(mut window) = window.single_mut() {
        apply_window(&owner.settings.preferences, &mut window);
    }
    let next = owner.settings.preferences.interface_scale as f32 / 100.;
    if scale.0 != next {
        scale.0 = next;
    }
}
fn apply_window(preferences: &crate::play::ReferencePreferences, window: &mut Window) {
    let mode = if preferences.graphics.fullscreen {
        WindowMode::BorderlessFullscreen(bevy::window::MonitorSelection::Current)
    } else {
        WindowMode::Windowed
    };
    let present = if preferences.graphics.vsync {
        PresentMode::AutoVsync
    } else {
        PresentMode::AutoNoVsync
    };
    if window.mode != mode {
        window.mode = mode;
    }
    if window.present_mode != present {
        window.present_mode = present;
    }
}

#[cfg(test)]
mod layout_tests;
