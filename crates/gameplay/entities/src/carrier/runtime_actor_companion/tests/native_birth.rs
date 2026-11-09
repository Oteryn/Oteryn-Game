use super::*;

#[test]
fn srx3_native_birth_reserves_real_spare_without_publishing_hp_or_presence() {
    let (mut runtime, owner, session) = owner();
    let at = runtime.read_actor_position(owner).unwrap().position();
    let before = runtime.positioned_actor_census().unwrap();
    let mut birth = runtime
        .prepare_native_creature_birth(owner, session, "creature:rat", at)
        .unwrap();
    let actor = birth.actor();
    assert!(!runtime.contains_live_creature(actor));
    runtime.reserve_native_creature_birth(&mut birth).unwrap();
    assert!(!runtime.contains_live_creature(actor));
    assert_eq!(runtime.positioned_actor_census().unwrap(), before);
    let retained = birth.clone();
    runtime.reserve_native_creature_birth(&mut birth).unwrap();
    assert_eq!(birth.actor(), retained.actor());
    let installed = runtime.install_native_creature_birth(birth).unwrap();
    assert_eq!(installed, actor);
    assert!(runtime.contains_live_creature(installed));
    let current = runtime.creature_combat_facts(installed, 0).unwrap();
    assert_eq!(current.health, current.maximum_health);
    assert_eq!(
        runtime.read_actor_position(installed).unwrap().position(),
        at
    );
    assert!(runtime.install_native_creature_birth(retained).is_err());
}

#[test]
fn srx3_native_birth_definite_rollback_releases_capacity_but_consumes_generation() {
    let (mut runtime, owner, session) = owner();
    let at = runtime.read_actor_position(owner).unwrap().position();
    let mut first = runtime
        .prepare_native_creature_birth(owner, session, "creature:rat", at)
        .unwrap();
    let original = first.actor();
    runtime.reserve_native_creature_birth(&mut first).unwrap();
    runtime.rollback_native_creature_birth(&first).unwrap();
    assert!(!runtime.contains_live_creature(original));
    assert!(runtime.rollback_native_creature_birth(&first).is_err());
    let next = runtime
        .prepare_native_creature_birth(owner, session, "creature:rat", at)
        .unwrap();
    assert_eq!(original.actor_local_id(), next.actor().actor_local_id());
    assert_ne!(original, next.actor());
}

#[test]
fn srx3_native_birth_unknown_commit_keeps_exact_capacity_reserved() {
    let (mut runtime, owner, session) = owner();
    let at = runtime.read_actor_position(owner).unwrap().position();
    let mut retained = Vec::new();
    while let Ok(mut birth) =
        runtime.prepare_native_creature_birth(owner, session, "creature:rat", at)
    {
        runtime.reserve_native_creature_birth(&mut birth).unwrap();
        retained.push(birth);
    }
    assert_eq!(retained.len(), 3);
    assert!(
        runtime
            .prepare_native_creature_birth(owner, session, "creature:rat", at)
            .is_err()
    );
    for birth in retained {
        runtime.validate_native_creature_birth(&birth).unwrap();
    }
}
