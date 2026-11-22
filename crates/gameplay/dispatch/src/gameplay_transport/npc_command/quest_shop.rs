//! Display snapshots are advisory; the ordinary trade writer rechecks every source predicate.
use super::*;
use crate::content::npc_catalogue::quest_shops;
use crate::content::{ProjectV2ServiceOffer, ProjectV2ServiceOfferDirection};
use crate::gameplay_transport::ComposedFreshAdmission;
use std::collections::BTreeSet;

type Visible = BTreeSet<(String, String, ProjectV2ServiceOfferDirection)>;

/// Preserve the blank scroll's additional completion and backpack requirement in the quote.
pub(super) async fn blank_scroll_available(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: super::super::connection::UseCommand,
    intent: &NpcTalkIntent,
) -> bool {
    let Some(fence) = command
        .item_fence
        .filter(|_| ChatText::parse(&intent.text).is_ok_and(|text| asks_to_trade(&text)))
    else {
        return false;
    };
    let session = command.game_session_id;
    let is_albinius = {
        let runtime = admission.runtime.lock().await;
        admission
            .states
            .npc_conversations
            .current_npc(&admission.states.chat, &runtime, actor, session, intent)
            .is_some_and(|(_, _, key)| key == quest_shops::ALBINIUS)
    };
    if !is_albinius {
        return false;
    }
    let completed = admission
        .states
        .quest_sessions
        .lock()
        .is_ok_and(|sessions| {
            sessions
                .get(&session)
                .and_then(|entry| entry.copy.as_ref())
                .and_then(|copy| copy.states().get(quest_shops::FORGOTTEN_KNOWLEDGE))
                .is_some_and(|state| state.completed)
        });
    completed
        && admission
            .root
            .read_character_backpack(admission.character, fence.character_id)
            .await
            .is_ok_and(|backpack| {
                backpack.is_some_and(|backpack| {
                    backpack
                        .entries
                        .iter()
                        .filter(|entry| {
                            entry.item.definition.family == "Item"
                                && entry.item.definition.production_key
                                    == quest_shops::HEAVY_OLD_TOME
                        })
                        .map(|entry| u64::from(entry.item.quantity))
                        .sum::<u64>()
                        >= 5
                })
            })
}

pub(super) fn eligible_items(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    native: Option<&crate::content::native_gameplay::NativeGameplayState>,
    blank_scroll_available: bool,
) -> Visible {
    let Ok(sessions) = admission.states.quest_sessions.lock() else {
        return Visible::new();
    };
    let Some(copy) = sessions.get(&session).and_then(|s| s.copy.as_ref()) else {
        return Visible::new();
    };
    let mut visible = Visible::new();
    if let Some(book) = admission.states.chat.npc_talk() {
        for (npc, _, track) in quest_shops::ORDINARY_REQUIREMENTS {
            if copy.tracks().get(track) != Some(&quest_shops::ORDINARY_UNLOCK_VALUE) {
                continue;
            }
            if let Some(entry) = book.get(npc) {
                for offer in entry.offers() {
                    if npc != quest_shops::ALBINIUS
                        || offer.item.key != quest_shops::BLANK_IMBUEMENT_SCROLL
                        || blank_scroll_available
                    {
                        visible.insert((npc.into(), offer.item.key.clone(), offer.direction));
                    }
                }
            }
        }
    }
    let (Some(native), Ok(shops)) = (native, quest_shops::shops()) else {
        return visible;
    };
    for shop in shops {
        for offer in &shop.offers {
            if !quest_shops::eligible(offer, |r| copy.tracks().get(&r.track).copied()) {
                continue;
            }
            let item = crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: offer.item.key.clone(),
                revision_ref: offer.item.revision.clone(),
            };
            if super::super::item_ref_admission::generation_item_facts(native, &item).is_ok_and(
                |facts| {
                    facts.weight.is_some()
                        && !matches!(
                            facts.stack,
                            crate::durability::item_transfer::ItemStackClass::Unknown
                        )
                },
            ) {
                visible.insert((shop.npc.clone(), offer.item.key.clone(), offer.direction));
            }
        }
    }
    visible
}

pub(super) fn visible(npc: &str, offer: &ProjectV2ServiceOffer, cache: &Visible) -> bool {
    if quest_shops::ordinary_requirement(npc).is_some() {
        return cache.contains(&(npc.into(), offer.item.key.clone(), offer.direction));
    }
    match quest_shops::shops().and_then(|shops| {
        quest_shops::required(
            shops,
            npc,
            &offer.item,
            offer.direction,
            offer.unit_price,
            offer.count.unwrap_or(1),
        )
    }) {
        Ok(None) => true,
        Ok(Some(_)) => cache.contains(&(npc.into(), offer.item.key.clone(), offer.direction)),
        Err(_) => false,
    }
}

impl NpcConversations {
    pub(super) fn refresh_quest_shop_context(&self, session: GameSessionId, visible: Visible) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.entry(session).or_default().quest_shop_offers = visible;
        }
    }
}
