use crate::play::{PreferenceStore, ReferencePreferences, SettingsState};
#[test]
fn t061_graphics_performance_preferences_survive_real_settings_apply_and_reload()
-> Result<(), Box<dyn std::error::Error>> {
    let mut value = serde_json::to_value(ReferencePreferences::default())?;
    value["performance"] = serde_json::json!({"foreground_fps":120,"background_fps":12,"show_fps":true,"show_latency":true});
    let preferences: ReferencePreferences = serde_json::from_value(value)?;
    let path = std::env::temp_dir().join(format!(
        "oteryn-t061-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    let mut settings = SettingsState::new(Some(path.clone()));
    settings.apply(preferences);
    let loaded = PreferenceStore::load(path.clone())?;
    let saved = serde_json::to_value(loaded.preferences())?;
    assert_eq!(saved["performance"]["foreground_fps"], 120);
    assert_eq!(saved["performance"]["background_fps"], 12);
    assert_eq!(saved["performance"]["show_latency"], true);
    std::fs::remove_file(path)?;
    Ok(())
}
#[test]
fn t061_scheduler_tracks_focus_occlusion_successful_frames_and_stale_samples()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{PerformancePacer as FramePacer, PerformanceSettings};
    use std::time::{Duration, Instant};
    let now = Instant::now();
    let settings = PerformanceSettings {
        foreground_fps: 100,
        background_fps: 10,
        show_fps: true,
        show_latency: true,
    };
    let mut pacer = FramePacer::default();
    assert!(pacer.begin_redraw(now, settings));
    assert!(!pacer.begin_redraw(now + Duration::from_millis(9), settings));
    pacer.focus(false);
    assert!(!pacer.due(now + Duration::from_millis(99), settings));
    assert!(pacer.due(now + Duration::from_millis(100), settings));
    pacer.focus(true);
    pacer.occluded(true);
    assert!(!pacer.due(now + Duration::from_millis(20), settings));
    pacer.occluded(false);
    assert!(pacer.due(now + Duration::from_millis(10), settings));
    assert!(
        pacer
            .indicator(now, settings, None)
            .ok_or("indicator")?
            .contains("FPS: —")
    );
    for i in 0..=4 {
        pacer.presented(now + Duration::from_millis(i * 250));
    }
    let text = pacer
        .indicator(
            now + Duration::from_secs(1),
            settings,
            Some((now, Duration::from_millis(23))),
        )
        .ok_or("indicator")?;
    assert!(text.contains("FPS: 4") && text.contains("Command RTT: 23 ms"));
    let stale = pacer
        .indicator(
            now + Duration::from_secs(20),
            settings,
            Some((now, Duration::ZERO)),
        )
        .ok_or("stale")?;
    assert!(stale.contains("FPS: —") && stale.contains("Command RTT: —"));
    assert!(
        pacer
            .indicator(now, PerformanceSettings::default(), None)
            .is_none()
    );
    let unlimited = PerformanceSettings {
        foreground_fps: 0,
        ..settings
    };
    assert_eq!(unlimited.interval(true), Duration::ZERO);
    let invalid = PerformanceSettings {
        background_fps: 0,
        ..settings
    };
    assert!(!invalid.valid());
    Ok(())
}

type Error = Box<dyn std::error::Error + Send + Sync>;
struct MeasuredPeer(std::sync::Arc<std::sync::atomic::AtomicBool>);
impl crate::play::Stepper for MeasuredPeer {
    fn feature_selected(&self, capability: u32) -> bool {
        capability == 10
    }
    async fn feature(
        &mut self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
    ) -> Result<oteryn_session::GatedCommandResult, oteryn_session::SessionError> {
        if command.kind != oteryn_session::analyser_catalogue::COMMAND_TYPE_ANALYSER_ITEM_QUERY {
            return Err(oteryn_session::SessionError::SessionUnusable);
        }
        tokio::time::sleep(std::time::Duration::from_millis(3)).await;
        Ok(oteryn_session::GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_session::CommandStatus::Accepted,
            payload: oteryn_session::analyser_catalogue::encode_reply(&[
                oteryn_session::analyser_catalogue::ItemInfo {
                    item_type: 1,
                    name: "Test item".into(),
                    coin_worth: 0,
                },
            ])
            .map_err(|_| oteryn_session::SessionError::SessionUnusable)?,
        })
    }
    async fn serve(
        &mut self,
        duration: std::time::Duration,
    ) -> Result<(), oteryn_session::SessionError> {
        self.0.store(true, std::sync::atomic::Ordering::Release);
        tokio::time::sleep(duration).await;
        Ok(())
    }
    fn drain(&mut self) -> Vec<oteryn_session::SessionEvent> {
        vec![]
    }
    async fn step(
        &mut self,
        _: oteryn_session::StepDirection,
    ) -> Result<oteryn_session::StepOutcome, oteryn_session::SessionError> {
        Err(oteryn_session::SessionError::SessionUnusable)
    }
    async fn combat(
        &mut self,
        _: crate::combat_input::CombatCommand,
    ) -> Result<crate::combat_input::CombatReport, oteryn_session::SessionError> {
        Err(oteryn_session::SessionError::SessionUnusable)
    }
    async fn logout(
        &mut self,
    ) -> Result<Option<oteryn_session::LogoutResult>, oteryn_session::SessionError> {
        Ok(None)
    }
}
#[test]
fn t061_latency_comes_from_real_session_command_caller_and_clears_on_logout() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), Error> {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                use std::sync::{
                    Arc,
                    atomic::{AtomicBool, Ordering},
                };
                use std::time::Duration;
                let (link, incoming, outbox) = crate::play::play_channel();
                let observer = outbox.clone();
                assert!(link.performance_command_latency().is_none());
                let ready = Arc::new(AtomicBool::new(false));
                let task = tokio::spawn(crate::play::run_session(
                    MeasuredPeer(ready.clone()),
                    incoming,
                    outbox,
                ));
                tokio::time::timeout(Duration::from_secs(3), async {
                    while !ready.load(Ordering::Acquire) {
                        tokio::task::yield_now().await;
                    }
                })
                .await
                .map_err(|_| "session never reached idle service")?;
                link.commands.send(crate::combat_input::PlayCommand::Feature(
                    oteryn_client_runtime::raw_feature::FeatureCommand {
                        capability: 10,
                        kind: oteryn_session::analyser_catalogue::COMMAND_TYPE_ANALYSER_ITEM_QUERY,
                        payload: oteryn_session::analyser_catalogue::encode_request(&[1])?,
                    },
                )).map_err(|_| "command queue")?;
                tokio::time::timeout(Duration::from_secs(3), async {
                    while link.performance_command_latency().is_none() {
                        tokio::task::yield_now().await;
                    }
                })
                .await
                .map_err(|_| format!("no latency sample; task finished: {}", task.is_finished()))?;
                assert!(
                    link.performance_command_latency().ok_or("sample")?.1
                        >= Duration::from_millis(3)
                );
                observer.logged_out();
                assert!(link.performance_command_latency().is_none());
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "latency caller thread")??;
    Ok(())
}

#[test]
fn t061_merged_graphics_controls_preserve_legacy_preferences_and_unlimited_frames()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{FramePreferences, PerformanceSettings};
    let mut settings = SettingsState::new(None);
    let mut legacy = settings.preferences.clone();
    legacy.performance.foreground_fps = 0;
    legacy.performance.show_latency = true;
    settings.apply(legacy.clone());
    assert_eq!(
        settings.preferences.render_performance(),
        legacy.performance
    );
    let mut merged = legacy;
    merged.frames = FramePreferences {
        foreground: 144,
        background: 360,
        show_fps: true,
        show_latency: true,
    };
    settings.apply(merged);
    assert_eq!(
        settings.preferences.render_performance().foreground_fps,
        144
    );
    assert_eq!(
        settings.preferences.render_performance().background_fps,
        360
    );
    let unlimited = FramePreferences {
        foreground: 0,
        ..Default::default()
    };
    assert!(unlimited.valid());
    let mut clock = crate::play::FramePacer::default();
    assert!(clock.request(0, unlimited, true).0);
    assert!(clock.request(0, unlimited, true).0);
    assert_eq!(
        PerformanceSettings::from(unlimited).interval(true),
        std::time::Duration::ZERO
    );
    Ok(())
}
