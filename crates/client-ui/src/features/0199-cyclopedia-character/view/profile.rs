use super::CyclopediaCharacterPanel;
fn requirement(rule: oteryn_client_runtime::cyclopedia_character::BadgeRule) -> String {
    use oteryn_client_runtime::cyclopedia_character::BadgeRule;
    match rule {
        BadgeRule::AccountOlderThan(years) => format!("Account age: {years} years"),
        BadgeRule::LoyaltyAbove(points) => format!("More than {points} loyalty points"),
        BadgeRule::LevelSumAbove(levels) => format!("Combined character levels above {levels}"),
        BadgeRule::FourVocations(level) => {
            format!("Level {level} with druid, knight, paladin and sorcerer")
        }
        BadgeRule::FiveVocations(level) => format!("Level {level} with each of the five vocations"),
        BadgeRule::Monk(level) => format!("Level {level} with a monk"),
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileRow {
    pub label: String,
    pub detail: String,
}
pub fn profile_heading(p: &CyclopediaCharacterPanel) -> String {
    let Some(profile) = p.profile.as_ref() else {
        return p
            .feedback
            .unwrap_or("No confirmed profile; refresh to request access")
            .into();
    };
    let id: String = profile
        .character
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    format!(
        "{} | {} | {}",
        profile.name,
        id,
        if p.badges_open { "Badges" } else { "Equipment" }
    )
}
pub fn profile_rows(p: &CyclopediaCharacterPanel) -> Vec<ProfileRow> {
    use oteryn_client_runtime::cyclopedia_character::{Badge, CharacterTab};
    if p.tab != CharacterTab::Inspection {
        return Vec::new();
    }
    let Some(profile) = p.profile.as_ref() else {
        return Vec::new();
    };
    if p.badges_open {
        super::badge_rows(p)
            .into_iter()
            .filter_map(|row| {
                let badge = Badge::from_id(row.id)?;
                let earned = row.earned;
                Some(ProfileRow {
                    label: format!(
                        "{} — {}{}",
                        badge.name(),
                        if earned { "Earned" } else { "Not earned" },
                        if row.hidden { "; hidden locally" } else { "" }
                    ),
                    detail: format!(
                        "{}\r\nRequirement: {}\r\nAccount-wide, permanent award. {}",
                        badge.name(),
                        requirement(badge.rule()),
                        if earned {
                            "Confirmed by server."
                        } else {
                            "Not awarded in this profile."
                        }
                    ),
                })
            })
            .collect()
    } else {
        profile.equipment.iter().map(|item|ProfileRow {label:format!("Slot {} | {} | ×{}",item.slot,item.definition,item.quantity),detail:format!("Equipment slot: {}\r\nItem definition: {}\r\nQuantity: {}\r\nConfirmed equipped items; empty slots are omitted.",item.slot,item.definition,item.quantity)}).collect()
    }
}
