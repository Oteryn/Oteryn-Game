//! Actual activated policies at the existing C13 name-to-physical-birth boundary.
//! Geometry/casting/lifecycle are not authorized or simulated by this focused case.
use super::*;
use crate::content_adapters::NativeGameplayRuntimeExt;

fn debug(value: impl std::fmt::Debug) -> String {
    format!("{value:?}")
}
fn id(tag: u8) -> [u8; 16] {
    let mut bytes = [tag; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes
}
#[test]
fn srx3_actual_native_familiar_acquisition_name_reaches_physical_birth()
-> Result<(), Box<dyn std::error::Error>> {
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| -> Result<(), String> {
        let world = oteryn_gameplay_content::content::native_gameplay_test_world_id().map_err(debug)?;
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../content/spells.manifest.json");
        let native = oteryn_gameplay_content::content::native_gameplay::retained_callback_native_fixture_at(world, &path);
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(world,
            ChannelId::decode(&id(131)).map_err(debug)?, NodeId::decode(&id(132)).map_err(debug)?,
            1, 1, 1, "runtime-scope-assignment:1", 8,
            ChannelContentPin::from_activation(world, 1, native.source_digest(), [2;32], [3;32], [4;32], (100,100,7))).map_err(debug)?;
        native.install_companion_policies(&mut runtime).map_err(debug)?;
        let session = GameSessionId::decode(&id(133)).map_err(debug)?;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let master = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime.bind_attacker_lease(master, session,
            CharacterLease::new(CharacterId::decode(&id(134)).map_err(debug)?,1).map_err(debug)?).map_err(debug)?;
        runtime.initialize_pinned_test_position(master, MovementLocalPosition{x:100,y:100,floor:7}).map_err(debug)?;
        // These are the literal current familiar_summon creature_name values.
        // Print only the actual matching full identities, never select by order.
        for name in ["Druid familiar", "Knight familiar", "Monk familiar", "Paladin familiar", "Sorcerer familiar"] {
            let matches: Vec<_> = native.policies().policies().iter()
                .filter(|policy| policy.display_name.eq_ignore_ascii_case(name))
                .map(|policy| (&policy.definition_key,&policy.definition_revision)).collect();
            eprintln!("SRX3_CONTROLLED_COMPANION acquisition_name={name:?} actual_policies={matches:?}");
            assert!(runtime.companion_policy(name).is_err(), "generic ambiguous names remain refused");
            let slug = name.split_whitespace().next().ok_or("vocation")?.to_ascii_lowercase();
            let key = format!("candidate:spell/{slug}_familiar");
            let profile = oteryn_gameplay_content::content::spell_data::native::profiles()
                .map_err(debug)?.iter().find(|profile| profile["spell"]["identity"]["key"] == key)
                .ok_or("actual source-qualified familiar spell")?;
            let spell = oteryn_gameplay_content::content::spell_data::native::spell_from_bundle(
                &serde_json::json!({"spell":profile["spell"]}), &profile["dependencies"]).map_err(debug)?;
            let definition = spell.familiar_creature_definition().ok_or("selected source Creature identity")?;
            let policy = runtime.companion_policy(&definition.key).map_err(debug)?;
            assert_eq!(policy.definition_revision,definition.revision);
            assert_eq!(policy.display_name,name);
            let before = runtime.positioned_actor_census().map_err(debug)?;
            let birth = runtime.prepare_companion_spawn(master, session, &definition.key,
                MovementLocalPosition{x:101,y:100,floor:7}, None, Some(1800), 1, 0, false);
            assert!(birth.is_ok(), "actual C13 name must resolve before offense can attach: {name}: {birth:?}");
            assert_eq!(runtime.positioned_actor_census().map_err(debug)?, before,
                "preparing a source-qualified birth does not publish an actor");
        }
        Ok(())
    })?.join().map_err(|_| "native acquisition scenario panicked")??;
    Ok(())
}
