//! Composition for MAIL_V1 through the same admitted command dispatch boundary.

use super::item_move::{LoopMoveRefusal, LoopMoveResult};
use super::{ComposedFreshAdmission, connection::AdmittedSession, item_view::ItemKey};
use crate::content::item_ref::ItemDefinitionIndex;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::map::overlay::TilePos;
use oteryn_gameplay_durability::durability::mail::{MailPostOutcome, MailPostRefusal};
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
    let (
        Some(fence),
        Ok(MailIntent::Post { .. }),
        Some(ItemKey::Instance(item)),
        Some(ItemKey::MapBase { placement_key, .. }),
    ) = (admitted.item_fence, decode_intent(payload), item, mailbox)
    else {
        return Some(MailOutcome::Rejected);
    };
    if !mailbox_in_reach(owner, actor, admitted.game_session_id, placement_key).await {
        return Some(MailOutcome::Rejected);
    }
    owner
        .root
        .post_admitted_mail(
            owner.character,
            owner.holder,
            fence,
            command_id,
            item,
            placement_key,
        )
        .await
        .ok()
        .map(wire_post_outcome)
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

/// A carried letter or parcel moved onto a mailbox tile posts it, as Canary `Tile::addThing`
/// hands the thing to the tile's mailbox (`tile.cpp:1695`) and `Mailbox::sendItem` moves it into
/// the recipient's Inbox (`mailbox.cpp:74`). `None` for an item that is no unstamped letter or
/// parcel in the sender's custody: that move drops it on the tile as any other move does.
pub(super) async fn post_moved(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    fence: CurrentCharacterItemFence,
    command_id: u64,
    item: [u8; 16],
    mailbox: u64,
) -> Option<LoopMoveResult> {
    match owner
        .root
        .post_admitted_mail(
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
async fn mailbox_in_reach(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    placement_key: u64,
) -> bool {
    let Some((pos, ordinal)) = crate::map::view::placement_of(placement_key) else {
        return false;
    };
    let mut runtime = owner.runtime.lock().await;
    if runtime.player_control_facts(actor, session).is_err()
        || owner.states.spell_states.lock().await.is_dead(actor)
        || super::item_ref_admission::pinned_index(&runtime, owner.active_generation).is_none()
    {
        return false;
    }
    let Some(at) =
        super::bundle_map_position::bundle_map_position(&mut runtime, owner.movement_cells, actor)
    else {
        return false;
    };
    let Some(index) = super::item_ref_admission::pinned_index(&runtime, owner.active_generation)
    else {
        return false;
    };
    i64::from(at.at.floor) == i64::from(pos.floor)
        && (i64::from(at.at.x) - i64::from(pos.x)).abs() <= 1
        && (i64::from(at.at.y) - i64::from(pos.y)).abs() <= 1
        && mailbox_ordinals(at.map, index, pos).contains(&ordinal)
}

/// The placement key of the mailbox a move onto `pos` posts into: the topmost unhidden one,
/// as Canary `Tile::getMailbox` (`tile.cpp:298`) takes the last mailbox of the tile.
pub(super) fn mailbox_at(
    map: &crate::map::boot::BundleMap,
    index: &ItemDefinitionIndex,
    pos: TilePos,
) -> Option<u64> {
    let ordinal = mailbox_ordinals(map, index, pos).pop()?;
    crate::map::view::placement_key(pos, ordinal)
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
            MailPostRefusal::ReplayConflict => MailOutcome::Rejected,
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
