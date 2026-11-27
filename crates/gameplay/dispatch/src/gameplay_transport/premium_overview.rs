//! T165: observe the authenticated actor's existing Premium fence and live gate.
use super::spell_premium_coordinator::RegisteredSpellAccessOwner;
use super::{
    ComposedFreshAdmission, character_info::CharacterInfoReply, connection::AdmittedSession,
};
use crate::durability::premium_fence::EntitlementState;
use crate::premium::PremiumClass;
use oteryn_protocol_oteryn::premium_overview::{Feature, Overview, State};

pub(super) async fn query(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> CharacterInfoReply {
    match observe(owner, admitted).await.and_then(|v| v.encode()) {
        Some(bytes) => CharacterInfoReply::Payload(bytes),
        None => CharacterInfoReply::Rejected,
    }
}
async fn observe(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> Option<Overview> {
    let fence = admitted.item_fence?;
    if !super::battle_instinct_status::actor_is_current(&*owner.runtime.lock().await, admitted) {
        return None;
    }
    let account = owner
        .root
        .premium_overview_account(owner.character, owner.holder, fence)
        .await
        .ok()?;
    let mut result = Overview {
        state: State::Unavailable,
        authority_revision: 0,
        observed_at_us: 0,
        expires_at_us: None,
        valid_for_us: 0,
        features: features(false),
    };
    let snapshot = owner.premium.wait_for_snapshot(account).await;
    let projection = populate(owner, account, &mut result).await;
    // An ownership/lease takeover during the observation cannot return a predecessor's account.
    if owner
        .root
        .premium_overview_account(owner.character, owner.holder, fence)
        .await
        .ok()?
        != account
        || !super::battle_instinct_status::actor_is_current(&*owner.runtime.lock().await, admitted)
    {
        return None;
    }
    if let Err(reason) = projection {
        super::operator_event(&format!(
            "event=premium_overview_unavailable reason={} snapshot={:?}",
            reason.label(),
            snapshot.err(),
        ));
    }
    Some(result)
}
#[derive(Debug)]
enum OverviewUnavailable {
    CoordinatorUnavailable,
    ClockUnavailable,
    FenceUnavailable,
    ConflictingEvidence,
    Consumer(PremiumClass),
    ActivationOrRevisionUnavailable,
}

impl OverviewUnavailable {
    fn label(&self) -> String {
        match self {
            Self::Consumer(class) => format!("{class:?}"),
            reason => format!("{reason:?}"),
        }
    }
}

async fn populate(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    account: [u8; 16],
    result: &mut Overview,
) -> Result<(), OverviewUnavailable> {
    use OverviewUnavailable as Unavailable;
    let coordinator = owner
        .premium_coordinator
        .ok_or(Unavailable::CoordinatorUnavailable)?;
    let gate = coordinator.premium_gate(account).await;
    let view = coordinator.view(account);
    let RegisteredSpellAccessOwner::Premium {
        consumer, clock, ..
    } = view.as_ref()
    else {
        return Err(Unavailable::ClockUnavailable);
    };
    let record = super::AdmissionRetryBudget::for_command(owner.root)
        .retry_command_guard(async || owner.root.load_premium_fence(account).await)
        .await
        .map_err(|_| Unavailable::FenceUnavailable)?;
    let now = clock
        .read_current_window()
        .ok_or(Unavailable::ClockUnavailable)?;
    result.observed_at_us = now.upper_us().max(0);
    if record.conflicting || record.fence.as_ref().is_some_and(|fence| fence.conflicting) {
        return Err(Unavailable::ConflictingEvidence);
    }
    let evidence = record
        .fence
        .ok_or(Unavailable::Consumer(PremiumClass::AuthorityUnavailable))?
        .latest;
    let mut stale = None;
    result.state = match evidence.state {
        EntitlementState::None => State::None,
        EntitlementState::Expired => State::Expired,
        EntitlementState::Revoked => State::Revoked,
        EntitlementState::NotYetEffective => State::Scheduled,
        EntitlementState::Active => {
            if let Some(current) = consumer.current_premium(account, Some(now)) {
                if gate.require().is_err()
                    || current.authority_revision != evidence.authority_revision
                {
                    return Err(Unavailable::ActivationOrRevisionUnavailable);
                }
                result.valid_for_us = u64::try_from(
                    current
                        .benefit_until_us
                        .saturating_sub(now.upper_us())
                        .clamp(0, 60_000_000),
                )
                .map_err(|_| Unavailable::ClockUnavailable)?;
            } else {
                let class = consumer.class(account, Some(now));
                if class != PremiumClass::StaleWithinBound {
                    return Err(Unavailable::Consumer(class));
                }
                // Keep historical expiry visible while its feature lease is closed.
                stale = Some(Unavailable::Consumer(class));
            }
            State::Active
        }
    };
    result.authority_revision = evidence.authority_revision;
    result.expires_at_us = (evidence.effective_until_us > 0).then_some(evidence.effective_until_us);
    result.features = features(result.valid_for_us > 0);
    stale.map_or(Ok(()), Err)
}

fn features(enabled: bool) -> Vec<Feature> {
    [
        (
            "Premium channels",
            "Enter Premium-only channels when the world and channel permit it.",
        ),
        (
            "Promotion benefits",
            "Use promotion benefits while the character has the required promotion.",
        ),
        (
            "Premium spells",
            "Use learned Premium spells when vocation, level and other spell rules permit it.",
        ),
        (
            "Private channels",
            "Create private channels where the channel service is available.",
        ),
        (
            "House services",
            "Qualify for Premium-gated house services, subject to ownership and world rules.",
        ),
    ]
    .into_iter()
    .map(|(title, description)| Feature {
        title: title.into(),
        description: description.into(),
        enabled,
    })
    .collect()
}
