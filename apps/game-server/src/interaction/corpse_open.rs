//! ITEM-MOVE-WIRE-0 §4.3: opening a corpse with `USE` and closing it again.
//!
//! Opening is a non-durable view action: it opens domain 11 for the corpse and writes nothing. It
//! is not a GAME-INTERACTION transition and has no occurrence of its own. Anyone within reach may
//! open a corpse; D133 and D134 are enforced when an item is taken (ITEM-MOVE-1), on the database
//! clock.
//!
//! **D133 disclosure (`PARITY_PENDING`).** During the D133 exclusivity window a non-owner in reach
//! can see a corpse's contents but cannot take them. Tibia refuses to open it; Oteryn does not gate
//! opening, because a gate on the runtime clock would disagree with the writer's database clock.

use crate::gameplay_transport::world_spatial::ActorPosition;

/// Reach (§4.3): same floor, Chebyshev distance at most 1.
pub(crate) fn within_reach(actor: ActorPosition, corpse: ActorPosition) -> bool {
    actor.floor == corpse.floor
        && (i64::from(actor.x) - i64::from(corpse.x)).abs() <= 1
        && (i64::from(actor.y) - i64::from(corpse.y)).abs() <= 1
}

/// What the item target of one `USE` names, as the Channel owner sees it now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UseItemTarget<C> {
    /// The item is gone or no longer visible to this session.
    Gone,
    /// A visible item that is not a corpse: only a corpse opens in this slice.
    NotACorpse,
    /// A corpse at `position` with its contents.
    Corpse {
        position: ActorPosition,
        contents: C,
    },
}

/// The decision of one `USE` with an item target (§4.3): `COMMITTED` opens the corpse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OpenDecision {
    Open,
    TooFar,
    StaleState,
    NothingToUse,
}

pub(crate) fn decide_open<C>(actor: ActorPosition, target: &UseItemTarget<C>) -> OpenDecision {
    match target {
        UseItemTarget::Gone => OpenDecision::StaleState,
        UseItemTarget::NotACorpse => OpenDecision::NothingToUse,
        UseItemTarget::Corpse { position, .. } if within_reach(actor, *position) => {
            OpenDecision::Open
        }
        UseItemTarget::Corpse { .. } => OpenDecision::TooFar,
    }
}

/// An event that may close the open corpse (§4.3). Opening another corpse closes the first as part
/// of the open itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CloseTrigger<K> {
    /// The character walked to `to`: the corpse closes when it is no longer within reach, which
    /// includes a change of floor.
    Moved {
        to: ActorPosition,
    },
    Teleported,
    Died,
    LoggedOut,
    ChannelTransfer,
    /// The corpse `corpse` decayed.
    Decayed {
        corpse: K,
    },
}

/// Whether `trigger` closes the corpse `open` at `position`.
pub(crate) fn closes<K: PartialEq>(
    open: &K,
    position: ActorPosition,
    trigger: &CloseTrigger<K>,
) -> bool {
    match trigger {
        CloseTrigger::Moved { to } => !within_reach(*to, position),
        CloseTrigger::Teleported
        | CloseTrigger::Died
        | CloseTrigger::LoggedOut
        | CloseTrigger::ChannelTransfer => true,
        CloseTrigger::Decayed { corpse } => corpse == open,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn at(x: i32, y: i32, floor: i16) -> ActorPosition {
        ActorPosition { x, y, floor }
    }

    #[test]
    fn a_corpse_opens_within_chebyshev_one_on_the_same_floor_only() {
        let corpse = at(10, 10, 7);
        let target = UseItemTarget::Corpse {
            position: corpse,
            contents: (),
        };
        for (dx, dy) in [(0, 0), (1, 1), (-1, 0), (1, -1), (0, 1)] {
            assert_eq!(
                decide_open(at(10 + dx, 10 + dy, 7), &target),
                OpenDecision::Open
            );
        }
        for actor in [at(12, 10, 7), at(10, 8, 7), at(10, 10, 6), at(11, 11, 8)] {
            assert_eq!(decide_open(actor, &target), OpenDecision::TooFar);
        }
        assert_eq!(
            decide_open::<()>(at(10, 10, 7), &UseItemTarget::Gone),
            OpenDecision::StaleState
        );
        assert_eq!(
            decide_open::<()>(at(10, 10, 7), &UseItemTarget::NotACorpse),
            OpenDecision::NothingToUse
        );
        // Far coordinates never overflow the distance.
        assert!(!within_reach(at(i32::MIN, 0, 7), at(i32::MAX, 0, 7)));
    }

    #[test]
    fn every_closing_trigger_closes_the_open_corpse() {
        let corpse = at(10, 10, 7);
        let open = 1_u8;
        // Leaving reach and changing floor close it; a step that stays in reach does not.
        assert!(closes(
            &open,
            corpse,
            &CloseTrigger::Moved { to: at(12, 10, 7) }
        ));
        assert!(closes(
            &open,
            corpse,
            &CloseTrigger::Moved { to: at(10, 10, 6) }
        ));
        assert!(!closes(
            &open,
            corpse,
            &CloseTrigger::Moved { to: at(11, 9, 7) }
        ));
        for trigger in [
            CloseTrigger::Teleported,
            CloseTrigger::Died,
            CloseTrigger::LoggedOut,
            CloseTrigger::ChannelTransfer,
            CloseTrigger::Decayed { corpse: open },
        ] {
            assert!(closes(&open, corpse, &trigger), "{trigger:?}");
        }
        // Another corpse decaying leaves this one open.
        assert!(!closes(&open, corpse, &CloseTrigger::Decayed { corpse: 2 }));
    }
}
