//! Closed final-outfit operations supplement the source NPC owner. They do not
//! modify the old Quest catalogue or turn Computed transitions executable.
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrackChange};
use std::collections::BTreeMap;
#[derive(Debug)]
pub(in crate::durability) enum NativeEndpointError {
    Unqualified,
    NeedsNewComponent,
    NeedsCompatibleBounds,
}
/// Loaded from the mature claim/MINT join under the current Character lock.
/// A client, source script, or absent storage default cannot construct this witness.
pub(in crate::durability) struct FinalClaimProjection {
    pub track: String,
    pub claim_transaction_id: [u8; 16],
    pub source_value: i64,
}

pub(in crate::durability) struct PreparedNativeEndpoint {
    pub quest: String,
    pub base_definition_hash: [u8; 32],
    pub source_program_hash: [u8; 32],
    pub change: QuestTrackChange,
    pub absent_initial_projection: bool,
    pub actual_claim_projection: Option<[u8; 16]>,
    /// A separately versioned native-owner operation, never a catalogue rewrite.
    pub compatible_extension: Option<NativeFinalCompatibility>,
}
/// The caller already selected the full source guard under the actual Root
/// locks and must seal this operation into the private Node source observation.
pub(in crate::durability) fn prepare_native_final_endpoint(
    source_key: &str,
    change: &oteryn_gameplay_quests::quest::npc_cosmetics::TrackChange,
    stored: &BTreeMap<String, i64>,
    catalogue: &QuestStateCatalogue,
    claim_projection: Option<&FinalClaimProjection>,
) -> Result<PreparedNativeEndpoint, NativeEndpointError> {
    let qualified = match (source_key, change.key.as_str()) {
        (
            "oteryn:npc-cosmetic/avar_tar/demon-base",
            "oteryn:quest-progress/quest/u7_24/the_annihilator/reward",
        ) => change.after == 2,
        (
            "oteryn:npc-cosmetic/avar_tar/demon-shield",
            "oteryn:quest-progress/quest/u6_4/demon_helmet/rewards/demon_helmet",
        ) => change.after == 2,
        (
            "oteryn:npc-cosmetic/avar_tar/demon-helmet",
            "oteryn:quest-progress/quest/u8_2/the_demon_oak/done",
        ) => change.after == 4,
        (
            "oteryn:npc-cosmetic/dedoras/battle-mage-base",
            "oteryn:quest-progress/quest/u11_80/the_secret_library/questlog",
        ) => change.after == change.before.max(1),
        (
            "oteryn:npc-cosmetic/dedoras/battle-mage-base",
            "oteryn:quest-progress/quest/u11_80/the_secret_library/library_permission",
        ) => change.after == 7,
        (
            "oteryn:npc-cosmetic/dedoras/battle-mage-addon-1",
            "oteryn:quest-progress/quest/u11_80/the_secret_library/questlog",
        ) => change.after == change.before.max(1),
        (
            "oteryn:npc-cosmetic/dedoras/battle-mage-addon-2",
            "oteryn:quest-progress/quest/u11_80/the_secret_library/questlog",
        ) => change.after == change.before.max(1),
        (
            "oteryn:npc-cosmetic/jack_springer/inquisition-hand",
            "oteryn:quest-progress/quest/u12_20/grave_danger/questline",
        ) => change.after == 5,
        (
            "oteryn:npc-cosmetic/palimuth/yalaharian-addon-1",
            "oteryn:quest-progress/quest/u8_4/in_service_of_yalahar/questline",
        ) => change.after == 55,
        (
            "oteryn:npc-cosmetic/palimuth/yalaharian-addon-2",
            "oteryn:quest-progress/quest/u8_4/in_service_of_yalahar/questline",
        ) => change.after == 55,
        ("oteryn:npc-cosmetic/navigator/deepling-helmet", "oteryn:quest-progress/navigator") => {
            change.after == 4
        }
        (
            "oteryn:npc-cosmetic/bozo/jester-addon-1",
            "oteryn:quest-progress/quest/u8_1/what_afoolish_quest/mission8",
        ) => change.after == 5,
        (
            "oteryn:npc-cosmetic/bozo/jester-addon-1",
            "oteryn:quest-progress/quest/u8_1/what_afoolish_quest/questline",
        ) => change.after == 25,
        (
            "oteryn:npc-cosmetic/bozo/jester-addon-2",
            "oteryn:quest-progress/quest/u8_1/what_afoolish_quest/mission11",
        ) => change.after == 4,
        (
            "oteryn:npc-cosmetic/bozo/jester-addon-2",
            "oteryn:quest-progress/quest/u8_1/what_afoolish_quest/questline",
        ) => change.after == 36,
        _ => false,
    };
    if !qualified {
        return Err(NativeEndpointError::Unqualified);
    }
    let required_before = match (source_key, change.key.as_str()) {
        (
            "oteryn:npc-cosmetic/avar_tar/demon-base",
            "oteryn:quest-progress/quest/u7_24/the_annihilator/reward",
        )
        | (
            "oteryn:npc-cosmetic/avar_tar/demon-shield",
            "oteryn:quest-progress/quest/u6_4/demon_helmet/rewards/demon_helmet",
        ) => Some(1),
        (
            "oteryn:npc-cosmetic/avar_tar/demon-helmet",
            "oteryn:quest-progress/quest/u8_2/the_demon_oak/done",
        ) => Some(3),
        (
            "oteryn:npc-cosmetic/dedoras/battle-mage-base",
            "oteryn:quest-progress/quest/u11_80/the_secret_library/library_permission",
        ) => Some(6),
        (
            "oteryn:npc-cosmetic/jack_springer/inquisition-hand",
            "oteryn:quest-progress/quest/u12_20/grave_danger/questline",
        ) => Some(4),
        (
            "oteryn:npc-cosmetic/palimuth/yalaharian-addon-1",
            "oteryn:quest-progress/quest/u8_4/in_service_of_yalahar/questline",
        )
        | (
            "oteryn:npc-cosmetic/palimuth/yalaharian-addon-2",
            "oteryn:quest-progress/quest/u8_4/in_service_of_yalahar/questline",
        ) => Some(54),
        (
            "oteryn:npc-cosmetic/bozo/jester-addon-1",
            "oteryn:quest-progress/quest/u8_1/what_afoolish_quest/questline",
        ) => Some(24),
        (
            "oteryn:npc-cosmetic/bozo/jester-addon-2",
            "oteryn:quest-progress/quest/u8_1/what_afoolish_quest/questline",
        ) => Some(35),
        _ => None,
    };
    if required_before.is_some_and(|expected| expected != change.before) {
        return Err(NativeEndpointError::Unqualified);
    }
    let track = catalogue
        .track(&change.key)
        .ok_or(NativeEndpointError::NeedsNewComponent)?;
    if track.quest != change.quest {
        return Err(NativeEndpointError::Unqualified);
    }
    let actual_before = stored.get(&change.key).copied().unwrap_or(track.initial);
    let absent_initial_projection = !stored.contains_key(&change.key) && change.before == -1;
    let actual_claim_projection = claim_projection
        .filter(|claim| {
            !stored.contains_key(&change.key)
                && change.before == 1
                && change.after == 2
                && claim.source_value == 1
                && claim.track == change.key
                && claim.claim_transaction_id != [0; 16]
                && matches!(
                    (source_key, change.key.as_str()),
                    (
                        "oteryn:npc-cosmetic/avar_tar/demon-base",
                        "oteryn:quest-progress/quest/u7_24/the_annihilator/reward"
                    ) | (
                        "oteryn:npc-cosmetic/avar_tar/demon-shield",
                        "oteryn:quest-progress/quest/u6_4/demon_helmet/rewards/demon_helmet"
                    )
                )
        })
        .map(|claim| claim.claim_transaction_id);
    if claim_projection.is_some() && actual_claim_projection.is_none() {
        return Err(NativeEndpointError::Unqualified);
    }
    if actual_before != change.before
        && !absent_initial_projection
        && actual_claim_projection.is_none()
    {
        return Err(NativeEndpointError::Unqualified);
    }
    let compatible_extension = if (track.min..=track.max).contains(&change.after) {
        None
    } else {
        Some(
            compatible_final_extension(source_key, change, track)
                .ok_or(NativeEndpointError::NeedsCompatibleBounds)?,
        )
    };
    let hash = catalogue
        .definition_hash(&track.quest)
        .ok_or(NativeEndpointError::Unqualified)?;
    Ok(PreparedNativeEndpoint {
        quest: track.quest.clone(),
        base_definition_hash: hash,
        source_program_hash: oteryn_gameplay_quests::quest::npc_cosmetics::source_program::digest(),
        change: QuestTrackChange {
            track: change.key.clone(),
            before: actual_before,
            after: change.after,
        },
        absent_initial_projection,
        actual_claim_projection,
        compatible_extension,
    })
}

/// These operation versions supplement the unchanged old definition. Their
/// immutable child receipt must include this tag, old definition pin, source
/// program hash, exact full source guard, and actual locked before/after. They
/// never authorize another source or a general widening of Quest track bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::durability) enum NativeFinalCompatibility {
    FoolishQuestFirstAddonAcknowledgementV1,
    FoolishQuestSecondAddonAcknowledgementV1,
}
fn compatible_final_extension(
    source: &str,
    change: &oteryn_gameplay_quests::quest::npc_cosmetics::TrackChange,
    track: &oteryn_gameplay_quests::quest::QuestTrack,
) -> Option<NativeFinalCompatibility> {
    use NativeFinalCompatibility::*;
    match (
        source,
        track.quest.as_str(),
        track.key.as_str(),
        track.initial,
        track.min,
        track.max,
        change.before,
        change.after,
    ) {
        (
            "oteryn:npc-cosmetic/bozo/jester-addon-1",
            "oteryn:quest.what_a_foolish_quest",
            "oteryn:quest-progress/quest/u8_1/what_afoolish_quest/questline",
            0,
            0,
            13,
            24,
            25,
        ) => Some(FoolishQuestFirstAddonAcknowledgementV1),
        (
            "oteryn:npc-cosmetic/bozo/jester-addon-2",
            "oteryn:quest.what_a_foolish_quest",
            "oteryn:quest-progress/quest/u8_1/what_afoolish_quest/questline",
            0,
            0,
            13,
            35,
            36,
        ) => Some(FoolishQuestSecondAddonAcknowledgementV1),
        _ => None,
    }
}

pub(in crate::durability) fn operation_version(
    source: &str,
    key: &str,
) -> Result<&'static str, NativeEndpointError> {
    match source {
        "oteryn:npc-cosmetic/avar_tar/demon-base" => Ok("annihilator_outfit_claim_ack_v1"),
        "oteryn:npc-cosmetic/avar_tar/demon-shield" => Ok("demon_helmet_outfit_claim_ack_v1"),
        "oteryn:npc-cosmetic/avar_tar/demon-helmet" => Ok("demon_oak_outfit_ack_v1"),
        "oteryn:npc-cosmetic/dedoras/battle-mage-base" if key.ends_with("/library_permission") => {
            Ok("library_permission_outfit_ack_v1")
        }
        "oteryn:npc-cosmetic/dedoras/battle-mage-base"
        | "oteryn:npc-cosmetic/dedoras/battle-mage-addon-1"
        | "oteryn:npc-cosmetic/dedoras/battle-mage-addon-2"
            if key.ends_with("/questlog") =>
        {
            Ok("secret_library_questlog_source_helper_v1")
        }
        "oteryn:npc-cosmetic/navigator/deepling-helmet" => Ok("navigator_outfit_ack_v1"),
        "oteryn:npc-cosmetic/jack_springer/inquisition-hand" => Ok("grave_danger_outfit_ack_v1"),
        "oteryn:npc-cosmetic/palimuth/yalaharian-addon-1"
        | "oteryn:npc-cosmetic/palimuth/yalaharian-addon-2" => Ok("palimuth_outfit_ack_v1"),
        "oteryn:npc-cosmetic/bozo/jester-addon-1" => Ok("foolish_first_addon_ack_v1"),
        "oteryn:npc-cosmetic/bozo/jester-addon-2" => Ok("foolish_second_addon_ack_v1"),
        _ => Err(NativeEndpointError::Unqualified),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_gameplay_quests::quest::npc_cosmetics::TrackChange;
    #[test]
    fn srv7_exact_native_final_endpoint_does_not_enable_old_computed_rule() -> Result<(), String> {
        let loaded = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(|error| format!("actual Quest catalogue: {error:?}"))?;
        let catalogue = loaded.catalogue();
        let key = "oteryn:quest-progress/quest/u8_2/the_demon_oak/done";
        let track = catalogue.track(key).ok_or("actual Demon Oak track")?;
        let old_hash = catalogue
            .definition_hash(&track.quest)
            .ok_or("actual Quest pin")?;
        let change = TrackChange {
            key: key.into(),
            quest: track.quest.clone(),
            before: 3,
            after: 4,
        };
        let stored = BTreeMap::from([(key.to_owned(), 3)]);
        let prepared = prepare_native_final_endpoint(
            "oteryn:npc-cosmetic/avar_tar/demon-helmet",
            &change,
            &stored,
            catalogue,
            None,
        )
        .map_err(|error| format!("closed source endpoint: {error:?}"))?;
        assert_eq!(prepared.base_definition_hash, old_hash);
        assert_eq!((prepared.change.before, prepared.change.after), (3, 4));
        assert_eq!(catalogue.definition_hash(&track.quest), Some(old_hash));
        let old = catalogue
            .transition("oteryn:quest-transition/quest/u8_2/the_demon_oak/done/npc_1")
            .ok_or("actual old source transition")?;
        assert!(
            catalogue.evaluate(old, &stored, 1).is_err(),
            "the legacy Computed refusal is unchanged"
        );
        assert!(
            prepare_native_final_endpoint(
                "oteryn:npc-cosmetic/hanna/citizen-hat",
                &change,
                &stored,
                catalogue,
                None,
            )
            .is_err()
        );
        let mut forged = change.clone();
        forged.after = 3;
        assert!(
            prepare_native_final_endpoint(
                "oteryn:npc-cosmetic/avar_tar/demon-helmet",
                &forged,
                &stored,
                catalogue,
                None,
            )
            .is_err()
        );
        forged = change.clone();
        forged.quest = "oteryn:quest.outfit_and_addon_quests".into();
        assert!(
            prepare_native_final_endpoint(
                "oteryn:npc-cosmetic/avar_tar/demon-helmet",
                &forged,
                &stored,
                catalogue,
                None,
            )
            .is_err()
        );
        forged = change;
        forged.before = -1;
        assert!(
            prepare_native_final_endpoint(
                "oteryn:npc-cosmetic/avar_tar/demon-helmet",
                &forged,
                &stored,
                catalogue,
                None,
            )
            .is_err()
        );
        Ok(())
    }
    #[test]
    fn srv7_native_final_compatible_versions_are_exact_and_keep_base_pins() -> Result<(), String> {
        let loaded = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(|error| format!("actual Quest catalogue: {error:?}"))?;
        let catalogue = loaded.catalogue();
        for (source, key, before, after) in [
            (
                "oteryn:npc-cosmetic/jack_springer/inquisition-hand",
                "oteryn:quest-progress/quest/u12_20/grave_danger/questline",
                4,
                5,
            ),
            (
                "oteryn:npc-cosmetic/bozo/jester-addon-1",
                "oteryn:quest-progress/quest/u8_1/what_afoolish_quest/questline",
                24,
                25,
            ),
            (
                "oteryn:npc-cosmetic/bozo/jester-addon-2",
                "oteryn:quest-progress/quest/u8_1/what_afoolish_quest/questline",
                35,
                36,
            ),
        ] {
            let track = catalogue.track(key).ok_or("existing mature-owner track")?;
            let old_pin = catalogue
                .definition_hash(&track.quest)
                .ok_or("old definition pin")?;
            let change = TrackChange {
                key: key.into(),
                quest: track.quest.clone(),
                before,
                after,
            };
            let stored = BTreeMap::from([(key.to_owned(), before)]);
            let prepared = prepare_native_final_endpoint(source, &change, &stored, catalogue, None)
                .map_err(|error| format!("versioned native endpoint: {error:?}"))?;
            assert_eq!(
                prepared.compatible_extension.is_some(),
                source.starts_with("oteryn:npc-cosmetic/bozo/")
            );
            assert_eq!(prepared.base_definition_hash, old_pin);
            assert_eq!(catalogue.definition_hash(&track.quest), Some(old_pin));
            for bad_before in [-1, 0, before - 1, before + 1] {
                let mut bad = change.clone();
                bad.before = bad_before;
                let other = BTreeMap::from([(key.to_owned(), bad_before)]);
                assert!(
                    prepare_native_final_endpoint(source, &bad, &other, catalogue, None).is_err()
                );
            }
            for bad_after in [0, after - 1, after + 1, i64::MAX] {
                let mut bad = change.clone();
                bad.after = bad_after;
                assert!(
                    prepare_native_final_endpoint(source, &bad, &stored, catalogue, None).is_err()
                );
            }
        }
        Ok(())
    }
}
