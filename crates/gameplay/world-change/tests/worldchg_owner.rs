use oteryn_gameplay_world_change::owner::{ContributionKind, SeasonalWindow, add_contribution};

#[test]
fn worldchg_contributions_are_checked_and_reset_is_a_new_epoch() {
    assert_eq!(add_contribution(8, 3), Some(11));
    assert_eq!(add_contribution(8, 0), None);
    assert_eq!(add_contribution(u64::MAX, 1), None);
    assert_eq!(ContributionKind::Coal.key(), "coal");
    assert_eq!(ContributionKind::Medicine.key(), "medicine");
    assert_eq!(ContributionKind::Votes.key(), "votes");
}

#[test]
fn worldchg_seasonal_windows_include_start_exclude_end_and_wrap_year() {
    let summer = SeasonalWindow {
        start: (6, 1),
        end: (7, 1),
    };
    assert!(!summer.contains(5, 31));
    assert!(summer.contains(6, 1));
    assert!(summer.contains(6, 30));
    assert!(!summer.contains(7, 1));
    let winter = SeasonalWindow {
        start: (12, 15),
        end: (1, 5),
    };
    assert!(winter.contains(12, 31));
    assert!(winter.contains(1, 4));
    assert!(!winter.contains(1, 5));
    assert!(!winter.contains(13, 1));
}

#[test]
fn worldchg_catalogue_covers_content_and_held_titles_never_qualify()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_gameplay_world_change::{
        daily::DailyWorldChanges,
        owner::{CATALOGUE_JSON, is_active},
    };
    let catalogue: serde_json::Value = serde_json::from_str(CATALOGUE_JSON)?;
    let entries = catalogue["entries"].as_array().ok_or("entries")?;
    let mut keys = std::collections::BTreeSet::new();
    let all = DailyWorldChanges::roll(7, 6, 13, |_| 0);
    let mut ready = 0;
    let mut qualified = 0;
    for entry in entries {
        let key = entry["key"].as_str().ok_or("key")?;
        assert!(keys.insert(key));
        assert!(!entry["reason"].as_str().ok_or("reason")?.is_empty());
        assert!(!entry["evidence"].as_array().ok_or("evidence")?.is_empty());
        if entry["status"] == "READY" {
            ready += 1;
            assert!(is_active(&all, key), "{key}");
        } else if entry["status"] == "QUALIFIED" {
            qualified += 1;
            assert!(entry["binding"].is_object(), "{key}");
            let mut active = all;
            if let Some(kind) = oteryn_gameplay_world_change::counter::CounterKind::from_key(key) {
                use oteryn_gameplay_world_change::counter::CounterKind as K;
                active.counter_stages[kind.index()] = match kind {
                    K::FireFeathered => 3,
                    K::Overhunting => 2,
                    _ => 1,
                };
            }
            if key == "stampede" {
                assert_eq!(entry["binding"]["raid_key"], "raid:porthope.stampede");
            }
            assert!(is_active(&active, key), "{key}");
        } else {
            assert_eq!(entry["status"], "HELD");
            if key == "fury_gates" {
                assert_eq!(is_active(&all, key), all.fury_gates.is_some());
            } else {
                assert!(!is_active(&all, key), "{key}");
            }
        }
    }
    assert_eq!((ready, qualified), (0, 12));
    assert_eq!(entries.len(), 43);
    for key in [
        "river_runs_deep",
        "spider_nest",
        "spirit_grounds",
        "stampede",
        "swamp_fever",
        "steamship",
        "overhunting",
        "noodles_is_gone",
        "nomads",
        "chakoya_iceberg",
        "rathleton",
        "rise_of_devovorga",
        "spring_into_life",
    ] {
        assert!(keys.contains(key), "{key}");
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    for name in [
        "quests-00000-00099.json",
        "quests-00100-00199.json",
        "quests-00200-00299.json",
        "quests-00300-00351.json",
    ] {
        let quests: serde_json::Value = serde_json::from_slice(&std::fs::read(
            root.join("content/quests/definitions").join(name),
        )?)?;
        for record in quests["records"].as_array().ok_or("records")? {
            let title = record["definition"]["display_name"]
                .as_str()
                .ok_or("title")?;
            if title.contains("World Change") {
                assert!(
                    entries.iter().any(|e| e["title"] == title),
                    "missing {title}"
                );
            }
        }
    }
    assert!(!is_active(&all, "unrecognized"));
    Ok(())
}

#[test]
fn worldchg_selection_replays_and_snapshot_retains_the_committed_epoch()
-> Result<(), serde_json::Error> {
    use oteryn_gameplay_world_change::daily::DailyWorldChanges;
    let select = |epoch| DailyWorldChanges::roll(epoch, 6, 13, |_| 0);
    assert_eq!(select(7), select(7));
    let bytes = serde_json::to_vec(&select(7))?;
    let mut immutable = select(7);
    immutable.counter_stages = [0; 8];
    immutable.counter_generations = [0; 8];
    immutable.counter_progress = [[0; 3]; 8];
    // The current phase projection is read separately from its durable owner.
    assert_eq!(
        serde_json::from_slice::<DailyWorldChanges>(&bytes)?,
        immutable
    );
    assert_ne!(select(7), select(8));
    Ok(())
}

#[test]
fn worldchg_authored_candidate_probabilities_control_selection() {
    use oteryn_gameplay_world_change::daily::{DailyCandidates, DailyWorldChanges};
    let select = |percent| {
        DailyWorldChanges::select(
            8,
            5,
            12,
            DailyCandidates {
                thawing_percent: percent,
                voice_percent: percent,
                trader_percent: percent,
                fire_earth_permille: 199,
                stampede_permille: u16::from(percent) * 10,
            },
            |_| u64::MAX,
        )
    };
    let none = select(0);
    assert!(!none.thawing && !none.their_masters_voice && none.oriental_trader.is_none());
    let all = select(100);
    assert!(all.thawing && all.their_masters_voice && all.oriental_trader.is_some());
    assert_eq!(none.reset_epoch, 8);
}

#[test]
fn worldchg_stampede_uses_accepted_daily_gate_and_preserves_legacy_snapshots()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_gameplay_world_change::{
        daily::{DailyCandidates, DailyWorldChanges},
        owner::is_active,
    };
    let select = |draw| {
        DailyWorldChanges::select(
            9,
            5,
            1,
            DailyCandidates {
                thawing_percent: 0,
                voice_percent: 0,
                trader_percent: 0,
                fire_earth_permille: 199,
                stampede_permille: 125,
            },
            |purpose| {
                if purpose == "stampede" {
                    draw
                } else {
                    u64::MAX
                }
            },
        )
    };
    let boundary = 1_u64 << 61; // exactly 125/1000 of the 64-bit draw range
    assert!(is_active(&select(boundary - 1), "stampede"));
    assert!(!is_active(&select(boundary), "stampede"));
    let mut legacy = serde_json::to_value(select(0))?;
    legacy.as_object_mut().ok_or("day")?.remove("stampede");
    let retained: DailyWorldChanges = serde_json::from_value(legacy)?;
    assert!(
        !is_active(&retained, "stampede"),
        "old immutable days must not gain a new event"
    );
    Ok(())
}

#[test]
fn worldchg_portal_candidates_preserve_sourced_pairs_without_claiming_native_admission()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let candidate: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/encounters/world-changes/portal-bindings-candidate.json"),
    )?)?;
    let layers: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/encounters/world-changes/layers.json"),
    )?)?;
    assert_eq!(candidate["native_admission"], false);
    let records = candidate["records"].as_array().ok_or("records")?;
    assert_eq!(records.len(), 2);
    for record in records {
        assert_eq!(record["native_admission"], false);
        let variants = record["variants"].as_array().ok_or("variants")?;
        let fury = record["key"] == "fury_gates";
        assert_eq!(variants.len(), if fury { 10 } else { 3 });
        for variant in variants {
            let layer = layers["layers"]
                .as_array()
                .ok_or("layers")?
                .iter()
                .find(|layer| layer["key"] == variant["layer"])
                .ok_or("native source layer")?;
            let tile = layer["tiles"]
                .as_array()
                .ok_or("tiles")?
                .iter()
                .find(|tile| tile["position"] == variant["position"])
                .ok_or("portal tile")?;
            assert!(
                tile["entries"]
                    .as_array()
                    .ok_or("entries")?
                    .iter()
                    .any(|entry| entry[0] == variant["appearance"]
                        && if fury {
                            entry[1] == variant["selector"]["action_id"]
                        } else {
                            entry[2] == variant["selector"]["unique_id"]
                        })
            );
            assert_eq!(
                variant["destination"],
                if fury {
                    serde_json::json!([33290, 31786, 13])
                } else {
                    serde_json::json!([33497, 32616, 8])
                }
            );
        }
        // Candidate facts do not activate a closed day; main static Fury is tested separately.
        assert!(!oteryn_gameplay_world_change::owner::is_active(
            &oteryn_gameplay_world_change::daily::DailyWorldChanges::roll(7, 6, 13, |_| u64::MAX),
            record["key"].as_str().ok_or("key")?,
        ));
    }
    assert_eq!(records[0]["requirements"]["min_level"], 60);
    assert_eq!(records[0]["return_marker"]["action_id"], 9715);
    assert_eq!(records[1]["return_marker"]["unique_id"], 35020);
    Ok(())
}

#[test]
fn worldchg_phase_query_keeps_unselected_voice_inactive() {
    use oteryn_gameplay_world_change::{daily::DailyWorldChanges, owner::is_active};
    let day = DailyWorldChanges::roll(7, 1, 1, |_| u64::MAX);
    assert!(!day.their_masters_voice);
    assert!(!is_active(&day, "their_masters_voice"));
    assert!(!is_active(&day, "their_masters_voice@1"));
    let mut selected = day;
    selected.their_masters_voice = true;
    assert!(is_active(&selected, "their_masters_voice@1"));
}

#[test]
fn worldchg_final_six_native_bindings_qualify_only_their_current_world_state()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_gameplay_world_change::{
        counter::CounterKind as K,
        daily::DailyWorldChanges,
        owner::{CATALOGUE_JSON, counter_admitted, is_active},
    };
    let catalogue: serde_json::Value = serde_json::from_str(CATALOGUE_JSON)?;
    let selected = DailyWorldChanges::roll(700, 6, 13, |_| 0);
    let inactive = DailyWorldChanges::roll(701, 1, 1, |_| u64::MAX);
    for key in [
        "deeplings",
        "hive_born",
        "grimvale",
        "oriental_trader",
        "thawing",
        "their_masters_voice",
    ] {
        let entry = catalogue["entries"]
            .as_array()
            .ok_or("entries")?
            .iter()
            .find(|entry| entry["key"] == key)
            .ok_or("native binding")?;
        assert_eq!(entry["status"], "QUALIFIED", "{key}");
        assert!(entry.get("missing_facts").is_none(), "{key}");
        assert!(is_active(&selected, key), "selected {key}");
    }
    for key in [
        "grimvale",
        "oriental_trader",
        "thawing",
        "their_masters_voice",
    ] {
        assert!(!is_active(&inactive, key), "unselected {key}");
    }
    for kind in [K::Deeplings, K::HiveBorn, K::TheirMastersVoice] {
        assert!(counter_admitted(kind));
        let mut completed = selected;
        completed.counter_stages[kind.index()] = 4;
        assert!(!is_active(&completed, &format!("{}@3", kind.key())));
        completed.world_counters = false;
        assert!(!is_active(&completed, &format!("{}@1", kind.key())));
    }
    Ok(())
}

#[test]
fn main_fury_static_layer_requirement_tracks_the_persisted_day() {
    use oteryn_gameplay_world_change::{daily::DailyWorldChanges, owner::is_active};
    let mut day = DailyWorldChanges::roll(7, 6, 13, |_| u64::MAX);
    assert!(day.fury_gates.is_none());
    assert!(!is_active(&day, "fury_gates"));
    day.fury_gates = Some("fury_gates/thais");
    assert!(is_active(&day, "fury_gates"));
    day.fury_gates = None;
    assert!(!is_active(&day, "fury_gates"));
}
