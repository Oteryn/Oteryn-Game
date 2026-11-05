use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_session::{
    CommandStatus, GatedCommandResult, LogoutResult, SessionError, SessionEvent, StepDirection,
    StepOutcome,
};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};
type Error = Box<dyn std::error::Error + Send + Sync>;
type Exchange = (u32, Vec<u8>, CommandStatus, Vec<u8>);
struct Script {
    entries: Mutex<VecDeque<Exchange>>,
    release: tokio::sync::Semaphore,
}
struct Peer {
    exchanges: Arc<Script>,
}
impl Stepper for Peer {
    fn feature_selected(&self, capability: u32) -> bool {
        capability == 55
    }
    async fn feature(
        &mut self,
        command: &FeatureCommand,
    ) -> Result<GatedCommandResult, SessionError> {
        let (kind, payload, status, reply) = self
            .exchanges
            .entries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop_front()
            .ok_or(SessionError::SessionUnusable)?;
        assert_eq!(command.capability, 55);
        assert_eq!(command.kind, kind);
        assert_eq!(command.payload, payload);
        self.exchanges
            .release
            .acquire()
            .await
            .map_err(|_| SessionError::SessionUnusable)?
            .forget();
        Ok(GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status,
            payload: reply,
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
async fn ready(link: &PlayLink) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !link.guild_bank_panel().selected || link.guild_bank_panel().pending {
            tokio::task::yield_now().await;
        }
    })
    .await
    .checked()?;
    Ok(())
}
async fn send(
    link: &PlayLink,
    exchanges: &Arc<Script>,
    intent: Intent,
    kind: u32,
    payload: Vec<u8>,
    status: CommandStatus,
    reply: Vec<u8>,
) -> Result<(), Error> {
    exchanges
        .entries
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push_back((kind, payload, status, reply));
    link.guild_bank_request(intent);
    assert!(link.guild_bank_panel().pending);
    // A second click before a response must not produce another command.
    link.guild_bank_request(intent);
    exchanges.release.add_permits(1);
    ready(link).await.checked()?;
    assert!(
        exchanges
            .entries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_empty()
    );
    Ok(())
}
#[test]
fn cp01_guild_bank_scenario() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), Error> {
            let runtime = oteryn_client_runtime::ClientRuntime::new().checked()?;
            runtime
                .block_on(async {
                    let exchanges = Arc::new(Script {
                        entries: Mutex::new(VecDeque::new()),
                        release: tokio::sync::Semaphore::new(0),
                    });
                    let (link, mut incoming, outbox) = play_channel();
                    link.guild_bank_request(Intent::Refresh);
                    assert!(!link.guild_bank_panel().pending);
                    assert!(incoming.try_recv().is_err());
                    let task = tokio::spawn(run_session(
                        Peer {
                            exchanges: Arc::clone(&exchanges),
                        },
                        incoming,
                        outbox,
                    ));
                    ready(&link).await.checked()?;
                    let event = oteryn_input_actions::NormalizedInputEvent::Key {
                        code: oteryn_input_actions::KeyCode::new(5).checked()?,
                        state: oteryn_input_actions::ButtonState::Pressed,
                        modifiers: oteryn_input_actions::Modifiers::one(
                            oteryn_input_actions::Modifier::Control,
                        )
                        .with(oteryn_input_actions::Modifier::Alt),
                        repeat: false,
                    };
                    assert!(link.guild_bank_input(&[event]));
                    assert!(link.guild_bank_panel().open);

                    link.guild_bank_edit(0, ID.into());
                    link.guild_bank_edit(1, "10".into());
                    let reply = |balance, entries, next, claims, next_claim| {
                        wire::encode_guild_bank_reply(&wire::GuildBankReply {
                            code: 1,
                            bank_balance: 1000,
                            guild_balance: balance,
                            claim: 0,
                            entries,
                            next,
                            claims,
                            next_claim,
                        })
                    };
                    let query = |before| {
                        wire::encode_guild_bank_query(&wire::GuildBankQuery {
                            guild: Some(ITEM),
                            before,
                            after_claim: None,
                        })
                    };
                    let entries = (901..=1000)
                        .rev()
                        .map(|ordinal| wire::GuildBankEntry {
                            ordinal,
                            kind: 1,
                            amount: 1,
                            before: ordinal,
                            after: ordinal + 1,
                            actor: ITEM,
                        })
                        .collect();
                    send(
                        &link,
                        &exchanges,
                        Intent::Refresh,
                        69,
                        query(None).checked()?,
                        CommandStatus::Accepted,
                        reply(100, entries, Some(901), vec![], None).checked()?,
                    )
                    .await
                    .checked()?;
                    assert_eq!(link.guild_bank_panel().next, Some(901));
                    assert!(link.guild_bank_panel().summary.contains("#1000"));
                    send(
                        &link,
                        &exchanges,
                        Intent::Older,
                        69,
                        query(Some(901)).checked()?,
                        CommandStatus::Accepted,
                        reply(100, vec![], None, vec![], None).checked()?,
                    )
                    .await
                    .checked()?;
                    assert_eq!(link.guild_bank_panel().next, None);
                    link.guild_bank_request(Intent::Older);
                    assert!(!link.guild_bank_panel().pending);
                    for (intent, action, balance) in [
                        (Intent::Deposit, wire::GuildBankAction::Deposit(10), 110),
                        (Intent::Withdraw, wire::GuildBankAction::Withdraw(10), 100),
                        (Intent::Claim, wire::GuildBankAction::Claim, 100),
                    ] {
                        send(
                            &link,
                            &exchanges,
                            intent,
                            68,
                            wire::encode_guild_bank_intent(&wire::GuildBankIntent {
                                guild: ITEM,
                                action,
                            })
                            .checked()?,
                            CommandStatus::Accepted,
                            reply(balance, vec![], None, vec![], None).checked()?,
                        )
                        .await
                        .checked()?;
                        assert!(
                            link.guild_bank_panel()
                                .summary
                                .contains(&format!("Guild bank: {balance}"))
                        );
                    }
                    let claims: Vec<_> = (1..=100)
                        .map(|i| {
                            let mut guild = ITEM;
                            guild[15] = i;
                            wire::GuildBankClaim { guild, amount: 10 }
                        })
                        .collect();
                    let mut last = ITEM;
                    last[15] = 100;
                    send(
                        &link,
                        &exchanges,
                        Intent::Claims,
                        69,
                        wire::encode_guild_bank_query(&wire::GuildBankQuery {
                            guild: None,
                            before: None,
                            after_claim: None,
                        })
                        .checked()?,
                        CommandStatus::Accepted,
                        reply(0, vec![], None, claims, Some(last)).checked()?,
                    )
                    .await
                    .checked()?;
                    assert_eq!(link.guild_bank_panel().next_claim, Some(last));
                    assert_eq!(
                        link.guild_bank_panel()
                            .summary
                            .matches("Open claim")
                            .count(),
                        100
                    );
                    send(
                        &link,
                        &exchanges,
                        Intent::MoreClaims,
                        69,
                        wire::encode_guild_bank_query(&wire::GuildBankQuery {
                            guild: None,
                            before: None,
                            after_claim: Some(last),
                        })
                        .checked()?,
                        CommandStatus::Accepted,
                        reply(0, vec![], None, vec![], None).checked()?,
                    )
                    .await
                    .checked()?;
                    for code in 2..=7 {
                        let refusal = wire::GuildBankReply {
                            code,
                            bank_balance: 0,
                            guild_balance: 0,
                            claim: 0,
                            entries: vec![],
                            next: None,
                            claims: vec![],
                            next_claim: None,
                        };
                        send(
                            &link,
                            &exchanges,
                            Intent::Refresh,
                            69,
                            query(None).checked()?,
                            CommandStatus::Accepted,
                            wire::encode_guild_bank_reply(&refusal).checked()?,
                        )
                        .await
                        .checked()?;
                        assert!(link.guild_bank_panel().summary.is_empty());
                        assert!(link.guild_bank_panel().next.is_none());
                    }
                    send(
                        &link,
                        &exchanges,
                        Intent::Refresh,
                        69,
                        query(None).checked()?,
                        CommandStatus::Accepted,
                        vec![],
                    )
                    .await
                    .checked()?;
                    assert!(link.guild_bank_panel().feedback.contains("Invalid"));
                    send(
                        &link,
                        &exchanges,
                        Intent::Refresh,
                        69,
                        query(None).checked()?,
                        CommandStatus::Rejected,
                        vec![],
                    )
                    .await
                    .checked()?;
                    assert!(link.guild_bank_panel().feedback.contains("refused"));
                    link.guild_bank_edit(1, "0".into());
                    link.guild_bank_request(Intent::Deposit);
                    assert!(!link.guild_bank_panel().pending);

                    drop(link);
                    task.await.checked()?;
                    Ok::<(), Error>(())
                })
                .checked()?
                .checked()?;
            Ok(())
        })
        .checked()?
        .join()
        .map_err(|_| "scenario thread failed")
        .checked()?
}

use oteryn_client_ui::guild_bank::Intent;
use oteryn_session::property_services::guild_bank as wire;
const ID: &str = "00000000-0000-7000-8000-000000000001";
const ITEM: [u8; 16] = [0, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1];

trait Checked<T> {
    fn checked(self) -> Result<T, Error>;
}
impl<T, E: std::fmt::Debug> Checked<T> for Result<T, E> {
    fn checked(self) -> Result<T, Error> {
        self.map_err(|error| format!("{error:?}").into())
    }
}
