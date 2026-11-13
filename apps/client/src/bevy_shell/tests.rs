use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
type TestResult = Result<(), Box<dyn std::error::Error>>;
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> std::io::Result<Self> {
        let path = std::env::temp_dir().join(format!(
            "oteryn-bevy-host-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path)?;
        Ok(Self(path))
    }
    fn preferences(&self) -> PathBuf {
        self.0.join("preferences.json")
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn admitted_view(character: [u8; 16]) -> Result<(PlayView, PlayLink), Box<dyn std::error::Error>> {
    // Test-only fixture. Production install_play is reached solely from Reentry::poll.
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.set_preference_character(character);
    let (link, _commands, _events) = crate::play::play_channel();
    Ok((view, link))
}
#[test]
fn apply_refreshes_accepted_draft_and_reloads_existing_store_cancel_discards() -> TestResult {
    let temp = Temp::new()?;
    let path = temp.preferences();
    let mut host = HostOwner::new(Some(path.clone()))?;
    let before = host.accepted();
    host.open_graphics();
    host.edit(|d| {
        d.fullscreen = !before.fullscreen;
        d.vsync = !before.vsync;
        d.integer_scaling = !before.integer_scaling;
    });
    let edited = host.draft().ok_or("draft")?;
    assert_eq!(host.accepted(), before);
    host.apply(false);
    assert_eq!(host.accepted(), edited);
    assert_eq!(host.draft(), Some(edited));
    assert_eq!(host.settings.feedback, "Applied and saved");
    host.edit(|d| d.vsync = !d.vsync);
    host.cancel();
    assert_eq!(host.accepted(), edited);
    assert_eq!(host.draft(), None);
    let reload = HostOwner::new(Some(path))?;
    assert_eq!(reload.accepted(), edited);
    host.open_graphics();
    host.edit(|d| d.vsync = !d.vsync);
    host.apply(true);
    assert_eq!(host.draft(), None);
    assert_ne!(host.accepted(), edited);
    Ok(())
}
#[test]
fn blocked_save_accepts_live_settings_and_preserves_exact_owner_feedback() -> TestResult {
    let temp = Temp::new()?;
    let path = temp.preferences();
    let mut host = HostOwner::new(Some(path.clone()))?;
    // A directory at the destination reliably refuses the existing owner's atomic save.
    std::fs::create_dir(&path)?;
    host.open_graphics();
    host.edit(|d| d.fullscreen = !d.fullscreen);
    let expected = host.draft().ok_or("draft")?;
    host.apply(false);
    assert_eq!(host.accepted(), expected);
    assert_eq!(host.draft(), Some(expected));
    assert!(
        host.settings
            .feedback
            .starts_with("Applied for this session; could not save:")
    );
    assert!(path.is_dir());
    Ok(())
}
#[test]
fn admission_and_disconnect_discard_drafts_and_restore_actual_character_scope() -> TestResult {
    let temp = Temp::new()?;
    let mut host = HostOwner::new(Some(temp.preferences()))?;
    host.open_graphics();
    host.edit(|d| d.fullscreen = !d.fullscreen);
    let old = host.draft.take().ok_or("old draft")?;
    let (view, link) = admitted_view([1; 16])?;
    assert!(host.install_play(view, link));
    assert_eq!(host.phase, Phase::Playing);
    assert_eq!(host.settings.character_scope(), Some([1; 16]));
    assert_eq!(host.draft(), None);
    let accepted = host.accepted();
    host.draft = Some(old);
    host.apply(false);
    assert_eq!(host.accepted(), accepted);
    assert_eq!(host.draft(), None);
    host.open_graphics();
    host.edit(|d| d.vsync = !d.vsync);
    let generation = host.generation();
    host.leave("Disconnected.");
    assert!(host.play.is_none());
    assert_eq!(host.draft(), None);
    assert_eq!(host.settings.character_scope(), None);
    assert!(host.generation() > generation);
    assert_eq!(host.phase, Phase::Offline);
    Ok(())
}
#[test]
fn same_character_new_admission_generation_rejects_old_draft_even_without_storage() -> TestResult {
    let mut host = HostOwner::new(None)?;
    let (view, link) = admitted_view([3; 16])?;
    host.install_play(view, link);
    host.open_graphics();
    host.edit(|d| d.vsync = !d.vsync);
    let stale = host.draft.take().ok_or("stale draft")?;
    let (view, link) = admitted_view([3; 16])?;
    host.install_play(view, link);
    let accepted = host.accepted();
    host.draft = Some(stale);
    host.apply(false);
    assert_eq!(host.accepted(), accepted);
    assert_eq!(host.draft(), None);
    Ok(())
}
#[test]
fn real_restore_projects_graphics_to_play_scene_and_actual_bevy_window() -> TestResult {
    let temp = Temp::new()?;
    let mut host = HostOwner::new(Some(temp.preferences()))?;
    let (view, link) = admitted_view([4; 16])?;
    host.install_play(view, link);
    let mut p = host.settings.preferences.clone();
    p.ui_scale = 150;
    p.interface_scale = 125;
    host.settings.apply(p);
    host.open_graphics();
    host.edit(|d| {
        d.fullscreen = true;
        d.vsync = false;
        d.integer_scaling = true;
    });
    host.apply(false);
    let play = host.play.as_mut().ok_or("play")?;
    host.settings.restore(&mut play.view, [960, 720])?;
    assert!(play.view.preferences().graphics.integer_scaling);
    assert_eq!(
        play.view.scene().view().tile_px(),
        host.settings
            .preferences
            .graphics
            .tile_pixels(150, [960, 720])
    );
    assert_eq!(
        play.view.scene().view().tile_px() % oteryn_renderer::PAGE_CELL_PX,
        0
    );
    assert_eq!(play.view.preferences().interface_scale, 125);
    assert_eq!(play.view.preferences().ui_scale, 150);
    let mut window = bevy::prelude::Window::default();
    super::super::apply_window(&host.settings.preferences, &mut window);
    assert!(matches!(
        window.mode,
        bevy::window::WindowMode::BorderlessFullscreen(_)
    ));
    assert_eq!(window.present_mode, bevy::window::PresentMode::AutoNoVsync);
    host.open_graphics();
    host.edit(|d| {
        d.fullscreen = false;
        d.vsync = true;
    });
    host.apply(true);
    super::super::apply_window(&host.settings.preferences, &mut window);
    assert_eq!(window.mode, bevy::window::WindowMode::Windowed);
    assert_eq!(window.present_mode, bevy::window::PresentMode::AutoVsync);
    Ok(())
}
#[test]
fn missing_identity_cannot_install_play_and_close_consumes_runtime_once() -> TestResult {
    let mut host = HostOwner::new(None)?;
    let view = PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 7, &[])?;
    let (link, _commands, _events) = crate::play::play_channel();
    assert!(!host.install_play(view, link));
    assert!(host.play.is_none());
    host.open_graphics();
    host.close();
    host.close();
    assert_eq!(host.phase, Phase::Closed);
    assert!(host.entry.is_none());
    assert!(host.draft().is_none());
    host.open_graphics();
    assert!(host.draft().is_none());
    Ok(())
}
#[test]
fn reentry_poll_is_nonblocking_and_missing_configuration_never_fabricates_play() -> TestResult {
    let mut host = HostOwner::new(None)?;
    assert!(host.entry.as_mut().ok_or("entry")?.request());
    host.phase = Phase::Entering;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while host.phase == Phase::Entering && std::time::Instant::now() < deadline {
        host.poll([960, 720]);
        std::thread::yield_now();
    }
    assert_eq!(host.phase, Phase::Offline);
    assert!(host.play.is_none());
    host.close();
    assert!(host.entry.is_none());
    Ok(())
}

#[test]
fn native_widget_events_reach_owner_apply_cancel_and_public_audit() -> TestResult {
    use super::super::{audit, ui};
    use bevy::{
        asset::AssetPlugin,
        input_focus::InputFocus,
        prelude::*,
        ui_widgets::{Activate, ValueChange},
    };
    let temp = Temp::new()?;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .init_resource::<InputFocus>()
        .insert_non_send(HostOwner::new(Some(temp.preferences()))?)
        .insert_resource(audit::AuditOutput::new(Some(temp.0.join("ui-audit.json"))))
        .add_systems(Startup, ui::setup)
        .add_systems(Update, ui::project);
    app.update();
    let find = |app: &mut App, id: &str| -> Result<Entity, Box<dyn std::error::Error>> {
        app.world_mut()
            .query::<(Entity, &ui::Control)>()
            .iter(app.world())
            .find(|(_, c)| c.id() == id)
            .map(|(e, _)| e)
            .ok_or_else(|| format!("missing {id}").into())
    };
    let options = find(&mut app, "shell.options")?;
    app.world_mut().trigger(Activate { entity: options });
    app.update();
    let before = app.world().non_send::<HostOwner>().accepted();
    let fullscreen = find(&mut app, "graphics.fullscreen")?;
    app.world_mut().trigger(ValueChange {
        source: fullscreen,
        value: !before.fullscreen,
        is_final: true,
    });
    app.update();
    assert_eq!(app.world().non_send::<HostOwner>().accepted(), before);
    let apply = find(&mut app, "graphics.apply")?;
    app.world_mut().trigger(Activate { entity: apply });
    app.update();
    assert_eq!(
        app.world().non_send::<HostOwner>().accepted().fullscreen,
        !before.fullscreen
    );
    let accepted = app.world().non_send::<HostOwner>().accepted();
    app.world_mut().trigger(ValueChange {
        source: fullscreen,
        value: before.fullscreen,
        is_final: true,
    });
    let cancel = find(&mut app, "graphics.cancel")?;
    app.world_mut().trigger(Activate { entity: cancel });
    app.update();
    assert_eq!(app.world().non_send::<HostOwner>().accepted(), accepted);
    assert_eq!(app.world().non_send::<HostOwner>().draft(), None);
    Ok(())
}

#[test]
fn existing_input_owners_turn_before_walk_and_clear_all_held_mappings_on_modal_reset() -> TestResult
{
    use crate::combat_input::PlayCommand;
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let mut host = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.set_preference_character([5; 16]);
    let (link, mut commands, _events) = crate::play::play_channel();
    host.install_play(view, link);
    let key = |mods| NormalizedInputEvent::Key {
        code: KeyCode::ARROW_UP,
        state: ButtonState::Pressed,
        modifiers: mods,
        repeat: false,
    };
    host.route_input(&[key(Modifiers::one(Modifier::Control))], false);
    let play = host.play.as_mut().ok_or("play")?;
    play.view
        .tick(&play.link)
        .map_err(|end| format!("play ended unexpectedly: {end:?}"))?;
    // With the target-move capability absent the turn is consumed, never converted to a walk.
    while let Ok(command) = commands.try_recv() {
        assert!(!matches!(command, PlayCommand::Step(_)));
    }
    host.open_graphics();
    host.route_input(&[key(Modifiers::NONE)], true);
    host.cancel();
    host.route_input(&[key(Modifiers::NONE)], false);
    let play = host.play.as_mut().ok_or("play")?;
    play.view
        .tick(&play.link)
        .map_err(|end| format!("play ended unexpectedly: {end:?}"))?;
    let mut walked = false;
    while let Ok(command) = commands.try_recv() {
        if let PlayCommand::Step(direction) = command {
            assert_eq!(direction, crate::input::StepDir::North);
            walked = true;
        }
    }
    assert!(
        walked,
        "a new unmodified arrow must walk after the modal clears held Ctrl mapping"
    );
    Ok(())
}

#[test]
fn full_preferences_draft_cancels_applies_and_preserves_invalid_edits() -> TestResult {
    let temp = Temp::new()?;
    let path = temp.preferences();
    let mut host = HostOwner::new(Some(path.clone()))?;
    let original = host.settings.preferences.interface_scale;
    host.open_graphics();
    host.edit_preferences(|preferences| preferences.interface_scale = 125);
    assert_eq!(
        host.preferences_draft().ok_or("draft")?.interface_scale,
        125
    );
    assert_eq!(host.settings.preferences.interface_scale, original);
    host.cancel();
    assert_eq!(host.settings.preferences.interface_scale, original);
    host.open_graphics();
    host.edit_preferences(|preferences| preferences.interface_scale = 125);
    host.apply(false);
    assert_eq!(host.settings.preferences.interface_scale, 125);
    assert_eq!(
        HostOwner::new(Some(path))?
            .settings
            .preferences
            .interface_scale,
        125
    );
    host.edit_preferences(|preferences| preferences.interface_scale = 0);
    host.apply(true);
    assert_eq!(host.settings.preferences.interface_scale, 125);
    assert_eq!(
        host.preferences_draft()
            .ok_or("invalid draft discarded")?
            .interface_scale,
        0
    );
    Ok(())
}
