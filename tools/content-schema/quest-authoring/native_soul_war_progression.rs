//! Qualification only: execute the unchanged production loader/reducer, never a substitute engine.
//! Applying returned changes below simulates a locked copy; it is NOT SQL/durable or transport proof.
#[allow(dead_code)]
#[path = "../../../apps/game-server/src/quest/mod.rs"]
mod native;

#[cfg(test)]
mod soul_war {
    use super::native::{QuestRefusal, QuestStateCatalogue, loader::parse_quest_state};
    use std::collections::BTreeMap;
    const FIXTURE: &str = include_str!("samples/soul-war-native-progression/quest-state.json");
    const BOSSES: [&str; 5] = ["malice", "greed", "spite", "cruelty", "hatred"];
    const TP: &str = "oteryn:quest-progress/soul-war-first-cycle/";
    const XP: &str = "oteryn:quest-transition/soul-war-first-cycle/";
    type Stored = BTreeMap<String, i64>;

    fn catalogue() -> QuestStateCatalogue {
        parse_quest_state(FIXTURE, "soul-war-first-cycle-qualification-r1")
            .expect("real native loader accepts the qualification input")
            .catalogue()
            .clone()
    }
    fn apply(
        c: &QuestStateCatalogue,
        s: &mut Stored,
        name: &str,
        now: i64,
    ) -> Result<(), QuestRefusal> {
        let transition = c
            .transition(&format!("{XP}{name}"))
            .expect("declared transition");
        assert!(
            !transition.completes,
            "this qualification cannot claim quest completion"
        );
        assert!(
            transition.experience.is_none(),
            "no reward is granted by the fixture"
        );
        // Evaluate the whole transition before applying ANY output to the test copy.
        let changes = c.evaluate(transition, s, now)?;
        for change in changes {
            s.insert(change.track, change.after);
        }
        Ok(())
    }
    fn rejected(c: &QuestStateCatalogue, s: &mut Stored, name: &str, expected: QuestRefusal) {
        let before = s.clone();
        assert_eq!(apply(c, s, name, 9999), Err(expected));
        assert_eq!(*s, before, "refusal must produce no partial progression");
    }
    fn permutations(items: &mut [usize], from: usize, out: &mut Vec<Vec<usize>>) {
        if from == items.len() {
            out.push(items.to_vec());
            return;
        }
        for i in from..items.len() {
            items.swap(from, i);
            permutations(items, from + 1, out);
            items.swap(from, i);
        }
    }

    #[test]
    fn every_one_of_120_boss_orders_requires_five_distinct_credits() {
        let c = catalogue();
        let mut orders = Vec::new();
        permutations(&mut [0, 1, 2, 3, 4], 0, &mut orders);
        assert_eq!(orders.len(), 120);
        for order in orders {
            let mut stored = Stored::new();
            apply(&c, &mut stored, "start", 900).unwrap();
            for (rank, index) in order.iter().enumerate() {
                rejected(&c, &mut stored, "final-access", QuestRefusal::StageMismatch);
                let name = format!("{}/taint-{}", BOSSES[*index], rank + 1);
                apply(&c, &mut stored, &name, 1000 + rank as i64).unwrap();
                assert_eq!(stored[&format!("{TP}taints")], rank as i64 + 1);
                assert_eq!(stored[&format!("{TP}first-taint-at")], 1000);
                assert_eq!(
                    BOSSES
                        .iter()
                        .filter(|b| stored.get(&format!("{TP}{b}")) == Some(&1))
                        .count(),
                    rank + 1
                );
                // Reload only the test copy, not a claim about a database restart.
                stored = serde_json::from_str(&serde_json::to_string(&stored).unwrap()).unwrap();
            }
            apply(&c, &mut stored, "final-access", 2000).unwrap();
            assert_eq!(stored[&format!("{TP}final-access")], 1);
            apply(&c, &mut stored, "victory", 3000).unwrap();
            rejected(&c, &mut stored, "finish", QuestRefusal::NotSupported);
        }
    }

    #[test]
    fn five_repeated_kills_never_open_the_final_gate() {
        let c = catalogue();
        let mut s = Stored::new();
        apply(&c, &mut s, "start", 900).unwrap();
        apply(&c, &mut s, "malice/taint-1", 1000).unwrap();
        for rank in 1..=5 {
            rejected(
                &c,
                &mut s,
                &format!("malice/taint-{rank}"),
                QuestRefusal::StageMismatch,
            );
        }
        assert_eq!(s[&format!("{TP}taints")], 1);
        rejected(&c, &mut s, "final-access", QuestRefusal::StageMismatch);
    }

    #[test]
    fn final_gate_rechecks_each_flag_not_an_aggregate_counter() {
        let c = catalogue();
        for missing in BOSSES {
            let mut s = Stored::from([(format!("{TP}started"), 1), (format!("{TP}taints"), 5)]);
            for boss in BOSSES {
                if boss != missing {
                    s.insert(format!("{TP}{boss}"), 1);
                }
            }
            rejected(&c, &mut s, "final-access", QuestRefusal::StageMismatch);
        }
        let gate = c.transition(&format!("{XP}final-access")).unwrap();
        assert_eq!(gate.effects.len(), super::native::QUESTSTATE0_RL_02);
    }

    #[test]
    fn unstarted_stale_and_replayed_events_cannot_advance() {
        let c = catalogue();
        let mut s = Stored::new();
        rejected(&c, &mut s, "malice/taint-1", QuestRefusal::StageMismatch);
        rejected(&c, &mut s, "victory", QuestRefusal::StageMismatch);
        apply(&c, &mut s, "start", 900).unwrap();
        rejected(&c, &mut s, "start", QuestRefusal::StageMismatch);
        apply(&c, &mut s, "malice/taint-1", 1000).unwrap();
        rejected(&c, &mut s, "greed/taint-1", QuestRefusal::StageMismatch);
        apply(&c, &mut s, "greed/taint-2", 1001).unwrap();
        assert_eq!(s[&format!("{TP}taints")], 2);
        assert_eq!(s[&format!("{TP}first-taint-at")], 1000);
    }

    #[test]
    fn partial_source_guard_is_not_supported_by_the_real_loader() {
        let mut input: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
        input["quests"][0]["transitions"][0]["effects"][0]["from_exact"] = false.into();
        let loaded = parse_quest_state(&input.to_string(), "qualification-partial-guard").unwrap();
        let mut s = Stored::new();
        rejected(
            loaded.catalogue(),
            &mut s,
            "start",
            QuestRefusal::NotSupported,
        );
    }
}
