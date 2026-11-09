pub(super) fn parse_target(text: &str) -> Option<[u8; 16]> {
    if text.len() != 32
        && (text.len() != 36 || ![8, 13, 18, 23].iter().all(|i| text.as_bytes()[*i] == b'-'))
    {
        return None;
    }
    let compact: String = text.chars().filter(|c| *c != '-').collect();
    if compact.len() != 32 || !compact.is_ascii() {
        return None;
    }
    let mut target = [0; 16];
    for (i, byte) in target.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&compact[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(target)
}

/// Profile snapshots are complete; paging buttons refresh the active profile view.
pub(super) fn submit_profile(
    p: &mut oteryn_client_ui::cyclopedia_character::CyclopediaCharacterPanel,
    id: usize,
    values: [&str; 3],
) -> Option<Option<oteryn_client_runtime::raw_feature::FeatureCommand>> {
    use oteryn_client_runtime::cyclopedia_character::CharacterTab;
    use oteryn_client_ui::cyclopedia_character::request;
    if !matches!(id, 107 | 111) && !(matches!(id, 112 | 113) && p.tab == CharacterTab::Inspection) {
        return None;
    }
    let badges = id == 111 || (matches!(id, 112 | 113) && p.badges_open);
    let target = if values[2].trim().is_empty() {
        None
    } else {
        let Some(target) = parse_target(values[2].trim()) else {
            p.feedback = Some("Enter a valid target UUID");
            return Some(None);
        };
        Some(target)
    };
    p.change_target(target);
    p.tab = CharacterTab::Inspection;
    p.badges_open = badges;
    let intent = refresh_intent(p);
    Some(request(p, intent))
}

/// Reopening a profile must refresh its rich snapshot rather than the legacy text tab.
pub(super) fn refresh_intent(
    p: &oteryn_client_ui::cyclopedia_character::CyclopediaCharacterPanel,
) -> oteryn_client_ui::cyclopedia_character::Intent {
    use oteryn_client_runtime::cyclopedia_character::CharacterTab;
    use oteryn_client_ui::cyclopedia_character::Intent;
    if p.augments.open && p.tab == CharacterTab::Offence {
        return Intent::SpellAugments(
            oteryn_client_ui::cyclopedia_character::AugmentIntent::Refresh,
        );
    }
    if p.command_kind==387 {return Intent::Badges;}
    if p.tab == CharacterTab::Inspection {
        if p.badges_open {
            Intent::ProfileBadges
        } else {
            Intent::ProfileInspection
        }
    } else {
        Intent::Tab
    }
}

pub(super) fn badge_filter(
    p: &mut oteryn_client_ui::cyclopedia_character::CyclopediaCharacterPanel,
    id: usize,
) -> bool {
    use oteryn_client_ui::cyclopedia_character::BadgeFilter;
    p.badge_filter = match id {
        145 => BadgeFilter::All,
        146 => BadgeFilter::Earned,
        147 => BadgeFilter::Locked,
        148 => BadgeFilter::Hidden,
        152 => BadgeFilter::Public,
        _ => return false,
    };
    true
}
