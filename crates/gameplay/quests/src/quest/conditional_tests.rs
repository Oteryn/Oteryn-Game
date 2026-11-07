use super::*;

const QUEST: &str = "oteryn:quest.test_conditional";
const GATE: &str = "oteryn:quest-progress/test/conditional/gate";
const COUNT: &str = "oteryn:quest-progress/test/conditional/count";
const STEP: &str = "oteryn:quest-transition/test/conditional";

fn catalogue(
    other_effect: u8,
    minimum: i64,
    value: i64,
) -> Result<QuestStateCatalogue, QuestCatalogueError> {
    QuestStateCatalogue::new(
        "conditional-test",
        vec![
            QuestTrack {
                key: GATE.into(),
                quest: QUEST.into(),
                initial: 5,
                min: 0,
                max: 7,
            },
            QuestTrack {
                key: COUNT.into(),
                quest: QUEST.into(),
                initial: 0,
                min: 0,
                max: 10,
            },
        ],
        vec![QuestTransition {
            key: STEP.into(),
            quest: QUEST.into(),
            effects: vec![
                QuestEffect {
                    track: GATE.into(),
                    from: QuestComparison::Eq(5),
                    effect: QuestEffectKind::SetWhenAtLeast {
                        other_effect,
                        minimum,
                        value,
                    },
                },
                QuestEffect {
                    track: COUNT.into(),
                    from: QuestComparison::Between(0, 10),
                    effect: QuestEffectKind::Set(0),
                },
            ],
            completes: false,
            experience: None,
        }],
    )
}

#[test]
fn conditional_set_uses_original_snapshot_and_preserves_below_threshold() -> Result<(), String> {
    let book = catalogue(1, 2, 6).map_err(|e| format!("{e:?}"))?;
    let step = book.transition(STEP).ok_or("step")?;
    for (count, gate) in [(0, 5), (1, 5), (2, 6), (10, 6)] {
        let changes = book
            .evaluate(
                step,
                &BTreeMap::from([(GATE.into(), 5), (COUNT.into(), count)]),
                0,
            )
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(changes[0].after, gate);
        assert_eq!(
            changes[1].after, 0,
            "condition reads before the other effect resets its input"
        );
    }
    assert_eq!(
        book.evaluate(
            step,
            &BTreeMap::from([(GATE.into(), 5), (COUNT.into(), -1)]),
            0
        ),
        Err(QuestRefusal::StageMismatch)
    );
    Ok(())
}

#[test]
fn conditional_set_rejects_bad_references_and_hashes_all_parameters() -> Result<(), String> {
    let book = catalogue(1, 2, 6).map_err(|e| format!("{e:?}"))?;
    for (reference, minimum, value) in [(1, 3, 6), (1, 2, 7)] {
        let changed = catalogue(reference, minimum, value).map_err(|e| format!("{e:?}"))?;
        assert_ne!(book.definition_hash(QUEST), changed.definition_hash(QUEST));
    }
    for (reference, value) in [(0, 6), (2, 6), (u8::MAX, 6), (1, 8)] {
        assert!(matches!(
            catalogue(reference, 2, value),
            Err(QuestCatalogueError::InvalidEffect(_))
        ));
    }
    Ok(())
}

#[test]
fn conditional_and_bound_reward_effect_tags_are_distinct() {
    let mut conditional = Vec::new();
    QuestEffectKind::SetWhenAtLeast {
        other_effect: 1,
        minimum: 3,
        value: 7,
    }
    .encode(&mut conditional);
    let mut reward = Vec::new();
    QuestEffectKind::SetWithBinding {
        value: 7,
        binding: [0; 32],
    }
    .encode(&mut reward);
    assert_ne!(conditional.first(), reward.first());
}
