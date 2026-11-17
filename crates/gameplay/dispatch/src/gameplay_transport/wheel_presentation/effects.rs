//! Text is generated from the existing executed typed projection, never donor prose/bonus tables.
use super::*;
use oteryn_gameplay_progression::{
    wheel::WheelStages,
    wheel_effects::{WheelRuntimeEffects, derive_runtime_effects},
};
fn number(v: u64) -> String {
    let mut s = format!("{}", v / 1000);
    if !v.is_multiple_of(1000) {
        s.push_str(format!(".{:03}", v % 1000).trim_end_matches('0'));
    }
    s
}
fn unit(data: &WheelGemData, vocation: &str, key: &str, kind: &str) -> Option<String> {
    let family =
        &data.wheel()["vocations"][oteryn_gameplay_progression::wheel::base_vocation(vocation)?];
    let perk = family["revelations"]
        .as_array()?
        .iter()
        .find(|r| r["key"].as_str() == Some(key))?;
    for stage in perk["stages"].as_array()? {
        if let Some(e) = stage["numeric_effects"]
            .as_array()?
            .iter()
            .find(|e| e["kind"].as_str() == Some(kind))
        {
            return Some(e["unit"].as_str()?.replace('_', " "));
        }
    }
    None
}
pub(super) fn summary(
    data: &WheelGemData,
    vocation: &str,
    e: &WheelRuntimeEffects,
) -> Option<Vec<String>> {
    let mut out = Vec::new();
    for (name, value, unit) in [
        ("Maximum health", u64::from(e.max_health) * 1000, ""),
        ("Maximum mana", u64::from(e.max_mana) * 1000, ""),
        ("Capacity", u64::from(e.capacity) * 1000, ""),
        (
            "Mitigation multiplier",
            u64::from(e.mitigation_milli_percent),
            "percentage points",
        ),
        (
            "Life leech",
            u64::from(e.life_leech_hundredths_percent) * 10,
            "percentage points",
        ),
        (
            "Mana leech",
            u64::from(e.mana_leech_hundredths_percent) * 10,
            "percentage points",
        ),
    ] {
        if value != 0 {
            out.push(format!("{name}: +{} {unit}", number(value)).trim().into());
        }
    }
    for (key, value) in &e.skill_bonuses {
        out.push(format!("{}: +{value}", key.replace('_', " ")));
    }
    for key in &e.active_unique_perks {
        out.push(format!(
            "Conditional perk available: {}",
            key.replace('_', " ")
        ));
    }
    if e.sockets.gem_vessel_nodes > 0 {
        out.push(format!(
            "Gem Vessel nodes: {} (+{} damage and healing)",
            e.sockets.gem_vessel_nodes,
            e.sockets.gem_vessel_nodes.saturating_mul(
                oteryn_gameplay_progression::atelier::vessels::GEM_VESSEL_FLAT_DAMAGE_HEALING
            )
        ));
    }
    for (key, stage) in &e.revelation_stages {
        out.push(format!(
            "{}: revelation stage {stage}",
            key.replace('_', " ")
        ));
    }
    for (key, values) in &e.revelation_numeric_effects {
        for (kind, value) in values {
            out.push(format!(
                "{} · {}: {} {}",
                key.replace('_', " "),
                kind.replace('_', " "),
                number(value.milli()),
                unit(data, vocation, key, kind)?
            ));
        }
    }
    for (id, grades) in &e.sockets.effective_grades {
        out.push(format!(
            "Socketed gem {id:032X}: {} active positions, effective grades {}",
            grades.len(),
            grades
                .iter()
                .map(|g| (u16::from(*g) + 1).to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    Some(out)
}
#[derive(Clone, Copy)]
pub(super) struct Facts<'a> {
    pub vocation: &'a str,
    pub level: u32,
    pub promoted: bool,
    pub premium: bool,
}
pub(super) fn configured(
    data: &WheelGemData,
    rules: &WheelRuleset,
    allocation: &WheelAllocation,
    facts: Facts<'_>,
    atelier: Option<&oteryn_gameplay_progression::atelier::AtelierState>,
) -> Option<Vec<String>> {
    let Facts {
        vocation,
        level,
        promoted,
        premium,
    } = facts;
    let mut e =
        derive_runtime_effects(data.wheel(), allocation, vocation, level, promoted, premium)?;
    if allocation.current
        && oteryn_gameplay_progression::wheel::eligibility(
            rules, vocation, level, promoted, premium,
        )
        .is_ok()
        && let Some(atelier) = atelier
    {
        e.contribute_gems(atelier, data, allocation)?;
    }
    let mut out = summary(data, vocation, &e)?;
    if allocation.current
        && oteryn_gameplay_progression::wheel::eligibility(
            rules, vocation, level, promoted, premium,
        )
        .is_ok()
    {
        let stages = WheelStages::derive(rules, allocation, vocation, level, promoted);
        for (key, targets) in &rules.vocation(vocation)?.augment_targets {
            let stage = stages.augment_stage(key);
            if stage > 0 {
                out.push(format!(
                    "{}: augment stage {stage}; {}",
                    key.replace('_', " "),
                    targets.join(", ")
                ));
            }
        }
    }
    Some(out)
}
pub(super) fn node_lines(
    data: &WheelGemData,
    rules: &WheelRuleset,
    vocation: &str,
    index: usize,
    points: u16,
) -> Option<Vec<String>> {
    let mut p = [0; 36];
    *p.get_mut(index)? = points;
    let allocation = WheelAllocation {
        wheel_ruleset_revision: Some(data.revision().into()),
        wheel_revision: 0,
        slots: WheelSlots::new(p).ok()?,
        current: true,
    };
    configured(
        data,
        rules,
        &allocation,
        Facts {
            vocation,
            level: u32::MAX,
            promoted: true,
            premium: true,
        },
        None,
    )
}
pub(super) fn revelation_lines(
    data: &WheelGemData,
    rules: &WheelRuleset,
    vocation: &str,
    domain: oteryn_gameplay_progression::wheel::WheelDomain,
    points: u32,
    key: &str,
) -> Option<Vec<String>> {
    let mut p = [0; 36];
    let mut remaining = points;
    for (index, node) in rules.slots().iter().enumerate() {
        if node.domain == domain {
            let n = remaining.min(u32::from(node.capacity));
            p[index] = u16::try_from(n).ok()?;
            remaining -= n;
        }
    }
    if remaining != 0 {
        return None;
    }
    let a = WheelAllocation {
        wheel_ruleset_revision: Some(data.revision().into()),
        wheel_revision: 0,
        slots: WheelSlots::new(p).ok()?,
        current: true,
    };
    let mut e = derive_runtime_effects(data.wheel(), &a, vocation, u32::MAX, true, true)?;
    e.max_health = 0;
    e.max_mana = 0;
    e.capacity = 0;
    e.mitigation_milli_percent = 0;
    e.skill_bonuses.clear();
    e.life_leech_hundredths_percent = 0;
    e.mana_leech_hundredths_percent = 0;
    e.active_unique_perks.clear();
    e.sockets = Default::default();
    e.revelation_stages.retain(|k, _| k == key);
    e.revelation_numeric_effects.retain(|k, _| k == key);
    summary(data, vocation, &e)
}
