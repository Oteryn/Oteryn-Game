//! T165: observe the authenticated actor's existing Premium fence and live gate.
use super::spell_premium_coordinator::RegisteredSpellAccessOwner;
use super::{
    ComposedFreshAdmission, character_info::CharacterInfoReply, connection::AdmittedSession,
};
use crate::durability::premium_fence::EntitlementState;
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
    if let Some(coordinator) = owner.premium_coordinator {
        let gate = coordinator.premium_gate(account).await;
        let view = coordinator.view(account);
        if let RegisteredSpellAccessOwner::Premium {
            consumer, clock, ..
        } = view.as_ref()
            && let Some(now) = clock.read_current_window()
        {
            result.observed_at_us = now.upper_us().max(0);
            let record = owner.root.load_premium_fence(account).await.ok()?;
            if !record.conflicting
                && let Some(fence) = record.fence.filter(|f| !f.conflicting)
            {
                let e = fence.latest;
                result.authority_revision = e.authority_revision;
                result.expires_at_us = (e.effective_until_us > 0).then_some(e.effective_until_us);
                result.state = match e.state {
                    EntitlementState::None => State::None,
                    EntitlementState::Active => State::Active,
                    EntitlementState::Expired => State::Expired,
                    EntitlementState::Revoked => State::Revoked,
                    EntitlementState::NotYetEffective => State::Scheduled,
                };
                if gate.require().is_ok()
                    && let Some(current) = consumer
                        .current_premium(account, Some(now))
                        .filter(|c| c.authority_revision == e.authority_revision)
                {
                    result.valid_for_us =
                        u64::try_from(current.benefit_until_us.checked_sub(now.upper_us())?)
                            .ok()?
                            .min(60_000_000);
                }
                result.features = features(result.valid_for_us > 0);
            }
        }
    }
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
    Some(result)
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
