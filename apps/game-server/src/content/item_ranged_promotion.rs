//! Distance, throwing, ammunition and wand weapon facts (RANGED-CONTENT-1, RANGED-0 §7), PR1:
//! the fields the artifact v7 `weapon` group already holds (`weapon_type`, `range`, `hit_chance`,
//! `max_hit_chance`, `ammunition`).
//!
//! The pinned facts packet `docs/agents/evidence/OTV2-20261013-ranged-weapon-facts-v1.json` is
//! produced by `tools/content-schema/item-authoring/lower_ranged_packet.py` from the pinned
//! TibiaWiki snapshot (first) and Canary `items.xml` (fallback). The other row fields
//! (`on_use`, `break_chance_pct`, `mana_per_shot`, wand damage and element, `missile`, requirements
//! and the quiver flag) have no v7 home; the PR2 artifact bump carries them, and this step only
//! decodes and counts them so the packet stays one strict shape.
//!
//! Rows target any Item record, materializable or not, as the wiki stats promotion does.
//!
//! A field already `Known` in the tree is never overwritten: equal is a no-op, a different value
//! is a counted conflict that keeps the tree value. A weapon group that is `NotApplicable` or in
//! `Conflict` is a counted conflict too. The step is strict and fail-closed: any drift in the
//! packet shape, the digest or a target is refused before the draft changes, and a count drift fails the run.
use super::{
    DefinitionIdentityDocument, ProjectReferenceRecord, ProjectV2Draft, ReferenceAmmoType,
    ReferenceCells, ReferenceItemField, ReferenceItemSemantics, ReferenceItemWeapon,
    ReferenceRationalPercent, ReferenceWeaponType, world_project_sha256,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

pub const ITEM_RANGED_PACKET: &[u8] =
    include_bytes!("../../../../docs/agents/evidence/OTV2-20261013-ranged-weapon-facts-v1.json");
pub const ITEM_RANGED_PACKET_SHA256: &str =
    "30bf0680ed814db651448652976d4ae3e6126240e77d40c46b9497108c317a0d";
pub const ITEM_RANGED_WEAPON_ROWS: usize = 90;
pub const ITEM_RANGED_QUIVER_ROWS: usize = 8;
/// What the pinned packet does to the consumables-admitted tree (checked on every run).
pub const ITEM_RANGED_FIELDS_SET: usize = 24;
pub const ITEM_RANGED_FIELDS_UNCHANGED: usize = 183;
pub const ITEM_RANGED_CONFLICTS: usize = 1;
const SCHEMA: &str = "OTERYN_ITEM_RANGED_PROMOTION/v1";

#[derive(Deserialize)]
struct Packet {
    schema: String,
    counts: Counts,
    promotions: Vec<Row>,
}

#[derive(Deserialize)]
struct Counts {
    items: usize,
    quivers: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Kind {
    Distance,
    Throwing,
    Ammunition,
    Wand,
}

#[derive(Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Ammo {
    Arrow,
    Bolt,
}

// The per-field source, parity and requirement records are evidence; the PR2 artifact owns them.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct Row {
    item_key: String,
    weapon_kind: Option<Kind>,
    ammo_type: Option<Ammo>,
    range_tiles: Option<u16>,
    hit_chance: Option<i8>,
    max_hit_chance: Option<u8>,
    break_chance_pct: Option<u8>,
    on_use: Option<String>,
    mana_per_shot: Option<u16>,
    damage_min: Option<u16>,
    damage_max: Option<u16>,
    element: Option<String>,
    missile: Option<String>,
    quiver: Option<bool>,
    container_size: Option<u8>,
    requirements: Option<serde_json::Value>,
    sources: BTreeMap<String, String>,
    wiki_pages: Option<Vec<u64>>,
    parity_pending: Option<Vec<String>>,
}

/// What one run did to the tree.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Applied {
    pub fields_set: usize,
    pub fields_unchanged: usize,
    pub conflicts: usize,
    pub quivers_deferred: usize,
}

pub fn apply_item_ranged_promotion_v1(draft: &mut ProjectV2Draft) -> Result<Applied, String> {
    if world_project_sha256(ITEM_RANGED_PACKET) != ITEM_RANGED_PACKET_SHA256 {
        return Err("item ranged packet digest drift".into());
    }
    let items = draft
        .core
        .records
        .iter_mut()
        .filter_map(|record| match record {
            ProjectReferenceRecord::Item {
                identity,
                semantics,
                ..
            } => Some((&*identity, semantics)),
            _ => None,
        });
    let applied = apply_rows(items, ITEM_RANGED_PACKET)?;
    if (
        applied.fields_set,
        applied.fields_unchanged,
        applied.conflicts,
    ) != (
        ITEM_RANGED_FIELDS_SET,
        ITEM_RANGED_FIELDS_UNCHANGED,
        ITEM_RANGED_CONFLICTS,
    ) || applied.quivers_deferred != ITEM_RANGED_QUIVER_ROWS
    {
        return Err(format!("item ranged promotion count drift: {applied:?}"));
    }
    Ok(applied)
}

fn weapon_type(kind: Kind) -> ReferenceWeaponType {
    match kind {
        Kind::Distance | Kind::Throwing => ReferenceWeaponType::Distance,
        Kind::Ammunition => ReferenceWeaponType::Ammunition,
        Kind::Wand => ReferenceWeaponType::Wand,
    }
}

/// The tree holds a hit chance as a reduced fraction of one (6 points is 3/50), like the
/// relative hit promotion.
fn percent(points: i16) -> Result<ReferenceRationalPercent, String> {
    let (mut a, mut b) = (u64::from(points.unsigned_abs()), 100_u64);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    ReferenceRationalPercent::new(i64::from(points) / a as i64, 100 / a)
        .map_err(|error| error.to_string())
}

/// Wiki-and-Canary values the row proposes for the v7 `weapon` fields.
struct Proposal {
    weapon_type: ReferenceWeaponType,
    range: Option<ReferenceCells>,
    hit_chance: Option<ReferenceRationalPercent>,
    max_hit_chance: Option<ReferenceRationalPercent>,
    ammunition: Option<ReferenceAmmoType>,
}

fn proposal(row: &Row, kind: Kind) -> Result<Proposal, String> {
    match (kind, row.ammo_type) {
        (Kind::Distance | Kind::Ammunition, None) => {
            return Err(format!(
                "{}: ammunition kind without ammo_type",
                row.item_key
            ));
        }
        (Kind::Throwing | Kind::Wand, Some(_)) => {
            return Err(format!(
                "{}: ammo_type on a throwing weapon or wand",
                row.item_key
            ));
        }
        _ => {}
    }
    if row
        .range_tiles
        .is_some_and(|tiles| !(1..=7).contains(&tiles))
        || row.hit_chance.is_some_and(|points| points < -100)
        || row.max_hit_chance.is_some_and(|points| points > 100)
    {
        return Err(format!("{}: field out of range", row.item_key));
    }
    Ok(Proposal {
        weapon_type: weapon_type(kind),
        range: row.range_tiles.map(ReferenceCells),
        hit_chance: row.hit_chance.map(|p| percent(i16::from(p))).transpose()?,
        max_hit_chance: row
            .max_hit_chance
            .map(|p| percent(i16::from(p)))
            .transpose()?,
        ammunition: row.ammo_type.map(|ammo| match ammo {
            Ammo::Arrow => ReferenceAmmoType::Arrow,
            Ammo::Bolt => ReferenceAmmoType::Bolt,
        }),
    })
}

fn merge<T: PartialEq>(field: &mut ReferenceItemField<T>, value: Option<T>, applied: &mut Applied) {
    let Some(value) = value else {
        return;
    };
    match field {
        ReferenceItemField::Unknown => {
            *field = ReferenceItemField::Known(value);
            applied.fields_set += 1;
        }
        ReferenceItemField::Known(old) if *old == value => applied.fields_unchanged += 1,
        _ => applied.conflicts += 1,
    }
}

fn empty_weapon() -> ReferenceItemWeapon {
    ReferenceItemWeapon {
        weapon_type: ReferenceItemField::Unknown,
        attack: ReferenceItemField::Unknown,
        defense: ReferenceItemField::Unknown,
        extra_defense: ReferenceItemField::Unknown,
        range: ReferenceItemField::Unknown,
        hit_chance: ReferenceItemField::Unknown,
        max_hit_chance: ReferenceItemField::Unknown,
        ammunition: ReferenceItemField::Unknown,
        elemental: ReferenceItemField::Unknown,
    }
}

fn apply_rows<'a>(
    items: impl Iterator<
        Item = (
            &'a DefinitionIdentityDocument,
            &'a mut ReferenceItemSemantics,
        ),
    >,
    bytes: &[u8],
) -> Result<Applied, String> {
    let packet: Packet = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    if packet.schema != SCHEMA {
        return Err("item ranged packet schema".into());
    }
    let mut rows = BTreeMap::new();
    for row in &packet.promotions {
        if rows.insert(row.item_key.as_str(), row).is_some() {
            return Err(format!("{}: duplicate ranged row", row.item_key));
        }
    }
    let quivers = packet
        .promotions
        .iter()
        .filter(|r| r.quiver == Some(true))
        .count();
    if packet.counts.items != packet.promotions.len()
        || packet.counts.quivers != quivers
        || packet
            .promotions
            .iter()
            .any(|r| (r.weapon_kind.is_some()) == (r.quiver == Some(true)))
    {
        return Err("item ranged packet shape or count drift".into());
    }
    // Validate every row against its target before mutating: a drift leaves the draft untouched.
    let mut targets = Vec::new();
    let mut seen = BTreeSet::new();
    for (identity, semantics) in items {
        let Some(row) = rows.get(identity.key.as_str()) else {
            continue;
        };
        if identity.family != "Item" || identity.revision != "definition-r1" {
            return Err(format!("{}: full identity drift", identity.key));
        }
        seen.insert(identity.key.as_str());
        targets.push((*row, semantics));
    }
    if let Some(missing) = rows.keys().find(|key| !seen.contains(*key)) {
        return Err(format!("{missing}: no Item record with this key"));
    }
    let mut applied = Applied::default();
    let mut staged = Vec::new();
    for (row, semantics) in targets {
        let Some(kind) = row.weapon_kind else {
            applied.quivers_deferred += 1;
            continue;
        };
        staged.push((proposal(row, kind)?, semantics));
    }
    for (proposal, semantics) in staged {
        if matches!(
            semantics.weapon,
            ReferenceItemField::NotApplicable | ReferenceItemField::Conflict
        ) {
            applied.conflicts += 1;
            continue;
        }
        if semantics.weapon.is_unknown() {
            semantics.weapon = ReferenceItemField::Known(empty_weapon());
        }
        let ReferenceItemField::Known(weapon) = &mut semantics.weapon else {
            return Err("weapon group state".into());
        };
        merge(
            &mut weapon.weapon_type,
            Some(proposal.weapon_type),
            &mut applied,
        );
        merge(&mut weapon.range, proposal.range, &mut applied);
        merge(&mut weapon.hit_chance, proposal.hit_chance, &mut applied);
        merge(
            &mut weapon.max_hit_chance,
            proposal.max_hit_chance,
            &mut applied,
        );
        merge(&mut weapon.ammunition, proposal.ammunition, &mut applied);
    }
    Ok(applied)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn pinned_packet_digest_and_counts() {
        assert_eq!(
            world_project_sha256(ITEM_RANGED_PACKET),
            ITEM_RANGED_PACKET_SHA256
        );
        let packet: Packet =
            serde_json::from_slice(ITEM_RANGED_PACKET).expect("pinned packet decodes");
        let quivers = packet
            .promotions
            .iter()
            .filter(|r| r.quiver == Some(true))
            .count();
        assert_eq!(quivers, ITEM_RANGED_QUIVER_ROWS);
        assert_eq!(packet.promotions.len() - quivers, ITEM_RANGED_WEAPON_ROWS);
    }

    fn item(key: &str) -> DefinitionIdentityDocument {
        DefinitionIdentityDocument {
            family: "Item".to_owned(),
            key: key.to_owned(),
            revision: "definition-r1".to_owned(),
        }
    }

    const KEY: &str = "oteryn:item.tibia.i3350";

    fn packet(rows: &str, items: usize, quivers: usize) -> Vec<u8> {
        format!(
            r#"{{"schema":"{SCHEMA}","counts":{{"items":{items},"quivers":{quivers}}},"promotions":[{rows}]}}"#
        )
        .into_bytes()
    }

    fn bow_row() -> String {
        format!(
            r#"{{"item_key":"{KEY}","weapon_kind":"DISTANCE","ammo_type":"ARROW","range_tiles":6,"hit_chance":6,"max_hit_chance":90,"on_use":"NONE","sources":{{"range_tiles":"CANARY"}}}}"#
        )
    }

    #[test]
    fn applies_merges_and_fails_closed() {
        let identity = item(KEY);
        let mut semantics = ReferenceItemSemantics::default();
        let bytes = packet(&bow_row(), 1, 0);
        let applied =
            apply_rows(std::iter::once((&identity, &mut semantics)), &bytes).expect("applies");
        assert_eq!(applied.fields_set, 5);
        let ReferenceItemField::Known(weapon) = &semantics.weapon else {
            panic!("weapon group");
        };
        assert_eq!(
            weapon.weapon_type,
            ReferenceItemField::Known(ReferenceWeaponType::Distance)
        );
        assert_eq!(weapon.range, ReferenceItemField::Known(ReferenceCells(6)));
        assert_eq!(
            weapon.hit_chance,
            ReferenceItemField::Known(ReferenceRationalPercent::new(3, 50).expect("3/50"))
        );
        assert_eq!(
            weapon.max_hit_chance,
            ReferenceItemField::Known(ReferenceRationalPercent::new(9, 10).expect("9/10"))
        );
        // A second run changes nothing; a different tree value is a counted conflict.
        let again =
            apply_rows(std::iter::once((&identity, &mut semantics)), &bytes).expect("applies");
        assert_eq!((again.fields_set, again.fields_unchanged), (0, 5));
        let ReferenceItemField::Known(weapon) = &mut semantics.weapon else {
            panic!("weapon group");
        };
        weapon.range = ReferenceItemField::Known(ReferenceCells(5));
        let conflicted =
            apply_rows(std::iter::once((&identity, &mut semantics)), &bytes).expect("applies");
        assert_eq!(conflicted.conflicts, 1);
        let ReferenceItemField::Known(weapon) = &semantics.weapon else {
            panic!("weapon group");
        };
        assert_eq!(weapon.range, ReferenceItemField::Known(ReferenceCells(5)));
        // A drifted identity revision, a row with no record, a count drift and a bad range are refused.
        let mut fresh = ReferenceItemSemantics::default();
        let mut drifted = item(KEY);
        drifted.revision = "definition-r2".to_owned();
        assert!(apply_rows(std::iter::once((&drifted, &mut fresh)), &bytes).is_err());
        assert!(apply_rows(std::iter::empty(), &bytes).is_err());
        assert!(
            apply_rows(
                std::iter::once((&identity, &mut fresh)),
                &packet(&bow_row(), 2, 0)
            )
            .is_err()
        );
        let bad = bow_row().replace("\"range_tiles\":6", "\"range_tiles\":9");
        assert!(
            apply_rows(
                std::iter::once((&identity, &mut fresh)),
                &packet(&bad, 1, 0)
            )
            .is_err()
        );
        assert!(fresh.weapon.is_unknown());
    }

    #[test]
    fn quiver_rows_are_deferred() {
        let identity = item(KEY);
        let mut semantics = ReferenceItemSemantics::default();
        let row =
            format!(r#"{{"item_key":"{KEY}","quiver":true,"sources":{{"quiver":"CANARY"}}}}"#);
        let applied = apply_rows(
            std::iter::once((&identity, &mut semantics)),
            &packet(&row, 1, 1),
        )
        .expect("applies");
        assert_eq!(applied.quivers_deferred, 1);
        assert!(semantics.weapon.is_unknown());
    }
}
