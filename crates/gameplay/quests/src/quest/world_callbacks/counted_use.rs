//! One consumed input advances one personal, bounded, one-shot map-use counter.
use super::*;
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CountedUse {
    pub track: String,
    pub limit: i64,
    #[serde(default, skip_serializing_if = "is_false")]
    pub project_on_complete: bool,
}

pub(super) fn valid(callback: &Callback) -> bool {
    let Some(counter) = &callback.counted_use else {
        return true;
    };
    super::super::valid_quest_key(&counter.track)
        && counter.track.starts_with("oteryn:quest-progress/")
        && (1..=1_000_000).contains(&counter.limit)
        && callback.trigger == Trigger::Use
        && callback.selector.position.is_some()
        && !callback.storage.is_empty()
        && !callback.is_read_only()
        && !callback.storage.iter().any(|s| s.track == counter.track)
        && callback.storage.len() < MAX_EFFECTS
        && callback
            .item_on_target
            .as_ref()
            .is_some_and(|item| item.count == 1)
        && callback.native_rule.is_none()
        && callback.teleport.is_none()
        && callback.effects.item_reward.is_none()
        && callback.effects.outfit_reward.is_none()
        && callback.effects.portals.is_none()
        && callback.effects.encounter.is_none()
        && callback.effects.item_cycle.is_none()
        && !callback.occupancy_driven
}

pub(super) fn extend(
    callback: &Callback,
    tracks: &mut Vec<QuestTrack>,
) -> Result<Option<QuestEffect>, String> {
    let Some(counter) = &callback.counted_use else {
        return Ok(None);
    };
    if let Some(track) = tracks.iter_mut().find(|t| t.key == counter.track) {
        if track.quest != callback.quest
            || track.initial != -1
            || track.min != -1
            || track.max > counter.limit
        {
            return Err("counted use has a foreign or incompatible counter".into());
        }
        track.max = counter.limit;
    } else {
        tracks.push(QuestTrack {
            key: counter.track.clone(),
            quest: callback.quest.clone(),
            initial: -1,
            min: -1,
            max: counter.limit,
        });
    }
    Ok(Some(QuestEffect {
        track: counter.track.clone(),
        from: QuestComparison::Between(0, counter.limit - 1),
        effect: QuestEffectKind::Add(1),
    }))
}

pub(super) fn validate_closed(
    catalogue: &QuestStateCatalogue,
    callbacks: &[Callback],
) -> Result<(), String> {
    let mut counters = BTreeSet::new();
    for callback in callbacks {
        let Some(counter) = &callback.counted_use else {
            continue;
        };
        if !counters.insert(&counter.track) {
            return Err("duplicate counted-use counter".into());
        }
        let mut activations = 0;
        for transition in catalogue.transitions.values() {
            for effect in &transition.effects {
                if effect.track != counter.track || transition.key == callback.key {
                    continue;
                }
                // Exact progress predicates may preserve a completed counter for a
                // later turn-in; ADD(0) cannot activate or advance the counter.
                if transition.quest == callback.quest && effect.effect == QuestEffectKind::Add(0) {
                    continue;
                }
                if transition.quest == callback.quest
                    && matches!(
                        effect.from,
                        QuestComparison::Eq(-1) | QuestComparison::Between(-1, -1)
                    )
                    && effect.effect == QuestEffectKind::Set(0)
                {
                    activations += 1;
                } else {
                    return Err("unregistered counted-use counter writer".into());
                }
            }
        }
        if activations != 1 {
            return Err("counted use requires one exact -1-to-zero activation".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counted_use_is_inactive_bounded_and_has_closed_writers() -> Result<(), String> {
        let lowered = super::super::super::loader::load_embedded_quest_state("counted-use-test")
            .map_err(|e| format!("{e:?}"))?;
        let catalogue = lowered.catalogue();
        let callbacks = load()?;
        validate_closed(catalogue, &callbacks)?;
        let callback = callbacks
            .iter()
            .find(|c| c.key == "oteryn:quest-callback/authored-nimmersatt-wall")
            .ok_or("wall")?;
        let counter = callback.counted_use.as_ref().ok_or("counter")?;
        // Evaluate only the arithmetic participant; the world plan witness deliberately
        // refuses plain catalogue execution and is covered by the composed PG test.
        let mut transition = catalogue
            .transition(&callback.key)
            .ok_or("transition")?
            .clone();
        transition
            .effects
            .retain(|effect| effect.track == counter.track);
        let mut values = std::collections::BTreeMap::from([
            ("oteryn:quest-progress/nimmersatt/stage".into(), 1),
            (counter.track.clone(), 19),
        ]);
        let changes = catalogue
            .evaluate(&transition, &values, 0)
            .map_err(|e| format!("{e:?}"))?;
        assert!(
            changes
                .iter()
                .any(|change| change.track == counter.track && change.after == 20)
        );
        values.insert(counter.track.clone(), 20);
        assert!(catalogue.evaluate(&transition, &values, 0).is_err());
        values.insert(counter.track.clone(), -1);
        assert!(catalogue.evaluate(&transition, &values, 0).is_err());
        let mut transitions = catalogue.transitions.values().cloned().collect::<Vec<_>>();
        transitions.push(QuestTransition {
            world_change: None,
            key: "oteryn:quest-transition/fixture-counter-bypass".into(),
            quest: callback.quest.clone(),
            effects: vec![QuestEffect {
                track: counter.track.clone(),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Set(20),
            }],
            completes: false,
            experience: None,
        });
        let bad = QuestStateCatalogue::new(
            catalogue.content_revision(),
            catalogue.tracks.values().cloned().collect(),
            transitions,
        )
        .map_err(|e| format!("{e:?}"))?;
        assert!(validate_closed(&bad, &callbacks).is_err());
        let mut bad = callback.clone();
        bad.counted_use.as_mut().ok_or("counter")?.limit = 0;
        assert!(!valid(&bad));
        let mut bad = callback.clone();
        bad.item_on_target.as_mut().ok_or("input")?.count = 2;
        assert!(!valid(&bad));
        Ok(())
    }
}
