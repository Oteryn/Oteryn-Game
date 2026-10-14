//! PREM-WIRE-1: the spell casts read the one Premium fence of Game, the PREM-1
//! [`PremiumConsumer`] the node's [`PremiumRefresher`](crate::premium::refresh::PremiumRefresher)
//! pulls, fences and proves (PREMIUM-DELIVERY-0 §6). There is no second Premium store: a view
//! reads the consumer live at every use, so a failed pull, a revocation, a lapsed lease or a lost
//! clock closes it at once, and a restart reconstructs nothing.
//!
//! [`SpellPremiumCoordinator::premium_gate`] and [`ComposedFreshAdmission::premium_gate`] are the
//! account-level reads, for Premium-gated decisions that are not spell casts. Every read needs a
//! registered trusted clock owner; without one every account reads Free.
use super::{
    ComposedFreshAdmission,
    spell_access_facts::{AccessFactsError, CurrentSpellAccessOwner, owner_registration},
};
use crate::domain::premium::PremiumGate;
use crate::durability::{
    fresh_admission::FreshAdmissionStore, item_transfer::CurrentCharacterItemFence,
};
use crate::foundation::{ExactActorRef, GameSessionId, GameSessionState, RuntimeScopeRefV1};
use crate::premium::{PremiumConsumer, TrustedNow};
use crate::spell::owned_cast_facts::{AccessProjections, CastFactsBinding, CurrentProjection};
use std::sync::Arc;
/// Explicit registration from a genuine independent trusted clock owner. SystemTime, database
/// receipt time and a readiness field with guessed zero uncertainty cannot implement this port.
pub(crate) mod clock_registration {
    pub(crate) trait Registered {}
}
pub(crate) trait RegisteredPremiumClock:
    clock_registration::Registered + Send + Sync
{
    /// The current trusted window (consumer contract §7), or `None` while unsynchronized.
    fn read_current_window(&self) -> Option<TrustedNow>;
}
pub(crate) enum RegisteredSpellAccessOwner {
    Unavailable {
        account: Option<[u8; 16]>,
    },
    Premium {
        account: [u8; 16],
        consumer: Arc<PremiumConsumer>,
        clock: Arc<dyn RegisteredPremiumClock>,
    },
}
impl RegisteredSpellAccessOwner {
    pub(crate) fn unavailable() -> Self {
        Self::Unavailable { account: None }
    }
    /// The account's Premium standing now. Closed for an unavailable view, a lost clock, and
    /// every class but current authority (PREMIUM-ACTIVATION §4.1).
    pub(crate) fn premium_gate(&self) -> PremiumGate {
        match self {
            Self::Unavailable { .. } => PremiumGate::CLOSED,
            Self::Premium {
                account,
                consumer,
                clock,
            } => match clock.read_current_window() {
                Some(now) => consumer.gate(*account, Some(now)),
                None => PremiumGate::CLOSED,
            },
        }
    }
}
/// The Premium projection of `account` for `binding`, read live: present only for current
/// authority, valid in the owner's monotonic domain (`now`) until the benefit ends. The remaining
/// time is taken from the conservative upper bound of the trusted window.
fn premium_projection(
    consumer: &PremiumConsumer,
    clock: &dyn RegisteredPremiumClock,
    account: [u8; 16],
    binding: &CastFactsBinding,
    now: u64,
) -> Option<CurrentProjection<bool>> {
    let trusted = clock.read_current_window()?;
    let current = consumer.current_premium(account, Some(trusted))?;
    let remaining = u64::try_from(current.benefit_until_us.checked_sub(trusted.upper_us())?)
        .ok()
        .filter(|remaining| *remaining > 0)?;
    Some(CurrentProjection {
        binding: binding.clone(),
        authority_revision: current.authority_revision,
        valid_until_micros: now.checked_add(remaining)?,
        value: true,
    })
}
impl owner_registration::Registered for RegisteredSpellAccessOwner {}
impl CurrentSpellAccessOwner for RegisteredSpellAccessOwner {
    fn account_id(&self) -> Option<[u8; 16]> {
        match self {
            Self::Unavailable { account } => *account,
            Self::Premium { account, .. } => Some(*account),
        }
    }
    fn read_current(
        &self,
        binding: &CastFactsBinding,
        now: u64,
    ) -> Result<AccessProjections, AccessFactsError> {
        Ok(match self {
            Self::Unavailable { .. } => AccessProjections::default(),
            Self::Premium {
                account,
                consumer,
                clock,
            } => AccessProjections {
                premium: premium_projection(consumer, clock.as_ref(), *account, binding, now),
                ..AccessProjections::default()
            },
        })
    }
}
impl owner_registration::Registered for Arc<RegisteredSpellAccessOwner> {}
impl CurrentSpellAccessOwner for Arc<RegisteredSpellAccessOwner> {
    fn account_id(&self) -> Option<[u8; 16]> {
        self.as_ref().account_id()
    }
    fn read_current(
        &self,
        binding: &CastFactsBinding,
        now: u64,
    ) -> Result<AccessProjections, AccessFactsError> {
        self.as_ref().read_current(binding, now)
    }
}
/// The node's Premium reads: the one PREM-1 consumer and the registered trusted clock owner.
pub(crate) struct SpellPremiumCoordinator {
    consumer: Arc<PremiumConsumer>,
    clock: Option<Arc<dyn RegisteredPremiumClock>>,
}
impl SpellPremiumCoordinator {
    pub(crate) fn new(
        consumer: Arc<PremiumConsumer>,
        clock: Option<Arc<dyn RegisteredPremiumClock>>,
    ) -> Self {
        Self { consumer, clock }
    }
    /// The consumer the node's Premium refresher must pull into: the same fence these reads use.
    pub(crate) fn consumer(&self) -> Arc<PremiumConsumer> {
        Arc::clone(&self.consumer)
    }
    /// The live view of `account`. Unavailable without a registered clock owner.
    pub(crate) fn view(&self, account: [u8; 16]) -> Arc<RegisteredSpellAccessOwner> {
        Arc::new(match &self.clock {
            Some(clock) => RegisteredSpellAccessOwner::Premium {
                account,
                consumer: Arc::clone(&self.consumer),
                clock: Arc::clone(clock),
            },
            None => RegisteredSpellAccessOwner::Unavailable {
                account: Some(account),
            },
        })
    }
    /// The account's Premium standing now: the one gate a Premium-gated decision reads. Closed
    /// for an account the consumer does not hold proven current, so restart and failure deny.
    pub(crate) fn premium_gate(&self, account: [u8; 16]) -> PremiumGate {
        self.view(account).premium_gate()
    }
}
/// Private binding produced only by actual session/Character/recovery/node/owner reads below.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PremiumOwnerBinding {
    account: [u8; 16],
    actor: ExactActorRef,
    session: GameSessionId,
    fence: CurrentCharacterItemFence,
    character_revision: u64,
    content_digest: [u8; 32],
    placement_identity: [u8; 16],
    placement_revision: u64,
    player_revision: u64,
    equipment: crate::durability::character_equipment::EquipmentSnapshot,
    build: crate::durability::character_build::DurableBuildState,
    level: u32,
}
impl ComposedFreshAdmission<'_, '_, '_> {
    async fn current_premium_binding(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<PremiumOwnerBinding> {
        let active = self.active_generation?;
        let current = FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(session)
            .await
            .ok()?
            .0;
        if current.session_state() != GameSessionState::Active {
            return None;
        }
        let character =
            crate::domain::CharacterId::from_bytes(*current.commit().character_id().as_bytes())
                .ok()?;
        let record = self
            .root
            .read_current_character(self.character, character)
            .await
            .ok()?;
        if record.world_id.as_bytes() != self.world_id.as_bytes() {
            return None;
        }
        let gameplay = crate::durability::character_progression::CurrentCharacterGameplayFence {
            character_id: character,
            game_session_id: session,
            connection_generation: current.current_connection_generation(),
            character_lease_generation: current.current_character_lease().generation(),
            runtime_scope: current.current_runtime_scope(),
            scope_ownership_generation: current.current_scope_generation(),
            expected_character_revision: record.revision,
        };
        let digest = active.identity().server_artifact_digest();
        let raw = self
            .root
            .read_lifecycle_durable_facts(self.character, self.holder, &gameplay, digest)
            .await
            .ok()?;
        if raw.account != *record.account_id.as_bytes() {
            return None;
        }
        let runtime = self.runtime.lock().await;
        let scope = RuntimeScopeRefV1::channel(
            runtime.binding().world_id(),
            runtime.binding().channel_id(),
        );
        if raw.fence.runtime_scope != scope
            || raw.fence.scope_ownership_generation != runtime.binding().scope_generation()
            || runtime.content_pin().server_artifact_digest() != digest
            || raw.equipment.character_revision != record.revision.get()
        {
            return None;
        }
        let physical = runtime.player_control_facts(actor, session).ok()?;
        if physical.control_loss.is_some() {
            return None;
        }
        let mut states = self.spell_states.lock().await;
        let state = states.get_mut(&runtime, actor, session)?;
        let player_revision = state.revision();
        Some(PremiumOwnerBinding {
            account: raw.account,
            actor,
            session,
            fence: raw.fence,
            character_revision: record.revision.get(),
            content_digest: digest,
            placement_identity: physical.placement_identity,
            placement_revision: physical.placement_revision,
            player_revision,
            equipment: raw.equipment,
            build: raw.build,
            level: raw.level,
        })
    }
    /// Apply the account transition only to the genuine current actor, after all current
    /// SQL reads and before source caster facts are sampled. Pending accepted snapshots freeze.
    pub(super) async fn apply_refreshed_spell_access(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        access: &impl CurrentSpellAccessOwner,
    ) -> Result<bool, AccessFactsError> {
        let current = self.current_premium_binding(actor, session).await.ok_or(
            AccessFactsError::Unavailable("current Premium transition owners"),
        )?;
        if access
            .account_id()
            .is_some_and(|account| account != current.account)
        {
            return Err(AccessFactsError::Unavailable(
                "Premium transition account changed",
            ));
        }
        let runtime = self.runtime.lock().await;
        let mut states = self.spell_states.lock().await;
        if states.has_pending_spell_commit(actor, session) {
            return Ok(false);
        }
        if runtime.content_pin().server_artifact_digest() != current.content_digest
            || runtime.binding().scope_generation() != current.fence.scope_ownership_generation
            || current.fence.runtime_scope
                != RuntimeScopeRefV1::channel(
                    runtime.binding().world_id(),
                    runtime.binding().channel_id(),
                )
        {
            return Err(AccessFactsError::Unavailable(
                "Premium transition scope/Content changed",
            ));
        }
        let physical = runtime
            .player_control_facts(actor, session)
            .map_err(|_| AccessFactsError::Unavailable("Premium transition current actor"))?;
        if physical.control_loss.is_some()
            || physical.placement_identity != current.placement_identity
            || physical.placement_revision != current.placement_revision
        {
            return Err(AccessFactsError::Unavailable(
                "Premium transition placement changed",
            ));
        }
        let state =
            states
                .get_mut(&runtime, actor, session)
                .ok_or(AccessFactsError::Unavailable(
                    "Premium transition current player",
                ))?;
        if state.revision() != current.player_revision {
            return Err(AccessFactsError::Unavailable(
                "Premium transition player changed",
            ));
        }
        let now = self.owner_now().get();
        let binding = CastFactsBinding {
            actor,
            session,
            character: *current.fence.character_id.as_bytes(),
            character_revision: current.character_revision,
            lease_generation: current.fence.character_lease_generation,
            connection_generation: current.fence.connection_generation.get(),
            player_revision: current.player_revision,
            content_digest: current.content_digest,
            equipment_revision: current.equipment.revision,
        };
        let projections = access.read_current(&binding, now)?;
        let facts = crate::spell::owned_cast_facts::OwnedCastFacts::from_owner_reads(
            binding,
            current.build,
            current.level,
            current.equipment,
            projections,
        )?;
        states
            .apply_current_premium(&runtime, actor, session, &facts, now)
            .map_err(|_| AccessFactsError::Unavailable("Premium transition refused"))
    }
    pub(super) async fn refresh_and_apply_spell_access(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Arc<RegisteredSpellAccessOwner> {
        let view = self.refresh_spell_access(actor, session).await;
        if self
            .apply_refreshed_spell_access(actor, session, &view)
            .await
            .is_err()
        {
            return Arc::new(RegisteredSpellAccessOwner::Unavailable {
                account: view.account_id(),
            });
        }
        view
    }
    /// The account selector is derived from current Game owners, never the client or caller.
    pub(super) async fn refresh_spell_access(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Arc<RegisteredSpellAccessOwner> {
        let Some(coordinator) = self.premium_coordinator else {
            return Arc::new(RegisteredSpellAccessOwner::unavailable());
        };
        let Some(current) = self.current_premium_binding(actor, session).await else {
            return Arc::new(RegisteredSpellAccessOwner::unavailable());
        };
        coordinator.view(current.account)
    }
    /// PREM-WIRE-1: the Premium gate of `account` (MAGIC-CAST-1 §4.6, Premium areas, purchases,
    /// houses, guilds). Closed on a node without the Premium reads.
    pub(crate) fn premium_gate(&self, account: [u8; 16]) -> PremiumGate {
        self.premium_coordinator
            .map_or(PremiumGate::CLOSED, |coordinator| {
                coordinator.premium_gate(account)
            })
    }
    /// The Premium gate of the account admitted with `session` on this node: closed for a
    /// session holding no Premium registration.
    pub(crate) fn session_premium_gate(&self, session: GameSessionId) -> PremiumGate {
        let account = self
            .premium_sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .account_of(session);
        account.map_or(PremiumGate::CLOSED, |account| self.premium_gate(account))
    }
}
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::domain::premium::PremiumRefusal;
    use crate::durability::premium_fence::{EntitlementState, PremiumEvidence};
    use std::sync::Mutex;

    const ACCOUNT: [u8; 16] = [1; 16];
    /// 2026-10-01T00:00:00Z in Unix microseconds.
    const T0: i64 = 1_790_812_800_000_000;
    const MINUTE: i64 = 60_000_000;

    struct TestClock(Mutex<Option<TrustedNow>>);
    impl clock_registration::Registered for TestClock {}
    impl RegisteredPremiumClock for TestClock {
        fn read_current_window(&self) -> Option<TrustedNow> {
            *self.0.lock().unwrap()
        }
    }
    fn clock_at(now_us: i64) -> Arc<TestClock> {
        Arc::new(TestClock(Mutex::new(TrustedNow::new(now_us, 1_000_000))))
    }
    /// Evidence issued at `T0` with a one-hour lease and `refresh_after` at 30 minutes.
    fn evidence(state: EntitlementState, effective_until_us: i64) -> PremiumEvidence {
        PremiumEvidence {
            account_id: ACCOUNT,
            producer_revision: "1".repeat(40),
            producer_profile: crate::premium::PRODUCER_PROFILE.to_owned(),
            product_id: crate::premium::PRODUCT_ID.to_owned(),
            product_version: crate::premium::PRODUCT_VERSION,
            entitlement_id: Some("01926c1e-8a40-7c3b-9f2e-5a1d3c4b6e70".to_owned()),
            state,
            lifecycle_revision: 1,
            authority_revision: 7,
            effective_from_us: T0 - 30 * 24 * 60 * MINUTE,
            effective_until_us,
            authority_issued_at_us: T0,
            authority_valid_until_us: T0 + 60 * MINUTE,
            refresh_after_us: T0 + 30 * MINUTE,
        }
    }
    fn proven(state: EntitlementState, effective_until_us: i64) -> Arc<PremiumConsumer> {
        let consumer = Arc::new(PremiumConsumer::default());
        consumer.prove_for_test(evidence(state, effective_until_us));
        consumer
    }
    fn binding() -> CastFactsBinding {
        let (_, actor, session) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(87);
        CastFactsBinding {
            actor,
            session,
            character: [3; 16],
            character_revision: 1,
            lease_generation: 1,
            connection_generation: 1,
            player_revision: 1,
            content_digest: [4; 32],
            equipment_revision: 1,
        }
    }
    const FAR: i64 = T0 + 30 * 24 * 60 * MINUTE;

    #[test]
    fn a_proven_current_account_opens_the_gate_and_the_spell_projection() {
        let clock = clock_at(T0 + MINUTE);
        let coordinator =
            SpellPremiumCoordinator::new(proven(EntitlementState::Active, FAR), Some(clock));
        assert_eq!(coordinator.premium_gate(ACCOUNT).require(), Ok(()));
        let view = coordinator.view(ACCOUNT);
        assert_eq!(view.premium_gate(), coordinator.premium_gate(ACCOUNT));
        let projection = view
            .read_current(&binding(), 5_000)
            .unwrap()
            .premium
            .unwrap();
        assert!(projection.value);
        assert_eq!(projection.authority_revision, 7);
        // Valid until `refresh_after`, from the window's upper bound, in the owner's domain.
        assert_eq!(
            projection.valid_until_micros,
            5_000 + u64::try_from(29 * MINUTE - 1_000_000).unwrap()
        );
        assert!(projection.current(&binding(), projection.valid_until_micros - 1));
        assert!(!projection.current(&binding(), projection.valid_until_micros));
    }

    #[test]
    fn unknown_expired_and_every_other_class_fail_closed() {
        let ended = [
            (EntitlementState::Expired, FAR, T0 + MINUTE),
            (EntitlementState::Revoked, FAR, T0 + MINUTE),
            (EntitlementState::None, FAR, T0 + MINUTE),
            (EntitlementState::NotYetEffective, FAR, T0 + MINUTE),
            // effective_until passed, refresh_after passed, the authority lease lapsed.
            (EntitlementState::Active, T0 + 10 * MINUTE, T0 + 10 * MINUTE),
            (EntitlementState::Active, FAR, T0 + 30 * MINUTE),
            (EntitlementState::Active, FAR, T0 + 60 * MINUTE),
        ];
        for (state, until, now) in ended {
            let coordinator =
                SpellPremiumCoordinator::new(proven(state, until), Some(clock_at(now)));
            assert_eq!(
                coordinator.premium_gate(ACCOUNT).require(),
                Err(PremiumRefusal::NotCurrent)
            );
            let access = coordinator
                .view(ACCOUNT)
                .read_current(&binding(), 1)
                .unwrap();
            assert!(access.premium.is_none());
        }
        // An unknown account, and the same proven account on another account's gate.
        let coordinator = SpellPremiumCoordinator::new(
            proven(EntitlementState::Active, FAR),
            Some(clock_at(T0 + MINUTE)),
        );
        assert_eq!(coordinator.premium_gate([2; 16]), PremiumGate::CLOSED);
        // A failed pull and a released account close it at once, for views already issued.
        let view = coordinator.view(ACCOUNT);
        let ticket = coordinator.consumer().ticket();
        coordinator.consumer().pull_failed(ACCOUNT, ticket);
        assert_eq!(view.premium_gate(), PremiumGate::CLOSED);
        assert!(view.read_current(&binding(), 1).unwrap().premium.is_none());
        let coordinator = SpellPremiumCoordinator::new(
            proven(EntitlementState::Active, FAR),
            Some(clock_at(T0 + MINUTE)),
        );
        let view = coordinator.view(ACCOUNT);
        coordinator.consumer().release(ACCOUNT);
        assert_eq!(view.premium_gate(), PremiumGate::CLOSED);
    }

    #[test]
    fn no_clock_owner_a_lost_clock_and_a_restart_fail_closed() {
        // No registered clock owner: every view is unavailable.
        let unclocked = SpellPremiumCoordinator::new(proven(EntitlementState::Active, FAR), None);
        assert_eq!(unclocked.premium_gate(ACCOUNT), PremiumGate::CLOSED);
        assert!(matches!(
            unclocked.view(ACCOUNT).as_ref(),
            RegisteredSpellAccessOwner::Unavailable { account: Some(a) } if *a == ACCOUNT
        ));
        assert_eq!(
            RegisteredSpellAccessOwner::unavailable().premium_gate(),
            PremiumGate::CLOSED
        );
        // A clock that loses synchronization closes views already issued.
        let clock = clock_at(T0 + MINUTE);
        let coordinator = SpellPremiumCoordinator::new(
            proven(EntitlementState::Active, FAR),
            Some(clock.clone()),
        );
        let view = coordinator.view(ACCOUNT);
        assert!(view.premium_gate().premium_current());
        *clock.0.lock().unwrap() = None;
        assert_eq!(view.premium_gate(), PremiumGate::CLOSED);
        assert!(view.read_current(&binding(), 1).unwrap().premium.is_none());
        // A restart: a fresh consumer holds nothing proven.
        let restarted = SpellPremiumCoordinator::new(Arc::default(), Some(clock_at(T0 + MINUTE)));
        assert_eq!(restarted.premium_gate(ACCOUNT), PremiumGate::CLOSED);
    }
}
