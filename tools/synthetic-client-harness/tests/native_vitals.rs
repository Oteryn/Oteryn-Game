//! C07: production session mailbox -> play projection -> native HUD, without a window.
use oteryn_client::combat_input::{CombatCommand, CombatReport};
use oteryn_client::play::{PlayView, Stepper, play_channel, run_session};
use oteryn_renderer::TileCoord;
use oteryn_session::{
    ActorVitals, LogoutResult, SessionError, SessionEvent, SpellCastDisposition, StepDirection,
    StepOutcome,
};
use std::{num::NonZeroU32, time::Duration};

#[test]
fn synthetic_session_vitals_scenario() -> Result<(), String> {
    let (link, commands, outbox) = play_channel();
    let initial = ActorVitals {
        health: 150,
        max_health: 150,
        mana: 55,
        max_mana: 60,
        ..ActorVitals::default()
    };
    // A session fixture exercises admission and a command that consumes its own vitals delta.
    struct VitalsSession(ActorVitals);
    impl Stepper for VitalsSession {
        fn actor_vitals(&self) -> Option<ActorVitals> {
            Some(self.0)
        }
        fn drain(&mut self) -> Vec<SessionEvent> {
            Vec::new()
        }
        async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
            tokio::time::sleep(duration).await;
            Ok(())
        }
        async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
            Err(SessionError::SessionUnusable)
        }
        async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
            self.0.mana = 30;
            Ok(CombatReport::Cast(SpellCastDisposition::Cast))
        }
        async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
            Ok(Some(LogoutResult::Accepted))
        }
    }
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let task = runtime.spawn(run_session(VitalsSession(initial), commands, outbox));
    let mut view = PlayView::new(TileCoord::new(100, 200), 0, &[]).map_err(|e| e.to_string())?;
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while link.actor_vitals().is_none() && std::time::Instant::now() < deadline {
        std::thread::yield_now();
    }
    assert_eq!(view.tick(&link), Ok(()));
    assert_eq!(view.actor_vitals(), Some(&initial));
    view.cast(NonZeroU32::new(1).ok_or("spell")?);
    assert_eq!(view.tick(&link), Ok(()));
    while !view.hud_status().contains("30/60") && std::time::Instant::now() < deadline {
        assert_eq!(view.tick(&link), Ok(()));
        std::thread::yield_now();
    }
    assert!(view.hud_status().contains("HP 150/150   MP 30/60"));
    drop(link);
    runtime.block_on(task).map_err(|e| e.to_string())?;
    Ok(())
}
