use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_runtime::wheel_allocation::{WheelCommand, WheelResponse, atelier, wheel};
use oteryn_client_ui::wheel_allocation::{Intent, panel_text};
use oteryn_session::{CommandStatus, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

type Error = Box<dyn std::error::Error + Send + Sync>;

struct Peer {
    wheel: wheel::WheelAllocationReply,
    atelier: atelier::AtelierReply,
    commands: Arc<Mutex<Vec<WheelCommand>>>,
}

impl Stepper for Peer {
    fn wheel_selected(&self) -> bool { true }

    async fn wheel(&mut self, request: &WheelCommand) -> Result<WheelResponse, SessionError> {
        let bad = || SessionError::SessionUnusable;
        self.commands.lock().unwrap_or_else(PoisonError::into_inner).push(request.clone());
        let payload = match request.kind {
            161 => {
                let value = wheel::decode_wheel_request(&request.payload).map_err(|_| bad())?;
                if value.action != 0 {
                    assert_eq!(value.expected_revision, self.wheel.revision);
                    self.wheel.revision += 1;
                    self.wheel.points = value.points;
                    if let Some(vessels) = value.vessels {
                        for gem in &mut self.atelier.gems {
                            gem.vessel = vessels.contains(&Some(gem.id));
                        }
                    }
                }
                wheel::encode_wheel_reply(&self.wheel).map_err(|_| bad())?
            }
            309 => {
                let value = atelier::decode_set_lock(&request.payload).map_err(|_| bad())?;
                assert_eq!(value.expected_revision, self.atelier.revision);
                self.atelier.gems.iter_mut().find(|gem| gem.id == value.gem).ok_or_else(bad)?.locked = value.locked;
                self.atelier.revision += 1;
                atelier::encode_reply(&self.atelier).map_err(|_| bad())?
            }
            424 => {
                let value = atelier::decode_mutate(&request.payload).map_err(|_| bad())?;
                assert_eq!(value.expected_revision, self.atelier.revision);
                match value.mutation {
                    atelier::Mutation::Reveal { quality } => self.atelier.gems.push(atelier::GemView {
                        id: [18; 16], quality, domain: 1, locked: false, vessel: false,
                        modifiers: vec![0; usize::from(quality) + 1],
                    }),
                    atelier::Mutation::Dismantle { gem } => self.atelier.gems.retain(|value| value.id != gem),
                    atelier::Mutation::SwitchDomain { gem } => {
                        self.atelier.gems.iter_mut().find(|value| value.id == gem).ok_or_else(bad)?.domain = 1;
                    }
                    atelier::Mutation::GradeUp { kind, mod_id } => self.atelier.grades.push(atelier::GradeView {
                        kind, mod_id, grade: 1,
                    }),
                }
                self.atelier.revision += 1;
                // Recorded receipts omit derived vessel flags and current quotes. The client
                // receives them only from the two live queries carried by WheelResponse.
                let mut receipt = self.atelier.clone();
                receipt.quality_quotes.clear();
                receipt.grade_quotes.clear();
                for gem in &mut receipt.gems { gem.vessel = false; }
                atelier::encode_reply(&receipt).map_err(|_| bad())?
            }
            _ => return Err(bad()),
        };
        Ok(WheelResponse {
            kind: request.kind, status: CommandStatus::Accepted, payload,
            wheel: Some(wheel::encode_wheel_reply(&self.wheel).map_err(|_| bad())?),
            atelier: Some(atelier::encode_reply(&self.atelier).map_err(|_| bad())?),
        })
    }

    async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
        tokio::time::sleep(duration).await; Ok(())
    }
    fn drain(&mut self) -> Vec<SessionEvent> { vec![] }
    async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> { Ok(None) }
}

async fn ready(link: &PlayLink, revisions: Option<(u64, u64)>) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let panel = link.wheel_panel();
            if panel.selected && panel.pending.is_none()
                && revisions.is_none_or(|(wheel, atelier)| {
                    panel.synchronized
                        && panel.confirmed_wheel.as_ref().is_some_and(|state| state.revision == wheel)
                        && panel.confirmed_atelier.as_ref().is_some_and(|state| state.revision == atelier)
                }) { break; }
            tokio::task::yield_now().await;
        }
    }).await?;
    Ok(())
}

#[test]
fn wheel_panel_controls_reach_the_session_queue_and_apply_live_server_views() -> Result<(), Error> {
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
        let runtime = oteryn_client_runtime::ClientRuntime::new()?;
        runtime.block_on(async {
            let commands = Arc::new(Mutex::new(Vec::new()));
            let peer = Peer {
                wheel: wheel::WheelAllocationReply { refused: false, revision: 4, points: [0; 36], vessels: None },
                atelier: atelier::AtelierReply {
                    refused: false, revision: 4,
                    gems: vec![atelier::GemView { id: [17; 16], quality: 2, domain: 0, locked: false, vessel: false, modifiers: vec![31, 0, 5] }],
                    grades: vec![],
                    quality_quotes: (0..3).map(|quality| atelier::QualityQuote { quality, reveal_gold: 12345,
                        switch_gold: 6789, fragment_kind: u8::from(quality == 2), fragment_min: 9, fragment_max: 11 }).collect(),
                    grade_quotes: vec![atelier::GradeQuote { kind: 1, grade: 1, gold: 999, fragments: 12 }],
                },
                commands: Arc::clone(&commands),
            };
            let (link, incoming, outbox) = play_channel();
            let task = tokio::spawn(run_session(peer, incoming, outbox));
            ready(&link, None).await?;
            link.wheel_request(Intent::Refresh);
            ready(&link, Some((4, 4))).await?;
            assert!(panel_text(&link.wheel_panel()).contains("12345 gold"));
            {
                let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
                assert!(mailbox.wheel.set_points(14, 50));
                assert!(mailbox.wheel.socket_selected());
            }
            link.wheel_request(Intent::Save);
            ready(&link, Some((5, 4))).await?;
            assert_eq!(link.wheel_panel().points[14], 50);
            assert_eq!(link.wheel_panel().vessels[0], Some([17; 16]));
            {
                let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
                assert!(mailbox.wheel.clear_vessel());
            }
            link.wheel_request(Intent::Save);
            ready(&link, Some((6, 4))).await?;
            assert_eq!(link.wheel_panel().vessels, [None; 4]);
            link.wheel_request(Intent::ToggleLock);
            ready(&link, Some((6, 5))).await?;
            assert!(link.wheel_panel().selected_gem().ok_or("gem")?.locked);
            link.wheel_request(Intent::ToggleLock);
            ready(&link, Some((6, 6))).await?;
            link.wheel_request(Intent::SwitchDomain);
            ready(&link, Some((6, 7))).await?;
            assert_eq!(link.wheel_panel().selected_gem().ok_or("gem")?.domain, 1);
            crate::play::mailbox::lock(&link.mailbox).wheel.modifier = Some((1, 5));
            link.wheel_request(Intent::GradeUp);
            ready(&link, Some((6, 8))).await?;
            assert_eq!(link.wheel_panel().grade(1, 5), 1);
            link.wheel_request(Intent::Reveal);
            ready(&link, Some((6, 9))).await?;
            assert_eq!(link.wheel_panel().confirmed_atelier.as_ref().ok_or("atelier")?.gems.len(), 2);
            link.wheel_request(Intent::Dismantle);
            ready(&link, Some((6, 10))).await?;
            assert_eq!(link.wheel_panel().gem, Some([18; 16]));
            link.wheel_request(Intent::Reset);
            ready(&link, Some((7, 10))).await?;
            assert_eq!(link.wheel_panel().points, [0; 36]);
            let recorded = commands.lock().unwrap_or_else(PoisonError::into_inner).clone();
            assert_eq!(recorded.iter().map(|value| value.kind).collect::<Vec<_>>(),
                vec![161, 161, 161, 309, 309, 424, 424, 424, 424, 161]);
            let ids = recorded.iter().filter(|value| value.mutation).map(|value| {
                match value.kind {
                    161 => wheel::decode_wheel_request(&value.payload).map(|value| value.occurrence).map_err(|_| SessionError::SessionUnusable),
                    309 => atelier::decode_set_lock(&value.payload).map(|value| value.occurrence).map_err(|_| SessionError::SessionUnusable),
                    _ => atelier::decode_mutate(&value.payload).map(|value| value.occurrence).map_err(|_| SessionError::SessionUnusable),
                }
            }).collect::<Result<Vec<_>, _>>()?;
            for (index, id) in ids.iter().enumerate() {
                assert_eq!(id[6] >> 4, 7);
                assert_eq!(id[8] >> 6, 2);
                assert!(!ids[..index].contains(id));
            }
            drop(link);
            task.await?;
            Ok::<(), Error>(())
        })?
    })?.join().map_err(|_| "Wheel UI test thread panicked")?
}
