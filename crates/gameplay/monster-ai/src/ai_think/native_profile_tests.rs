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
        (1865, 7209, 15, 8)
    );
    Ok(())
}
