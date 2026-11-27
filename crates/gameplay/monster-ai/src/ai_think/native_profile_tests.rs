//! W01c: test the production closure, rather than infer availability from the legacy skip.
use super::*;
use crate::content::ProjectV2AbilityEffect;
use crate::content::native_gameplay::CreatureProfilesDocument;

#[test]
fn srvwld_canonical_schedules_resolve_every_callback_at_its_source_index()
-> Result<(), Box<dyn std::error::Error>> {
    let document: CreatureProfilesDocument = serde_json::from_slice(include_bytes!(
        "../../../../../content/creatures/definitions/spell-native-profiles.json"
    ))?;
    let (mut creatures, mut schedules, mut attacks, mut defenses) = (0, 0, 0, 0);
    for record in &document.records {
        if !record.profile.target.key.starts_with("oteryn:creature.") {
            continue;
        }
        creatures += 1;
        let closure = record
            .attack_definitions
            .as_ref()
            .ok_or("canonical closure")?;
        let profile = project_native_record(record).map_err(|e| format!("{e:?}"))?;
        let Some(Data::Behavior(source)) = record.behavior.as_ref().map(|p| &p.data) else {
            return Err("canonical behavior".into());
        };
        // Crystal 201189f src/creatures/monsters/monster.cpp:1201-1222:
        // each resolved source spell executes at its original schedule slot.
        for (list, entries, original) in [
            (
                profile_schedule::ScheduleList::Attack,
                &profile.behavior().attacks,
                &source.attacks,
            ),
            (
                profile_schedule::ScheduleList::Defence,
                &profile.behavior().defenses,
                &source.defenses,
            ),
        ] {
            assert_eq!(entries.len(), original.len());
            for (index, entry) in entries.iter().enumerate() {
                schedules += 1;
                assert_eq!(entry.ability, original[index].ability);
                let details = profile_schedule::validate_entry(entry, profile.abilities())
                    .map_err(|e| {
                        format!("{} {list:?}[{index}]: {e:?}", record.profile.target.key)
                    })?;
                if !details.effects.iter().any(|effect| {
                    matches!(effect,
                    ProjectV2AbilityEffect::Executable(reference)
                    if reference.key.contains("effect-callback-"))
                }) {
                    continue;
                }
                match list {
                    profile_schedule::ScheduleList::Attack => {
                        // This is the production physical_spell::composed::source_for parser.
                        let parsed = crate::creature_damage_spell::composed::ComposedSource::from_native_parts(
                            &record.profile.target, index, &closure.records, &closure.profiles, [1; 32],
                        ).map_err(|e| format!("{} callback[{index}]: {e:?}", record.profile.target.key))?;
                        assert_eq!(parsed.ability, entry.ability);
                        assert!(!parsed.callbacks.is_empty());
                        attacks += 1;
                    }
                    profile_schedule::ScheduleList::Defence => defenses += 1,
                }
            }
        }
    }
    // Regenerate pins: tools/content-migration/refresh_season_creature_inventory.py.
    assert_eq!(
        (creatures, schedules, attacks, defenses),
        (1896, 7408, 15, 8)
    );
    Ok(())
}

#[test]
fn dryad_loader_qualifies_source_self_defenses() -> Result<(), Box<dyn std::error::Error>> {
    let document: CreatureProfilesDocument = serde_json::from_slice(include_bytes!(
        "../../../../../content/creatures/definitions/spell-native-profiles.json"
    ))?;
    let record = document
        .records
        .iter()
        .find(|record| record.profile.target.key == "oteryn:creature.dryad")
        .ok_or("Dryad profile")?;
    let profile = project_native_record(record).map_err(|error| format!("{error:?}"))?;
    let defenses = &profile.behavior().defenses;
    assert_eq!(
        defenses
            .iter()
            .map(|entry| (entry.interval_ms, entry.chance_ppm))
            .collect::<Vec<_>>(),
        [(2000, 250_000), (2000, 400_000)]
    );
    for entry in defenses {
        profile_schedule::validate_entry(entry, profile.abilities())
            .map_err(|error| format!("{error:?}"))?;
    }
    Ok(())
}

#[test]
fn crystal_parity_schedules_project_with_exact_cadence_and_ability()
-> Result<(), Box<dyn std::error::Error>> {
    use serde_json::Value;
    let packet: Value = serde_json::from_slice(include_bytes!(
        "../../../../../imports/crystalserver/creature-parity-201189f.json"
    ))?;
    let document: CreatureProfilesDocument = serde_json::from_slice(include_bytes!(
        "../../../../../content/creatures/definitions/spell-native-profiles.json"
    ))?;
    let mut missing = Vec::new();
    for row in packet["schedules"].as_array().ok_or("Crystal schedules")? {
        let creature = row["creature"]["key"].as_str().ok_or("exact Creature")?;
        let record = document
            .records
            .iter()
            .find(|r| r.profile.target.key == creature)
            .ok_or_else(|| format!("missing {creature}"))?;
        let profile =
            project_native_record(record).map_err(|error| format!("{creature}: {error:?}"))?;
        let entries = match row["group"].as_str() {
            Some("attacks") => &profile.behavior().attacks,
            Some("defenses") => &profile.behavior().defenses,
            _ => return Err("invalid schedule list".into()),
        };
        let expected = serde_json::from_value::<crate::content::ProjectV2AbilitySchedule>(
            row["schedule"].clone(),
        )?;
        let matching: Vec<_> = entries
            .iter()
            .filter(|entry| entry.ability == expected.ability)
            .collect();
        if matching != [&expected] {
            missing.push(expected.ability.key);
            continue;
        }
        profile_schedule::validate_entry(&expected, profile.abilities())
            .map_err(|error| format!("{creature}: {error:?}"))?;
    }
    assert!(missing.is_empty(), "unbound Crystal slots: {missing:?}");
    Ok(())
}
