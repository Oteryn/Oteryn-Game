//! Shared party intervals, using the same fenced World membership as command 735.
use super::Totals;
use crate::content::item_ref::ItemDefinitionIndex;
use crate::durability::{
    DurabilityError,
    spell_item_transaction::SpellItemAuthority,
    world_party::ingress::{self, HuntChange, HuntPrice, HuntSettings},
};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, PartySnapshot};
use crate::gameplay_transport::analyser::AnalyserStreams;
use oteryn_protocol_oteryn::{
    analyser_party::{HuntInterval, HuntRequest, Member, Price, ValuationSource},
    party::PartyResult,
};
use sqlx::{Postgres, Transaction};
use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroU32,
    sync::Arc,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::gameplay_transport) struct Boundary {
    party: [u8; 16],
    interval: u64,
    membership_order: u64,
}

#[derive(Clone, Copy)]
pub(in crate::gameplay_transport) struct LocalMember {
    pub actor: ExactActorRef,
    pub session: GameSessionId,
    pub boundary: Boundary,
}

pub(in crate::gameplay_transport) struct Context<'a> {
    pub runtime: &'a ChannelRuntimeV1,
    pub streams: &'a AnalyserStreams,
    pub index: &'a ItemDefinitionIndex,
    pub request: &'a HuntRequest,
}

pub(in crate::gameplay_transport) type Measurement = (Totals, Option<Arc<ItemDefinitionIndex>>);

pub(in crate::gameplay_transport) struct Plan {
    settings: HuntSettings,
    prices: BTreeMap<u32, Price>,
    local: Vec<LocalMember>,
    measurements: Vec<Option<Measurement>>,
    reset: bool,
}

pub(in crate::gameplay_transport) fn identity(
    index: &ItemDefinitionIndex,
    id: u32,
) -> Option<(&str, &str)> {
    index.definition_at(NonZeroU32::new(id)?)
}

pub(in crate::gameplay_transport) async fn prepare(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    party: &PartySnapshot,
    context: Context<'_>,
) -> Result<Plan, DurabilityError> {
    let reject = || DurabilityError::Unavailable;
    let party_id = party.party_id.ok_or_else(reject)?;
    let census = context
        .runtime
        .positioned_actor_census()
        .map_err(|_| reject())?;
    let local_by_character: BTreeMap<_, _> = census
        .into_iter()
        .filter_map(|(actor, _, session)| {
            let session = session?;
            let lease = context.runtime.bound_attacker_lease(actor, session).ok()?;
            Some((*lease.character_id().as_bytes(), (actor, session)))
        })
        .collect();
    let reset = matches!(context.request, HuntRequest::Reset);
    let change = match context.request {
        HuntRequest::Reset => Some(HuntChange::Reset),
        HuntRequest::Valuation(ValuationSource::Market, _) => Some(HuntChange::Market),
        HuntRequest::Valuation(ValuationSource::Leader, prices) => Some(HuntChange::Leader(
            prices
                .iter()
                .map(|price| {
                    let (key, revision) = identity(context.index, price.item).ok_or_else(reject)?;
                    Ok(HuntPrice {
                        key: key.to_owned(),
                        revision: revision.to_owned(),
                        loot: price.loot,
                        supply: price.supply,
                    })
                })
                .collect::<Result<Vec<_>, DurabilityError>>()?,
        )),
        HuntRequest::Interval => None,
        HuntRequest::Legacy(_) => return Err(reject()),
    };
    let settings = ingress::hunt_settings_in_transaction(tx, authority, party, change)
        .await
        .map_err(|_| reject())?;
    let mut local = Vec::new();
    let mut measurements = Vec::with_capacity(party.members.len());
    let mut needed = BTreeSet::new();
    for member in &party.members {
        let measurement =
            local_by_character
                .get(&member.character)
                .and_then(|&(actor, session)| {
                    let member = LocalMember {
                        actor,
                        session,
                        boundary: Boundary {
                            party: party_id,
                            interval: settings.interval,
                            membership_order: member.invitation_order,
                        },
                    };
                    local.push(member);
                    let (mut totals, index) =
                        context.streams.hunt_accounting(member, settings.interval)?;
                    if reset {
                        totals = Totals::default();
                    } else if settings.leader_prices.is_none() {
                        for id in totals.loot.keys().chain(totals.supplies.keys()) {
                            if needed.len() >= super::MAX_ITEMS {
                                break;
                            }
                            if let Some(definition) = identity(context.index, *id)
                                && index.as_deref().and_then(|index| identity(index, *id))
                                    == Some(definition)
                                && crate::domain::currency::Coin::from_production_key(definition.0)
                                    .is_none()
                            {
                                needed.insert(definition);
                            }
                        }
                    }
                    Some((totals, index))
                });
        measurements.push(measurement);
    }
    let prices = if let Some(prices) = &settings.leader_prices {
        prices
            .iter()
            .filter_map(|price| {
                let item = context
                    .index
                    .definition_ref("Item", &price.key, &price.revision)?
                    .get();
                Some((
                    item,
                    Price {
                        item,
                        loot: price.loot,
                        supply: price.supply,
                    },
                ))
            })
            .collect()
    } else {
        let definitions: Vec<_> = needed
            .into_iter()
            .map(|(key, rev)| (key.to_owned(), rev.to_owned()))
            .collect();
        ingress::hunt_market_prices_in_transaction(tx, authority, &definitions)
            .await
            .map_err(|_| reject())?
            .into_iter()
            .filter_map(|(key, rev, price)| {
                let item = context.index.definition_ref("Item", &key, &rev)?.get();
                Some((
                    item,
                    Price {
                        item,
                        loot: Some(price),
                        supply: Some(price),
                    },
                ))
            })
            .collect()
    };
    Ok(Plan {
        settings,
        prices,
        local,
        measurements,
        reset,
    })
}

impl Plan {
    pub(in crate::gameplay_transport) fn apply(&self, streams: &AnalyserStreams) -> Option<()> {
        if self.settings.interval > 0 {
            streams.set_hunt_boundaries(&self.local, self.reset)?;
        }
        Some(())
    }

    pub(in crate::gameplay_transport) fn finish(
        self,
        projected: &PartyResult,
        index: &ItemDefinitionIndex,
    ) -> Option<Vec<u8>> {
        if projected.members.len() != self.measurements.len() {
            return None;
        }
        let source = if self.settings.leader_prices.is_some() {
            ValuationSource::Leader
        } else {
            ValuationSource::Market
        };
        let elapsed = self.settings.elapsed_seconds;
        let members = projected
            .members
            .iter()
            .zip(self.measurements)
            .map(|(member, measurement)| match measurement {
                Some((totals, observed_index)) => totals.member(
                    member.handle,
                    member.name.clone(),
                    elapsed,
                    observed_index.as_deref(),
                    index,
                    &self.prices,
                ),
                None => Member {
                    handle: member.handle,
                    name: member.name.clone(),
                    ..Default::default()
                },
            })
            .collect();
        HuntInterval {
            number: self.settings.interval,
            elapsed_seconds: elapsed,
            source,
            members,
        }
        .encode()
        .ok()
    }
}
