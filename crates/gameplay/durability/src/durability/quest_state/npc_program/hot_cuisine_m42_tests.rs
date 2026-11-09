use super::*;
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::{hot_cuisine::HotCuisineProgram, npc_rules::NpcRuleEffect};

#[test]
fn m42_locked_source_selects_all_fifteen_utc_recipes() -> std::result::Result<(), String> {
    let source = NpcRuleCatalog::runtime().map_err(|e| format!("{e:?}"))?;
    let native = HotCuisineProgram::embedded().map_err(|e| format!("{e:?}"))?;
    let inventory = native
        .optimistic_inventory()
        .map_err(|e| format!("{e:?}"))?;
    let catalogue =
        QuestStateCatalogue::new("content-1", vec![], vec![]).map_err(|e| format!("{e:?}"))?;
    let session = GameSessionId::decode(&[1, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1])
        .map_err(|e| format!("{e:?}"))?;
    for index in 1..=15 {
        let request = NpcProgramRequest {
            command: CommandRef::new(
                session,
                CommandId::new(index).map_err(|e| format!("{e:?}"))?,
            ),
            npc: "oteryn:npc.jean_pierre".into(),
            trigger: NpcRuleTrigger::Say,
            message: "yes".into(),
            topic: (index * 2 + 3) as i32,
            transition_key: format!(
                "oteryn:quest-npc/hot_cuisine_quest/jean_pierre/native-cook-{index}"
            ),
        };
        let prefix = "oteryn:quest-progress/quest/u8_5/hot_cuisine_quest/";
        let tracks = BTreeMap::from([
            (format!("{prefix}quest_start"), 1),
            (format!("{prefix}current_dish"), index as i64),
        ]);
        for eligibility in [
            NpcEligibility {
                level: 34,
                premium_current: true,
                vocation: String::new(),
            },
            NpcEligibility {
                level: 35,
                premium_current: false,
                vocation: String::new(),
            },
        ] {
            assert!(
                evaluate_source(
                    source,
                    &request,
                    &tracks,
                    &catalogue,
                    1_785_542_400,
                    NpcEvaluationFacts { appearance: (&BTreeMap::new(), 0),
                        eligibility: &eligibility,
                        player_name: None
                    },
                    Some(&inventory)
                )
                .map_err(|e| format!("eligibility: {e:?}"))?
                .is_none()
            );
        }
        let plan = evaluate_source(
            source,
            &request,
            &tracks,
            &catalogue,
            1_785_542_400,
            NpcEvaluationFacts { appearance: (&BTreeMap::new(), 0),
                eligibility: &NpcEligibility {
                    level: 35,
                    premium_current: true,
                    vocation: "oteryn:vocation.none".into(),
                },
                player_name: None,
            },
            Some(&inventory),
        )
        .map_err(|e| format!("recipe {index}: {e:?}"))?
        .ok_or_else(|| format!("recipe {index} has no dispatched plan"))?;
        assert_eq!(
            plan.selected_rule_keys.as_slice(),
            std::slice::from_ref(&request.transition_key)
        );
        assert!(
            plan.effects
                .iter()
                .any(|e| matches!(e, NpcRuleEffect::GiveItem { count: 1, .. }))
        );
        assert!(
            plan.effects
                .iter()
                .any(|e| matches!(e, NpcRuleEffect::TakeItem { .. }))
        );
        let mut mismatched = request.clone();
        let next = index % 15 + 1;
        mismatched.topic = (next * 2 + 3) as i32;
        mismatched.transition_key =
            format!("oteryn:quest-npc/hot_cuisine_quest/jean_pierre/native-cook-{next}");
        assert!(
            evaluate_source(
                source,
                &mismatched,
                &tracks,
                &catalogue,
                1_785_542_400,
                NpcEvaluationFacts { appearance: (&BTreeMap::new(), 0),
                    eligibility: &NpcEligibility {
                        level: 35,
                        premium_current: true,
                        vocation: "oteryn:vocation.none".into()
                    },
                    player_name: None
                },
                Some(&inventory)
            )
            .map_err(|e| format!("mismatched recipe: {e:?}"))?
            .is_none()
        );
    }
    Ok(())
}
