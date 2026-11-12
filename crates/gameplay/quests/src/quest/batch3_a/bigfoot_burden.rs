//! Bigfoot source-only qualification; no production caller.
use super::input;
use crate::quest::npc_rules::{NpcInventorySnapshot, NpcRuleCatalog, NpcRuleEffect, NpcRuleError};
use std::collections::BTreeMap;

pub const BIGFOOT_TRACK_PREFIX: &str = "oteryn:quest-progress/quest/u9_60/bigfoots_burden/";

/// Resolve the typed matchmaker obligation using the fenced request's draw.
/// Crystal 201189f871d23597b82caf88fd611454a2ddefb8:
/// data-global/npc/gnomeral.lua:85.
pub fn bigfoot_matchmaker_target(draw: u8) -> Result<Vec<NpcRuleEffect>, NpcRuleError> {
    let target = matchmaker_target(draw)
        .ok_or_else(|| NpcRuleError::Invalid("Bigfoot matchmaker draw".into()))?;
    Ok(vec![NpcRuleEffect::Set {
        track: format!("{BIGFOOT_TRACK_PREFIX}matchmaker_id_needed"),
        value: i64::from(target),
    }])
}

/// Validate every draw before publishing any of the seven persistent tones.
/// Crystal 201189f871d23597b82caf88fd611454a2ddefb8:
/// data-global/npc/gnomelvis.lua:75-77.
pub fn bigfoot_soul_melody(draws: [u8; 7]) -> Result<Vec<NpcRuleEffect>, NpcRuleError> {
    let tones = soul_melody(draws)
        .ok_or_else(|| NpcRuleError::Invalid("Bigfoot soul melody draws".into()))?;
    Ok(tones
        .into_iter()
        .enumerate()
        .map(|(index, tone)| NpcRuleEffect::Set {
            track: format!("{BIGFOOT_TRACK_PREFIX}melody_tone{}", index + 1),
            value: i64::from(tone),
        })
        .collect())
}

/// `data-global/npc/gnomeral.lua:85`: one of the seven crystal appearances.
#[must_use]
pub fn matchmaker_target(draw: u8) -> Option<u16> {
    (draw < 7).then(|| 15809 + u16::from(draw))
}

/// `data-global/npc/gnomelvis.lua:75`: seven independent four-way tones.
#[must_use]
pub fn soul_melody(draws: [u8; 7]) -> Option<[u8; 7]> {
    if draws.iter().any(|draw| *draw >= 4) {
        return None;
    }
    Some(draws.map(|draw| draw + 1))
}

const BIGFOOT: &str = include_str!(
    "../../../../../../content/dialogues/quest-rules/quest-batch3-a-bigfoot_burden.json"
);

fn source() -> Result<NpcRuleCatalog, NpcRuleError> {
    NpcRuleCatalog::from_json_documents(&[BIGFOOT])
}

fn progress(entries: &[(&str, i64)]) -> BTreeMap<String, i64> {
    entries
        .iter()
        .map(|(key, value)| (format!("{BIGFOOT_TRACK_PREFIX}{key}"), *value))
        .collect()
}

#[test]
fn named_needs_beer_blocks_the_entire_test_until_the_source_reset() -> Result<(), NpcRuleError> {
    // donor bug fix / Q A-NEEDS-BEER:
    // https://github.com/Oteryn/Oteryn-Game/pull/2843#issuecomment-6589103264
    // Gnomerik:68/289 and actions_beer.lua:3/4 specify the same named state.
    let source = source()?;
    let mut tracks = progress(&[("quest_line", 5), ("test", 0), ("needs_beer", 1)]);
    let inventory = NpcInventorySnapshot::default();
    let outfits = BTreeMap::new();
    let blocked = source
        .plan(&input("oteryn:npc.gnomerik", &tracks, &inventory, &outfits))?
        .ok_or_else(|| NpcRuleError::Invalid("missing beer blocker".into()))?;
    assert!(blocked.track_changes.is_empty());
    assert_eq!(blocked.next_topic, 2);
    tracks.insert(format!("{BIGFOOT_TRACK_PREFIX}needs_beer"), 0);
    let accepted = source
        .plan(&input("oteryn:npc.gnomerik", &tracks, &inventory, &outfits))?
        .ok_or_else(|| NpcRuleError::Invalid("missing answer".into()))?;
    assert_eq!(accepted.next_topic, 3);
    assert!(
        accepted
            .track_changes
            .iter()
            .any(|change| change.key.ends_with("/test") && change.after == 7)
    );
    Ok(())
}

#[test]
fn gnomerik_results_preserve_source_score_threshold_and_full_text() -> Result<(), NpcRuleError> {
    let source = source()?;
    let inventory = NpcInventorySnapshot::default();
    let outfits = BTreeMap::new();
    for (score, track, value) in [(98, "needs_beer", 1), (105, "quest_line", 6)] {
        let tracks = progress(&[("quest_line", 5), ("test", score), ("needs_beer", 0)]);
        let mut request = input("oteryn:npc.gnomerik", &tracks, &inventory, &outfits);
        request.message = "result";
        request.topic = 33;
        let plan = source
            .plan(&request)?
            .ok_or_else(|| NpcRuleError::Invalid("missing test result".into()))?;
        assert!(plan.reply[0].contains(&format!("{score} of 112 possible points")));
        assert!(plan.track_changes.iter().any(|change| change.key
            == format!("{BIGFOOT_TRACK_PREFIX}{track}")
            && change.after == value));
    }
    Ok(())
}

#[test]
fn repair_adds_five_reputation_and_promotes_after_that_increment() -> Result<(), NpcRuleError> {
    let source = source()?;
    // The repair task itself needs rank 120 (gnomeral.lua:118). A rank-115
    // fixture must be refused; the first reachable +5 promotion is 475 -> 480.
    let tracks = progress(&[
        ("quest_line", 27),
        ("rank", 115),
        ("mission_tinkers_bell", 1),
        ("golem_count", 4),
        ("tinker_bell_timeout", -1),
    ]);
    let inventory = NpcInventorySnapshot::default();
    let outfits = BTreeMap::new();
    let mut request = input("oteryn:npc.gnomerik", &tracks, &inventory, &outfits);
    request.npc = "oteryn:npc.gnomeral";
    request.message = "repair";
    request.topic = 0;
    let refusal = source
        .plan(&request)?
        .ok_or_else(|| NpcRuleError::Invalid("missing rank refusal".into()))?;
    assert!(refusal.track_changes.is_empty());
    assert!(refusal.reply[0].contains("required rank"));
    let mut eligible_tracks = tracks.clone();
    eligible_tracks.insert(format!("{BIGFOOT_TRACK_PREFIX}rank"), 475);
    request.tracks = &eligible_tracks;
    let plan = source
        .plan(&request)?
        .ok_or_else(|| NpcRuleError::Invalid("missing repair reward".into()))?;
    assert!(
        plan.track_changes
            .iter()
            .any(|change| change.key.ends_with("/rank")
                && change.before == 475
                && change.after == 480)
    );
    assert!(
        plan.track_changes
            .iter()
            .any(|change| change.key.ends_with("/quest_line") && change.after == 28)
    );
    assert!(plan.effects.iter().any(
        |effect| matches!(effect, NpcRuleEffect::Achievement { name } if name == "Gnomelike")
    ));
    Ok(())
}

#[test]
fn matchmaker_rejects_outside_owner_draw_range() {
    assert_eq!(matchmaker_target(0), Some(15809));
    assert_eq!(matchmaker_target(6), Some(15815));
    assert_eq!(matchmaker_target(7), None);
}

#[test]
fn melody_validates_all_seven_draws_before_returning_any_tone() {
    assert_eq!(
        soul_melody([0, 1, 2, 3, 0, 1, 2]),
        Some([1, 2, 3, 4, 1, 2, 3])
    );
    assert_eq!(soul_melody([0, 1, 2, 3, 0, 1, 4]), None);
}
