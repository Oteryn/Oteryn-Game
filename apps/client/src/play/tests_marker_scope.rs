//! Automatic reads and queued feature commands must publish the destination barrier.
use super::*;
use oteryn_session::activity_scope::{ActivityScope, ActivityScopeSnapshot, encode_activity_scope};

struct AutomaticQueryHandoff {
    scope: ActivityScopeSnapshot,
    changed: bool,
    activity: bool,
    capability: u32,
    pending: Vec<SessionEvent>,
    serves: Arc<AtomicUsize>,
}
impl Stepper for AutomaticQueryHandoff {
    fn feature_selected(&self, capability: u32) -> bool {
        capability == self.capability
    }
    async fn feature(
        &mut self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
    ) -> Result<oteryn_session::GatedCommandResult, SessionError> {
        assert_eq!(command.capability, self.capability);
        assert!(
            !self.changed,
            "scope transition must not replay its source query"
        );
        self.changed = true;
        self.pending.push(SessionEvent::ActivityScope(AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: self.scope.clone(),
        }));
        self.pending.push(spatial_event(2, 32000));
        Err(if self.activity {
            SessionError::ActivityScopeChanged
        } else {
            SessionError::ScopeChanged
        })
    }
    fn feature_snapshot(&self, domain: u32) -> Option<Vec<u8>> {
        if self.changed && domain == 99 {
            encode_activity_scope(&self.scope).ok()
        } else {
            None
        }
    }
    fn drain(&mut self) -> Vec<SessionEvent> {
        std::mem::take(&mut self.pending)
    }
    async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
        self.serves.fetch_add(1, Ordering::SeqCst);
        tokio::time::sleep(duration).await;
        Ok(())
    }
    async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
        Ok(CombatReport::Stop(AttackIntentDisposition::Ok))
    }
    async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
        Ok(moved_to(32000, 200, 0))
    }
    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        Ok(Some(LogoutResult::Busy))
    }
}

fn automatic_query_handoff_continues(capability: u32, activity: bool) -> Result<(), String> {
    query_handoff_continues(capability, activity, None)
}

fn query_handoff_continues(
    capability: u32,
    activity: bool,
    queued: Option<crate::combat_input::PlayCommand>,
) -> Result<(), String> {
    let mut source = [1; 16];
    source[6] = 0x70;
    source[8] = 0x80;
    let mut destination = source;
    destination[15] = 2;
    let scope = ActivityScopeSnapshot {
        handoff: source,
        source_session: source,
        session: destination,
        scope: ActivityScope::Instance {
            world: source,
            instance: destination,
        },
        scope_generation: 2,
        connection_generation: 1,
        lease_generation: 2,
        selection: vec![117, capability],
        requires_ack: false,
    };
    let serves = Arc::new(AtomicUsize::new(0));
    let (link, commands, outbox) = play_channel();
    if let Some(command) = queued {
        link.commands
            .send(command)
            .map_err(|error| error.to_string())?;
    }
    let runtime = ClientRuntime::new().map_err(|error| error.to_string())?;
    runtime
        .spawn(run_session(
            AutomaticQueryHandoff {
                scope,
                changed: false,
                activity,
                capability,
                pending: Vec::new(),
                serves: Arc::clone(&serves),
            },
            commands,
            outbox,
        ))
        .map_err(|error| error.to_string())?;
    let mut published = false;
    for _ in 0..200 {
        match link.poll() {
            Some(PlayEvent::Pushed(pushed)) if pushed.activity_scope.is_some() => {
                assert_eq!(
                    pushed.activity_scope.as_ref().map(|scope| scope.session),
                    Some(destination)
                );
                assert_eq!(pushed.own.map(|position| position.x), Some(32000));
                published = true;
                link.scope_presented(destination);
            }
            Some(PlayEvent::Ended(error)) => {
                return Err(format!("automatic query handoff ended play: {error:?}"));
            }
            _ => {}
        }
        if published && serves.load(Ordering::SeqCst) > 1 {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        published,
        "automatic refresh must publish its committed destination"
    );
    assert!(
        serves.load(Ordering::SeqCst) > 1,
        "destination session must continue servicing liveness"
    );
    runtime.shutdown(Duration::from_millis(250));
    Ok(())
}

#[test]
fn marker_refresh_activity_scope_changed_publishes_destination_and_continues() -> Result<(), String>
{
    automatic_query_handoff_continues(451, true)
}
#[test]
fn marker_refresh_scope_changed_publishes_destination_and_continues() -> Result<(), String> {
    automatic_query_handoff_continues(451, false)
}

#[test]
fn modal_refresh_activity_scope_changed_publishes_destination_and_continues() -> Result<(), String>
{
    automatic_query_handoff_continues(450, true)
}

#[test]
fn modal_refresh_scope_changed_publishes_destination_and_continues() -> Result<(), String> {
    automatic_query_handoff_continues(450, false)
}

fn support_delivery_handoff_continues(activity: bool) -> Result<(), String> {
    use oteryn_client_runtime::support_intake::{CAPABILITY_SUPPORT_INTAKE_V1, CRASH};
    let command = oteryn_client_ui::support_intake::submission(
        CRASH,
        "Reference: scope-transition test",
        true,
    )
    .ok_or("valid support submission")?;
    query_handoff_continues(
        CAPABILITY_SUPPORT_INTAKE_V1,
        activity,
        Some(crate::combat_input::PlayCommand::SupportDelivery(
            command,
            std::path::PathBuf::from("unused-scope-transition-report.txt"),
        )),
    )
}

#[test]
fn support_delivery_scope_changed_publishes_destination_and_continues() -> Result<(), String> {
    support_delivery_handoff_continues(false)
}

#[test]
fn support_delivery_activity_scope_changed_publishes_destination_and_continues()
-> Result<(), String> {
    support_delivery_handoff_continues(true)
}
