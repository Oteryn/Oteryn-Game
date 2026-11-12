//! C36: party controls through the production play loop and mailbox.
use oteryn_client::combat_input::{CombatCommand, CombatReport};
use oteryn_client::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_ui::party::{Intent, panel_text};
use oteryn_client_ui::party::{PartyCommand, PartyResponse, party};
use oteryn_session::{
    CommandStatus, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

type Error = Box<dyn std::error::Error + Send + Sync>;

struct Peer {
    view: party::PartyResult,
    commands: Arc<Mutex<Vec<PartyCommand>>>,
}

impl Stepper for Peer {
    fn feature_selected(&self, capability: u32) -> bool {
        capability == oteryn_session::CAPABILITY_CHAT_V1
    }

    fn party_selected(&self) -> bool {
        true
    }
    async fn party(&mut self, request: &PartyCommand) -> Result<PartyResponse, SessionError> {
        self.commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(request.clone());
        let bad = || SessionError::SessionUnusable;
        let operation = if request.kind == party::COMMAND_TYPE_PARTY_QUERY {
            assert!(request.payload.is_empty());
            party::encode_party_result(&self.view).map_err(|_| bad())?
        } else {
            match party::decode_party_intent(&request.payload).map_err(|_| bad())? {
                party::PartyIntent::Accept(handle) => {
                    let invitation = self
                        .view
                        .invitations
                        .iter()
                        .find(|entry| entry.handle == handle)
                        .ok_or_else(bad)?
                        .clone();
                    self.view.invitations.clear();
                    self.view.members.push(party::PartyEntry {
                        list: Vec::new(),
                        expires_at: 0,
                        party_spell_remaining_ms: None,
                        ..invitation
                    });
                    self.view.members.push(party::PartyEntry {
                        list: Vec::new(),
                        handle: 9,
                        name: "Own".into(),
                        leader: true,
                        expires_at: 0,
                        party_spell_remaining_ms: None,
                    });
                    self.view.members.push(party::PartyEntry {
                        list: Vec::new(),
                        handle: 10,
                        name: "Other".into(),
                        leader: false,
                        expires_at: 0,
                        party_spell_remaining_ms: None,
                    });
                }
                party::PartyIntent::Invite(_) => {
                    return Ok(PartyResponse {
                        status: CommandStatus::Accepted,
                        operation: party::encode_party_result(&party::PartyResult::refused(
                            party::PartyDisposition::NotVisible,
                        ))
                        .map_err(|_| bad())?,
                        view: Some(party::encode_party_result(&self.view).map_err(|_| bad())?),
                    });
                }
                party::PartyIntent::PassLeadership(handle) => {
                    for entry in &mut self.view.members {
                        entry.leader = entry.handle == handle;
                    }
                }
                party::PartyIntent::Kick(handle) => {
                    self.view.members.retain(|entry| entry.handle != handle)
                }
                party::PartyIntent::Leave => self.view.members.clear(),
            }
            party::encode_party_result(&party::PartyResult::refused(party::PartyDisposition::Ok))
                .map_err(|_| bad())?
        };
        Ok(PartyResponse {
            status: CommandStatus::Accepted,
            operation,
            view: Some(party::encode_party_result(&self.view).map_err(|_| bad())?),
        })
    }
    async fn chat_public(
        &mut self,
        intent: &oteryn_session::ChatIntent,
    ) -> Result<oteryn_session::ChatOutcome, SessionError> {
        assert_eq!(
            *intent,
            oteryn_session::ChatIntent::Say {
                mode: oteryn_session::ChatSpeechMode::Say,
                text: "/sharedxp off".into()
            }
        );
        self.view.shared_xp_enabled = false;
        Ok(oteryn_session::ChatOutcome {
            command_id: 1,
            status: CommandStatus::Accepted,
            disposition: oteryn_session::ChatDisposition::Ok,
            wait_seconds: 0,
            result_server_sequence: 1,
        })
    }
    async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
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
        Ok(None)
    }
}

async fn ready(link: &PlayLink, members: usize) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let panel = link.party_panel();
            if panel.selected
                && panel.pending.is_none()
                && panel
                    .state
                    .as_ref()
                    .is_some_and(|state| state.members.len() == members)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}

#[test]
fn party_controls_use_real_codec_requests_and_apply_server_views() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            runtime.block_on(async {
                let commands = Arc::new(Mutex::new(Vec::new()));
                let peer = Peer {
                    view: party::PartyResult {
                        disposition: party::PartyDisposition::Ok,
                        shared_xp_enabled: true,
                        members: vec![],
                        invitations: vec![party::PartyEntry {
                            list: Vec::new(),
                            handle: 8,
                            name: "Aster".into(),
                            leader: false,
                            expires_at: 99,
                            party_spell_remaining_ms: None,
                        }],
                    },
                    commands: Arc::clone(&commands),
                };
                let (link, incoming, outbox) = play_channel();
                link.party_own_character("Own");
                let task = tokio::spawn(run_session(peer, incoming, outbox));
                tokio::time::timeout(Duration::from_secs(3), async {
                    while !link.party_panel().selected {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                link.party_request(Intent::Refresh);
                ready(&link, 0).await?;
                assert!(panel_text(&link.party_panel()).contains("Shared experience: enabled"));
                link.party_select_invitation(0);
                link.party_request(Intent::Join);
                ready(&link, 3).await?;
                assert!(panel_text(&link.party_panel()).contains("Member: Aster"));
                let recorded = commands
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clone();
                assert_eq!(recorded.len(), 2);
                assert_eq!(
                    party::decode_party_intent(&recorded[1].payload),
                    Ok(party::PartyIntent::Accept(8))
                );
                link.party_request(Intent::ToggleSharedExperience);
                ready(&link, 3).await?;
                assert!(panel_text(&link.party_panel()).contains("Shared experience: disabled"));
                link.party_request(Intent::Invite(oteryn_session::EntityRef {
                    identity: [3; 16],
                    generation: 2,
                }));
                ready(&link, 3).await?;
                assert!(panel_text(&link.party_panel()).contains("not visible"));
                link.party_select_member(2);
                link.party_request(Intent::Kick);
                ready(&link, 2).await?;
                link.party_select_member(0);
                link.party_request(Intent::PassLeadership);
                ready(&link, 2).await?;
                assert!(
                    link.party_panel()
                        .state
                        .as_ref()
                        .is_some_and(|state| state.members[0].leader)
                );
                link.party_request(Intent::Kick);
                assert!(panel_text(&link.party_panel()).contains("Only the party leader"));
                link.party_request(Intent::Leave);
                ready(&link, 0).await?;
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })
        })?
        .join()
        .map_err(|_| "Party UI test thread panicked")?
}
