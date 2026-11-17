//! Independent periodic reads must not inherit an earlier projection's holder grant.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::DurabilityRoot;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::gameplay_transport::AdmissionRetryBudget;
use crate::gameplay_transport::actor_spell::ActorVitals;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmissionRefusal, FirstEntryOutcome, FreshAdmissionAttempt, FreshAdmissionAuthority,
    IDLE_LIVENESS, serve_admitted,
};
use crate::gameplay_transport::world_spatial::{
    ActorPosition, ChannelEntities, ChannelEntity, VisibleKind, WorldSpatialObservation,
};
use std::cell::Cell;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::sync::Notify;

struct CadenceReads {
    root: DurabilityRoot,
    visibility: bool,
    session: GameSessionId,
    entered: Notify,
    resume: Notify,
    checked: Notify,
    armed: Cell<bool>,
    overtook: Cell<bool>,
    unit_completed: Cell<bool>,
    ticks: Cell<u8>,
    observed_before_tick: Cell<bool>,
}

fn vitals() -> (u64, ActorVitals) {
    (
        1,
        ActorVitals {
            health: 185,
            max_health: 185,
            mana: 90,
            max_mana: 90,
            soul: 100,
            harmony: 0,
            serene: false,
        },
    )
}

impl FreshAdmissionAuthority for CadenceReads {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<super::AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Unavailable)
    }
    async fn background_operation<F: Future>(&self, future: F) -> F::Output {
        self.root.with_background_operation(future).await
    }
    async fn background_operation_progress<F: Future>(&self, future: F) -> (F::Output, bool) {
        self.root.with_background_operation_progress(future).await
    }
    async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(WorldSpatialObservation {
            content_generation: [7; 32],
            actor_position: ActorPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
        })
    }
    async fn observe_sounds(
        &self,
        _: ExactActorRef,
        session: GameSessionId,
        after: u64,
    ) -> Option<(
        u64,
        Vec<oteryn_protocol_oteryn::presentation::PresentationEvent>,
    )> {
        if self.visibility && after != u64::MAX {
            FreshAdmissionStore::from_root(self.root.clone())
                .current_session_at(session)
                .await
                .ok()?;
            self.armed.set(true);
            self.entered.notify_one();
            self.resume.notified().await;
            self.unit_completed.set(
                FreshAdmissionStore::from_root(self.root.clone())
                    .current_session_at(session)
                    .await
                    .is_ok(),
            );
        }
        Some((0, Vec::new()))
    }
    async fn observe_visible_entities(&self, actor: ExactActorRef) -> Option<ChannelEntities> {
        if self.visibility && self.armed.replace(false) {
            self.overtook.set(
                FreshAdmissionStore::from_root(self.root.clone())
                    .current_session_at(self.session)
                    .await
                    .is_ok(),
            );
            self.checked.notify_one();
        }
        let own = self.observe(actor).await?;
        Some(ChannelEntities {
            content_generation: own.content_generation,
            observer: actor.placement_identity(),
            entities: vec![ChannelEntity {
                display_name: None,
                health_percent: 100,
                direction: oteryn_protocol_oteryn::world_spatial::StepDirection::South,
                kind: VisibleKind::Player,
                identity: actor.placement_identity(),
                generation: 0,
                position: own.actor_position,
                revision: 0,
            }],
            lights: Default::default(),
            observed_at_ms: 0,
        })
    }
    async fn tick_vitals(
        &self,
        _: ExactActorRef,
        session: GameSessionId,
    ) -> Option<(u64, ActorVitals)> {
        if self.ticks.replace(self.ticks.get() + 1) > 0 {
            let (_, mut value) = vitals();
            value.health = 180;
            return Some((3, value));
        }
        // The same canonical read used throughout the production vitals cadence;
        // on main its holder is attributed to admission_journal.rs:87.
        FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(session)
            .await
            .ok()?;
        self.armed.set(true);
        self.entered.notify_one();
        self.resume.notified().await;
        // The bounded component must still finish its own multi-pass work.
        self.unit_completed.set(
            FreshAdmissionStore::from_root(self.root.clone())
                .current_session_at(session)
                .await
                .is_ok(),
        );
        None
    }
    async fn observe_vitals(
        &self,
        _: ExactActorRef,
        session: GameSessionId,
    ) -> Option<(u64, ActorVitals)> {
        if self.visibility {
            return None;
        }
        let first = self.armed.replace(false);
        if first {
            let result = FreshAdmissionStore::from_root(self.root.clone())
                .current_session_at(session)
                .await;
            self.overtook.set(result.is_ok());
            self.checked.notify_one();
        }
        if !first && self.ticks.get() == 1 {
            self.observed_before_tick.set(true);
            let (_, mut value) = vitals();
            value.health = 182;
            return Some((2, value));
        }
        Some(vitals())
    }
}

#[test]
fn serene_journal_reads_yield_to_training_between_independent_units_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::item_look::large_stack(|| scenario(false))
}

#[test]
fn visibility_journal_reads_yield_to_appearance_between_independent_units_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::item_look::large_stack(|| scenario(true))
}

fn scenario(visibility: bool) -> TestResult {
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "serene_holder_starvation", false, false)
                .await?;
        prepare_current_session(&h).await?;
        let deps = Dependencies::new()?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let offered = [
            crate::gameplay_transport::capabilities::OfferedCapability {
                id: 6,
                requires: &[],
            },
            crate::gameplay_transport::capabilities::OfferedCapability {
                id: 23,
                requires: &[6],
            },
        ];
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offered, if visibility { &[6, 23] } else { &[] })
                .ok_or("capabilities")?;
        assert_eq!(
            admitted.continuity.selected_capabilities.contains(23),
            visibility
        );
        let cadence = CadenceReads {
            root: h.root.clone(),
            visibility,
            session: admitted.game_session_id,
            entered: Notify::new(),
            resume: Notify::new(),
            checked: Notify::new(),
            armed: Cell::new(false),
            overtook: Cell::new(false),
            unit_completed: Cell::new(false),
            ticks: Cell::new(0),
            observed_before_tick: Cell::new(false),
        };
        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        let result: TestResult = tokio::time::timeout(Duration::from_secs(10), async {
            let serving = DurabilityRoot::with_holder_owner(serve_admitted(&mut server, admitted, &cadence, IDLE_LIVENESS));
            tokio::pin!(serving);
            tokio::select! {
                end = &mut serving => return Err(format!("connection ended before cadence: {end:?}").into()),
                () = cadence.entered.notified() => {},
            }
            assert!(h.root.is_ready(), "the first journal read returned its holder");
            let budget = AdmissionRetryBudget::for_foreground_command(&h.root).map_err(debug)?;
            cadence.resume.notify_one();
            // Poll the background continuation first, before the foreground read.
            tokio::select! {
                end = &mut serving => return Err(format!("connection ended during cadence: {end:?}").into()),
                () = cadence.checked.notified() => {},
            }
            assert!(cadence.unit_completed.get(), "bounded multi-pass component lost its grant");
            assert!(!cadence.overtook.get(), "periodic refresh reused a journal grant to overtake waiting foreground work");
            let store = FreshAdmissionStore::from_root(h.root.clone());
            let (current, _) = budget.retry_durability(if visibility { "appearance_observation" } else { "training_session" }, async || {
                store.current_session_at(admitted.game_session_id).await
            }).await.map_err(debug)?;
            assert_eq!(current.current_game_session_id(), admitted.game_session_id);
            drop(budget);
            if !visibility {
                let read_latest = async {
                    loop {
                        let length = usize::try_from(client.read_u32().await?)?;
                        if length > 1 << 20 { return Err("oversized frame".into()); }
                        let mut bytes = vec![0; length];
                        client.read_exact(&mut bytes).await?;
                        let envelope = oteryn_protocol_oteryn::decode_wire_envelope(&bytes).map_err(debug)?;
                        if envelope.message_type() == oteryn_protocol_oteryn::MessageType::StateDelta {
                            let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload()).map_err(debug)?;
                            if delta.domain_id == 3 && delta.new_revision == 3 {
                                let value = oteryn_protocol_oteryn::actor_spell::decode_actor_vitals(delta.payload).map_err(debug)?;
                                assert_eq!(value.health, 180, "rotated sweep published the older observation");
                                return Ok::<(), Box<dyn std::error::Error>>(());
                            }
                        }
                    }
                };
                tokio::select! {
                    end = &mut serving => return Err(format!("connection ended after handoff: {end:?}").into()),
                    result = read_latest => result?,
                }
                assert!(cadence.observed_before_tick.get(), "refused components did not receive the next turn");
            }
            eprintln!("PG_EXECUTED holder_starvation visibility={visibility} background_refused=1 foreground=ok");
            Ok(())
        }).await?;
        let cleanup = h.cleanup().await;
        result?;
        cleanup
    })
}

#[test]
fn background_training_and_appearance_yield_after_one_busy_journal_read_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::item_look::large_stack(|| {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(
                admin,
                "background_journal_yield",
                false,
                false,
            )
            .await?;
            prepare_current_session(&h).await?;
            let deps = Dependencies::new()?;
            let store = FreshAdmissionStore::from_root(h.root.clone());
            let held = h.root.try_issue_semantic_pass().map_err(debug)?;
            for stage in ["training_session", "appearance_observation"] {
                let calls = Cell::new(0);
                let result = DurabilityRoot::with_background_holder(async {
                    let budget = AdmissionRetryBudget::for_command(&h.root);
                    budget
                        .retry_durability(stage, async || {
                            calls.set(calls.get() + 1);
                            store
                                .current_session_at(deps.admitted.game_session_id)
                                .await
                        })
                        .await
                })
                .await;
                assert!(matches!(
                    result,
                    Err(crate::durability::DurabilityError::RootUnavailable)
                ));
                assert_eq!(
                    calls.get(),
                    1,
                    "scheduled {stage} retried a refused checkout instead of yielding"
                );
            }
            // A foreground checkpoint still uses the existing retry owner and
            // reports failure rather than acknowledging an unavailable read.
            let calls = Cell::new(0);
            let budget = AdmissionRetryBudget::for_foreground_command(&h.root).map_err(debug)?;
            let result = budget
                .retry_durability("training_session", async || {
                    calls.set(calls.get() + 1);
                    store
                        .current_session_at(deps.admitted.game_session_id)
                        .await
                })
                .await;
            assert!(matches!(
                result,
                Err(crate::durability::DurabilityError::RootUnavailable)
            ));
            assert!(calls.get() > 1, "foreground retry policy changed");
            drop(budget);
            drop(held);
            eprintln!(
                "PG_EXECUTED background_journal_yield stages=2 attempts_each=1 foreground_retries={}",
                calls.get()
            );
            h.cleanup().await
        })
    })
}
