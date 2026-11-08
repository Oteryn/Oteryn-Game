//! Bounded connection-interval counters, disclosed only through the current party owner.
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::content::item_ref::ItemDefinitionIndex;
use crate::foundation::ChannelRuntimeV1;
use oteryn_protocol_oteryn::{
    analyser::{AnalyserFact, AnalyserImpact},
    analyser_party::{Member, NeededPrice, Price, encode_reply},
};
use std::collections::BTreeMap;
pub(super) type AccountingSnapshot = (u64, Totals, Option<std::sync::Arc<ItemDefinitionIndex>>);
const MAX_ITEMS: usize = 16384;
#[derive(Debug, Clone, Default)]
pub(super) struct Totals {
    kills: u64,
    xp: u64,
    raw_xp: u64,
    gold: u64,
    damage: u64,
    healing: u64,
    loot: BTreeMap<u32, u64>,
    supplies: BTreeMap<u32, u64>,
    incomplete: bool,
}
fn add(total: &mut u64, value: u64, incomplete: &mut bool) {
    match total.checked_add(value) {
        Some(value) => *total = value,
        None => {
            *total = u64::MAX;
            *incomplete = true;
        }
    }
}
fn item(map: &mut BTreeMap<u32, u64>, id: u32, count: u64, incomplete: &mut bool) {
    if !map.contains_key(&id) && map.len() >= MAX_ITEMS {
        *incomplete = true;
        return;
    }
    add(map.entry(id).or_default(), count, incomplete);
}
impl Totals {
    pub(super) fn observe(&mut self, fact: &AnalyserFact) {
        match fact {
            AnalyserFact::Kill { .. } => add(&mut self.kills, 1, &mut self.incomplete),
            AnalyserFact::Experience { raw, gained } => {
                add(&mut self.raw_xp, *raw, &mut self.incomplete);
                add(&mut self.xp, *gained, &mut self.incomplete);
            }
            AnalyserFact::KillLoot {
                corpse_items, gold, ..
            } => {
                add(&mut self.gold, *gold, &mut self.incomplete);
                for row in corpse_items {
                    item(
                        &mut self.loot,
                        row.item_type.get(),
                        u64::from(row.count.get()),
                        &mut self.incomplete,
                    );
                }
            }
            AnalyserFact::SupplyUsed(row) => item(
                &mut self.supplies,
                row.item_type.get(),
                u64::from(row.count.get()),
                &mut self.incomplete,
            ),
            AnalyserFact::Impact(AnalyserImpact::Damage { value, .. }) => add(
                &mut self.damage,
                u64::from(value.get()),
                &mut self.incomplete,
            ),
            AnalyserFact::Impact(AnalyserImpact::Healing(value)) => add(
                &mut self.healing,
                u64::from(value.get()),
                &mut self.incomplete,
            ),
            // Queue loss does not lose these independent counters.
            AnalyserFact::DamageInput { .. } | AnalyserFact::Dropped(_) => {}
        }
    }
    fn member(
        &self,
        handle: u64,
        name: String,
        elapsed_seconds: u64,
        index: Option<&ItemDefinitionIndex>,
        requester: &ItemDefinitionIndex,
        prices: &[Price],
    ) -> Member {
        let mut row = Member {
            handle,
            name,
            available: true,
            incomplete: self.incomplete,
            elapsed_seconds,
            kills: self.kills,
            xp: self.xp,
            raw_xp: self.raw_xp,
            loot_value: self.gold,
            damage: self.damage,
            healing: self.healing,
            ..Default::default()
        };
        let prices: BTreeMap<_, _> = prices.iter().map(|p| (p.item, p)).collect();
        for (supply, items) in [(false, &self.loot), (true, &self.supplies)] {
            for (id, count) in items {
                let identity = std::num::NonZeroU32::new(*id)
                    .and_then(|id| index.and_then(|index| index.definition_at(id)));
                let same = std::num::NonZeroU32::new(*id).is_some_and(|id| {
                    identity.is_some() && identity == requester.definition_at(id)
                });
                let coin = identity
                    .and_then(|(key, _)| crate::domain::currency::Coin::from_production_key(key))
                    .map(|coin| coin.worth());
                let unit = if supply {
                    coin.or_else(|| {
                        if same {
                            prices.get(id).and_then(|p| p.supply)
                        } else {
                            None
                        }
                    })
                } else if coin.is_some() {
                    Some(0)
                } else if same {
                    prices.get(id).and_then(|p| p.loot)
                } else {
                    None
                };
                let (quantity, value, unpriced) = if supply {
                    (
                        &mut row.supplies,
                        &mut row.supply_cost,
                        &mut row.unpriced_supplies,
                    )
                } else {
                    (
                        &mut row.loot_items,
                        &mut row.loot_value,
                        &mut row.unpriced_loot,
                    )
                };
                add(quantity, *count, &mut row.incomplete);
                if let Some(unit) = unit {
                    match count.checked_mul(unit) {
                        Some(amount) => add(value, amount, &mut row.incomplete),
                        None => row.incomplete = true,
                    }
                } else {
                    add(unpriced, *count, &mut row.incomplete);
                    if same {
                        if let Some(needed) = row.needed_prices.iter_mut().find(|p| p.item == *id) {
                            if supply {
                                needed.supply = true;
                            } else {
                                needed.loot = true;
                            }
                        } else if row.needed_prices.len() < 32 {
                            row.needed_prices.push(NeededPrice {
                                item: *id,
                                loot: !supply,
                                supply,
                            });
                        }
                    }
                }
                if identity.is_none() || (!same && coin.is_none()) {
                    row.incomplete = true;
                }
            }
        }
        row
    }
}
pub(super) fn project(
    runtime: &ChannelRuntimeV1,
    streams: &super::analyser::AnalyserStreams,
    members: &[([u8; 16], String)],
    projected: &oteryn_protocol_oteryn::party::PartyResult,
    requester: &ItemDefinitionIndex,
    prices: &[Price],
) -> Option<Vec<u8>> {
    let census = runtime.positioned_actor_census().ok()?;
    if members.len() != projected.members.len() {
        return None;
    }
    let mut rows = Vec::new();
    for ((id, name), member) in members.iter().zip(&projected.members) {
        let snapshot = census.iter().find_map(|(actor, _, session)| {
            let session = (*session)?;
            let lease = runtime.bound_attacker_lease(*actor, session).ok()?;
            if lease.character_id().as_bytes() != id {
                return None;
            }
            streams.accounting(*actor, session)
        });
        rows.push(match snapshot {
            Some((seconds, totals, index)) => totals.member(
                member.handle,
                name.clone(),
                seconds,
                index.as_deref(),
                requester,
                prices,
            ),
            None => Member {
                handle: member.handle,
                name: name.clone(),
                ..Default::default()
            },
        });
    }
    encode_reply(&rows).ok()
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn analyser_party_page(
        &self,
        admitted: AdmittedSession,
        command_id: u64,
        prices: &[Price],
        view: &mut super::party_command::SessionPartyView,
    ) -> Option<Vec<u8>> {
        let runtime = self.runtime.lock().await;
        if !super::battle_instinct_status::actor_is_current(&runtime, admitted) {
            return None;
        }
        let index = super::item_ref_admission::pinned_index(&runtime, self.active_generation)?;
        if prices.iter().any(|p| {
            std::num::NonZeroU32::new(p.item).is_none_or(|id| index.definition_at(id).is_none())
        }) {
            return None;
        }
        drop(runtime);
        view.hunt_prices = prices.to_vec();
        view.hunt_reply = None;
        let _ = self
            .party_wire_command(
                admitted,
                command_id,
                oteryn_protocol_oteryn::analyser_party::COMMAND_TYPE_ANALYSER_PARTY_QUERY,
                &[],
                view,
            )
            .await;
        view.hunt_prices.clear();
        view.hunt_reply.take()
    }
}
macro_rules! authority_methods {
    () => {
        fn analyser_party_query(
            &self,
            _admitted: $crate::gameplay_transport::AdmittedSession,
            _command_id: u64,
            _prices: &[oteryn_protocol_oteryn::analyser_party::Price],
            _view: &mut $crate::gameplay_transport::party_command::SessionPartyView,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn analyser_party_query(
            &self,
            admitted: connection::AdmittedSession,
            command_id: u64,
            prices: &[oteryn_protocol_oteryn::analyser_party::Price],
            view: &mut $crate::gameplay_transport::party_command::SessionPartyView,
        ) -> Option<Vec<u8>> {
            self.analyser_party_page(admitted, command_id, prices, view)
                .await
        }
    };
}
pub(super) use composed_methods;

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_protocol_oteryn::analyser::AnalyserItemCount;
    #[test]
    fn party_accounting_values_committed_items_without_double_counting_coins()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = ItemDefinitionIndex::decode(br#"{"schema":"OTERYN_NATIVE_ITEM_KEYS/v1","records":[["oteryn:item.tibia.i3031","definition-r1"],["oteryn:item.tibia.i5000","definition-r1"]]}"#)?;
        let item = |id, count| -> Result<AnalyserItemCount, &'static str> {
            Ok(AnalyserItemCount {
                item_type: std::num::NonZeroU32::new(id).ok_or("item")?,
                count: std::num::NonZeroU16::new(count).ok_or("count")?,
            })
        };
        let mut totals = Totals::default();
        totals.observe(&AnalyserFact::Kill { race: None });
        totals.observe(&AnalyserFact::KillLoot {
            race: None,
            corpse_items: vec![item(1, 10)?, item(2, 2)?],
            gold: 10,
        });
        totals.observe(&AnalyserFact::SupplyUsed(item(2, 3)?));
        let missing = totals.member(1, "Scout".into(), 60, Some(&index), &index, &[]);
        assert_eq!(
            (
                missing.kills,
                missing.loot_items,
                missing.loot_value,
                missing.unpriced_loot,
                missing.unpriced_supplies
            ),
            (1, 12, 10, 2, 3)
        );
        assert_eq!(
            missing.needed_prices,
            vec![NeededPrice {
                item: 2,
                loot: true,
                supply: true
            }]
        );
        let valued = totals.member(
            1,
            "Scout".into(),
            60,
            Some(&index),
            &index,
            &[
                Price {
                    item: 1,
                    loot: Some(1000),
                    supply: None,
                },
                Price {
                    item: 2,
                    loot: Some(5),
                    supply: Some(7),
                },
            ],
        );
        assert_eq!(
            (
                valued.loot_value,
                valued.supply_cost,
                valued.unpriced_loot,
                valued.unpriced_supplies
            ),
            (20, 21, 0, 0)
        );
        assert!(!valued.incomplete);
        let other = ItemDefinitionIndex::decode(br#"{"schema":"OTERYN_NATIVE_ITEM_KEYS/v1","records":[["oteryn:item.tibia.i3031","definition-r1"],["oteryn:item.tibia.i5001","definition-r1"]]}"#)?;
        let incompatible = totals.member(
            1,
            "Scout".into(),
            60,
            Some(&index),
            &other,
            &[Price {
                item: 2,
                loot: Some(1000),
                supply: Some(1000),
            }],
        );
        assert!(incompatible.incomplete);
        assert_eq!((incompatible.loot_value, incompatible.supply_cost), (10, 0));
        assert!(incompatible.needed_prices.is_empty());
        Ok(())
    }
    #[test]
    fn party_accounting_survives_queue_drain_but_resets_with_connection_generation()
    -> Result<(), &'static str> {
        let (_, actor, session) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(31);
        let streams = super::super::analyser::AnalyserStreams::default();
        streams
            .snapshot(actor, session, 1, None)
            .ok_or("snapshot")?;
        streams.push(actor, session, AnalyserFact::Kill { race: None });
        streams.next(actor, session, 1).ok_or("batch")?;
        assert_eq!(
            streams
                .accounting(actor, session)
                .ok_or("counters")?
                .1
                .kills,
            1
        );
        streams
            .snapshot(actor, session, 2, None)
            .ok_or("replacement")?;
        assert_eq!(
            streams.accounting(actor, session).ok_or("fresh")?.1.kills,
            0
        );
        streams.forget(session);
        assert!(streams.accounting(actor, session).is_none());
        Ok(())
    }
}
