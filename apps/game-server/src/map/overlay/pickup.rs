//! MAP-OVERLAY-1b: picking up a map-authored item (ADR-0021 §4.4; DUR-03 §39.3).
//!
//! - [`eligibility`] decides from the bundle's own tile record whether a top-level entry may be
//!   picked up: pickupable, not on a house tile, and with no action, unique, door, depot or
//!   teleport binding (a dropped zero-destination teleport included), no contents, and no
//!   `text`, `description` or `charges`. Everything else stays in place.
//! - [`hide_origin_at_freeze`] hides the origin when the MINT is frozen; the hide may be refused
//!   for the budget, and then the pickup is refused. [`settle_origin`] unhides it only on a
//!   proven non-commit.
//! - [`within_reach`] is checked before every TRANSFER of the minted item, a retried MINT that
//!   returns the existing item included.
//! - [`rehide_taken_origins`] re-hides, after a crash, every origin with a receipt for this
//!   Channel, digest and reset epoch; it is never refused for the budget (counted and alarmed).
//!
//! The durable MINT is [`crate::durability::map_item_mint`].

use super::{Admission, ChannelOverlay, OverlayError, TilePos};
use crate::durability::map_item_mint::{CommittedMapItemMint, MapItemPlacement};
use oteryn_world_bundle::bundle::placement_key;
use oteryn_world_bundle::sector::Tile;

/// The largest quantity an entry can mint: `GAMEITEM01-STACK-QUANTITY-MAX`.
const QUANTITY_MAX: u32 = 100;

/// Why an entry stays in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ineligible {
    /// The ordinal names no top-level entry of the tile.
    NotTopLevel,
    /// The entry's item is not pickupable.
    NotPickupable,
    House,
    Action,
    Unique,
    Door,
    Depot,
    Teleport,
    /// The bundle dropped the entry's zero-destination teleport (ADR-0021 §4.5).
    DroppedTeleport,
    /// The entry holds items one level deeper.
    Contents,
    Text,
    Description,
    Charges,
    /// The entry's count is zero or above the stack quantity maximum.
    Quantity,
}

/// An eligible origin: its placement and the whole quantity the MINT takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EligibleEntry {
    pub placement_key: u64,
    pub origin: TilePos,
    pub ordinal: u8,
    /// The compact palette index of the entry's item.
    pub palette: u32,
    pub quantity: u32,
}

/// Whether the top-level entry `ordinal` of the bundle `tile` at native `floor` may be picked
/// up. `pickupable` is the item definition's Content fact; `dropped_teleports` is the bundle
/// manifest's ascending list.
pub fn eligibility(
    tile: &Tile,
    floor: i8,
    ordinal: u8,
    pickupable: bool,
    dropped_teleports: &[u64],
) -> Result<EligibleEntry, Ineligible> {
    let at = tile
        .items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.depth == 0)
        .nth(usize::from(ordinal))
        .map(|(at, _)| at)
        .ok_or(Ineligible::NotTopLevel)?;
    let key = placement_key(floor, tile.x, tile.y, ordinal).ok_or(Ineligible::NotTopLevel)?;
    let item = &tile.items[at];
    let attrs = &item.attrs;
    let refused = [
        (!pickupable, Ineligible::NotPickupable),
        (tile.house != 0, Ineligible::House),
        (attrs.action.is_some(), Ineligible::Action),
        (attrs.unique.is_some(), Ineligible::Unique),
        (attrs.door.is_some(), Ineligible::Door),
        (attrs.depot.is_some(), Ineligible::Depot),
        (attrs.teleport.is_some(), Ineligible::Teleport),
        (
            dropped_teleports.binary_search(&key).is_ok(),
            Ineligible::DroppedTeleport,
        ),
        (
            tile.items.get(at + 1).is_some_and(|next| next.depth > 0),
            Ineligible::Contents,
        ),
        (attrs.text.is_some(), Ineligible::Text),
        (attrs.description.is_some(), Ineligible::Description),
        (attrs.charges.is_some(), Ineligible::Charges),
    ];
    if let Some((_, reason)) = refused.into_iter().find(|(refused, _)| *refused) {
        return Err(reason);
    }
    let quantity = attrs.count.map_or(1, u32::from);
    if !(1..=QUANTITY_MAX).contains(&quantity) {
        return Err(Ineligible::Quantity);
    }
    Ok(EligibleEntry {
        placement_key: key,
        origin: TilePos {
            x: tile.x,
            y: tile.y,
            floor,
        },
        ordinal,
        palette: item.palette,
        quantity,
    })
}

/// The origin tile and ordinal a `placement_key` names.
fn origin_of(key: u64) -> Option<(TilePos, u8)> {
    let placement = MapItemPlacement::of_key(key)?;
    Some((
        TilePos {
            x: placement.x,
            y: placement.y,
            floor: placement.floor,
        },
        placement.ordinal,
    ))
}

/// Whether a player at `player` reaches the item at `item`: the same floor and at most one tile
/// away in each direction.
pub fn within_reach(player: TilePos, item: TilePos) -> bool {
    player.floor == item.floor && player.x.abs_diff(item.x) <= 1 && player.y.abs_diff(item.y) <= 1
}

/// Whether a player at `player` reaches the minted item before its TRANSFER, for a fresh MINT
/// and a retried one that returned the existing item alike.
pub fn minted_within_reach(player: TilePos, minted: &CommittedMapItemMint) -> bool {
    let item = TilePos {
        x: minted.placement.x,
        y: minted.placement.y,
        floor: minted.placement.floor,
    };
    within_reach(player, item)
}

/// Hides the origin of `placement_key` when its MINT is frozen. Refused, and the pickup with
/// it, over the budget, for an origin already hidden (taken, or a pickup in flight) and for a
/// key that names no top-level base entry.
pub fn hide_origin_at_freeze(
    overlay: &mut ChannelOverlay,
    placement_key: u64,
) -> Result<(), OverlayError> {
    let (pos, ordinal) = origin_of(placement_key).ok_or(OverlayError::Ordinal)?;
    overlay.hide(pos, ordinal, Admission::Refusable)
}

/// What the MINT of a hidden origin resolved to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MintResolution<'a> {
    /// The MINT committed, or already had: the origin stays hidden.
    Committed(&'a CommittedMapItemMint),
    /// Reconciliation proved nothing committed for the CommandRef: the origin is shown again.
    ProvenNotCommitted,
    /// The outcome is not known yet: the origin stays hidden until it is.
    Unknown,
}

/// Settles the origin of `placement_key` once its MINT resolved: unhidden only on
/// [`MintResolution::ProvenNotCommitted`]. Returns whether the origin is still hidden.
pub fn settle_origin(
    overlay: &mut ChannelOverlay,
    placement_key: u64,
    resolution: MintResolution<'_>,
) -> Result<bool, OverlayError> {
    match resolution {
        MintResolution::Committed(_) | MintResolution::Unknown => Ok(true),
        MintResolution::ProvenNotCommitted => {
            let (pos, ordinal) = origin_of(placement_key).ok_or(OverlayError::Ordinal)?;
            overlay.unhide(pos, ordinal)?;
            Ok(false)
        }
    }
}

/// Why a rebuild re-hide failed closed: the first receipt's key refused, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RehideError {
    pub placement_key: u64,
    pub reason: OverlayError,
}

/// Re-hides after a crash every origin with a receipt for this Channel, digest and reset epoch
/// (`taken`, from [`crate::durability::DurabilityRoot::read_map_item_mint_placements`]). Each is
/// admitted whatever the budget; an overflow is counted and raises the alarm. An origin already
/// hidden stays hidden. A key that names no top-level base entry fails closed.
pub fn rehide_taken_origins(
    overlay: &mut ChannelOverlay,
    taken: &[u64],
) -> Result<(), RehideError> {
    for &placement_key in taken {
        let refused = |reason| RehideError {
            placement_key,
            reason,
        };
        let (pos, ordinal) = origin_of(placement_key).ok_or(refused(OverlayError::Ordinal))?;
        match overlay.hide(pos, ordinal, Admission::Durable) {
            Ok(()) | Err(OverlayError::AlreadyHidden) => {}
            Err(reason) => return Err(refused(reason)),
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod map_overlay_pickup_tests {
    use super::*;
    use oteryn_world_bundle::sector::{Attrs, Item};

    fn item(depth: u8, attrs: Attrs) -> Item {
        Item {
            palette: 1,
            depth,
            attrs,
        }
    }

    fn tile(house: u32, items: Vec<Item>) -> Tile {
        Tile {
            x: 100,
            y: 200,
            flags: 0,
            house,
            zones: Vec::new(),
            items,
        }
    }

    fn plain() -> Item {
        item(0, Attrs::default())
    }

    #[test]
    fn map_overlay_pickup_takes_an_eligible_entry_whole() {
        let counted = item(
            0,
            Attrs {
                count: Some(7),
                ..Attrs::default()
            },
        );
        let tile = tile(0, vec![plain(), counted, plain()]);
        let entry = eligibility(&tile, -7, 1, true, &[]).expect("eligible");
        assert_eq!(
            entry,
            EligibleEntry {
                placement_key: placement_key(-7, 100, 200, 1).expect("key"),
                origin: TilePos {
                    x: 100,
                    y: 200,
                    floor: -7
                },
                ordinal: 1,
                palette: 1,
                quantity: 7,
            }
        );
        assert_eq!(
            eligibility(&tile, -7, 0, true, &[]).map(|e| e.quantity),
            Ok(1)
        );
        assert_eq!(
            origin_of(entry.placement_key),
            Some((entry.origin, entry.ordinal))
        );
    }

    #[test]
    fn map_overlay_pickup_leaves_every_ineligible_kind_in_place() {
        let attrs = |change: fn(&mut Attrs)| {
            let mut attrs = Attrs::default();
            change(&mut attrs);
            vec![item(0, attrs)]
        };
        let cases: Vec<(Tile, bool, Ineligible)> = vec![
            (tile(0, vec![plain()]), false, Ineligible::NotPickupable),
            (tile(9, vec![plain()]), true, Ineligible::House),
            (
                tile(0, attrs(|a| a.action = Some(1000))),
                true,
                Ineligible::Action,
            ),
            (
                tile(0, attrs(|a| a.unique = Some(1000))),
                true,
                Ineligible::Unique,
            ),
            (tile(0, attrs(|a| a.door = Some(1))), true, Ineligible::Door),
            (
                tile(0, attrs(|a| a.depot = Some(1))),
                true,
                Ineligible::Depot,
            ),
            (
                tile(0, attrs(|a| a.teleport = Some((1, 2, 7)))),
                true,
                Ineligible::Teleport,
            ),
            (
                tile(0, vec![plain(), item(1, Attrs::default())]),
                true,
                Ineligible::Contents,
            ),
            (
                tile(0, attrs(|a| a.text = Some("x".into()))),
                true,
                Ineligible::Text,
            ),
            (
                tile(0, attrs(|a| a.description = Some("x".into()))),
                true,
                Ineligible::Description,
            ),
            (
                tile(0, attrs(|a| a.charges = Some(3))),
                true,
                Ineligible::Charges,
            ),
            (
                tile(0, attrs(|a| a.count = Some(0))),
                true,
                Ineligible::Quantity,
            ),
            (
                tile(0, attrs(|a| a.count = Some(101))),
                true,
                Ineligible::Quantity,
            ),
        ];
        for (tile, pickupable, reason) in cases {
            assert_eq!(
                eligibility(&tile, -7, 0, pickupable, &[]),
                Err(reason),
                "{reason:?}"
            );
            // The same entry without the one refused property is eligible.
            if reason != Ineligible::NotPickupable && reason != Ineligible::House {
                let plain_tile = Tile {
                    items: vec![plain()],
                    ..tile.clone()
                };
                assert!(eligibility(&plain_tile, -7, 0, true, &[]).is_ok());
            }
        }
        let key = placement_key(-7, 100, 200, 0).expect("key");
        assert_eq!(
            eligibility(&tile(0, vec![plain()]), -7, 0, true, &[key - 1, key]),
            Err(Ineligible::DroppedTeleport)
        );
        // A contained item is never a top-level ordinal, nor one past the last.
        let nested = tile(0, vec![plain(), item(1, Attrs::default())]);
        assert_eq!(
            eligibility(&nested, -7, 1, true, &[]),
            Err(Ineligible::NotTopLevel)
        );
        assert_eq!(
            eligibility(&nested, 1, 0, true, &[]),
            Err(Ineligible::NotTopLevel)
        );
    }

    #[test]
    fn map_overlay_pickup_reach_is_one_tile_on_the_same_floor() {
        let at = |x, y, floor| TilePos { x, y, floor };
        let item = at(100, 200, -7);
        for (dx, dy) in [(-1, -1), (0, 0), (1, 0), (1, 1), (0, -1)] {
            let player = at(
                100_u16.wrapping_add_signed(dx),
                200_u16.wrapping_add_signed(dy),
                -7,
            );
            assert!(within_reach(player, item));
        }
        for player in [at(102, 200, -7), at(100, 198, -7), at(100, 200, -6)] {
            assert!(!within_reach(player, item));
        }
        assert!(!within_reach(at(0, 0, 0), at(u16::MAX, 0, 0)));
    }
}
