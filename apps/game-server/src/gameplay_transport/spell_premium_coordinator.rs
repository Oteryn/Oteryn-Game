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
use crate::domain::premium::{PreDelivery, PremiumGate, PremiumStatus};
use crate::durability::item_mint_audit::Type2Transaction;
use crate::durability::premium_activation_latch::{
    EmptyLatchHeld, PreDeliveryBypass, PremiumActivationLatch, PremiumActivationRead,
    PremiumGateTx, PromotionPreDeliveryBypass, PromotionPremiumGateOperation,
};
use crate::durability::{
    DurabilityError, DurabilityRoot, fresh_admission::FreshAdmissionStore,
    item_transfer::CurrentCharacterItemFence,
};
use crate::foundation::{ExactActorRef, GameSessionId, GameSessionState, RuntimeScopeRefV1};
use crate::premium::{PremiumConsumer, TrustedNow};
use crate::spell::owned_cast_facts::{AccessProjections, CastFactsBinding, CurrentProjection};
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::{Arc, OnceLock};
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
/// The production clock owner: the kernel's NTP-disciplined clock and its maximum error
/// (PREMIUM-ACTIVATION-0 §1.1), read at every use.
pub(crate) struct SystemTrustedClock;
impl clock_registration::Registered for SystemTrustedClock {}
impl RegisteredPremiumClock for SystemTrustedClock {
    fn read_current_window(&self) -> Option<TrustedNow> {
        crate::premium::clock::system_trusted_now()
    }
}
/// The configured Premium activation record: its id and switch-over instant `S`, in Unix
/// microseconds (PREMIUM-ACTIVATION-0 §1.2, PREMIUM-DELIVERY-0 §10.3). Production composition
/// passes `None`: setting one needs PREM-1's activation record and separate owner authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PremiumActivation {
    pub(crate) activation_id: String,
    pub(crate) switch_over_us: i64,
}
/// Whether Premium has been delivered on this deployment: the configured activation and the
/// durable latch row, once this process has read or written it. Only ever moves from "none held"
/// to "held"; while none is held, the gate re-reads the table.
#[derive(Debug, Default)]
pub(crate) struct PremiumDelivery {
    activation: Option<PremiumActivation>,
    held: OnceLock<PremiumActivationLatch>,
}
impl PremiumDelivery {
    pub(crate) fn new(activation: Option<PremiumActivation>) -> Self {
        Self {
            activation,
            held: OnceLock::new(),
        }
    }
    fn hold(&self, latch: PremiumActivationLatch) {
        let _ = self.held.set(latch);
    }
    /// §1.2 step 1, from memory: `Some(now)` only when a row is held, it is the configured
    /// activation, the clock reads and the whole window is at or after `S`. `None` otherwise.
    fn delivered_now(&self, now: Option<TrustedNow>) -> Option<TrustedNow> {
        let latch = self.held.get()?;
        let activation = self.activation.as_ref()?;
        let now = now?;
        (activation.activation_id == latch.activation_id
            && activation.switch_over_us == latch.switch_over_us
            && now.lower_us() >= latch.switch_over_us)
            .then_some(now)
    }
    /// The status of `account` with a held row (§1.2 step 1): never `NotActivated`.
    fn delivered_status(
        &self,
        consumer: &PremiumConsumer,
        account: [u8; 16],
        now: Option<TrustedNow>,
    ) -> PremiumStatus<'static> {
        match self.delivered_now(now) {
            Some(now) if consumer.premium_current(account, Some(now)) => PremiumStatus::Current,
            _ => PremiumStatus::NotCurrent,
        }
    }
}
/// What the gate decides for an empty latch table, from the activation and the clock (§1.2 steps
/// 3-6), before it reads the table.
enum EmptyLatch {
    NotActivated,
    NotCurrent,
    Latch { now: TrustedNow },
}
/// A Premium gate operation: runs once with the account's status and, for `NotActivated` only,
/// the gate's transaction (PREMIUM-ACTIVATION-0 §1.2). It owns what it captures.
pub(crate) trait PremiumGateOperation<T>:
    for<'g, 't> FnOnce(
        PremiumStatus<'g>,
        Option<&'g mut PremiumGateTx<'t>>,
    ) -> Pin<Box<dyn Future<Output = T> + Send + 'g>>
    + Send
    + 'static
{
}
impl<T, F> PremiumGateOperation<T> for F where
    F: for<'g, 't> FnOnce(
            PremiumStatus<'g>,
            Option<&'g mut PremiumGateTx<'t>>,
        ) -> Pin<Box<dyn Future<Output = T> + Send + 'g>>
        + Send
        + 'static
{
}

struct PromotionGateBypass<T, E, F> {
    operation: F,
    _output: PhantomData<fn() -> Result<T, E>>,
}

impl<T: Send, E: Send, F: PromotionPremiumGateOperation<T, E>> PromotionPreDeliveryBypass
    for PromotionGateBypass<T, E, F>
{
    type Output = T;
    type Error = E;
    fn apply<'a, 't>(
        self,
        held: &'a EmptyLatchHeld,
        tx: &'a mut Type2Transaction<'t>,
    ) -> Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'a>>
    where
        Self: 'a,
    {
        self.operation.run(
            PremiumStatus::NotActivated(PreDelivery::inside_gate(held)),
            Some(tx),
        )
    }
}
/// A multi-account Premium gate operation: runs once with one status per account, in order, and,
/// for `NotActivated` only, the gate's transaction (PREMIUM-ACTIVATION-0 §1.2).
pub(crate) trait PremiumBatchGateOperation<T>:
    for<'g, 't> FnOnce(
        Vec<PremiumStatus<'g>>,
        Option<&'g mut PremiumGateTx<'t>>,
    ) -> Pin<Box<dyn Future<Output = T> + Send + 'g>>
    + Send
    + 'static
{
}
impl<T, F> PremiumBatchGateOperation<T> for F where
    F: for<'g, 't> FnOnce(
            Vec<PremiumStatus<'g>>,
            Option<&'g mut PremiumGateTx<'t>>,
        ) -> Pin<Box<dyn Future<Output = T> + Send + 'g>>
        + Send
        + 'static
{
}
/// What the gate settled for one command: the bypass applied in its transaction, or handed back
/// with whether a held row decides the status.
enum Settled<B: PreDeliveryBypass> {
    Applied(B::Output),
    Decided { bypass: B, delivered: bool },
}
/// An operation with a status decided after delivery: no transaction.
async fn run_delivered<T, F: PremiumGateOperation<T>>(
    operation: F,
    status: PremiumStatus<'static>,
) -> T {
    operation(status, None).await
}
/// The operation as the latch read's pre-delivery bypass.
struct GateBypass<T, F> {
    operation: F,
    _output: PhantomData<fn() -> T>,
}
impl<T: Send, F: PremiumGateOperation<T>> PreDeliveryBypass for GateBypass<T, F> {
    type Output = T;
    fn apply<'a, 't>(
        self,
        held: &'a EmptyLatchHeld,
        tx: &'a mut PremiumGateTx<'t>,
    ) -> Pin<Box<dyn Future<Output = T> + Send + 'a>>
    where
        Self: 'a,
    {
        (self.operation)(
            PremiumStatus::NotActivated(PreDelivery::inside_gate(held)),
            Some(tx),
        )
    }
}
/// The multi-account operation as the latch read's pre-delivery bypass: every account is
/// `NotActivated`.
struct BatchBypass<T, F> {
    accounts: usize,
    operation: F,
    _output: PhantomData<fn() -> T>,
}
impl<T: Send, F: PremiumBatchGateOperation<T>> PreDeliveryBypass for BatchBypass<T, F> {
    type Output = T;
    fn apply<'a, 't>(
        self,
        held: &'a EmptyLatchHeld,
        tx: &'a mut PremiumGateTx<'t>,
    ) -> Pin<Box<dyn Future<Output = T> + Send + 'a>>
    where
        Self: 'a,
    {
        let statuses = (0..self.accounts)
            .map(|_| PremiumStatus::NotActivated(PreDelivery::inside_gate(held)))
            .collect();
        (self.operation)(statuses, Some(tx))
    }
}
pub(crate) enum RegisteredSpellAccessOwner {
    Unavailable {
        account: Option<[u8; 16]>,
    },
    Premium {
        account: [u8; 16],
        consumer: Arc<PremiumConsumer>,
        clock: Arc<dyn RegisteredPremiumClock>,
        delivery: Arc<PremiumDelivery>,
    },
}
impl RegisteredSpellAccessOwner {
    pub(crate) fn unavailable() -> Self {
        Self::Unavailable { account: None }
    }
    /// The account's Premium standing now, from memory: open only once Premium has been
    /// delivered (a held latch row) and the account reads current authority (§1.2 step 1).
    /// Closed for an unavailable view, a lost clock and every other class.
    #[cfg_attr(
        not(test),
        allow(dead_code, reason = "MAGIC-CAST-1 §4.6 reads the gate from the view")
    )]
    pub(crate) fn premium_gate(&self) -> PremiumGate {
        match self {
            Self::Unavailable { .. } => PremiumGate::CLOSED,
            Self::Premium {
                account,
                consumer,
                clock,
                delivery,
            } => delivery
                .delivered_status(consumer, *account, clock.read_current_window())
                .gate(),
        }
    }
}
/// The Premium projection of `account` for `binding`, read live: present only once Premium has
/// been delivered and for current authority, valid in the owner's monotonic domain (`now`) until
/// the benefit ends. The remaining time is taken from the conservative upper bound of the trusted
/// window.
fn premium_projection(
    consumer: &PremiumConsumer,
    clock: &dyn RegisteredPremiumClock,
    delivery: &PremiumDelivery,
    account: [u8; 16],
    binding: &CastFactsBinding,
    now: u64,
) -> Option<CurrentProjection<bool>> {
    let trusted = delivery.delivered_now(clock.read_current_window())?;
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
                delivery,
            } => AccessProjections {
                premium: premium_projection(
                    consumer,
                    clock.as_ref(),
                    delivery,
                    *account,
                    binding,
                    now,
                ),
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
/// The node's Premium reads: the one PREM-1 consumer, the registered trusted clock owner, the
/// configured activation and the durable activation latch.
pub(crate) struct SpellPremiumCoordinator {
    consumer: Arc<PremiumConsumer>,
    clock: Option<Arc<dyn RegisteredPremiumClock>>,
    delivery: Arc<PremiumDelivery>,
    latch: Option<DurabilityRoot>,
}
impl SpellPremiumCoordinator {
    pub(crate) fn new(
        consumer: Arc<PremiumConsumer>,
        clock: Option<Arc<dyn RegisteredPremiumClock>>,
        activation: Option<PremiumActivation>,
        latch: Option<DurabilityRoot>,
    ) -> Self {
        Self {
            consumer,
            clock,
            delivery: Arc::new(PremiumDelivery::new(activation)),
            latch,
        }
    }
    /// The consumer the node's Premium refresher must pull into: the same fence these reads use.
    pub(crate) fn consumer(&self) -> Arc<PremiumConsumer> {
        Arc::clone(&self.consumer)
    }
    /// The configured activation: `None` in production composition.
    #[cfg(test)]
    pub(crate) fn activation(&self) -> Option<&PremiumActivation> {
        self.delivery.activation.as_ref()
    }
    /// Whether the gate reads the durable latch.
    #[cfg(test)]
    pub(crate) const fn reads_latch(&self) -> bool {
        self.latch.is_some()
    }
    /// Whether a clock owner is registered.
    #[cfg(test)]
    pub(crate) const fn has_clock(&self) -> bool {
        self.clock.is_some()
    }
    /// The live view of `account`. Unavailable without a registered clock owner. It reads the
    /// latch from memory, so a caller settles delivery with [`Self::premium_gate`] first.
    pub(crate) fn view(&self, account: [u8; 16]) -> Arc<RegisteredSpellAccessOwner> {
        Arc::new(match &self.clock {
            Some(clock) => RegisteredSpellAccessOwner::Premium {
                account,
                consumer: Arc::clone(&self.consumer),
                clock: Arc::clone(clock),
                delivery: Arc::clone(&self.delivery),
            },
            None => RegisteredSpellAccessOwner::Unavailable {
                account: Some(account),
            },
        })
    }
    /// The Premium gate (PREMIUM-ACTIVATION-0 §1.2): decide `account`'s [`PremiumStatus`] once
    /// and run `operation` with it, once. The clock is read once.
    ///
    /// `NotActivated` is only ever given inside a transaction that took the latch's shared lock
    /// and read an empty table: `operation` gets that transaction as `tx`, applies its bypass in
    /// it and the gate commits after it returns. Every other status gets `tx = None`. A failed
    /// lock, read or latch gives `NotCurrent`. `operation` owns what it captures.
    ///
    /// The gate commits whatever `operation` left in `tx`, whatever it returns. An operation that
    /// can reject after a write makes its writes in a savepoint (`tx.begin()`) and commits that
    /// savepoint only on success; dropping it rolls the writes back.
    ///
    /// # Errors
    /// Only when `operation` ran with `NotActivated` and its transaction did not commit: its
    /// writes are not durable and the outcome of the command is unknown.
    pub(crate) async fn with_premium_gate<T, F>(
        &self,
        account: [u8; 16],
        operation: F,
    ) -> Result<T, DurabilityError>
    where
        T: Send,
        F: PremiumGateOperation<T>,
    {
        let now = self.read_now();
        let bypass = GateBypass {
            operation,
            _output: PhantomData,
        };
        match self.settle(now, bypass).await? {
            Settled::Applied(applied) => Ok(applied),
            Settled::Decided { bypass, delivered } => {
                let status = if delivered {
                    self.delivered_status(account, now)
                } else {
                    PremiumStatus::NotCurrent
                };
                Ok(run_delivered(bypass.operation, status).await)
            }
        }
    }
    /// Promotion-specific D1106 composition. The ordinary gate above is
    /// unchanged: only this fee-bearing bypass opens Type2 before the latch.
    pub(crate) async fn with_promotion_gate<T, E, F>(
        &self,
        account: [u8; 16],
        operation: F,
    ) -> Result<Result<T, E>, DurabilityError>
    where
        T: Send,
        E: Send,
        F: PromotionPremiumGateOperation<T, E>,
    {
        let now = self.read_now();
        if self.delivery.held.get().is_some() {
            return Ok(operation
                .run(self.delivered_status(account, now), None)
                .await);
        }
        let empty = match (&self.delivery.activation, now) {
            (None, _) => EmptyLatch::NotActivated,
            (Some(_), None) => EmptyLatch::NotCurrent,
            (Some(activation), Some(now)) if now.upper_us() < activation.switch_over_us => {
                EmptyLatch::NotActivated
            }
            (Some(_), Some(now)) => EmptyLatch::Latch { now },
        };
        let Some(root) = &self.latch else {
            return Ok(operation.run(PremiumStatus::NotCurrent, None).await);
        };
        if !matches!(empty, EmptyLatch::NotActivated) {
            let read = root.read_premium_activation::<T>().await;
            let delivered = self.after_read(&empty, read).await;
            let status = if delivered {
                self.delivered_status(account, now)
            } else {
                PremiumStatus::NotCurrent
            };
            return Ok(operation.run(status, None).await);
        }
        let mut slot = Some(PromotionGateBypass {
            operation,
            _output: PhantomData,
        });
        match (root.read_premium_promotion_with(&mut slot).await, slot) {
            (Ok(PremiumActivationRead::PreDelivery(applied)), _) => Ok(applied),
            (Ok(PremiumActivationRead::Latched(latch)), Some(bypass)) => {
                self.delivery.hold(latch);
                Ok(bypass
                    .operation
                    .run(self.delivered_status(account, now), None)
                    .await)
            }
            (Ok(PremiumActivationRead::Empty) | Err(_), Some(bypass)) => {
                Ok(bypass.operation.run(PremiumStatus::NotCurrent, None).await)
            }
            (Err(error), None) => Err(error),
            (Ok(_), None) => Err(DurabilityError::InvalidStoredState),
        }
    }
    /// The multi-account Premium gate (PREMIUM-ACTIVATION-0 §1.2; the GUILD daily job and HOUSE
    /// settlement): delivery is decided once, under one shared latch lock, before the operation
    /// takes any guild or house rows, and `operation` runs once with one status per account, in
    /// the order of `accounts`. `NotActivated` is decided for the deployment, so it is given to
    /// every account at once, with the gate's transaction as `tx`; otherwise each account is
    /// `Current` or `NotCurrent` on its own and `tx = None`. The clock is read once.
    ///
    /// # Errors
    /// As [`Self::with_premium_gate`].
    pub(crate) async fn with_premium_batch_gate<T, F>(
        &self,
        accounts: &[[u8; 16]],
        operation: F,
    ) -> Result<T, DurabilityError>
    where
        T: Send,
        F: PremiumBatchGateOperation<T>,
    {
        let now = self.read_now();
        let bypass = BatchBypass {
            accounts: accounts.len(),
            operation,
            _output: PhantomData,
        };
        match self.settle(now, bypass).await? {
            Settled::Applied(applied) => Ok(applied),
            Settled::Decided { bypass, delivered } => {
                let statuses = accounts
                    .iter()
                    .map(|account| {
                        if delivered {
                            self.delivered_status(*account, now)
                        } else {
                            PremiumStatus::NotCurrent
                        }
                    })
                    .collect();
                Ok((bypass.operation)(statuses, None).await)
            }
        }
    }
    fn read_now(&self) -> Option<TrustedNow> {
        self.clock
            .as_ref()
            .and_then(|clock| clock.read_current_window())
    }
    /// §1.2 for one command: apply `bypass` inside the shared-lock transaction that read an empty
    /// table (`NotActivated`), or hand it back with whether a held row decides each status
    /// (`delivered`). A failed lock, read or latch is not delivered: `NotCurrent`.
    async fn settle<B: PreDeliveryBypass>(
        &self,
        now: Option<TrustedNow>,
        bypass: B,
    ) -> Result<Settled<B>, DurabilityError> {
        if self.delivery.held.get().is_some() {
            return Ok(Settled::Decided {
                bypass,
                delivered: true,
            });
        }
        let empty = match (&self.delivery.activation, now) {
            (None, _) => EmptyLatch::NotActivated,
            (Some(_), None) => EmptyLatch::NotCurrent,
            (Some(activation), Some(now)) if now.upper_us() < activation.switch_over_us => {
                EmptyLatch::NotActivated
            }
            (Some(_), Some(now)) => EmptyLatch::Latch { now },
        };
        let Some(root) = &self.latch else {
            return Ok(Settled::Decided {
                bypass,
                delivered: false,
            });
        };
        if !matches!(empty, EmptyLatch::NotActivated) {
            let read = root.read_premium_activation::<B::Output>().await;
            let delivered = self.after_read(&empty, read).await;
            return Ok(Settled::Decided { bypass, delivered });
        }
        // The read takes the bypass only if the table is empty.
        let mut slot = Some(bypass);
        let read = root.read_premium_activation_with(&mut slot).await;
        match (read, slot) {
            (Ok(PremiumActivationRead::PreDelivery(applied)), _) => Ok(Settled::Applied(applied)),
            (Ok(PremiumActivationRead::Latched(latch)), Some(bypass)) => {
                self.delivery.hold(latch);
                Ok(Settled::Decided {
                    bypass,
                    delivered: true,
                })
            }
            (Ok(PremiumActivationRead::Empty) | Err(_), Some(bypass)) => Ok(Settled::Decided {
                bypass,
                delivered: false,
            }),
            // The bypass ran and its transaction did not commit.
            (Err(error), None) => Err(error),
            (Ok(_), None) => Err(DurabilityError::InvalidStoredState),
        }
    }
    /// The gate after a read that ran no bypass: a found row is held (step 1); an empty table is
    /// latched or stays undelivered (steps 4 and 6); a failure is undelivered.
    async fn after_read<T>(
        &self,
        empty: &EmptyLatch,
        read: Result<PremiumActivationRead<T>, DurabilityError>,
    ) -> bool {
        match (read, empty, &self.delivery.activation, &self.latch) {
            (Ok(PremiumActivationRead::Latched(latch)), _, _, _) => {
                self.delivery.hold(latch);
                true
            }
            (
                Ok(PremiumActivationRead::Empty),
                EmptyLatch::Latch { now: at },
                Some(activation),
                Some(root),
            ) => match root
                .latch_premium_activation(
                    &activation.activation_id,
                    activation.switch_over_us,
                    at.upper_us(),
                )
                .await
            {
                Ok(latch) => {
                    self.delivery.hold(latch);
                    true
                }
                Err(_) => false,
            },
            _ => false,
        }
    }
    fn delivered_status(
        &self,
        account: [u8; 16],
        now: Option<TrustedNow>,
    ) -> PremiumStatus<'static> {
        self.delivery.delivered_status(&self.consumer, account, now)
    }
    /// The account's Premium gate, for a consumer with no pre-delivery rule: open only for
    /// `Current`. Before any row is held, with no activation configured, it is closed without a
    /// read: a row of another activation and an empty table both read Free.
    ///
    /// Erased, so a caller's future need not prove `Send` for the gate's higher-ranked operation.
    pub(crate) fn premium_gate(
        &self,
        account: [u8; 16],
    ) -> Pin<Box<dyn Future<Output = PremiumGate> + Send + '_>> {
        Box::pin(async move {
            if self.delivery.held.get().is_none() && self.delivery.activation.is_none() {
                return PremiumGate::CLOSED;
            }
            self.with_premium_gate(account, |status, _| Box::pin(async move { status.gate() }))
                .await
                .unwrap_or(PremiumGate::CLOSED)
        })
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

/// Expected source facts read through the existing native actor/session and
/// pinned item owners. They are not a durable promotion or an authority token;
/// the purchase writer independently checks its current database fences.
pub(crate) struct PromotionCommandFacts {
    pub(crate) account: [u8; 16],
    pub(crate) fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
    pub(crate) npc_key: String,
    pub(crate) gold: crate::durability::item_mint::TypedDefinitionRef,
    pub(crate) platinum: crate::durability::item_mint::TypedDefinitionRef,
    pub(crate) crystal: crate::durability::item_mint::TypedDefinitionRef,
    pub(crate) backpack: crate::durability::item_transfer::ItemDefinitionFacts,
}

/// The change facts of a Character with no equipped backpack. The fee writer then reads no
/// backpack and no coin, so only the bank part can pay (BANK-FEE-0 A3): a promotion is refused
/// for insufficient funds, a blessing may be paid from the bank. The facts name no definition, so
/// a backpack equipped before the fee transaction refuses the fee; the capacity of one only
/// satisfies the writer's request shape and is never used without a backpack.
fn no_backpack() -> crate::durability::item_transfer::ItemDefinitionFacts {
    crate::durability::item_transfer::ItemDefinitionFacts {
        weight: None,
        definition: crate::durability::item_mint::TypedDefinitionRef {
            family: String::new(),
            production_key: String::new(),
            revision_ref: String::new(),
        },
        stack: crate::durability::item_transfer::ItemStackClass::NonStackable,
        container_capacity: Some(1),
        container_slot_equip_pattern: false,
    }
}

pub(crate) struct PromotionActorFacts {
    pub(crate) fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(crate) async fn promotion_actor_facts(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<PromotionActorFacts> {
        let current = self.current_premium_binding(actor, session).await?;
        Some(PromotionActorFacts {
            fence: crate::durability::character_progression::CurrentCharacterGameplayFence {
                character_id: current.fence.character_id,
                game_session_id: current.fence.game_session_id,
                connection_generation: current.fence.connection_generation,
                character_lease_generation: current.fence.character_lease_generation,
                runtime_scope: current.fence.runtime_scope,
                scope_ownership_generation: current.fence.scope_ownership_generation,
                expected_character_revision: crate::domain::CharacterRevision::new(
                    current.character_revision,
                )
                .ok()?,
            },
        })
    }

    /// The gameplay fence of `session` for answering an already-sent Premium purchase command
    /// on a resumed connection: an Active session, its committed Character in this world and
    /// this runtime's scope ownership. It needs no actor, spell state, control or Premium facts,
    /// so a purchase whose answer was lost stays answerable while the actor re-enters. The
    /// receipt read re-checks this fence in its own transaction; `None` holds the command.
    pub(crate) async fn premium_reconcile_fence(
        &self,
        session: GameSessionId,
    ) -> Option<crate::durability::character_progression::CurrentCharacterGameplayFence> {
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
        let fence = crate::durability::character_progression::CurrentCharacterGameplayFence {
            character_id: character,
            game_session_id: session,
            connection_generation: current.current_connection_generation(),
            character_lease_generation: current.current_character_lease().generation(),
            runtime_scope: current.current_runtime_scope(),
            scope_ownership_generation: current.current_scope_generation(),
            expected_character_revision: record.revision,
        };
        let runtime = self.runtime.lock().await;
        (fence.runtime_scope
            == RuntimeScopeRefV1::channel(
                runtime.binding().world_id(),
                runtime.binding().channel_id(),
            )
            && fence.scope_ownership_generation == runtime.binding().scope_generation())
        .then_some(fence)
    }

    pub(crate) async fn promotion_command_facts(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        npc_ref: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    ) -> Option<PromotionCommandFacts> {
        use crate::domain::currency::{COIN_DEFINITION_FAMILY, Coin};
        use crate::durability::item_mint::TypedDefinitionRef;
        use oteryn_protocol_oteryn::npc_service::TALK_RANGE_TILES;
        let binding = self.current_premium_binding(actor, session).await?;
        let backpack = self
            .root
            .read_character_backpack(self.character, binding.fence.character_id)
            .await
            .ok()?;
        let native = self.active_generation?.native_gameplay()?;
        let backpack = match backpack {
            Some(current) => super::item_ref_admission::generation_item_facts(
                native,
                &current.backpack.definition,
            )
            .ok()?,
            None => no_backpack(),
        };
        let runtime = self.runtime.lock().await;
        let index = super::item_ref_admission::pinned_index(&runtime, self.active_generation)?;
        let physical = runtime.player_control_facts(actor, session).ok()?;
        if physical.control_loss.is_some() {
            return None;
        }
        let visible = runtime.visible_entities();
        let speaker = visible
            .players
            .iter()
            .find(|player| player.actor == actor)?;
        let npc = visible.npcs.iter().find(|npc| {
            npc.actor.placement_identity() == npc_ref.identity
                && npc.generation == npc_ref.generation
        })?;
        if self.chat.engaged(session) != Some(npc.actor)
            || speaker.position.floor != npc.position.floor
            || speaker.position.x.abs_diff(npc.position.x) > TALK_RANGE_TILES
            || speaker.position.y.abs_diff(npc.position.y) > TALK_RANGE_TILES
            || binding.fence.runtime_scope
                != RuntimeScopeRefV1::channel(
                    runtime.binding().world_id(),
                    runtime.binding().channel_id(),
                )
            || binding.fence.scope_ownership_generation != runtime.binding().scope_generation()
        {
            return None;
        }
        let npc_key = runtime.npc_key(npc.actor)?.to_owned();
        let coin = |coin: Coin| {
            Some(TypedDefinitionRef {
                family: COIN_DEFINITION_FAMILY.to_owned(),
                production_key: coin.production_key().to_owned(),
                revision_ref: index.revision(coin.production_key())?.to_owned(),
            })
        };
        Some(PromotionCommandFacts {
            account: binding.account,
            fence: crate::durability::character_progression::CurrentCharacterGameplayFence {
                character_id: binding.fence.character_id,
                game_session_id: binding.fence.game_session_id,
                connection_generation: binding.fence.connection_generation,
                character_lease_generation: binding.fence.character_lease_generation,
                runtime_scope: binding.fence.runtime_scope,
                scope_ownership_generation: binding.fence.scope_ownership_generation,
                expected_character_revision: crate::domain::CharacterRevision::new(
                    binding.character_revision,
                )
                .ok()?,
            },
            npc_key,
            gold: coin(Coin::Gold)?,
            platinum: coin(Coin::Platinum)?,
            crystal: coin(Coin::Crystal)?,
            backpack,
        })
    }

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
        // Settle delivery (§1.2 steps 2-6) before the view reads the latch from memory.
        let _ = coordinator.premium_gate(current.account).await;
        coordinator.view(current.account)
    }
    /// PREM-WIRE-1: the Premium gate of `account` (PREMIUM-ACTIVATION-0 §1.2) for MAGIC-CAST-1
    /// §4.6, GUILD-1 and HOUSE-1a: see [`SpellPremiumCoordinator::with_premium_gate`]. A node
    /// without the Premium reads gives `NotCurrent`.
    ///
    /// # Errors
    /// Only when `operation` ran with `NotActivated` and its transaction did not commit.
    pub(crate) async fn with_premium_gate<T, F>(
        &self,
        account: [u8; 16],
        operation: F,
    ) -> Result<T, DurabilityError>
    where
        T: Send,
        F: PremiumGateOperation<T>,
    {
        match self.premium_coordinator {
            Some(coordinator) => coordinator.with_premium_gate(account, operation).await,
            None => Ok(run_delivered(operation, PremiumStatus::NotCurrent).await),
        }
    }
    pub(crate) async fn with_promotion_gate<T, E, F>(
        &self,
        account: [u8; 16],
        operation: F,
    ) -> Result<Result<T, E>, DurabilityError>
    where
        T: Send,
        E: Send,
        F: PromotionPremiumGateOperation<T, E>,
    {
        match self.premium_coordinator {
            Some(coordinator) => coordinator.with_promotion_gate(account, operation).await,
            None => Ok(operation.run(PremiumStatus::NotCurrent, None).await),
        }
    }
    /// The multi-account Premium gate: see [`SpellPremiumCoordinator::with_premium_batch_gate`].
    /// Every account is `NotCurrent` on a node without the Premium reads.
    ///
    /// # Errors
    /// Only when `operation` ran with `NotActivated` and its transaction did not commit.
    #[allow(
        dead_code,
        reason = "the gate of the GUILD-1 daily job and HOUSE-1a settlement"
    )]
    pub(crate) async fn with_premium_batch_gate<T, F>(
        &self,
        accounts: &[[u8; 16]],
        operation: F,
    ) -> Result<T, DurabilityError>
    where
        T: Send,
        F: PremiumBatchGateOperation<T>,
    {
        match self.premium_coordinator {
            Some(coordinator) => {
                coordinator
                    .with_premium_batch_gate(accounts, operation)
                    .await
            }
            None => {
                let statuses = accounts.iter().map(|_| PremiumStatus::NotCurrent).collect();
                Ok(operation(statuses, None).await)
            }
        }
    }
    /// The Premium gate of `account` for a consumer with no pre-delivery rule (Premium areas,
    /// purchases, chat): open only for `Current`. Closed on a node without the Premium reads.
    pub(crate) async fn premium_gate(&self, account: [u8; 16]) -> PremiumGate {
        match self.premium_coordinator {
            Some(coordinator) => coordinator.premium_gate(account).await,
            None => PremiumGate::CLOSED,
        }
    }
    /// The account admitted with `session` on this node, from its Premium registration: the
    /// account a session's Premium gate reads.
    pub(crate) fn session_account(&self, session: GameSessionId) -> Option<[u8; 16]> {
        self.premium_sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .account_of(session)
    }
    /// The Premium gate of the account admitted with `session`: closed for a session holding no
    /// Premium registration.
    pub(crate) async fn session_premium_gate(&self, session: GameSessionId) -> PremiumGate {
        match self.session_account(session) {
            Some(account) => self.premium_gate(account).await,
            None => PremiumGate::CLOSED,
        }
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
    fn block_on<F: std::future::Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(future)
    }
    fn activation(switch_over_us: i64) -> PremiumActivation {
        PremiumActivation {
            activation_id: "premium-activation-1".to_owned(),
            switch_over_us,
        }
    }
    fn latch(activation_id: &str, switch_over_us: i64) -> PremiumActivationLatch {
        PremiumActivationLatch {
            activation_id: activation_id.to_owned(),
            switch_over_us,
            latched_at_us: switch_over_us,
        }
    }
    /// A coordinator on a deployment where Premium was delivered at `T0 - MINUTE`.
    fn delivered(
        consumer: Arc<PremiumConsumer>,
        clock: Option<Arc<dyn RegisteredPremiumClock>>,
    ) -> SpellPremiumCoordinator {
        let coordinator =
            SpellPremiumCoordinator::new(consumer, clock, Some(activation(T0 - MINUTE)), None);
        coordinator
            .delivery
            .hold(latch("premium-activation-1", T0 - MINUTE));
        coordinator
    }
    fn gate(coordinator: &SpellPremiumCoordinator, account: [u8; 16]) -> PremiumGate {
        block_on(coordinator.premium_gate(account))
    }
    fn status_name(coordinator: &SpellPremiumCoordinator) -> &'static str {
        block_on(coordinator.with_premium_gate(ACCOUNT, |status, tx| {
            Box::pin(async move {
                match (status, tx.is_some()) {
                    (PremiumStatus::NotActivated(_), _) => "not-activated",
                    (PremiumStatus::Current, false) => "current",
                    (PremiumStatus::NotCurrent, false) => "not-current",
                    _ => "transaction",
                }
            })
        }))
        .unwrap()
    }

    #[test]
    fn a_proven_current_account_opens_the_gate_and_the_spell_projection() {
        let clock = clock_at(T0 + MINUTE);
        let coordinator = delivered(proven(EntitlementState::Active, FAR), Some(clock));
        assert_eq!(gate(&coordinator, ACCOUNT).require(), Ok(()));
        assert_eq!(status_name(&coordinator), "current");
        let view = coordinator.view(ACCOUNT);
        assert_eq!(view.premium_gate(), gate(&coordinator, ACCOUNT));
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
            let coordinator = delivered(proven(state, until), Some(clock_at(now)));
            assert_eq!(
                gate(&coordinator, ACCOUNT).require(),
                Err(PremiumRefusal::NotCurrent)
            );
            let access = coordinator
                .view(ACCOUNT)
                .read_current(&binding(), 1)
                .unwrap();
            assert!(access.premium.is_none());
        }
        // An unknown account, and the same proven account on another account's gate.
        let coordinator = delivered(
            proven(EntitlementState::Active, FAR),
            Some(clock_at(T0 + MINUTE)),
        );
        assert_eq!(gate(&coordinator, [2; 16]), PremiumGate::CLOSED);
        // A failed pull and a released account close it at once, for views already issued.
        let view = coordinator.view(ACCOUNT);
        let ticket = coordinator.consumer().ticket();
        coordinator.consumer().pull_failed(ACCOUNT, ticket);
        assert_eq!(view.premium_gate(), PremiumGate::CLOSED);
        assert!(view.read_current(&binding(), 1).unwrap().premium.is_none());
        let coordinator = delivered(
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
        let unclocked = delivered(proven(EntitlementState::Active, FAR), None);
        assert_eq!(gate(&unclocked, ACCOUNT), PremiumGate::CLOSED);
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
        let coordinator = delivered(proven(EntitlementState::Active, FAR), Some(clock.clone()));
        let view = coordinator.view(ACCOUNT);
        assert!(view.premium_gate().premium_current());
        *clock.0.lock().unwrap() = None;
        assert_eq!(view.premium_gate(), PremiumGate::CLOSED);
        assert!(view.read_current(&binding(), 1).unwrap().premium.is_none());
        // A restart: a fresh consumer holds nothing proven.
        let restarted = delivered(Arc::default(), Some(clock_at(T0 + MINUTE)));
        assert_eq!(gate(&restarted, ACCOUNT), PremiumGate::CLOSED);
    }

    #[test]
    fn the_status_is_current_only_after_delivery_of_the_configured_activation() {
        let consumer = || proven(EntitlementState::Active, FAR);
        let clock = || -> Option<Arc<dyn RegisteredPremiumClock>> { Some(clock_at(T0 + MINUTE)) };
        // No activation configured: closed without a read, and no transaction without the latch.
        let free = SpellPremiumCoordinator::new(consumer(), clock(), None, None);
        assert_eq!(gate(&free, ACCOUNT), PremiumGate::CLOSED);
        assert_eq!(status_name(&free), "not-current");
        // An activation but no latch read: Premium is not current.
        let unread = SpellPremiumCoordinator::new(consumer(), clock(), Some(activation(T0)), None);
        assert_eq!(status_name(&unread), "not-current");
        // A held row of another activation, or one whose window is not wholly after `S`.
        let other = SpellPremiumCoordinator::new(consumer(), clock(), Some(activation(T0)), None);
        other.delivery.hold(latch("premium-activation-0", T0));
        assert_eq!(status_name(&other), "not-current");
        let early =
            SpellPremiumCoordinator::new(consumer(), clock(), Some(activation(T0 + MINUTE)), None);
        early
            .delivery
            .hold(latch("premium-activation-1", T0 + MINUTE));
        assert_eq!(status_name(&early), "not-current");
        // The held row of the configured activation, with the window after `S`.
        let current = delivered(consumer(), clock());
        assert_eq!(status_name(&current), "current");
        // A held row is never undone by a later, different latch.
        current.delivery.hold(latch("premium-activation-0", T0));
        assert_eq!(status_name(&current), "current");
    }

    fn batch_names(
        coordinator: &SpellPremiumCoordinator,
        accounts: &[[u8; 16]],
    ) -> Vec<&'static str> {
        block_on(
            coordinator.with_premium_batch_gate(accounts, |statuses, tx| {
                Box::pin(async move {
                    let in_tx = tx.is_some();
                    statuses
                        .into_iter()
                        .map(|status| match (status, in_tx) {
                            (PremiumStatus::NotActivated(_), _) => "not-activated",
                            (PremiumStatus::Current, false) => "current",
                            (PremiumStatus::NotCurrent, false) => "not-current",
                            _ => "transaction",
                        })
                        .collect()
                })
            }),
        )
        .unwrap()
    }

    #[test]
    fn the_batch_gate_classifies_every_account_in_order_under_one_decision() {
        let clock = || -> Option<Arc<dyn RegisteredPremiumClock>> { Some(clock_at(T0 + MINUTE)) };
        let current = delivered(proven(EntitlementState::Active, FAR), clock());
        // N accounts, one of them ineligible, in the order given; none runs in a transaction.
        assert_eq!(
            batch_names(&current, &[ACCOUNT, [2; 16], ACCOUNT, [3; 16]]),
            ["current", "not-current", "current", "not-current"]
        );
        assert_eq!(batch_names(&current, &[]), Vec::<&str>::new());
        // An ineligible proven account: every account reads not current.
        let expired = delivered(proven(EntitlementState::Expired, FAR), clock());
        assert_eq!(
            batch_names(&expired, &[ACCOUNT, [2; 16]]),
            ["not-current", "not-current"]
        );
        // Not delivered, without the latch: every account is not current.
        let unread = SpellPremiumCoordinator::new(
            proven(EntitlementState::Active, FAR),
            clock(),
            Some(activation(T0)),
            None,
        );
        assert_eq!(
            batch_names(&unread, &[ACCOUNT, [2; 16]]),
            ["not-current", "not-current"]
        );
    }
}
