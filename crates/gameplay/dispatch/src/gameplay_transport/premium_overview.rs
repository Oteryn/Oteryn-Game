//! T165: observe the authenticated actor's existing Premium fence and live gate.
use super::spell_premium_coordinator::RegisteredSpellAccessOwner;
use super::{
    ComposedFreshAdmission, character_info::CharacterInfoReply, connection::AdmittedSession,
};
use crate::durability::{
    DurabilityError,
    character_authority::CharacterAuthorityError,
    item_transfer::CurrentCharacterItemFence,
    premium_fence::{EntitlementState, PremiumAccountRecord},
};
use oteryn_error_codes::{FieldValue, Level, Line, emit};
use std::time::Duration;
use tokio::time::Instant;

const SNAPSHOT_WAIT: Duration = Duration::from_secs(2);
const SNAPSHOT_POLL: Duration = Duration::from_millis(100);
const ACCOUNT_WAIT: Duration = Duration::from_millis(250);
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
    let account = resolve_account(owner, fence).await?;
    let mut result = Overview {
        state: State::Unavailable,
        authority_revision: 0,
        observed_at_us: 0,
        expires_at_us: None,
        valid_for_us: 0,
        features: features(false),
    };
    if let Some(coordinator) = owner.premium_coordinator {
        let view = coordinator.view(account);
        if let RegisteredSpellAccessOwner::Premium {
            consumer, clock, ..
        } = view.as_ref()
            && clock.read_current_window().is_some()
        {
            let record = match snapshot_record(owner, account).await {
                Ok(record) => record,
                Err(_) => return None,
            };
            let gate = coordinator.premium_gate(account).await;
            let now = clock.read_current_window();
            if let Some(now) = now {
                result.observed_at_us = now.upper_us().max(0);
            }
            if !record.conflicting
                && let Some(now) = now
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
    if resolve_account(owner, fence).await? != account
        || !super::battle_instinct_status::actor_is_current(&*owner.runtime.lock().await, admitted)
    {
        return None;
    }
    Some(result)
}
// The scheduled pull also uses the shared holder. Retrying only checkout
// misses preserves the account/lease verification on both sides of the wait.
async fn resolve_account(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    fence: CurrentCharacterItemFence,
) -> Option<[u8; 16]> {
    let deadline = Instant::now() + ACCOUNT_WAIT;
    let mut attempt = 0;
    loop {
        let read = owner
            .root
            .premium_overview_account(owner.character, owner.holder, fence);
        match tokio::time::timeout_at(deadline, read).await {
            Ok(Ok(account)) => return Some(account),
            Ok(Err(CharacterAuthorityError::Unavailable(DurabilityError::RootUnavailable))) => {
                attempt += 1;
                log_wait("account_holder_unavailable", attempt);
            }
            Ok(Err(_)) | Err(_) => return None,
        }
        if Instant::now() >= deadline {
            return None;
        }
        tokio::time::sleep_until((Instant::now() + SNAPSHOT_POLL).min(deadline)).await;
    }
}

fn log_wait(reason: &str, attempt: u64) {
    emit(
        "oteryn-game-server",
        &Line::new(Level::Info, "social", "premium_overview_wait")
            .field("reason", FieldValue::Token(reason))
            .field("attempt", FieldValue::U64(attempt)),
    );
}

/// Serve existing accepted evidence immediately. A first snapshot can be in
/// flight after admission, so allow a short wait for its durable result.
async fn snapshot_record(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    account: [u8; 16],
) -> Result<PremiumAccountRecord, DurabilityError> {
    if !owner.premium.has_schedule(account) {
        return owner.root.load_premium_fence(account).await;
    }
    let deadline = Instant::now() + SNAPSHOT_WAIT;
    let mut attempt = 0;
    loop {
        let result =
            tokio::time::timeout_at(deadline, owner.root.load_premium_fence(account)).await;
        let reason = match result {
            Ok(Ok(record)) if record.conflicting || record.fence.is_some() => return Ok(record),
            Ok(Ok(_)) => "awaiting_first_snapshot",
            Ok(Err(DurabilityError::RootUnavailable)) => "holder_unavailable",
            Ok(Err(error)) => return Err(error),
            Err(_) => "deadline",
        };
        attempt += 1;
        log_wait(reason, attempt);
        if Instant::now() >= deadline || !owner.premium.has_schedule(account) {
            return Ok(PremiumAccountRecord {
                fence: None,
                conflicting: false,
            });
        }
        tokio::time::sleep_until((Instant::now() + SNAPSHOT_POLL).min(deadline)).await;
    }
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
