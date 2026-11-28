//! Map warnings are projections of committed firings, separate from raid step delivery.
use super::*;
use crate::durability::character_cyclopedia::CyclopediaRaidWarnings;
use oteryn_gameplay_encounter_rules::raid::{RaidArea, RaidCatalogue, SelectedRaid};
use oteryn_protocol_oteryn::cyclopedia::raid_warnings::{self as warnings_wire, Warning};
use oteryn_world_bundle::subarea::Table;
use std::collections::BTreeSet;
use std::sync::LazyLock;

const AREA_LEAD_MS: i64 = 60 * 60 * 1_000;
const SUBAREA_LEAD_MS: i64 = 15 * 60 * 1_000;
const GROUND_FLOOR: i8 = -7;
static CATALOGUE: LazyLock<Option<RaidCatalogue>> =
    LazyLock::new(|| RaidCatalogue::embedded().ok());

pub(super) async fn dispatch(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    id: u64,
    payload: &[u8],
) -> GuildReply {
    let selected = admitted.continuity.selected_capabilities;
    if payload != warnings_wire::QUERY
        || CommandId::new(id).is_err()
        || !selected.contains(wire::CAPABILITY_CYCLOPEDIA_MAP_V1)
        || !selected.contains(warnings_wire::CAPABILITY_CYCLOPEDIA_RAID_WARNINGS_V1)
    {
        return GuildReply::Rejected;
    }
    let Some(fence) = admitted.item_fence else {
        return GuildReply::Rejected;
    };
    {
        let runtime = owner.runtime.lock().await;
        if !super::super::battle_instinct_status::actor_is_current(&runtime, admitted) {
            return GuildReply::Rejected;
        }
    }
    let Some(map) = owner.bundle_map() else {
        return GuildReply::Rejected;
    };
    let Some(catalogue) = CATALOGUE.as_ref() else {
        return GuildReply::Unavailable;
    };
    let base = map.overlay.base();
    let now_ms = i64::try_from(oteryn_error_codes::unix_ms()).unwrap_or(i64::MAX);
    let snapshot = owner
        .root
        .read_cyclopedia_raid_warnings(owner.character, owner.holder, fence, base.digest(), now_ms)
        .await;
    let snapshot = match snapshot {
        Ok(Some(snapshot)) => snapshot,
        Ok(None) => return GuildReply::Rejected,
        Err(_) => return GuildReply::Unavailable,
    };
    let warnings = project(&base.subareas, base.digest(), catalogue, &snapshot, now_ms);
    // A handoff during the database observation cannot publish the old snapshot.
    {
        let runtime = owner.runtime.lock().await;
        if !super::super::battle_instinct_status::actor_is_current(&runtime, admitted) {
            return GuildReply::Rejected;
        }
    }
    warnings_wire::encode(&warnings).map_or(GuildReply::Rejected, GuildReply::Payload)
}

fn project(
    table: &Table,
    bundle: [u8; 32],
    catalogue: &RaidCatalogue,
    snapshot: &CyclopediaRaidWarnings,
    now_ms: i64,
) -> Vec<Warning> {
    let mut warnings = BTreeSet::new();
    for plan in &snapshot.plans {
        let Some(raid) = catalogue.get(&plan.key).filter(|raid| raid.active) else {
            continue;
        };
        let Ok(selected) = raid.select(bundle, plan.firing_id) else {
            continue;
        };
        warnings.extend(raid_warnings(
            table,
            &selected,
            plan.fired_at_ms,
            now_ms,
            |subarea| snapshot.subarea_discovered(subarea),
        ));
    }
    warnings.into_iter().collect()
}

fn raid_warnings(
    table: &Table,
    selected: &SelectedRaid,
    fired_at_ms: i64,
    now_ms: i64,
    discovered: impl Fn(u32) -> bool,
) -> BTreeSet<Warning> {
    let mut warnings = BTreeSet::new();
    let Some(delay) = selected
        .definition
        .waves
        .iter()
        .map(|wave| wave.delay_ms)
        .min()
    else {
        return warnings;
    };
    let Some(lead) = fired_at_ms
        .checked_add(i64::from(delay))
        .and_then(|start| start.checked_sub(now_ms))
        .filter(|lead| (1..=AREA_LEAD_MS).contains(lead))
    else {
        return warnings;
    };
    let mut subareas = BTreeSet::new();
    for area in selected
        .definition
        .waves
        .iter()
        .filter(|wave| wave.delay_ms == delay)
        .flat_map(|wave| &wave.entries)
        .filter_map(|entry| selected.areas.get(&entry.area_anchor))
    {
        subareas.extend(overlapping_subareas(table, area));
    }
    let ground: BTreeSet<_> = if lead <= SUBAREA_LEAD_MS && !subareas.is_empty() {
        let start = table.runs.partition_point(|run| run.floor < GROUND_FLOOR);
        table.runs[start..]
            .iter()
            .take_while(|run| run.floor == GROUND_FLOOR)
            .map(|run| run.subarea)
            .collect()
    } else {
        BTreeSet::new()
    };
    for subarea in subareas {
        let Some(area) = table
            .areas
            .binary_search_by_key(&subarea, |entry| entry.id)
            .ok()
            .and_then(|index| table.areas[index].parent)
        else {
            continue;
        };
        if !table
            .areas
            .iter()
            .filter(|entry| entry.parent == Some(area))
            .all(|entry| discovered(entry.id))
        {
            continue;
        }
        warnings.insert(Warning {
            area,
            subarea: None,
        });
        if ground.contains(&subarea) {
            warnings.insert(Warning {
                area,
                subarea: Some(subarea),
            });
        }
    }
    warnings
}

fn overlapping_subareas<'a>(
    table: &'a Table,
    area: &'a RaidArea,
) -> impl Iterator<Item = u32> + 'a {
    (area.from.floor..=area.to.floor).flat_map(move |floor| {
        let start = table.runs.partition_point(|run| {
            (i16::from(run.floor), i32::from(run.y)) < (-floor, area.from.y)
        });
        table.runs[start..]
            .iter()
            .take_while(move |run| i16::from(run.floor) == -floor && i32::from(run.y) <= area.to.y)
            .filter(|run| i32::from(run.begin) <= area.to.x && i32::from(run.end) > area.from.x)
            .map(|run| run.subarea)
    })
}

#[cfg(test)]
#[path = "raid_warnings_tests.rs"]
mod tests;
