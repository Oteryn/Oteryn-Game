#![allow(
    clippy::unwrap_used,
    reason = "focused assertions over explicitly constructed valid state-machine inputs"
)]
use oteryn_gameplay_world_change::counter::{CounterKind as K, CounterState, Credit, Parameters};

#[test]
fn world_counter_demon_save_boundaries_and_strict_prince_threshold() {
    let parameters = Parameters::default();
    let initial = CounterState::new(K::DemonWars, 41, false);
    assert_eq!(initial.stage, 3);
    let count = |n| {
        let mut state = initial;
        for _ in 0..n {
            state = state
                .credit(K::DemonWars, Credit::Kill(0), &parameters, 0)
                .unwrap();
        }
        state
            .at_save(K::DemonWars, 42, false, &parameters, 0)
            .unwrap()
    };
    assert_eq!(count(99).stage, 3);
    assert_eq!(count(100).stage, 4);
    assert_eq!(count(400).stage, 4);
    assert_eq!(count(401).stage, 5);
    let saved = count(401);
    assert_eq!(
        saved,
        saved
            .at_save(K::DemonWars, 42, false, &parameters, 0)
            .unwrap()
    );
    assert!(
        saved
            .at_save(K::DemonWars, 40, false, &parameters, 0)
            .is_err()
    );
}
#[test]
fn world_counter_serpent_is_immediate_and_requires_both_reset_conditions() {
    let p = Parameters::default();
    let mut state = CounterState::new(K::FireFeathered, 1, false);
    for _ in 0..1000 {
        state = state
            .credit(K::FireFeathered, Credit::Kill(0), &p, 0)
            .unwrap();
    }
    assert_eq!(state.stage, 2);
    assert!(!state.is_active(K::FireFeathered));
    for _ in 0..1000 {
        state = state
            .credit(K::FireFeathered, Credit::Kill(0), &p, 0)
            .unwrap();
    }
    assert_eq!(state.stage, 3);
    assert!(state.is_active(K::FireFeathered));
    for _ in 0..2000 {
        state = state
            .credit(K::FireFeathered, Credit::Kill(1), &p, 0)
            .unwrap();
    }
    assert_eq!(
        state.stage, 3,
        "kills alone cannot reset mission-dependent state"
    );
    assert_eq!(
        state
            .credit(K::FireFeathered, Credit::MissionComplete, &p, 0)
            .unwrap()
            .stage,
        1
    );
}
#[test]
fn world_counter_mage_requires_energized_death_and_restarts_its_collapse_clock() {
    let p = Parameters::default();
    let mut state = CounterState::new(K::MagesTower, 1, false);
    for _ in 0..2000 {
        state = state.credit(K::MagesTower, Credit::Kill(0), &p, 0).unwrap();
    }
    assert_eq!(
        state.stage, 1,
        "world kills alone do not kill the energized boss"
    );
    state = state
        .credit(K::MagesTower, Credit::Kill(1), &p, 100)
        .unwrap();
    assert_eq!(state.stage, 2);
    state = state
        .credit(K::MagesTower, Credit::Kill(2), &p, 200)
        .unwrap();
    assert_eq!(state.tick(K::MagesTower, 300199).unwrap().stage, 3);
    let closed = state.tick(K::MagesTower, 300200).unwrap();
    assert_eq!(closed.stage, 4);
    assert_eq!(
        closed
            .at_save(K::MagesTower, 2, false, &p, 300201)
            .unwrap()
            .stage,
        1
    );
}
#[test]
fn world_counter_hive_actions_do_not_confuse_points_or_individual_kills() {
    let p = Parameters::default();
    let mut state = CounterState::new(K::HiveBorn, 1, false);
    for _ in 0..200 {
        state = state.credit(K::HiveBorn, Credit::Task(0), &p, 0).unwrap();
    }
    state = state.at_save(K::HiveBorn, 2, false, &p, 0).unwrap();
    assert_eq!(state.stage, 2);
    for _ in 0..100 {
        state = state.credit(K::HiveBorn, Credit::Task(4), &p, 0).unwrap();
    }
    assert_eq!(
        state.at_save(K::HiveBorn, 3, false, &p, 0).unwrap().stage,
        2,
        "200 points are only100 actions"
    );
    for _ in 0..100 {
        state = state.credit(K::HiveBorn, Credit::Task(5), &p, 0).unwrap();
    }
    state = state.at_save(K::HiveBorn, 3, false, &p, 0).unwrap();
    assert_eq!(state.stage, 3);
    assert_eq!(
        state.at_save(K::HiveBorn, 7, false, &p, 0).unwrap().stage,
        3
    );
    assert_eq!(
        state.at_save(K::HiveBorn, 8, false, &p, 0).unwrap().stage,
        1
    );
}
#[test]
fn world_counter_fire_earth_fifty_world_kills_do_not_activate_or_complete_it() {
    let p = Parameters::default();
    let mut state = CounterState::new(K::FireEarth, 1, false);
    for _ in 0..50 {
        state = state.credit(K::FireEarth, Credit::Kill(0), &p, 0).unwrap();
    }
    assert!(!state.is_active(K::FireEarth));
    state = state.at_save(K::FireEarth, 2, true, &p, 0).unwrap();
    for _ in 0..50 {
        state = state.credit(K::FireEarth, Credit::Kill(0), &p, 0).unwrap();
    }
    assert_eq!(
        state.stage, 1,
        "50 is personal achievement progress, not a world phase"
    );
}

#[test]
fn world_counter_projection_milestones_keep_the_credit_cycle_until_reset() {
    let p = Parameters::default();
    let mut state = CounterState::new(K::FireFeathered, 1, false);
    for _ in 0..1000 {
        state = state
            .credit(K::FireFeathered, Credit::Kill(0), &p, 0)
            .unwrap();
    }
    assert_eq!(state.stage, 2);
    assert_eq!(state.credit_generation, 1);
    for _ in 0..1000 {
        state = state
            .credit(K::FireFeathered, Credit::Kill(0), &p, 0)
            .unwrap();
    }
    for _ in 0..2000 {
        state = state
            .credit(K::FireFeathered, Credit::Kill(1), &p, 0)
            .unwrap();
    }
    state = state
        .credit(K::FireFeathered, Credit::MissionComplete, &p, 0)
        .unwrap();
    assert_eq!(state.stage, 1);
    assert_eq!(state.credit_generation, 2);
}
#[test]
fn world_counter_membership_excludes_fury_serpents_and_other_fire_caves() {
    let bindings: oteryn_gameplay_world_change::counter::Bindings =
        serde_json::from_str(oteryn_gameplay_world_change::counter::BINDINGS_JSON).unwrap();
    bindings.validate().unwrap();
    assert_eq!(
        bindings.credits("oteryn:creature.seacrest_serpent", (33530, 31630, 14)),
        vec![(K::FireFeathered, Credit::Kill(0))]
    );
    assert!(
        bindings
            .credits("oteryn:creature.seacrest_serpent", (33393, 31855, 14))
            .is_empty()
    );
    assert!(
        bindings
            .credits("oteryn:creature.fire_elemental", (100, 100, 7))
            .is_empty()
    );
}
#[test]
fn world_counter_configured_population_and_deepling_weights_are_distinct() {
    let p = Parameters {
        deepling_points: 7,
        deer_kills: 5,
        wolf_recovery: 20,
    };
    let initial = CounterState::new(K::Deeplings, 1, false);
    assert!(
        initial
            .at_save(K::Deeplings, 2, false, &Parameters::default(), 0)
            .is_err()
    );
    assert!(
        initial
            .credit(K::Deeplings, Credit::Kill(0), &p, 0)
            .is_err()
    );
    let state = initial
        .credit(K::Deeplings, Credit::HeartDestroyed, &p, 0)
        .unwrap()
        .credit(K::Deeplings, Credit::SoulNetDestroyed, &p, 0)
        .unwrap();
    assert_eq!(
        state.at_save(K::Deeplings, 2, false, &p, 0).unwrap().stage,
        2
    );
    let mut state = CounterState::new(K::Overhunting, 1, false);
    for _ in 0..5 {
        state = state
            .credit(K::Overhunting, Credit::Kill(0), &p, 0)
            .unwrap();
    }
    state = state.at_save(K::Overhunting, 2, false, &p, 0).unwrap();
    assert_eq!(state.stage, 2);
    state = state
        .credit(K::Overhunting, Credit::TrappedWolves(2), &p, 0)
        .unwrap();
    assert_eq!(
        state
            .at_save(K::Overhunting, 3, false, &p, 0)
            .unwrap()
            .stage,
        1
    );
}

#[cfg(test)]
mod tests {
    use oteryn_gameplay_world_change::counter::*;
    #[test]
    fn source_station_offsets_are_bounded_sorted_and_optional_resource() -> Result<(), String> {
        let mut all: Vec<oteryn_gameplay_world_change::variants::Definition> =
            serde_json::from_str(oteryn_gameplay_world_change::variants::RECIPES)
                .map_err(|e| e.to_string())?;
        let d = all.iter_mut().find(|d| d.key == "awash").ok_or("awash")?;
        let station = d.stations.first_mut().ok_or("station")?;
        assert!(station.resource.is_some());
        station.resource = None;
        station.restore_after_save_ms = vec![28_800_000, 57_600_000];
        oteryn_gameplay_world_change::variants::validate(&all).map_err(str::to_owned)?;
        for offsets in [
            vec![0],
            vec![86_400_001],
            vec![1, 1],
            vec![2, 1],
            vec![1, 2, 3],
        ] {
            let d = all.iter_mut().find(|d| d.key == "awash").ok_or("awash")?;
            d.stations
                .first_mut()
                .ok_or("station")?
                .restore_after_save_ms = offsets;
            assert!(oteryn_gameplay_world_change::variants::validate(&all).is_err());
        }
        Ok(())
    }
    #[test]
    fn recipe_fingerprint_commits_native_counter_bindings() {
        use sha2::{Digest, Sha256};
        let expected: [u8; 32] = Sha256::new()
            .chain_update(b"oteryn:wvar:recipes:v1\0")
            .chain_update(oteryn_gameplay_world_change::variants::RECIPES.as_bytes())
            .chain_update(oteryn_gameplay_world_change::variants::LAYERS.as_bytes())
            .chain_update(BINDINGS_JSON.as_bytes())
            .finalize()
            .into();
        assert_eq!(
            oteryn_gameplay_world_change::variants::recipe_digest(),
            expected
        );
    }
    #[test]
    fn counter_variants_require_explicit_valid_phase_mapping() -> Result<(), String> {
        let all = oteryn_gameplay_world_change::variants::definitions().map_err(str::to_owned)?;
        let d = all
            .iter()
            .find(|d| d.key == "thornfire")
            .ok_or("thornfire")?;
        let r = d.reset_counter.as_ref().ok_or("counter rule")?;
        assert_eq!(r.variant(1), Some(1));
        assert_eq!(r.variant(2), Some(0));
        assert_eq!(r.variant(3), None);
        for bad in [
            serde_json::json!({"kind":"demon_wars","stages":[[1,1]]}),
            serde_json::json!({"kind":"thornfire","stages":[[1,1],[1,0]]}),
            serde_json::json!({"kind":"thornfire","stages":[[1,2]]}),
        ] {
            let mut defs: serde_json::Value =
                serde_json::from_str(oteryn_gameplay_world_change::variants::RECIPES)
                    .map_err(|e| e.to_string())?;
            let row = defs
                .as_array_mut()
                .ok_or("defs")?
                .iter_mut()
                .find(|v| v["key"] == "thornfire")
                .ok_or("row")?;
            row["reset_counter"] = bad;
            let parsed: Vec<oteryn_gameplay_world_change::variants::Definition> =
                serde_json::from_value(defs).map_err(|e| e.to_string())?;
            assert!(oteryn_gameplay_world_change::variants::validate(&parsed).is_err());
        }
        Ok(())
    }
    #[test]
    fn thornfire_sources_are_fifty_four_exact_native_homes() -> Result<(), String> {
        let b: Bindings = serde_json::from_str(BINDINGS_JSON).map_err(|e| e.to_string())?;
        b.validate().map_err(|e| format!("{e:?}"))?;
        let d = b.definitions.last().ok_or("definition")?;
        assert_eq!(d.kind, CounterKind::Thornfire);
        assert_eq!(d.sources.len(), 54);
        for (i, s) in d.sources.iter().enumerate() {
            let lane = u8::try_from(i).map_err(|e| e.to_string())?;
            assert_eq!(
                b.credits(
                    &s.creature,
                    (
                        s.area[0],
                        s.area[2],
                        i16::try_from(s.area[4]).map_err(|e| e.to_string())?
                    )
                )
                .iter()
                .filter(|(k, c)| *k == CounterKind::Thornfire && *c == Credit::Kill(lane))
                .count(),
                1
            );
        }
        Ok(())
    }
    #[test]
    fn thornfire_requires_unique_homes_and_waits_for_next_save() -> Result<(), String> {
        let kind = CounterKind::Thornfire;
        let p = Parameters::default();
        let mut s = CounterState::new(kind, 11, false);
        for _ in 0..54 {
            s = s
                .credit(kind, Credit::Kill(0), &p, 0)
                .map_err(|e| format!("{e:?}"))?;
        }
        assert_eq!(s.counts[0], 1);
        assert_eq!(s.stage, 1);
        for i in 1..54 {
            s = s
                .credit(kind, Credit::Kill(i), &p, 0)
                .map_err(|e| format!("{e:?}"))?;
        }
        assert_eq!(s.stage, 1);
        assert_eq!(s.counts[1], 7);
        assert!(s.credit(kind, Credit::Kill(54), &p, 0).is_err());
        let burning = s
            .at_save(kind, 12, false, &p, 0)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(burning.stage, 2);
        assert_eq!(
            burning
                .at_save(kind, 12, false, &p, 0)
                .map_err(|e| format!("{e:?}"))?,
            burning
        );
        let restored = burning
            .at_save(kind, 13, false, &p, 0)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(restored.stage, 1);
        assert_eq!(restored.counts, [0; 3]);
        assert!(restored.at_save(kind, 12, false, &p, 0).is_err());
        Ok(())
    }
}
