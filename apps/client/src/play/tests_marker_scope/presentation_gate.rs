use super::*;
use crate::play::mailbox::lock;

type Error = Box<dyn std::error::Error + Send + Sync>;

struct Destination {
    scope: ActivityScopeSnapshot,
    events: Vec<SessionEvent>,
    cooldown_snapshot: oteryn_session::ActorCooldownUpdate,
    queries: Arc<Mutex<Vec<u32>>>,
    served: Arc<tokio::sync::Notify>,
}
impl Stepper for Destination {
    fn feature_selected(&self, capability: u32) -> bool {
        matches!(capability, 450 | 451)
    }
    fn feature_snapshot(&self, domain: u32) -> Option<Vec<u8>> {
        (domain == 99)
            .then(|| encode_activity_scope(&self.scope).ok())
            .flatten()
    }
    fn actor_cooldown_snapshot(&self) -> Option<oteryn_session::ActorCooldownUpdate> {
        Some(self.cooldown_snapshot.clone())
    }
    async fn feature(
        &mut self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
    ) -> Result<oteryn_session::GatedCommandResult, SessionError> {
        self.queries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(command.capability);
        Ok(oteryn_session::GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: if command.capability == 451 {
                CommandStatus::Accepted
            } else {
                CommandStatus::Rejected
            },
            payload: if command.capability == 451 {
                marks().encode().ok_or(SessionError::SessionUnusable)?
            } else {
                vec![]
            },
        })
    }
    async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
        self.served.notify_one();
        tokio::time::sleep(duration).await;
        Ok(())
    }
    fn drain(&mut self) -> Vec<SessionEvent> {
        std::mem::take(&mut self.events)
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

fn marks() -> oteryn_client_ui::personal_map_marks::PersonalMapMarks {
    use oteryn_client_ui::personal_map_marks::{PersonalMapMark, PersonalMapMarks};
    PersonalMapMarks(vec![PersonalMapMark {
        key: "destination".into(),
        x: 31948,
        y: 31924,
        floor: 7,
        text: "Treasure".into(),
    }])
}

#[tokio::test]
async fn modal_and_marks_wait_for_presentation_then_keep_fetched_marks() -> Result<(), Error> {
    let mut id = [1; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    let mut session = id;
    session[15] = 2;
    let scope = ActivityScopeSnapshot {
        handoff: id,
        source_session: id,
        session,
        scope: ActivityScope::Instance {
            world: id,
            instance: id,
        },
        scope_generation: 2,
        connection_generation: 1,
        lease_generation: 2,
        selection: vec![117, 450, 451],
        requires_ack: false,
    };
    let (link, commands, outbox) = play_channel();
    let queries = Arc::<Mutex<Vec<u32>>>::default();
    let served = Arc::new(tokio::sync::Notify::new());
    use oteryn_session::{ActorCooldown, ActorCooldownKind, ActorCooldownUpdate, ActorCooldowns};
    let decoded_at = std::time::Instant::now();
    let spell = std::num::NonZeroU32::MIN;
    let cooldown_snapshot = ActorCooldownUpdate {
        value: ActorCooldowns {
            server_now_ms: 1000,
            rtt_ms: None,
            entries: vec![ActorCooldown {
                kind: ActorCooldownKind::Spell,
                id: spell.get(),
                expires_at_ms: 91_000,
            }],
        },
        received_at: decoded_at,
        snapshot: true,
    };
    let unaffected = std::num::NonZeroU32::new(2).ok_or("unaffected spell")?;
    let mut cooldown_snapshot = cooldown_snapshot;
    cooldown_snapshot.value.entries.push(ActorCooldown {
        kind: ActorCooldownKind::Spell,
        id: unaffected.get(),
        expires_at_ms: 11_000,
    });
    let mut shortened = cooldown_snapshot.clone();
    shortened.value.entries.truncate(1);
    shortened.snapshot = false;
    shortened.value.server_now_ms = 0;
    shortened.value.entries[0].expires_at_ms = 30_000;
    let task = tokio::spawn(run_session(
        Destination {
            events: vec![
                SessionEvent::ActivityScope(AppliedDelta {
                    server_sequence: 1,
                    base_revision: 0,
                    new_revision: 1,
                    value: scope.clone(),
                }),
                SessionEvent::ActorCooldowns(AppliedDelta {
                    server_sequence: 2,
                    base_revision: 0,
                    new_revision: 1,
                    value: shortened,
                }),
            ],
            cooldown_snapshot,
            scope,
            queries: queries.clone(),
            served: served.clone(),
        },
        commands,
        outbox,
    ));
    tokio::time::timeout(Duration::from_secs(2), served.notified()).await?;
    assert!(
        queries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_empty(),
        "destination modal or mark query ran before presentation"
    );
    assert_eq!(
        lock(&link.mailbox)
            .spell_bar
            .remaining_ms_at(spell, &[], decoded_at),
        30_000,
        "destination delta must apply while the scope waits for presentation"
    );
    assert_eq!(
        lock(&link.mailbox)
            .spell_bar
            .remaining_ms_at(unaffected, &[], decoded_at),
        10_000,
        "destination snapshot must be installed before presentation"
    );
    let mut view = view()?;
    view.tick(&link).map_err(|error| format!("{error:?}"))?;
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            served.notified().await;
            if queries
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .contains(&451)
            {
                break;
            }
        }
    })
    .await?;
    assert_eq!(lock(&link.mailbox).pushed.personal_map_marks, Some(marks()));
    assert_eq!(
        *queries.lock().unwrap_or_else(PoisonError::into_inner),
        vec![450, 451]
    );
    assert_eq!(
        lock(&link.mailbox)
            .spell_bar
            .remaining_ms_at(spell, &[], decoded_at),
        30_000,
        "destination initialization must preserve the shortened cooldown"
    );
    drop(link);
    task.await?;
    Ok(())
}
