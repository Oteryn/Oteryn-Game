//! The same immutable native bodies used by the actual dispatch must qualify
//! every existing typed cast/spawn/map callback without a fabricated Draft.
use crate::content::{ProjectV2DefinitionRef, native_gameplay};
use oteryn_gameplay_foundation::foundation::profile_schedule::ScheduleList;

#[test]
fn srx3_native_callback_constructors_qualify_all_source_cast_spawn_and_map_entries()
-> Result<(), Box<dyn std::error::Error>> {
    let result = std::thread::Builder::new().stack_size(64 << 20).spawn(|| -> Result<(), String> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../content/spells.manifest.json");
        let native = native_gameplay::retained_callback_native_fixture_at(
            crate::content::native_gameplay_test_world_id().map_err(|error|format!("{error:?}"))?, &path,
        );
        let witness: serde_json::Value = serde_json::from_str(include_str!("../../../../content/src/content/native_monster_callback_witness.json"))
            .map_err(|error|error.to_string())?;
        let entries = witness["entries"].as_array().ok_or("source entries")?;
        let mut counts = [0,0,0];
        let mut thresholds = 0;
        for entry in entries {
            let creature: ProjectV2DefinitionRef = serde_json::from_value(entry["creature"].clone())
                .map_err(|error|error.to_string())?;
            let index = entry["index"].as_u64().ok_or("source index")? as usize;
            let list = match entry["phase"].as_str() {
                Some("attacks") => ScheduleList::Attack,
                Some("defenses") => ScheduleList::Defence,
                _ => return Err("source phase".into()),
            };
            counts[0] += usize::from(super::source_callback_spawn::SourceCallbackSpawn::qualify_native(
                &native,&creature,index,list,
            ).map_err(|error|format!("spawn {creature:?}:{index}: {error:?}"))?.is_some());
            counts[1] += usize::from(super::source_callback_cast::SourceCallbackCast::qualify_native(
                &native,&creature,index,list,
            ).map_err(|error|format!("cast {creature:?}:{index}: {error:?}"))?.is_some());
            counts[2] += usize::from(crate::bosses::seven_encounters::source_encounter_seven::SevenSource::qualify_native(
                &native,&creature,index,list,
            ).map_err(|error|format!("map {creature:?}:{index}: {error:?}"))?.is_some());
            if matches!(entry["ability"]["key"].as_str(),Some("oteryn:ability.creature.lisa.field-fill-lisa_heal"|"oteryn:ability.creature.tyrn.field-fill-tyrn_heal"|"oteryn:ability.spell.glooth_fairy_healing")) {
                crate::bosses::threshold_heals::source_threshold_heal::ThresholdHealSource::qualify_native(
                    &native,&creature,index,*crate::content::native_gameplay_test_world_id().map_err(|error|format!("{error:?}"))?.as_bytes(),
                ).map_err(|error|format!("threshold {creature:?}:{index}: {error:?}"))?;
                thresholds += 1;
            }
        }
        if counts != [16,9,7] { return Err(format!("native source constructor census: {counts:?}")); }
        if thresholds != 3 {return Err(format!("native threshold source census: {thresholds}"));}
        Ok(())
    })?.join().map_err(|_|"native callback constructor thread panicked")?;
    result.map_err(Into::into)
}

#[test]
fn srx3_native_callback_current_owner_entrypoints_are_available() {
    // The native dispatch borrows the actual carrier's non-Clone current fence;
    // it cannot manufacture a second mutable fence for the legacy owner ABI.
    let _spawn = super::source_callback_spawn::CallbackSpawnOwner::new_current;
    let _cast = super::source_callback_cast::CallbackCastOwner::new_current;
    let _cast_schedule = super::source_callback_cast::CallbackCastOwner::schedule_current;
    let _cast_drain =
        |owner: &mut super::source_callback_cast::CallbackCastOwner,
         runtime: &mut oteryn_gameplay_entities::ChannelRuntimeV1,
         stamp: oteryn_gameplay_foundation::foundation::RuntimeWorkStamp,
         vitals: &mut dyn crate::player_lethal::PlayerLethalVitals,
         damage: &crate::creature_damage_spell::DamageSpellOwner,
         reader: &mut dyn crate::creature_damage_spell::SpellWorldReader,
         clock: &oteryn_gameplay_foundation::foundation::owner_timer::VirtualOwnerClock| {
            owner.drain_current(runtime, stamp, vitals, damage, reader, clock)
        };
    let _damage = crate::creature_damage_spell::DamageSpellOwner::apply_target_current;
    let _anchor = crate::creature_damage_spell::SourcePositionAnchor::capture;
    let _area = crate::creature_area_heal::AreaHealSource::from_current_native;
    let _area_execute = crate::creature_area_heal::AreaHealOwner::execute_current;
    let _threshold_cast =
        crate::bosses::threshold_heals::source_threshold_heal::ThresholdHealOwner::cast_current;
    let _threshold_tick =
        crate::bosses::threshold_heals::source_threshold_heal::ThresholdHealOwner::tick_current;
    let _speech_capture = crate::map::facts::MovementSpeech::capture_monster;
    let _effect_capture = crate::map::facts::CreatureVisualCue::effect;
    let _missile_capture = crate::map::facts::CreatureVisualCue::missile;
    let _cue_emit = crate::map::facts::BundleFacts::emit_creature_visual;
}
