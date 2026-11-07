//! Pure native item identities and source-removal plans; no commit authority.

use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NativeItemRef {
    pub key: String,
    pub revision: String,
}

impl NativeItemRef {
    /// Recognize the two explicit Item identity forms; no identity is allocated.
    pub fn numeric_id(&self) -> Result<u32, Error> {
        let suffix = self
            .key
            .strip_prefix("candidate:item/")
            .or_else(|| self.key.strip_prefix("oteryn:item.tibia.i"))
            .ok_or(Error::InvalidReference)?;
        if suffix.is_empty()
            || !suffix.bytes().all(|c| c.is_ascii_digit())
            || suffix.starts_with('0')
            || self.revision.is_empty()
        {
            return Err(Error::InvalidReference);
        }
        suffix
            .parse::<u32>()
            .ok()
            .filter(|id| *id > 0)
            .ok_or(Error::InvalidReference)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectPosition {
    Caster,
    TargetTile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemPresentation {
    pub asset_binding: String,
    pub position: EffectPosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemRefusal {
    MissingTarget,
    CreatureTarget,
    ImmovableItem,
    ProtectionZone,
    NoListedField,
    MissingTile,
    FloorChange,
    CreatureOnTile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    InvalidParameter(&'static str),
    InvalidReference,
    RandomOutOfRange,
    Refused {
        reason: ItemRefusal,
        message: Option<String>,
        presentation: Option<ItemPresentation>,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldItemFacts {
    pub instance_key: String,
    pub item: NativeItemRef,
    pub movable: bool,
    pub script_tagged: bool,
    pub action_tagged: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectedItemSource {
    TileTopItem,
    ContainerSlot,
    EquipmentSlot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemOperation {
    MimicItem {
        item: NativeItemRef,
        duration_ms: u32,
        selected_from: SelectedItemSource,
    },
    Disintegrate {
        remove_instances: Vec<String>,
        visited_items: u32,
    },
    RemoveField {
        remove_instance: String,
    },
}

pub fn validate_world_item(item: &WorldItemFacts) -> Result<(), Error> {
    item.item.numeric_id()?;
    if item.instance_key.is_empty() {
        Err(Error::InvalidParameter("instance_key"))
    } else {
        Ok(())
    }
}

/// Current Item owner witnesses in source x-major/y-minor iteration order.
/// `top_visible_item=None` includes a creature/empty top; never infer it from tile_items.
#[derive(Debug, Clone)]
pub struct SourceRemovalTile {
    pub offset: (i8, i8),
    pub tile_exists: bool,
    pub tile_items: Vec<WorldItemFacts>,
    pub top_visible_item: Option<WorldItemFacts>,
}
/// Source selection only. Output reuses the existing exact-instance durable removal operation;
/// caller still needs current transaction/custody authority and retained commit/replay.
pub fn plan_source_item_removals(
    selection: crate::content::ProjectV2RemoveItemsSelection,
    listed: &[NativeItemRef],
    tiles: &[SourceRemovalTile],
) -> Result<Vec<(usize, ItemOperation)>, Error> {
    use crate::content::ProjectV2RemoveItemsSelection as S;
    let expected: &[u32] = match selection {
        S::FirstListedPerTile => &[2130, 2129],
        S::TopItemFirstTile => &[10181, 2128, 10182, 2130],
    };
    if listed.len() != expected.len()
        || listed
            .iter()
            .zip(expected)
            .any(|(r, id)| r.numeric_id() != Ok(*id))
    {
        return Err(Error::InvalidParameter("source removal list"));
    }
    let radius: i8 = if selection == S::FirstListedPerTile {
        1
    } else {
        2
    };
    let side =
        usize::try_from(radius * 2 + 1).map_err(|_| Error::InvalidParameter("source radius"))?;
    if tiles.len() != side * side {
        return Err(Error::InvalidParameter("source tile closure"));
    }
    // Qualify the whole bounded closure before any caller can begin physical writes.
    let mut identities = BTreeSet::new();
    for (index, tile) in tiles.iter().enumerate() {
        let expected_offset = (
            i8::try_from(index / side).map_err(|_| Error::InvalidParameter("offset"))? - radius,
            i8::try_from(index % side).map_err(|_| Error::InvalidParameter("offset"))? - radius,
        );
        if tile.offset != expected_offset
            || (!tile.tile_exists
                && (!tile.tile_items.is_empty() || tile.top_visible_item.is_some()))
        {
            return Err(Error::InvalidParameter("source ordered tile witness"));
        }
        for item in &tile.tile_items {
            validate_world_item(item)?;
            if !identities.insert(item.instance_key.clone()) {
                return Err(Error::InvalidParameter("duplicate source item identity"));
            }
        }
        if let Some(top) = &tile.top_visible_item {
            validate_world_item(top)?;
            if !tile.tile_items.contains(top) {
                return Err(Error::InvalidParameter("top visible witness mismatch"));
            }
        }
    }
    let mut result = Vec::new();
    for (index, tile) in tiles.iter().enumerate() {
        if !tile.tile_exists {
            continue;
        }
        let chosen = match selection {
            S::FirstListedPerTile => listed
                .iter()
                .find_map(|wanted| tile.tile_items.iter().find(|i| &i.item == wanted)),
            S::TopItemFirstTile => tile
                .top_visible_item
                .as_ref()
                .filter(|top| listed.contains(&top.item)),
        };
        if let Some(item) = chosen {
            result.push((
                index,
                ItemOperation::RemoveField {
                    remove_instance: item.instance_key.clone(),
                },
            ));
            if selection == S::TopItemFirstTile {
                break;
            }
        }
    }
    Ok(result)
}
#[cfg(test)]
mod source_removal_tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use crate::content::ProjectV2RemoveItemsSelection as S;
    fn item(id: u32, key: &str) -> WorldItemFacts {
        WorldItemFacts {
            instance_key: key.into(),
            item: NativeItemRef {
                key: format!("oteryn:item.tibia.i{id}"),
                revision: "definition-r1".into(),
            },
            movable: false,
            script_tagged: false,
            action_tagged: false,
        }
    }
    fn tiles(radius: i8) -> Vec<SourceRemovalTile> {
        (-radius..=radius)
            .flat_map(|x| {
                (-radius..=radius).map(move |y| SourceRemovalTile {
                    offset: (x, y),
                    tile_exists: true,
                    tile_items: vec![],
                    top_visible_item: None,
                })
            })
            .collect()
    }
    #[test]
    fn source_anomaly_prioritizes_id2130_over_earlier2129_and_removes_one_per_tile() {
        let listed = vec![item(2130, "a").item, item(2129, "b").item];
        let mut t = tiles(1);
        t[0].tile_items = vec![item(2129, "old-first"), item(2130, "priority")];
        t[1].tile_items = vec![item(2129, "next")];
        let p = plan_source_item_removals(S::FirstListedPerTile, &listed, &t)
            .expect("qualified source removal fixture");
        assert_eq!(
            p,
            vec![
                (
                    0,
                    ItemOperation::RemoveField {
                        remove_instance: "priority".into()
                    }
                ),
                (
                    1,
                    ItemOperation::RemoveField {
                        remove_instance: "next".into()
                    }
                )
            ]
        );
    }
    #[test]
    fn source_destroy_uses_real_top_visible_and_stops_after_first_source_order_tile() {
        let listed = [10181, 2128, 10182, 2130].map(|id| item(id, "list").item);
        let mut t = tiles(2);
        t[0].tile_items = vec![item(2130, "under-creature")]; // None top: current visible creature cannot be an Item.
        t[1].tile_items = vec![item(2128, "first")];
        t[1].top_visible_item = Some(t[1].tile_items[0].clone());
        t[2].tile_items = vec![item(2130, "later")];
        t[2].top_visible_item = Some(t[2].tile_items[0].clone());
        assert_eq!(
            plan_source_item_removals(S::TopItemFirstTile, &listed, &t)
                .expect("qualified source removal fixture"),
            vec![(
                1,
                ItemOperation::RemoveField {
                    remove_instance: "first".into()
                }
            )]
        );
        let mut invalid = t.clone();
        invalid[24].top_visible_item = Some(item(2130, "forged"));
        assert!(plan_source_item_removals(S::TopItemFirstTile, &listed, &invalid).is_err()); // all preflight, even after chosen first
        let mut reversed = t;
        reversed.swap(0, 1);
        assert!(plan_source_item_removals(S::TopItemFirstTile, &listed, &reversed).is_err());
    }
}
