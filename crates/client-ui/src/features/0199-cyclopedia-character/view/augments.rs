use super::CyclopediaCharacterPanel;
use oteryn_client_runtime::cyclopedia_character::offence::{Effect, SourceKind};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AugmentRow {
    pub label: String,
    pub detail: String,
}
fn source(kind: SourceKind) -> &'static str {
    match kind {
        SourceKind::Equipment => "Equipment",
        SourceKind::Proficiency => "Proficiency",
        SourceKind::Wheel => "Wheel",
    }
}
fn effect(kind: Effect) -> &'static str {
    match kind {
        Effect::BaseDamage => "Base damage",
        Effect::BaseHealing => "Base healing",
        Effect::CriticalChance => "Critical chance",
        Effect::CriticalDamage => "Critical extra damage",
        Effect::LifeLeech => "Life leech",
        Effect::ManaLeech => "Mana leech",
        Effect::Cooldown => "Individual cooldown",
        Effect::WheelStage => "Wheel spell stage",
        Effect::GroupCooldown => "Secondary Focus cooldown",
    }
}
fn value(kind: Effect, value: i64) -> String {
    match kind {
        Effect::Cooldown | Effect::GroupCooldown => format!("{value} ms reduction"),
        Effect::WheelStage => format!("stage {value}"),
        _ => format!("{}.{:02}%", value / 100, value % 100),
    }
}
pub fn augment_heading(p: &CyclopediaCharacterPanel) -> String {
    let Some(page) = &p.augments.page else {
        return p
            .feedback
            .unwrap_or(if p.pending {
                "Loading spell modifiers"
            } else {
                "No confirmed spell modifier observation"
            })
            .into();
    };
    let observed = format!(
        "Observed {} ms | gear revision {} | page {}",
        page.stats.observed_at_unix_ms,
        page.stats.equipment_revision,
        u16::from(page.query.page) + 1
    );
    let Some(facts) = &page.facts else {
        return format!("{observed} | Modifiers unavailable for this saved observation");
    };
    let availability = [
        SourceKind::Equipment,
        SourceKind::Proficiency,
        SourceKind::Wheel,
    ]
    .into_iter()
    .map(|kind| {
        format!(
            "{} {}",
            source(kind),
            if facts.known_sources & kind.bit() != 0 {
                "observed"
            } else {
                "unavailable"
            }
        )
    })
    .collect::<Vec<_>>()
    .join("; ");
    format!(
        "{observed} | {} matching modifiers | {availability}",
        page.total
    )
}
pub fn augment_rows(p: &CyclopediaCharacterPanel) -> Vec<AugmentRow> {
    if !p.augments.open {
        return Vec::new();
    }
    let Some(page) = &p.augments.page else {
        return Vec::new();
    };
    let Some(facts) = &page.facts else {
        return Vec::new();
    };
    facts.rows.iter().filter_map(|row| {
    let origin=facts.sources.get(usize::from(row.source))?;let amount=value(row.effect,row.value);
    let note=match row.effect {Effect::Cooldown=>"Individual reductions combine before the half-base cooldown minimum.",Effect::GroupCooldown=>"Reduces the secondary Focus group cooldown.",Effect::WheelStage=>"Current Wheel augment stage; effects depend on the spell and cast context.",_=>"Cast, vocation and target requirements still apply."};
    let origin_label=match origin.kind {
        SourceKind::Equipment=>format!("Equipment slot {} | {}",origin.slot,origin.key),
        SourceKind::Proficiency=>format!("Selected weapon rewards | {}",origin.key),
        SourceKind::Wheel=>origin.key.clone(),
    };
    Some(AugmentRow {label:format!("{} | {} | {} | {}",row.spell,effect(row.effect),amount,source(origin.kind)),detail:format!("{}\r\n{}: {}\r\nSource: {} | {}\r\nObserved: {} ms | Character revision {} | Equipment revision {}\r\n{}",row.spell,effect(row.effect),amount,source(origin.kind),origin_label,page.stats.observed_at_unix_ms,page.stats.character_revision,page.stats.equipment_revision,note)})
 }).collect()
}
