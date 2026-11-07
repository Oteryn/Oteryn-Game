use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{Stepper, play_channel, run_session};
use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_session::{
    CommandStatus, GatedCommandResult, LogoutResult, SessionError, SessionEvent, StepDirection,
    StepOutcome,
};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
use tokio::sync::Notify;
type Error = Box<dyn std::error::Error + Send + Sync>;
struct Peer {
    selected: bool,
    capability: u32,
    kind: u32,
    replies: VecDeque<(CommandStatus, Vec<u8>)>,
    calls: Arc<Mutex<usize>>,
    release: Arc<Notify>,
    served: Arc<Notify>,
}
impl Stepper for Peer {
    fn feature_selected(&self, capability: u32) -> bool {
        self.selected && capability == self.capability
    }
    async fn feature(
        &mut self,
        command: &FeatureCommand,
    ) -> Result<GatedCommandResult, SessionError> {
        assert_eq!(
            (command.capability, command.kind),
            (self.capability, self.kind)
        );
        assert!(command.payload.is_empty());
        let id = {
            let mut calls = self.calls.lock().unwrap_or_else(PoisonError::into_inner);
            *calls += 1;
            *calls as u64
        };
        if id == 1 {
            self.release.notified().await;
        }
        let (status, payload) = self
            .replies
            .pop_front()
            .ok_or(SessionError::SessionUnusable)?;
        Ok(GatedCommandResult {
            command_id: id,
            server_sequence: id,
            status,
            payload,
        })
    }
    async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
        self.served.notify_one();
        tokio::time::sleep(duration).await;
        Ok(())
    }
    fn drain(&mut self) -> Vec<SessionEvent> {
        vec![]
    }
    async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        Ok(Some(LogoutResult::Accepted))
    }
}
async fn ready(mut condition: impl FnMut() -> bool) -> Result<(), Error> {
    Ok(tokio::time::timeout(Duration::from_secs(3), async {
        while !condition() {
            tokio::task::yield_now().await;
        }
    })
    .await?)
}
macro_rules! flow {
    ($test:ident,$feature:ident,$panel:ident,$request:ident,$input:ident,$init:ident,$cap:expr,$cmd:expr,$key:expr,$fixture:expr) => {
        #[test]
        fn $test() -> Result<(), Error> {
            std::thread::Builder::new()
                .stack_size(64 << 20)
                .spawn(|| -> Result<(), Error> {
                    let runtime = oteryn_client_runtime::ClientRuntime::new()?;
                    runtime.block_on(async {
                        let snapshot = $fixture;
                        let payload = snapshot.encode().ok_or("fixture")?;
                        let calls = Arc::new(Mutex::new(0));
                        let release = Arc::new(Notify::new());
                        let peer = Peer {
                            selected: true,
                            capability: $cap,
                            kind: $cmd,
                            calls: Arc::clone(&calls),
                            release: Arc::clone(&release),
                            served: Arc::default(),
                            replies: [
                                (CommandStatus::Accepted, payload.clone()),
                                (CommandStatus::Rejected, payload.clone()),
                                (CommandStatus::Accepted, vec![255]),
                                (CommandStatus::DuplicateReplay, payload),
                            ]
                            .into(),
                        };
                        let (link, incoming, outbox) = play_channel();
                        let task = tokio::spawn(run_session(peer, incoming, outbox));
                        ready(|| link.$panel().selected).await?;
                        let modifiers = oteryn_input_actions::Modifiers::one(
                            oteryn_input_actions::Modifier::Control,
                        )
                        .with(oteryn_input_actions::Modifier::Alt)
                        .with(oteryn_input_actions::Modifier::Shift);
                        let event = oteryn_input_actions::NormalizedInputEvent::Key {
                            code: oteryn_input_actions::KeyCode::new($key)?,
                            state: oteryn_input_actions::ButtonState::Pressed,
                            modifiers,
                            repeat: false,
                        };
                        assert!(link.$input(&[event]));
                        assert!(link.$panel().pending);
                        link.$request();
                        release.notify_one();
                        ready(|| link.$panel().snapshot.is_some() && !link.$panel().pending)
                            .await?;
                        assert!(link.$panel().open);
                        assert_eq!(link.$panel().snapshot.as_ref(), Some(&snapshot));
                        assert!(!oteryn_client_ui::$feature::panel_text(&link.$panel()).is_empty());
                        for _ in 0..2 {
                            link.$request();
                            ready(|| link.$panel().error.is_some() && !link.$panel().pending)
                                .await?;
                            if $cap == 277 {
                                assert!(link.$panel().snapshot.is_none());
                            } else {
                                assert_eq!(link.$panel().snapshot.as_ref(), Some(&snapshot));
                            }
                        }
                        link.$request();
                        ready(|| !link.$panel().pending).await?;
                        assert!(link.$panel().error.is_none());
                        assert_eq!(*calls.lock().unwrap_or_else(PoisonError::into_inner), 4);
                        link.request_logout();
                        task.await?;
                        assert_eq!(link.$panel(), Default::default());
                        link.$request();
                        assert_eq!(link.$panel(), Default::default());
                        let served = Arc::new(Notify::new());
                        let calls = Arc::new(Mutex::new(0));
                        let peer = Peer {
                            selected: false,
                            capability: $cap,
                            kind: $cmd,
                            replies: VecDeque::new(),
                            calls: Arc::clone(&calls),
                            release: Arc::default(),
                            served: Arc::clone(&served),
                        };
                        let (link, incoming, outbox) = play_channel();
                        let task = tokio::spawn(run_session(peer, incoming, outbox));
                        tokio::time::timeout(Duration::from_secs(3), served.notified()).await?;
                        link.$request();
                        link.request_logout();
                        task.await?;
                        assert_eq!(*calls.lock().unwrap_or_else(PoisonError::into_inner), 0);
                        let (link, incoming, outbox) = play_channel();
                        link.$request();
                        assert!(!link.$panel().pending);
                        outbox.$init(true);
                        drop(incoming);
                        link.$request();
                        assert!(!link.$panel().pending);
                        assert_eq!(link.$panel().error, Some("Session ended"));
                        Ok::<(), Error>(())
                    })?
                })?
                .join()
                .map_err(|_| "native status test thread panicked")?
        }
    };
}
flow!(
    condition_status_native_queue_refusal_replay_logout,
    condition_status,
    condition_status_panel,
    condition_status_request,
    condition_status_input,
    condition_status_initialize,
    276,
    492,
    6,
    oteryn_session::condition_status::ConditionStatus {
        conditions: vec![oteryn_session::condition_status::ActiveCondition {
            icon: 1,
            sub_id: 0,
            remaining_ms: 5000
        }]
    }
);
flow!(
    rest_status_native_queue_refusal_replay_logout,
    rest_status,
    rest_status_panel,
    rest_status_request,
    rest_status_input,
    rest_status_initialize,
    277,
    493,
    21,
    oteryn_session::rest_status::RestStatus {
        inside_resting_area: true,
        streak: 7,
        food_remaining_ms: 180000,
        health_multiplier: 2,
        mana_multiplier: 2,
        stamina: true,
        soul: true
    }
);
flow!(
    battle_instinct_status_native_queue_refusal_replay_logout,
    battle_instinct_status,
    battle_instinct_status_panel,
    battle_instinct_status_request,
    battle_instinct_status_input,
    battle_instinct_status_initialize,
    278,
    494,
    5,
    oteryn_session::battle_instinct_status::BattleInstinctStatus {
        active: true,
        melee: 5,
        shielding: 5
    }
);
flow!(
    skill_progress_native_queue_refusal_replay_logout,
    skill_progress,
    skill_progress_panel,
    skill_progress_request,
    skill_progress_input,
    skill_progress_initialize,
    279,
    495,
    14,
    oteryn_session::skill_progress::SkillProgressSnapshot {
        skills: [oteryn_session::skill_progress::SkillProgress {
            level: 10,
            progress: 50,
            required: 100,
            percent: 5000
        }; 8]
    }
);
flow!(
    party_effects_native_queue_refusal_replay_logout,
    party_effects,
    party_effects_panel,
    party_effects_request,
    party_effects_input,
    party_effects_initialize,
    280,
    496,
    19,
    oteryn_session::party_effects::PartyEffects {
        in_party: true,
        shared_enabled: true,
        shared_active: true,
        eligible: true,
        bonuses: [6, 12, 6, 12, 50],
        ..Default::default()
    }
);

flow!(
    nx8_potion_durations_native_queue_refusal_replay_logout,
    potion_durations,
    potion_durations_panel,
    potion_durations_request,
    potion_durations_input,
    potion_durations_initialize,
    381,
    662,
    23,
    oteryn_session::potion_durations::PotionDurations {
        active: vec![oteryn_session::potion_durations::PotionDuration {
            source: 1,
            item_id: 36727,
            percent: 0,
            remaining_ms: 3600000
        }]
    }
);

#[test]
fn nx7_complete_skills_native_play_queue_decode_and_display() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), Error> {
            oteryn_client_runtime::ClientRuntime::new()?.block_on(async {
                let full = oteryn_session::skill_display::SkillDisplay {
                    trained: oteryn_session::skill_progress::SkillProgressSnapshot {
                        skills: [oteryn_session::skill_progress::SkillProgress {
                            level: 30,
                            progress: 50,
                            required: 100,
                            percent: 5000,
                        }; 8],
                    },
                    level: 8,
                    experience: 5300,
                    level_percent: 5000,
                    loyalty_bonus_percent: 50,
                    loyalty_levels: [31; 8],
                    current_levels: Some([35; 8]),
                };
                let release = Arc::new(Notify::new());
                let peer = Peer {
                    selected: true,
                    capability: 370,
                    kind: 640,
                    replies: [
                        (CommandStatus::Accepted, full.encode().ok_or("encode")?),
                        (CommandStatus::Accepted, vec![255]),
                    ]
                    .into(),
                    calls: Arc::default(),
                    release: Arc::clone(&release),
                    served: Arc::default(),
                };
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(peer, incoming, outbox));
                ready(|| link.skill_progress_panel().complete_selected).await?;
                link.skill_progress_request();
                release.notify_one();
                ready(|| link.skill_progress_panel().complete.is_some()).await?;
                let panel = link.skill_progress_panel();
                assert_eq!(panel.complete, Some(full));
                let text = oteryn_client_ui::skill_progress::panel_text(&panel);
                for expected in [
                    "Level: 8",
                    "Experience: 5300",
                    "50.00% to next level",
                    "Loyalty skill-point bonus: 50%",
                    "Magic: 30 / 31 / 35",
                    "Fishing: 30 / 31 / 35",
                ] {
                    assert!(text.contains(expected), "{text}");
                }
                link.skill_progress_request();
                ready(|| link.skill_progress_panel().error.is_some()).await?;
                assert_eq!(link.skill_progress_panel().complete, Some(full));
                link.request_logout();
                task.await?;
                assert_eq!(link.skill_progress_panel(), Default::default());
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "NX-7 play thread panicked")?
}

flow!(
    cl6_mechanics_status_native_queue_refusal_replay_logout,
    mechanics_status,
    mechanics_status_panel,
    mechanics_status_request,
    mechanics_status_input,
    mechanics_status_initialize,
    384,
    669,
    24,
    oteryn_session::mechanics_status::MechanicsStatus {
        observed_unix_ms: 100,
        dust: 73,
        dust_limit: 100,
        deaths: vec![oteryn_session::mechanics_status::DeathConsumption { at_unix_ms: 50, consumed: vec!["oteryn:blessing.twist_of_fate".into()] }],
        timers: vec![oteryn_session::mechanics_status::EffectTimer { key: "wealth_duplex".into(), remaining_ms: 1234, online_only: true }],
        members: vec![oteryn_session::mechanics_status::MemberEffects { bonuses: [0; 5], party_spell_remaining_ms: [0; 5], character: [1;16], name: "Friend".into(), conditions: Some(oteryn_session::condition_status::ConditionStatus { conditions: vec![oteryn_session::condition_status::ActiveCondition { icon: 13, sub_id: 0, remaining_ms: 4001 }] }) }],
    }
);
