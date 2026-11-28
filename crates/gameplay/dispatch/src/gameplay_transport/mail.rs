//! Composition for MAIL_V1 through the same admitted command dispatch boundary.

#[path = "mail_text.rs"]
mod text;

use super::item_move::{LoopMoveRefusal, LoopMoveResult};
use super::{ComposedFreshAdmission, connection::AdmittedSession, item_view::ItemKey};
use crate::content::item_ref::ItemDefinitionIndex;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::map::overlay::TilePos;
use oteryn_gameplay_durability::durability::mail::{
    MailPostOutcome, MailPostRefusal, MailPostTarget,
};
use oteryn_protocol_oteryn::mail::{MailIntent, MailOutcome, decode_intent};
use oteryn_protocol_oteryn::world_map::MapDefinition;

macro_rules! authority_methods {
    () => {
        fn mail_command(
            &self,
            _actor: $crate::foundation::ExactActorRef,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _command_id: u64,
            _payload: &[u8],
            _item: Option<$crate::gameplay_transport::item_view::ItemKey>,
            _mailbox: Option<$crate::gameplay_transport::item_view::ItemKey>,
        ) -> impl std::future::Future<Output = Option<oteryn_protocol_oteryn::mail::MailOutcome>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn mail_command(
            &self,
            actor: $crate::foundation::ExactActorRef,
            admitted: $crate::gameplay_transport::connection::AdmittedSession,
            command_id: u64,
            payload: &[u8],
            item: Option<$crate::gameplay_transport::item_view::ItemKey>,
            mailbox: Option<$crate::gameplay_transport::item_view::ItemKey>,
        ) -> Option<oteryn_protocol_oteryn::mail::MailOutcome> {
            $crate::gameplay_transport::mail::execute(
                self, actor, admitted, command_id, payload, item, mailbox,
            )
            .await
        }
    };
}
pub(super) use composed_methods;

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    admitted: AdmittedSession,
    command_id: u64,
    payload: &[u8],
    item: Option<ItemKey>,
    mailbox: Option<ItemKey>,
) -> Option<MailOutcome> {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(u32::from(oteryn_protocol_oteryn::mail::CAPABILITY_MAIL_V1))
    {
        return Some(MailOutcome::Rejected);
    }
    if let Ok(MailIntent::WriteText { text, .. }) = decode_intent(payload) {
        return text::execute(owner, actor, admitted, command_id, item, text).await;
    }
    let (
        Some(fence),
        Ok(MailIntent::Post { .. }),
        Some(ItemKey::Instance(item)),
        Some(ItemKey::MapBase { placement_key, .. }),
    ) = (admitted.item_fence, decode_intent(payload), item, mailbox)
    else {
        return Some(MailOutcome::Rejected);
    };
    let Some(target) = mailbox_target(owner, actor, admitted.game_session_id, placement_key).await
    else {
        return Some(MailOutcome::Rejected);
    };
    match owner
        .root
        .post_admitted_mail_with_target(
            owner.character,
            owner.holder,
            fence,
            command_id,
            item,
            target,
        )
        .await
    {
        Ok(outcome) => Some(wire_post_outcome(outcome)),
        Err(_) => None,
    }
}

/// Canary 52550d5e `data/items/items.xml` `type="mailbox"`: 3501 (L10952), 3508 (L10991),
/// 23399 (L45964), 23400 (L45971), 23401 (L45978) and 23402 (L45985).
const MAILBOX_KEYS: [&str; 6] = [
    "oteryn:item.tibia.i3501",
    "oteryn:item.tibia.i3508",
    "oteryn:item.tibia.i23399",
    "oteryn:item.tibia.i23400",
    "oteryn:item.tibia.i23401",
    "oteryn:item.tibia.i23402",
];

// Tibia Fandom /wiki/Mailbox, Royal Mailboxes (locked). The World placement
// regions identify the mailbox tiles at the listed pins and landmarks.
// Ordinary boxes share the same appearances, so access is a placement property.
const ROYAL_MAILBOX_TILES: [(u16, u16, i8); 17] = [
    (31921, 32651, -8),  // Talahu teleportation hub
    (32008, 32862, -5),  // Nargor
    (32386, 32950, -7),  // Laguna Islands
    (32423, 32095, -15), // Mintwallin
    (32448, 31964, -10), // Kazordoon Mine Hub
    (32454, 31975, -10), // Kazordoon Mine Hub
    (32459, 31964, -10), // Kazordoon Mine Hub
    (32524, 32025, -13), // Robson's Isle
    (32535, 31969, -7),  // Kazordoon surface
    (32615, 31434, -7),  // Mistrock
    (32970, 31778, -7),  // Orc Fortress
    (32995, 32446, -7),  // Drefia north beach
    (33075, 31212, -7),  // Razachai magistracy
    (33083, 32184, -8),  // Shadowthorn, north of the floor descent
    (33200, 31064, -9),  // Zao Rebel Camp
    (33271, 31656, -8),  // Cyclopolis
    (33307, 32292, -7),  // Minotaur Pyramid
];

/// A carried letter or parcel moved onto a mailbox tile posts it, as Canary `Tile::addThing`
/// hands the thing to the tile's mailbox (`tile.cpp:1695`) and `Mailbox::sendItem` moves it into
/// the recipient's Inbox (`mailbox.cpp:74`). `None` for an item that is no unstamped letter or
/// parcel in the sender's custody: that move drops it on the tile as any other move does.
pub(super) async fn post_moved(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    fence: CurrentCharacterItemFence,
    command_id: u64,
    item: [u8; 16],
    mailbox: MailPostTarget,
) -> Option<LoopMoveResult> {
    match owner
        .root
        .post_admitted_mail_with_target(
            owner.character,
            owner.holder,
            fence,
            command_id,
            item,
            mailbox,
        )
        .await
    {
        Ok(MailPostOutcome::Posted | MailPostOutcome::AlreadyPosted) => {
            Some(LoopMoveResult::Committed)
        }
        Ok(MailPostOutcome::Refused(MailPostRefusal::NotMailable)) => None,
        Ok(MailPostOutcome::Refused(_)) => Some(LoopMoveResult::Refused(LoopMoveRefusal::Rejected)),
        Err(_) => Some(LoopMoveResult::Unknown),
    }
}

/// The posted-to map entry must be an unhidden authored mailbox next to the living actor, on
/// its floor: the reach every other map destination of an item move has (`loop_move` in_reach).
/// Canary posts on any move into a mailbox (`mailbox.cpp:18`), so its throw range of 15 and line
/// of sight (`game.cpp:2221`) would also admit a farther mailbox; Oteryn keeps its move reach.
async fn mailbox_target(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    placement_key: u64,
) -> Option<MailPostTarget> {
    let (pos, ordinal) = crate::map::view::placement_of(placement_key)?;
    let mut runtime = owner.runtime.lock().await;
    if runtime.player_control_facts(actor, session).is_err()
        || owner.states.spell_states.lock().await.is_dead(actor)
    {
        return None;
    }
    let index = super::item_ref_admission::pinned_index(&runtime, owner.active_generation)?;
    let at = super::bundle_map_position::bundle_map_position(
        &mut runtime,
        owner.movement_cells(),
        actor,
    )?;
    if i64::from(at.at.floor) == i64::from(pos.floor)
        && (i64::from(at.at.x) - i64::from(pos.x)).abs() <= 1
        && (i64::from(at.at.y) - i64::from(pos.y)).abs() <= 1
        && mailbox_ordinals(at.map, index, pos).contains(&ordinal)
    {
        Some(reachable_mailbox(pos, placement_key))
    } else {
        Some(MailPostTarget::ReplayOnly(placement_key))
    }
}

fn reachable_mailbox(pos: TilePos, key: u64) -> MailPostTarget {
    if ROYAL_MAILBOX_TILES.contains(&(pos.x, pos.y, pos.floor)) {
        MailPostTarget::Royal(key)
    } else {
        MailPostTarget::Reachable(key)
    }
}

/// The mailbox a move onto `pos` posts into: the topmost unhidden one,
/// as Canary `Tile::getMailbox` (`tile.cpp:298`) takes the last mailbox of the tile.
pub(super) fn mailbox_at(
    map: &crate::map::boot::BundleMap,
    index: &ItemDefinitionIndex,
    pos: TilePos,
) -> Option<MailPostTarget> {
    let ordinal = mailbox_ordinals(map, index, pos).pop()?;
    let key = crate::map::view::placement_key(pos, ordinal)?;
    Some(reachable_mailbox(pos, key))
}

/// The ordinals of `pos`'s unhidden top-level base entries whose admitted definition is a
/// mailbox, ascending.
fn mailbox_ordinals(
    map: &crate::map::boot::BundleMap,
    index: &ItemDefinitionIndex,
    pos: TilePos,
) -> Vec<u8> {
    use crate::map::view::MapFacts;
    let Some(tile) = map.overlay.base().tile(pos.x, pos.y, pos.floor) else {
        return Vec::new();
    };
    let overlay = map.overlay.tile(pos);
    tile.ids()
        .iter()
        .zip(tile.depths())
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
        .filter_map(|(ordinal, (id, _))| {
            let ordinal = u8::try_from(ordinal).ok()?;
            if overlay
                .as_ref()
                .is_some_and(|overlay| overlay.is_hidden(ordinal))
            {
                return None;
            }
            let MapDefinition::Item(reference) =
                map.facts.base_entry(pos, ordinal, *id)?.definition
            else {
                return None;
            };
            let definition = super::item_look::typed_definition(index, reference)?;
            MAILBOX_KEYS
                .contains(&definition.production_key.as_str())
                .then_some(ordinal)
        })
        .collect()
}

/// The transport owns decoding only. The implementation owns handle reach,
/// Content and session fencing, replay, and the atomic database transaction.
pub trait MailCommandOwner {
    fn post(&mut self, item_handle: u64, mailbox_handle: u64) -> MailPostOutcome;
    fn write_text(&mut self, item_handle: u64, text: &str) -> MailOutcome;
}

pub fn dispatch_mail(owner: &mut impl MailCommandOwner, payload: &[u8]) -> MailOutcome {
    match decode_intent(payload) {
        Ok(MailIntent::Post {
            item_handle,
            mailbox_handle,
        }) => wire_post_outcome(owner.post(item_handle, mailbox_handle)),
        Ok(MailIntent::WriteText { item_handle, text }) => owner.write_text(item_handle, &text),
        Err(_) => MailOutcome::Rejected,
    }
}

fn wire_post_outcome(outcome: MailPostOutcome) -> MailOutcome {
    match outcome {
        MailPostOutcome::Posted | MailPostOutcome::AlreadyPosted => MailOutcome::Posted,
        MailPostOutcome::Refused(refusal) => match refusal {
            MailPostRefusal::UnknownRecipient => MailOutcome::UnknownRecipient,
            MailPostRefusal::NotMailable => MailOutcome::NotMailable,
            MailPostRefusal::RecipientInboxFull => MailOutcome::RecipientInboxFull,
            MailPostRefusal::Exhausted => MailOutcome::Exhausted,
            MailPostRefusal::MailboxOutOfReach
            | MailPostRefusal::PostmanRequired
            | MailPostRefusal::ReplayConflict => MailOutcome::Rejected,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_protocol_oteryn::mail::{MailWireError, encode_intent};

    struct Owner(Vec<String>);
    impl MailCommandOwner for Owner {
        fn post(&mut self, item: u64, mailbox: u64) -> MailPostOutcome {
            self.0.push(format!("post:{item}:{mailbox}"));
            MailPostOutcome::Posted
        }
        fn write_text(&mut self, item: u64, text: &str) -> MailOutcome {
            self.0.push(format!("write:{item}:{text}"));
            MailOutcome::TextWritten
        }
    }
    #[test]
    fn every_post_outcome_maps_through_dispatch() -> Result<(), MailWireError> {
        struct ResultOwner(MailPostOutcome);
        impl MailCommandOwner for ResultOwner {
            fn post(&mut self, _: u64, _: u64) -> MailPostOutcome {
                self.0
            }
            fn write_text(&mut self, _: u64, _: &str) -> MailOutcome {
                unreachable!()
            }
        }
        let payload = encode_intent(&MailIntent::Post {
            item_handle: 1,
            mailbox_handle: 2,
        })?;
        for (outcome, expected) in [
            (MailPostOutcome::Posted, MailOutcome::Posted),
            (MailPostOutcome::AlreadyPosted, MailOutcome::Posted),
            (
                MailPostOutcome::Refused(MailPostRefusal::UnknownRecipient),
                MailOutcome::UnknownRecipient,
            ),
            (
                MailPostOutcome::Refused(MailPostRefusal::NotMailable),
                MailOutcome::NotMailable,
            ),
            (
                MailPostOutcome::Refused(MailPostRefusal::RecipientInboxFull),
                MailOutcome::RecipientInboxFull,
            ),
            (
                MailPostOutcome::Refused(MailPostRefusal::Exhausted),
                MailOutcome::Exhausted,
            ),
            (
                MailPostOutcome::Refused(MailPostRefusal::ReplayConflict),
                MailOutcome::Rejected,
            ),
        ] {
            assert_eq!(dispatch_mail(&mut ResultOwner(outcome), &payload), expected);
        }
        Ok(())
    }

    #[test]
    fn composes_both_real_commands_and_refuses_malformed_input() -> Result<(), MailWireError> {
        let mut owner = Owner(Vec::new());
        let post = encode_intent(&MailIntent::Post {
            item_handle: 3,
            mailbox_handle: 4,
        })?;
        let write = encode_intent(&MailIntent::WriteText {
            item_handle: 3,
            text: "Ada".into(),
        })?;
        assert_eq!(dispatch_mail(&mut owner, &post), MailOutcome::Posted);
        assert_eq!(dispatch_mail(&mut owner, &write), MailOutcome::TextWritten);
        assert_eq!(dispatch_mail(&mut owner, &[]), MailOutcome::Rejected);
        assert_eq!(owner.0, ["post:3:4", "write:3:Ada"]);
        Ok(())
    }
}
