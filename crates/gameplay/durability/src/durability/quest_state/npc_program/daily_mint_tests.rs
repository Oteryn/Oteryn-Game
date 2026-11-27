use super::*;
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::npc_rules::{NpcComparison, NpcRuleEffect, NpcRuleGuard};

#[test]
fn repeatable_daily_npc_item_plans_use_fresh_claims() -> std::result::Result<(), String> {
    let source = NpcRuleCatalog::runtime().map_err(|e| format!("{e:?}"))?;
    let loaded = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(|e| format!("{e:?}"))?;
    let session = GameSessionId::decode(&[1, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1])
        .map_err(|e| format!("{e:?}"))?;
    let rules: Vec<_> = source
        .programs()
        .iter()
        .flat_map(|program| &program.document.rules)
        .filter(|rule| {
            (rule.key.starts_with("oteryn:quest-npc/dangerous_depths/")
                || rule.key.starts_with("oteryn:quest-npc/spike/"))
                && rule.effects.iter().any(|effect| matches!(effect, NpcRuleEffect::GiveItem { .. }))
        })
        .collect();
    assert_eq!(rules.len(), 7);
    for rule in rules {
        let mut tracks = BTreeMap::new();
        let mut level = 100;
        for guard in &rule.guards {
            match guard {
                NpcRuleGuard::Storage { track, value, .. } => {
                    tracks.insert(track.clone(), *value);
                }
                NpcRuleGuard::StorageTime { track, .. } => {
                    tracks.insert(track.clone(), 1_785_542_399);
                }
                NpcRuleGuard::Level { op: NpcComparison::Ge, value, .. } => level = *value as u32,
                _ => {}
            }
        }
        let mut claims = Vec::new();
        for (attempt, now) in [(1, 1_785_542_400), (2, 1_785_614_401)] {
            let request = NpcProgramRequest {
                command: CommandRef::new(session, CommandId::new(attempt).map_err(|e| format!("{e:?}"))?),
                npc: rule.npc.clone(),
                trigger: rule.trigger,
                message: rule.keywords.first().ok_or("daily keyword")?.clone(),
                topic: rule.topics.as_ref().and_then(|topics| topics.first()).copied().unwrap_or(0),
                transition_key: rule.key.clone(),
            };
            let plan = evaluate_source(
                source,
                &request,
                &tracks,
                loaded.catalogue(),
                now,
                NpcEvaluationFacts {
                    appearance: (&BTreeMap::new(), 0),
                    eligibility: &NpcEligibility {
                        level,
                        premium_current: true,
                        vocation: "oteryn:vocation.knight".into(),
                    },
                    player_name: None,
                },
                Some(&NpcInventorySnapshot::default()),
            )
            .map_err(|e| format!("{}: {e:?}", rule.key))?
            .ok_or_else(|| format!("{}: daily reward has no admitted plan", rule.key))?;
            assert_eq!(plan.selected_rule_keys, [request.transition_key]);
            let key = hot_cuisine_m42::claim_key(&plan, request.command).map_err(|e| format!("{e:?}"))?;
            assert_eq!(hot_cuisine_m42::claim_key(&plan, request.command).map_err(|e| format!("{e:?}"))?, key);
            claims.push(key);
            for guard in &rule.guards {
                if let NpcRuleGuard::StorageTime { track, .. } = guard {
                    tracks.insert(track.clone(), now + 72_000);
                }
            }
        }
        assert_ne!(claims[0], claims[1], "fresh MINT custody for {}", rule.key);
    }
    Ok(())
}
