//! PREM-5 NPC policy and D1106 native promotion purchase consumer.
//! Qualified blessing catalogue data and final dispatch registration remain pending.
//!
//! A quote or successful Premium check is data, not a payment/authority permit. The future
//! owner must recheck current Character facts and Premium inside the actual command boundary,
//! and commit the coins-only GOLD-FEE-1b burn and promotion/provenance together, idempotently.

use crate::domain::premium::{PremiumGate, PremiumRefusal, PremiumStatus};
use crate::premium::promotion::{PROMOTION_PRICE, PromotionRefusal, check_promotion_eligibility};

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
    pub(crate) prices: Vec<BlessingPriceBand>,
}

#[derive(Debug, Clone)]
pub(crate) struct BlessingPriceBand {
    pub(crate) minimum_level: u32,
    pub(crate) maximum_level: u32,
    pub(crate) gold_units: u64,
}

fn admitted_promotion_offer(
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
fn admitted_blessing_offer(
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

struct PromotionPurchaseOperation {
    context: crate::durability::character_revision_sequencer::NpcPurchaseWriteContext,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
    request: crate::premium::promotion::PromotionPurchaseRequest,
}

impl
    crate::durability::premium_activation_latch::PromotionPremiumGateOperation<
        crate::premium::promotion::PendingPromotionPurchase,
        crate::premium::promotion::PromotionPurchaseError,
    > for PromotionPurchaseOperation
{
    fn run<'g, 't>(
        self,
        status: PremiumStatus<'g>,
        tx: Option<&'g mut crate::durability::item_mint_audit::Type2Transaction<'t>>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        crate::premium::promotion::PendingPromotionPurchase,
                        crate::premium::promotion::PromotionPurchaseError,
                    >,
                > + Send
                + 'g,
        >,
    >
    where
        Self: 'g,
    {
        Box::pin(async move {
            match tx {
                Some(tx) if matches!(status, PremiumStatus::NotActivated(_)) => {
                    crate::premium::promotion::prepare_promotion_purchase(
                        &self.context,
                        tx,
                        self.fence,
                        &self.request,
                        &status,
                    )
                    .await
                }
                Some(_) => Err(crate::premium::promotion::PromotionPurchaseError::InvalidInput),
                None => {
                    self.context
                        .prepare_delivered_promotion_purchase(self.fence, &self.request, &status)
                        .await
                }
            }
        })
    }
}

impl crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_> {
    /// The actual native NPC purchase consumer. The slot spans replay, the
    /// current actor/NPC checks, the single gate operation and its final COMMIT.
    /// Dispatch/registration stays in the final CP slot.
    pub(crate) async fn buy_npc_promotion(
        &self,
        actor: crate::foundation::ExactActorRef,
        command: crate::foundation::CommandRef,
        npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
        catalogue: &dyn PremiumNpcServiceCatalogue,
    ) -> Result<
        crate::premium::promotion::PromotionPurchaseOutcome,
        crate::premium::promotion::PromotionPurchaseError,
    > {
        use crate::durability::character_progression::CharacterProgressionError;
        use crate::durability::character_revision_sequencer::NpcPurchaseWriteContext;
        use crate::durability::item_fee_burn::{FeeBurnCause, FeeBurnRequest, FeeChangeFacts};
        use crate::premium::promotion::{
            PromotionNpcSource, PromotionPurchaseError, PromotionPurchaseRequest,
        };
        let actor_facts = self
            .promotion_actor_facts(actor, command.game_session_id())
            .await
            .ok_or(PromotionPurchaseError::Current(
                CharacterProgressionError::AuthorityRejected,
            ))?;
        let mut slot = self
            .revision_sequencer
            .acquire(actor_facts.fence.character_id)
            .await;
        let context =
            NpcPurchaseWriteContext::from_native_owner(self.root, self.character, self.holder)?;
        let occurrence = promotion_command_id(b"oteryn:promotion-occurrence:v1", command, npc);
        // The NPC may have gone away since the first COMMIT. Its historical
        // answer needs no live conversation, price table or Premium approval.
        match context
            .read_retained_promotion(actor_facts.fence.character_id, occurrence, npc)
            .await
        {
            Ok(Some(outcome)) => return slot.finish_promotion_gate(Ok(outcome)),
            Ok(None) => {}
            Err(error) => return slot.finish_promotion_gate(Err(error)),
        }
        let expected = match slot.cursor(self.root, self.character).await {
            Ok(expected) => expected,
            Err(crate::durability::character_authority::CharacterAuthorityError::Unavailable(
                error,
            )) => {
                return slot.finish_promotion_gate(Err(PromotionPurchaseError::Unavailable(error)));
            }
            Err(_) => {
                return Err(PromotionPurchaseError::Current(
                    CharacterProgressionError::AuthorityRejected,
                ));
            }
        };
        // Independently re-read the native physical actor and NPC after waiting
        // for the slot/read. No earlier snapshot manufactures current authority.
        let facts = self
            .promotion_command_facts(actor, command.game_session_id(), npc)
            .await
            .ok_or(PromotionPurchaseError::Current(
                CharacterProgressionError::AuthorityRejected,
            ))?;
        if facts.fence.character_id != slot.character_id() {
            return Err(PromotionPurchaseError::InvalidInput);
        }
        let offer = admitted_promotion_offer(catalogue, &facts.npc_key)
            .ok_or(PromotionPurchaseError::InvalidInput)?;
        let id = |tag: &[u8]| promotion_command_id(tag, command, npc);
        let occurred_at_unix_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|time| i64::try_from(time.as_millis()).ok())
            .filter(|time| *time > 0)
            .ok_or(PromotionPurchaseError::InvalidInput)?;
        let request = PromotionPurchaseRequest {
            occurrence,
            account_id: facts.account,
            source: PromotionNpcSource {
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
                occurred_at_unix_ms,
                server_build_id: crate::durability::character_authority::SERVER_BUILD_ID.to_owned(),
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
        let result = self
            .with_promotion_gate(
                facts.account,
                PromotionPurchaseOperation {
                    context,
                    fence:
                        crate::durability::character_progression::CurrentCharacterGameplayFence {
                            expected_character_revision: expected,
                            ..facts.fence
                        },
                    request,
                },
            )
            .await;
        let outcome = match result {
            Ok(Ok(pending)) => Ok(pending.after_native_commit()),
            Ok(Err(error)) => Err(error),
            Err(error) => Err(PromotionPurchaseError::Unavailable(error)),
        };
        slot.finish_promotion_gate(outcome)
    }
}

struct BlessingPurchaseOperation {
    context: crate::durability::character_revision_sequencer::NpcPurchaseWriteContext,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
    request: crate::premium::blessing_purchase::BlessingPurchaseRequest,
}
impl BlessingPurchaseOperation {
    fn run<'g, 't>(
        self,
        status: PremiumStatus<'g>,
        tx: Option<&'g mut crate::durability::premium_activation_latch::PremiumGateTx<'t>>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        crate::premium::blessing_purchase::PendingBlessingPurchase,
                        crate::premium::blessing_purchase::BlessingPurchaseError,
                    >,
                > + Send
                + 'g,
        >,
    > {
        Box::pin(async move {
            // An ordinary gate supplies a borrowed TX only before delivery.
            // Blessings have no exception and never adopt it as Type2.
            if tx.is_some() || matches!(status, PremiumStatus::NotActivated(_)) {
                return Err(
                    crate::premium::blessing_purchase::BlessingPurchaseError::Premium(
                        PremiumRefusal::NotCurrent,
                    ),
                );
            }
            // The native source resolves a raced stored outcome before Current-only
            // fresh admission, then opens/commits one actual source Type2 TX.
            self.context
                .prepare_delivered_blessing_purchase(self.fence, &self.request, &status)
                .await
        })
    }
}
impl crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_> {
    /// A selected native blessing conversation; client input contains no key/fee.
    /// The retained answer precedes live NPC, catalogue and Premium resolution.
    /// Final npc_talk dispatch remains its explicit CP slot after #2197.
    pub(crate) async fn buy_npc_blessing(
        &self,
        actor: crate::foundation::ExactActorRef,
        command: crate::foundation::CommandRef,
        npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
        service: PremiumBlessingService,
        catalogue: &dyn PremiumNpcServiceCatalogue,
    ) -> Result<
        crate::premium::blessing_purchase::BlessingPurchaseOutcome,
        crate::premium::blessing_purchase::BlessingPurchaseError,
    > {
        use crate::durability::character_progression::CharacterProgressionError;
        use crate::durability::character_revision_sequencer::NpcPurchaseWriteContext;
        use crate::durability::item_fee_burn::{FeeBurnCause, FeeBurnRequest, FeeChangeFacts};
        use crate::premium::blessing_purchase::{BlessingPurchaseError, BlessingPurchaseRequest};
        use crate::premium::promotion::NpcPurchaseSource;
        let actor_facts = self
            .promotion_actor_facts(actor, command.game_session_id())
            .await
            .ok_or(BlessingPurchaseError::Current(
                CharacterProgressionError::AuthorityRejected,
            ))?;
        let mut slot = self
            .revision_sequencer
            .acquire(actor_facts.fence.character_id)
            .await;
        let context =
            NpcPurchaseWriteContext::from_native_owner(self.root, self.character, self.holder)?;
        let occurrence = promotion_command_id(b"oteryn:blessing-occurrence:v1", command, npc);
        match context
            .read_retained_blessing(actor_facts.fence.character_id, occurrence, npc, service)
            .await
        {
            Ok(Some(outcome)) => return slot.finish_blessing_gate(Ok(outcome)),
            Ok(None) => {}
            Err(error) => return slot.finish_blessing_gate(Err(error)),
        }
        let expected = match slot.cursor(self.root, self.character).await {
            Ok(expected) => expected,
            Err(crate::durability::character_authority::CharacterAuthorityError::Unavailable(
                error,
            )) => return slot.finish_blessing_gate(Err(BlessingPurchaseError::Unavailable(error))),
            Err(_) => {
                return Err(BlessingPurchaseError::Current(
                    CharacterProgressionError::AuthorityRejected,
                ));
            }
        };
        let facts = self
            .promotion_command_facts(actor, command.game_session_id(), npc)
            .await
            .ok_or(BlessingPurchaseError::Current(
                CharacterProgressionError::AuthorityRejected,
            ))?;
        if facts.fence.character_id != slot.character_id() {
            return Err(BlessingPurchaseError::InvalidInput);
        }
        let offer = admitted_blessing_offer(catalogue, &facts.npc_key, service)
            .ok_or(BlessingPurchaseError::InvalidInput)?;
        let id = |tag: &[u8]| promotion_command_id(tag, command, npc);
        let occurred_at_unix_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|time| i64::try_from(time.as_millis()).ok())
            .filter(|time| *time > 0)
            .ok_or(BlessingPurchaseError::InvalidInput)?;
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
                // The real source overwrites this nonauthoritative placeholder using
                // the qualified price band selected at its locked Character level.
                fee_gold_units: 0,
                transaction_id: id(b"oteryn:blessing-fee-transaction:v1"),
                event_id: id(b"oteryn:blessing-fee-event:v1"),
                occurred_at_unix_ms,
                server_build_id: crate::durability::character_authority::SERVER_BUILD_ID.to_owned(),
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
        let operation = BlessingPurchaseOperation {
            context,
            fence: crate::durability::character_progression::CurrentCharacterGameplayFence {
                expected_character_revision: expected,
                ..facts.fence
            },
            request,
        };
        // Exactly one ordinary Premium gate for this command. Native status stays
        // inside its closure through the source TX/COMMIT; no cached admission.
        let result = self
            .with_premium_gate(facts.account, move |status, tx| operation.run(status, tx))
            .await;
        let outcome = match result {
            Ok(Ok(pending)) => Ok(pending.after_native_commit()),
            Ok(Err(error)) => Err(error),
            Err(error) => Err(BlessingPurchaseError::Unavailable(error)),
        };
        slot.finish_blessing_gate(outcome)
    }
}
