//! PREM-5 NPC policy and D1106 native promotion and blessing purchase consumers.
//! `npc_command` dispatches the talk topics here (command 7): `promotion` or a blessing name
//! opens one offer prompt, and the next `yes` of the same conversation buys it through the
//! native owner. The offers come only from the installed [`PremiumNpcServiceCatalogue`]: the
//! node installs the source-backed one (`premium_blessing_catalogue`) over its pinned NPC
//! catalogue. An NPC that declares no admitted promotion or blessing Service answers no offer, so
//! the topic fails closed: no prompt, no purchase, nothing written.
//!
//! A quote or successful Premium check is data, not a payment/authority permit. The future
//! owner must recheck current Character facts and Premium inside the actual command boundary,
//! and commit the coins-only GOLD-FEE-1b burn and promotion/provenance together, idempotently.

use crate::domain::premium::{PremiumGate, PremiumRefusal, PremiumStatus};
use crate::gameplay_transport::premium_promotion::{
    PROMOTION_PRICE, PromotionRefusal, check_promotion_eligibility,
};

/// ACCEPT_BATCH §3 and PREMIUM_ACTIVATION §4.2: one-time promotion, paid in coins.
pub(crate) const PROMOTION_FEE_GOLD_UNITS: u64 = PROMOTION_PRICE;

/// Price policy only; funds, coin selection/change, burn ledger and atomicity are GOLD-FEE-1b
/// responsibilities. No bank fallback is selected or authorized by this coins-only quote.
pub(crate) fn promotion_coin_fee_quote(
    level: u32,
    durable_promoted: bool,
    premium: &PremiumStatus<'_>,
) -> Result<u64, PromotionRefusal> {
    check_promotion_eligibility(level, durable_promoted, premium)?;
    Ok(PROMOTION_FEE_GOLD_UNITS)
}

/// The three Premium blessing NPC purchases of PREMIUM_ACTIVATION §4.4. No content keys,
/// persistence bits or unsourced blessing prices are assigned here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PremiumBlessingService {
    WisdomOfSolitude,
    BloodOfTheMountain,
    HeartOfTheMountain,
}

/// Premium gates purchase only. Held blessings are not inspected, removed or consumed by this
/// policy; lapse does not take them away (§4.4). This does not grant a blessing or payment.
pub(crate) const fn check_premium_blessing_purchase(
    _service: PremiumBlessingService,
    premium: PremiumGate,
) -> Result<(), PremiumRefusal> {
    premium.require()
}

/// D1106 source catalogue port. Its implementation belongs to the qualified,
/// pinned NPC/blessing source owner. The client supplies neither a fee nor a
/// blessing key. An unpopulated/unknown source returns no offer and grants nothing.
pub(crate) trait PremiumNpcServiceCatalogue: Send + Sync {
    fn content_revision(&self) -> &str;
    fn npcs(&self) -> &crate::content::NpcDataCatalogue;
    fn promotion(&self, native_npc_key: &str) -> Option<PromotionNpcOffer>;
    fn blessing(
        &self,
        native_npc_key: &str,
        service: PremiumBlessingService,
    ) -> Option<BlessingNpcOffer>;
}

#[derive(Debug, Clone)]
pub(crate) struct PromotionNpcOffer {
    pub(crate) npc: crate::content::ProjectV2DefinitionRef,
    pub(crate) service: crate::content::ProjectV2DefinitionRef,
}

/// Values are a source-owner projection of its admitted price table, not
/// command arguments. The durable writer chooses the band at the locked level.
#[derive(Debug, Clone)]
pub(crate) struct BlessingNpcOffer {
    pub(crate) source: PromotionNpcOffer,
    pub(crate) blessing: PremiumBlessingService,
    pub(crate) storage_key: String,
    /// The same bands for a promoted buyer: a promotion has no price effect (owner ruling
    /// 2026-10-18).
    pub(crate) prices: Vec<BlessingPriceBand>,
}

/// One source price band: `gold_units + gold_per_level × (level − base_level)` for every level
/// from `minimum_level` to `maximum_level`. A flat band has `gold_per_level` 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BlessingPriceBand {
    pub(crate) minimum_level: u32,
    pub(crate) maximum_level: u32,
    pub(crate) gold_units: u64,
    pub(crate) gold_per_level: u64,
    pub(crate) base_level: u32,
}

pub(crate) fn admitted_promotion_offer(
    catalogue: &dyn PremiumNpcServiceCatalogue,
    native_npc_key: &str,
) -> Option<PromotionNpcOffer> {
    use crate::content::{ProjectV2Declaration, ProjectV2Family};
    let offer = catalogue.promotion(native_npc_key)?;
    if offer.npc.key != native_npc_key
        || offer.npc.family != ProjectV2Family::Npc
        || offer.service.family != ProjectV2Family::Service
    {
        return None;
    }
    let ProjectV2Declaration::Npc { services, .. } = catalogue.npcs().declaration(&offer.npc)?
    else {
        return None;
    };
    if !services.contains(&offer.service)
        || !matches!(
            catalogue.npcs().declaration(&offer.service),
            Some(ProjectV2Declaration::Service { .. })
        )
    {
        return None;
    }
    Some(offer)
}

/// Distinct blessing conversation/source qualification. A promotion service
/// reference can never stand in for the requested blessing service.
pub(crate) fn admitted_blessing_offer(
    catalogue: &dyn PremiumNpcServiceCatalogue,
    native_npc_key: &str,
    requested: PremiumBlessingService,
) -> Option<BlessingNpcOffer> {
    use crate::content::{ProjectV2Declaration, ProjectV2Family};
    let offer = catalogue.blessing(native_npc_key, requested)?;
    if offer.blessing != requested
        || offer.source.npc.key != native_npc_key
        || offer.source.npc.family != ProjectV2Family::Npc
        || offer.source.service.family != ProjectV2Family::Service
    {
        return None;
    }
    let ProjectV2Declaration::Npc { services, .. } =
        catalogue.npcs().declaration(&offer.source.npc)?
    else {
        return None;
    };
    if !services.contains(&offer.source.service)
        || !matches!(
            catalogue.npcs().declaration(&offer.source.service),
            Some(ProjectV2Declaration::Service { .. })
        )
    {
        return None;
    }
    Some(offer)
}

/// Same CommandRef and target keep the same source occurrence and item/audit
/// identities on every retry. Different NPC generations cannot alias it.
fn promotion_command_id(
    tag: &[u8],
    command: crate::foundation::CommandRef,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
) -> [u8; 16] {
    use sha2::{Digest, Sha256};
    let hash = Sha256::new()
        .chain_update(tag)
        .chain_update(command.game_session_id().as_bytes())
        .chain_update(command.command_id().get().to_be_bytes())
        .chain_update(npc.identity)
        .chain_update(npc.generation.to_be_bytes())
        .finalize();
    let mut value = [0; 16];
    value.copy_from_slice(&hash[..16]);
    value[6] = (value[6] & 0x0f) | 0x70;
    value[8] = (value[8] & 0x3f) | 0x80;
    value
}

use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence,
};
use crate::durability::item_fee_burn::{FeeBurnCause, FeeBurnRequest, FeeChangeFacts};
use crate::durability::item_mint_audit::Type2Transaction;
use crate::durability::npc_purchase::NpcPurchaseOwner;
use crate::durability::premium_activation_latch::{PremiumGateTx, PromotionPremiumGateOperation};
use crate::foundation::{CommandRef, ExactActorRef};
use crate::gameplay_transport::premium_blessing_purchase::{
    BlessingPurchaseError, BlessingPurchaseOperation, BlessingPurchaseOutcome,
    BlessingPurchaseRequest, BlessingReplayRead, PendingBlessingPurchase,
};
use crate::gameplay_transport::premium_promotion::{
    NpcPurchaseSource, PendingPromotionPurchase, PromotionPurchaseError,
    PromotionPurchaseOperation, PromotionPurchaseOutcome, PromotionPurchaseRequest,
    PromotionReplayRead, prepare_promotion_purchase,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::future::Future;
use std::pin::Pin;

/// The one promotion operation of a command, run once by the native promotion gate.
struct PromotionGateOp {
    owner: NpcPurchaseOwner,
    fence: CurrentCharacterGameplayFence,
    request: PromotionPurchaseRequest,
}

impl PromotionPremiumGateOperation<PendingPromotionPurchase, PromotionPurchaseError>
    for PromotionGateOp
{
    fn run<'g, 't>(
        self,
        status: PremiumStatus<'g>,
        tx: Option<&'g mut Type2Transaction<'t>>,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<PendingPromotionPurchase, PromotionPurchaseError>>
                + Send
                + 'g,
        >,
    >
    where
        Self: 'g,
    {
        Box::pin(async move {
            match tx {
                // WHEEL-0 §6.2: before delivery the purchase joins the gate's own Type2
                // transaction, which holds the shared latch lock; the gate commits it on success
                // and rolls it back on a typed refusal.
                Some(tx) if matches!(status, PremiumStatus::NotActivated(_)) => {
                    let write = self.owner.bind(tx).await?;
                    prepare_promotion_purchase(&write, tx, self.fence, &self.request, &status).await
                }
                Some(_) => Err(PromotionPurchaseError::InvalidInput),
                // Delivered (or unknown): the native owner opens and commits one Type2
                // transaction; the writer checks the status it is given.
                None => {
                    self.owner
                        .commit(PromotionPurchaseOperation {
                            fence: self.fence,
                            request: &self.request,
                            premium: &status,
                        })
                        .await
                }
            }
        })
    }
}

/// The one blessing operation of a command, run once by the ordinary Premium gate. Blessings
/// have no pre-delivery exception: they need `Current`.
struct BlessingGateOp {
    owner: NpcPurchaseOwner,
    fence: CurrentCharacterGameplayFence,
    request: BlessingPurchaseRequest,
}

impl BlessingGateOp {
    fn run<'g, 't>(
        self,
        status: PremiumStatus<'g>,
        tx: Option<&'g mut PremiumGateTx<'t>>,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<PendingBlessingPurchase, BlessingPurchaseError>> + Send + 'g,
        >,
    > {
        Box::pin(async move {
            // The ordinary gate lends a transaction only before delivery; a blessing never
            // writes in it.
            if tx.is_some() || matches!(status, PremiumStatus::NotActivated(_)) {
                return Err(BlessingPurchaseError::Premium(PremiumRefusal::NotCurrent));
            }
            self.owner
                .commit(BlessingPurchaseOperation {
                    fence: self.fence,
                    request: &self.request,
                    premium: &status,
                })
                .await
        })
    }
}

fn occurred_at_unix_ms() -> Option<i64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|time| i64::try_from(time.as_millis()).ok())
        .filter(|time| *time > 0)
}

impl crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_> {
    /// One NPC promotion purchase (PREMIUM_ACTIVATION §4.2, D1106). The revision slot spans the
    /// replay read, the current actor/NPC checks, the one gate operation and its final COMMIT.
    /// The client supplies neither the fee nor the service: both come from `catalogue`.
    pub(crate) async fn buy_npc_promotion(
        &self,
        actor: ExactActorRef,
        command: CommandRef,
        npc: EntityRef,
        catalogue: &dyn PremiumNpcServiceCatalogue,
    ) -> Result<PromotionPurchaseOutcome, PromotionPurchaseError> {
        let rejected =
            PromotionPurchaseError::Current(CharacterProgressionError::AuthorityRejected);
        let actor_facts = self
            .promotion_actor_facts(actor, command.game_session_id())
            .await
            .ok_or(rejected)?;
        let character = actor_facts.fence.character_id;
        let occurrence = promotion_command_id(b"oteryn:promotion-occurrence:v1", command, npc);
        let mut slot = self.revision_sequencer.acquire(character).await;
        slot.commit_npc_purchase(
            self.root,
            self.character,
            self.holder,
            actor_facts.fence,
            |owner, fence| async move {
                // The NPC may have gone away since the first COMMIT. Its historical answer needs
                // no live conversation, price table or Premium approval.
                if let Some(outcome) = owner
                    .commit(PromotionReplayRead {
                        character,
                        occurrence,
                        npc,
                    })
                    .await?
                {
                    return Ok(outcome);
                }
                // Re-read the physical actor and NPC under the slot; no earlier snapshot
                // stands in for current authority.
                let facts = self
                    .promotion_command_facts(actor, command.game_session_id(), npc)
                    .await
                    .ok_or(PromotionPurchaseError::Current(
                        CharacterProgressionError::AuthorityRejected,
                    ))?;
                if facts.fence.character_id != character {
                    return Err(PromotionPurchaseError::InvalidInput);
                }
                let offer = admitted_promotion_offer(catalogue, &facts.npc_key)
                    .ok_or(PromotionPurchaseError::InvalidInput)?;
                let id = |tag: &[u8]| promotion_command_id(tag, command, npc);
                let request = PromotionPurchaseRequest {
                    occurrence,
                    account_id: facts.account,
                    source: NpcPurchaseSource {
                        npc: offer.npc,
                        service: offer.service,
                        actor: npc,
                        content_revision: catalogue.content_revision().to_owned(),
                    },
                    fee: FeeBurnRequest {
                        cause: FeeBurnCause::PromotionPurchase { occurrence },
                        fee_gold_units: PROMOTION_PRICE,
                        transaction_id: id(b"oteryn:promotion-fee-transaction:v1"),
                        event_id: id(b"oteryn:promotion-fee-event:v1"),
                        occurred_at_unix_ms: occurred_at_unix_ms()
                            .ok_or(PromotionPurchaseError::InvalidInput)?,
                        server_build_id: crate::durability::character_authority::SERVER_BUILD_ID
                            .to_owned(),
                        change: FeeChangeFacts {
                            item_instance_ids: [
                                id(b"oteryn:promotion-change-platinum:v1"),
                                id(b"oteryn:promotion-change-gold:v1"),
                            ],
                            platinum: facts.platinum,
                            gold: facts.gold,
                            crystal: facts.crystal,
                            backpack: facts.backpack,
                        },
                    },
                };
                let operation = PromotionGateOp {
                    owner,
                    fence: CurrentCharacterGameplayFence {
                        expected_character_revision: fence.expected_character_revision,
                        ..facts.fence
                    },
                    request,
                };
                match self.with_promotion_gate(facts.account, operation).await {
                    Ok(Ok(pending)) => Ok(pending.after_native_commit()),
                    Ok(Err(error)) => Err(error),
                    Err(error) => Err(PromotionPurchaseError::Unavailable(error)),
                }
            },
        )
        .await
    }

    /// One Premium blessing purchase (PREMIUM_ACTIVATION §4.4, DEATH_COMPLETE0 §4.7). The
    /// client input names only the service; the blessing key and price band come from
    /// `catalogue`, and the band is chosen by the writer at the locked level.
    pub(crate) async fn buy_npc_blessing(
        &self,
        actor: ExactActorRef,
        command: CommandRef,
        npc: EntityRef,
        service: PremiumBlessingService,
        catalogue: &dyn PremiumNpcServiceCatalogue,
    ) -> Result<BlessingPurchaseOutcome, BlessingPurchaseError> {
        let rejected = BlessingPurchaseError::Current(CharacterProgressionError::AuthorityRejected);
        let actor_facts = self
            .promotion_actor_facts(actor, command.game_session_id())
            .await
            .ok_or(rejected)?;
        let character = actor_facts.fence.character_id;
        let occurrence = promotion_command_id(b"oteryn:blessing-occurrence:v1", command, npc);
        let mut slot = self.revision_sequencer.acquire(character).await;
        slot.commit_npc_purchase(
            self.root,
            self.character,
            self.holder,
            actor_facts.fence,
            |owner, fence| async move {
                if let Some(outcome) = owner
                    .commit(BlessingReplayRead {
                        character,
                        occurrence,
                        npc,
                        service,
                    })
                    .await?
                {
                    return Ok(outcome);
                }
                let facts = self
                    .promotion_command_facts(actor, command.game_session_id(), npc)
                    .await
                    .ok_or(BlessingPurchaseError::Current(
                        CharacterProgressionError::AuthorityRejected,
                    ))?;
                if facts.fence.character_id != character {
                    return Err(BlessingPurchaseError::InvalidInput);
                }
                let offer = admitted_blessing_offer(catalogue, &facts.npc_key, service)
                    .ok_or(BlessingPurchaseError::InvalidInput)?;
                let id = |tag: &[u8]| promotion_command_id(tag, command, npc);
                let request = BlessingPurchaseRequest {
                    occurrence,
                    account_id: facts.account,
                    source: NpcPurchaseSource {
                        npc: offer.source.npc.clone(),
                        service: offer.source.service.clone(),
                        actor: npc,
                        content_revision: catalogue.content_revision().to_owned(),
                    },
                    fee: FeeBurnRequest {
                        cause: FeeBurnCause::BlessingPurchase {
                            occurrence,
                            blessing: offer.storage_key.clone(),
                        },
                        // Not authoritative: the writer replaces it with the band selected at
                        // its locked Character level.
                        fee_gold_units: 0,
                        transaction_id: id(b"oteryn:blessing-fee-transaction:v1"),
                        event_id: id(b"oteryn:blessing-fee-event:v1"),
                        occurred_at_unix_ms: occurred_at_unix_ms()
                            .ok_or(BlessingPurchaseError::InvalidInput)?,
                        server_build_id: crate::durability::character_authority::SERVER_BUILD_ID
                            .to_owned(),
                        change: FeeChangeFacts {
                            item_instance_ids: [
                                id(b"oteryn:blessing-change-platinum:v1"),
                                id(b"oteryn:blessing-change-gold:v1"),
                            ],
                            platinum: facts.platinum,
                            gold: facts.gold,
                            crystal: facts.crystal,
                            backpack: facts.backpack,
                        },
                    },
                    offer,
                };
                let operation = BlessingGateOp {
                    owner,
                    fence: CurrentCharacterGameplayFence {
                        expected_character_revision: fence.expected_character_revision,
                        ..facts.fence
                    },
                    request,
                };
                // Exactly one ordinary Premium gate for this command; the status stays inside it
                // through the source transaction and its COMMIT.
                match self
                    .with_premium_gate(facts.account, move |status, tx| operation.run(status, tx))
                    .await
                {
                    Ok(Ok(pending)) => Ok(pending.after_native_commit()),
                    Ok(Err(error)) => Err(error),
                    Err(error) => Err(BlessingPurchaseError::Unavailable(error)),
                }
            },
        )
        .await
    }
}

/// The receipt keys of every Premium purchase of one Character: its occurrence, the NPC
/// generation it was bought from and whether it is a blessing. Read, never written.
struct PremiumReceiptKey {
    occurrence: [u8; 16],
    npc: EntityRef,
    blessing: bool,
}

struct PremiumReceiptKeys(Vec<PremiumReceiptKey>);

impl crate::durability::character_revision_sequencer::SequencedOutcome for PremiumReceiptKeys {
    fn committed_revision(&self) -> Option<crate::domain::CharacterRevision> {
        None
    }
}

struct PremiumReceiptKeysRead {
    character: crate::domain::CharacterId,
}

impl crate::durability::npc_purchase::NpcPurchaseOperation for PremiumReceiptKeysRead {
    type Output = PremiumReceiptKeys;
    type Error = BlessingPurchaseError;

    fn apply<'a, 't>(
        self,
        _context: &'a crate::durability::npc_purchase::NpcPurchaseWriteContext,
        tx: &'a mut Type2Transaction<'t>,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send + 'a>>
    where
        Self: 'a,
    {
        Box::pin(async move {
            let rows: Vec<(Vec<u8>, Vec<u8>, String, bool)> = sqlx::query_as(
                "SELECT decode(replace(purchase_occurrence_id::text,'-',''),'hex'), \
                        decode(replace(npc_actor_id::text,'-',''),'hex'), \
                        npc_actor_generation::text, false \
                   FROM game_character_promotion_receipts \
                  WHERE character_id = encode($1,'hex')::uuid \
                 UNION ALL \
                 SELECT decode(replace(purchase_occurrence_id::text,'-',''),'hex'), \
                        decode(replace(npc_actor_id::text,'-',''),'hex'), \
                        npc_actor_generation::text, true \
                   FROM game_character_blessing_purchase_receipts \
                  WHERE character_id = encode($1,'hex')::uuid",
            )
            .bind(self.character.as_bytes().as_slice())
            .fetch_all(&mut ***tx)
            .await?;
            rows.into_iter()
                .map(|(occurrence, identity, generation, blessing)| {
                    Some(PremiumReceiptKey {
                        occurrence: occurrence.try_into().ok()?,
                        npc: EntityRef {
                            identity: identity.try_into().ok()?,
                            generation: generation.parse().ok()?,
                        },
                        blessing,
                    })
                })
                .collect::<Option<Vec<_>>>()
                .map(PremiumReceiptKeys)
                .ok_or(BlessingPurchaseError::from(
                    crate::durability::DurabilityError::InvalidStoredState,
                ))
        })
    }
}

/// The held blessing set of one Character, in the byte order the death writer compares.
struct HeldBlessings(Vec<String>);

impl crate::durability::character_revision_sequencer::SequencedOutcome for HeldBlessings {
    fn committed_revision(&self) -> Option<crate::domain::CharacterRevision> {
        None
    }
}

struct HeldBlessingsRead {
    character: crate::domain::CharacterId,
}

impl crate::durability::npc_purchase::NpcPurchaseOperation for HeldBlessingsRead {
    type Output = HeldBlessings;
    type Error = BlessingPurchaseError;

    fn apply<'a, 't>(
        self,
        _context: &'a crate::durability::npc_purchase::NpcPurchaseWriteContext,
        tx: &'a mut Type2Transaction<'t>,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send + 'a>>
    where
        Self: 'a,
    {
        Box::pin(async move {
            let held: Vec<String> = sqlx::query_scalar(
                "SELECT blessing_key COLLATE \"C\" FROM game_character_blessings \
                  WHERE character_id = encode($1,'hex')::uuid ORDER BY 1",
            )
            .bind(self.character.as_bytes().as_slice())
            .fetch_all(&mut ***tx)
            .await?;
            Ok::<_, BlessingPurchaseError>(HeldBlessings(held))
        })
    }
}

/// The durable answer of a Premium purchase command after a lost COMMIT outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PremiumReconcile {
    /// No Premium purchase committed under this command.
    None,
    /// It committed: the NPC it was bought from and the answer the command would have had.
    Committed { npc: EntityRef, line: &'static str },
    /// Not classifiable with current authority yet: the command stays unsequenced.
    Held,
}

const PROMOTED_LINE: &str = "Congratulations! You are now promoted.";
const BLESSED_LINE: &str = "You have been blessed.";

impl crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_> {
    /// Whether a promotion or blessing purchase committed under `command`, for a resumed
    /// connection whose first command repeats an unsequenced CommandId. A purchase occurrence is
    /// keyed by the CommandRef and the NPC generation, so every receipt of the Character is
    /// re-keyed with this CommandRef: only the purchase of exactly this command matches. Nothing
    /// is written and no current NPC, offer or Premium state is needed.
    pub(crate) async fn reconcile_premium_purchase(
        &self,
        actor: ExactActorRef,
        command: CommandRef,
    ) -> PremiumReconcile {
        use crate::durability::character_revision_sequencer::SequencedError;
        let Some(facts) = self
            .promotion_actor_facts(actor, command.game_session_id())
            .await
        else {
            return PremiumReconcile::Held;
        };
        let character = facts.fence.character_id;
        let mut slot = self.revision_sequencer.acquire(character).await;
        let keys = slot
            .commit_npc_purchase(
                self.root,
                self.character,
                self.holder,
                facts.fence,
                |owner, _fence| async move { owner.commit(PremiumReceiptKeysRead { character }).await },
            )
            .await;
        drop(slot);
        let keys = match keys {
            Ok(keys) => keys,
            Err(error) if error.is_unknown() => return PremiumReconcile::Held,
            // Without current authority nothing can be bought either; dispatch refuses it.
            Err(_) => return PremiumReconcile::None,
        };
        keys.0
            .into_iter()
            .find_map(|key| {
                let (tag, line): (&[u8], _) = if key.blessing {
                    (b"oteryn:blessing-occurrence:v1", BLESSED_LINE)
                } else {
                    (b"oteryn:promotion-occurrence:v1", PROMOTED_LINE)
                };
                (promotion_command_id(tag, command, key.npc) == key.occurrence)
                    .then_some(PremiumReconcile::Committed { npc: key.npc, line })
            })
            .unwrap_or(PremiumReconcile::None)
    }

    /// DEATH_COMPLETE0 §4.7: the held blessing set a death of `occurrence` consumes. A death that
    /// already committed keeps the set it consumed, so its retry replays the same intent;
    /// otherwise the current set is read under the revision slot. `None`: unavailable now.
    pub(crate) async fn death_held_blessings(
        &self,
        fence: CurrentCharacterGameplayFence,
        occurrence: crate::durability::character_death::PlayerDeathOccurrence,
    ) -> Option<Vec<String>> {
        if let Some(committed) = self
            .root
            .reconcile_character_death(self.character, occurrence)
            .await
            .ok()?
        {
            return Some(committed.blessings_before);
        }
        let character = fence.character_id;
        let mut slot = self.revision_sequencer.acquire(character).await;
        let held = slot
            .commit_npc_purchase(
                self.root,
                self.character,
                self.holder,
                fence,
                |owner, _fence| async move { owner.commit(HeldBlessingsRead { character }).await },
            )
            .await
            .ok()?;
        Some(held.0)
    }
}

/// The talk topic of a Premium NPC service: `None` is the promotion, `Some` one of the three
/// Premium blessings (PREMIUM_ACTIVATION §4.4). Matched whole and case-insensitively.
pub(crate) fn premium_topic(text: &str) -> Option<Option<PremiumBlessingService>> {
    let text = text.trim_matches(' ');
    if text.eq_ignore_ascii_case("promotion") {
        return Some(None);
    }
    [
        (
            "wisdom of solitude",
            PremiumBlessingService::WisdomOfSolitude,
        ),
        (
            "blood of the mountain",
            PremiumBlessingService::BloodOfTheMountain,
        ),
        (
            "heart of the mountain",
            PremiumBlessingService::HeartOfTheMountain,
        ),
    ]
    .into_iter()
    .find(|(name, _)| text.eq_ignore_ascii_case(name))
    .map(|(_, service)| Some(service))
}

/// The offer prompt of `topic` at the NPC `native_npc_key`, only when `catalogue` admits that
/// NPC's service. The prompt is display text: it is not a quote the purchase relies on.
pub(crate) fn premium_prompt(
    catalogue: &dyn PremiumNpcServiceCatalogue,
    native_npc_key: &str,
    topic: Option<PremiumBlessingService>,
) -> Option<String> {
    match topic {
        None => admitted_promotion_offer(catalogue, native_npc_key).map(|_| {
            format!("Do you want to be promoted in your vocation for {PROMOTION_PRICE} gold?")
        }),
        Some(service) => admitted_blessing_offer(catalogue, native_npc_key, service).map(|_| {
            format!(
                "Do you want to receive the {} blessing?",
                blessing_name(service)
            )
        }),
    }
}

const fn blessing_name(service: PremiumBlessingService) -> &'static str {
    match service {
        PremiumBlessingService::WisdomOfSolitude => "Wisdom of Solitude",
        PremiumBlessingService::BloodOfTheMountain => "Blood of the Mountain",
        PremiumBlessingService::HeartOfTheMountain => "Heart of the Mountain",
    }
}

/// One offer the NPC prompted and the speaker has not confirmed yet. It names the NPC
/// generation and the conversation it was made in; it is not a quote or a permit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PendingPremiumService {
    pub(crate) npc: EntityRef,
    pub(crate) service: Option<PremiumBlessingService>,
    pub(crate) conversation_generation: u64,
}

/// What one Premium service talk intent does, decided under the Channel runtime lock.
pub(crate) enum PremiumTalkStep {
    Prompt(String),
    Confirm {
        pending: PendingPremiumService,
        catalogue: std::sync::Arc<dyn PremiumNpcServiceCatalogue>,
    },
}

/// The answer to a confirmed offer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PremiumTalkAnswer {
    Line {
        text: String,
        inventory_changed: bool,
    },
    /// The durable outcome is unknown: not a refusal.
    Unknown,
    Rejected,
}

fn line(text: &str, inventory_changed: bool) -> PremiumTalkAnswer {
    PremiumTalkAnswer::Line {
        text: text.to_owned(),
        inventory_changed,
    }
}

fn fee_answer(error: &crate::durability::item_fee_burn::FeeBurnError) -> PremiumTalkAnswer {
    use crate::durability::item_fee_burn::FeeBurnError;
    match error {
        FeeBurnError::InsufficientFunds => line("You do not have enough money.", false),
        FeeBurnError::CapacityExceeded | FeeBurnError::ChangeDoesNotFit => {
            line("There is not enough room in your backpack.", false)
        }
        FeeBurnError::Unavailable(_) => PremiumTalkAnswer::Unknown,
        _ => PremiumTalkAnswer::Rejected,
    }
}

/// Buys the confirmed offer as one command `command`, whose CommandRef keys the purchase
/// occurrence: a retry of the same command replays its receipt and burns nothing twice.
/// The caller holds no runtime or conversation lock.
pub(crate) async fn confirm_premium_service(
    admission: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: CommandRef,
    pending: PendingPremiumService,
    catalogue: &dyn PremiumNpcServiceCatalogue,
) -> PremiumTalkAnswer {
    use crate::durability::character_revision_sequencer::SequencedError;
    match pending.service {
        None => match admission
            .buy_npc_promotion(actor, command, pending.npc, catalogue)
            .await
        {
            Ok(_) => line(PROMOTED_LINE, true),
            Err(error) if error.is_unknown() => PremiumTalkAnswer::Unknown,
            Err(PromotionPurchaseError::Eligibility(refusal)) => match refusal {
                PromotionRefusal::Premium(_) => {
                    line("You need a premium account in order to be promoted.", false)
                }
                PromotionRefusal::LevelTooLow => line(
                    "You need to be at least level 20 in order to be promoted.",
                    false,
                ),
                PromotionRefusal::AlreadyPromoted => line("You are already promoted!", false),
            },
            Err(PromotionPurchaseError::Fee(error)) => fee_answer(&error),
            Err(_) => PremiumTalkAnswer::Rejected,
        },
        Some(service) => match admission
            .buy_npc_blessing(actor, command, pending.npc, service, catalogue)
            .await
        {
            Ok(_) => line(BLESSED_LINE, true),
            Err(error) if error.is_unknown() => PremiumTalkAnswer::Unknown,
            Err(BlessingPurchaseError::Premium(_)) => line(
                "You need a premium account in order to receive this blessing.",
                false,
            ),
            Err(BlessingPurchaseError::AlreadyHeld) => {
                line("You already possess this blessing.", false)
            }
            Err(BlessingPurchaseError::Fee(error)) => fee_answer(&error),
            Err(_) => PremiumTalkAnswer::Rejected,
        },
    }
}
