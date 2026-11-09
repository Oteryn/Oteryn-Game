use super::*;
use crate::quest::{QuestTrack, ice_islands_seals as seals};

fn base(extra: Option<QuestTrack>) -> Result<QuestStateCatalogue, String> {
    // Preserve the actual canonical quest's existing Questline/Mission04 metadata.
    let loaded = crate::quest::loader::load_embedded_quest_state("content-1")
        .map_err(|error| format!("{error:?}"))?;
    let mut tracks = [seals::QUESTLINE, seals::MISSION04]
        .into_iter()
        .map(|key| {
            loaded
                .catalogue()
                .track(key)
                .cloned()
                .ok_or_else(|| format!("missing canonical track {key}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    tracks.extend(extra);
    QuestStateCatalogue::new("content-1", tracks, vec![]).map_err(|error| format!("{error:?}"))
}

#[test]
fn seal_callbacks_declare_the_owned_counter_on_the_canonical_quest() -> Result<(), String> {
    let callback = seals::callbacks::definition(0)?;
    let catalogue = extend_callbacks(base(None)?, std::slice::from_ref(&callback))?;
    let track = catalogue.track(seals::TARGETS).ok_or("seal counter")?;
    assert_eq!(
        (track.initial, track.min, track.max),
        (0, 0, (1_i64 << 48) - 1)
    );
    assert_eq!(track.quest, seals::QUEST);
    let transition = catalogue
        .transition(&callback.key)
        .ok_or("seal transition")?;
    let stored = std::collections::BTreeMap::from([
        (seals::QUESTLINE.into(), 8),
        (seals::MISSION04.into(), 1),
    ]);
    let changes = catalogue
        .evaluate(transition, &stored, 0)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        changes
            .iter()
            .find(|change| change.track == seals::TARGETS)
            .map(|change| change.after),
        Some(1)
    );
    assert!(!seals::consumes_paint(0, &changes).map_err(|error| format!("{error:?}"))?);
    Ok(())
}

#[test]
fn seal_owned_counter_cannot_silently_reuse_a_legacy_sentinel() -> Result<(), String> {
    let bad = QuestTrack {
        key: seals::TARGETS.into(),
        quest: seals::QUEST.into(),
        initial: -1,
        min: -1,
        max: (1_i64 << 48) - 1,
    };
    assert!(extend_callbacks(base(Some(bad))?, &[seals::callbacks::definition(0)?]).is_err());
    Ok(())
}

#[test]
fn unrelated_callback_sets_do_not_declare_a_seal_counter() -> Result<(), String> {
    let catalogue = extend_callbacks(base(None)?, &[])?;
    assert!(catalogue.track(seals::TARGETS).is_none());
    Ok(())
}

#[test]
fn production_catalogue_registers_every_checked_paint_target() -> Result<(), String> {
    let callbacks = load()?;
    let paints = callbacks
        .iter()
        .filter(|callback| matches!(callback.native_rule, Some(NativeRule::IceSealPaint { .. })))
        .collect::<Vec<_>>();
    assert_eq!(paints.len(), usize::from(seals::targets::TARGET_COUNT));
    let loaded = crate::quest::loader::load_embedded_quest_state("content-1")
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        loaded
            .catalogue()
            .track(seals::TARGETS)
            .map(|track| track.initial),
        Some(0)
    );
    for callback in paints {
        assert!(
            loaded.catalogue().transition(&callback.key).is_some(),
            "{}",
            callback.key
        );
    }
    Ok(())
}
