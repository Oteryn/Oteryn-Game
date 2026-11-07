#![allow(clippy::expect_used)]
use crate::durability::character_build::DurableBuildState;
use crate::durability::character_equipment::EquipmentSnapshot;
use crate::spell::cast::PlayerSpellState;
use crate::spell::harmony::HarmonyMultiplier;
use crate::spell::owned_cast_facts::*;
use std::collections::BTreeMap;
fn binding() -> CastFactsBinding {
    let (_, actor, session) =
        crate::gameplay_transport::actor_spell::tests::runtime_with_player(87);
    CastFactsBinding {
        actor,
        session,
        character: [7; 16],
        character_revision: 4,
        lease_generation: 1,
        connection_generation: 1,
        player_revision: 3,
        content_digest: [9; 32],
        equipment_revision: 2,
    }
}
#[test]
fn current_projection_requires_every_owner_revision_and_unexpired_authority() {
    let expected = binding();
    let mut projection = CurrentProjection {
        binding: expected.clone(),
        authority_revision: 7,
        valid_until_micros: 100,
        value: true,
    };
    assert!(projection.current(&expected, 99));
    assert!(!projection.current(&expected, 100));
    projection.authority_revision = 0;
    assert!(!projection.current(&expected, 0));
    projection.authority_revision = 7;
    for field in 0..5 {
        let mut replaced = expected.clone();
        match field {
            0 => replaced.character_revision += 1,
            1 => replaced.equipment_revision += 1,
            2 => replaced.lease_generation += 1,
            3 => replaced.player_revision += 1,
            _ => replaced.content_digest = [8; 32],
        }
        assert!(!projection.current(&replaced, 0));
    }
}
#[test]
fn owner_snapshot_rejects_mixed_equipment_and_keeps_missing_grants_unknown() {
    let binding = binding();
    let build = DurableBuildState::new("knight", (0, 0), [(10, 0); 7]).expect("explicit build");
    let mut equipment = EquipmentSnapshot {
        character: binding.character,
        content_digest: binding.content_digest,
        character_revision: binding.character_revision,
        revision: binding.equipment_revision,
        combat_mode: None,
        items: vec![],
    };
    equipment.revision += 1;
    assert!(matches!(
        OwnedCastFacts::from_owner_reads(
            binding.clone(),
            build.clone(),
            8,
            equipment.clone(),
            AccessProjections::default()
        ),
        Err(OwnedFactsError::StaleBinding)
    ));
    equipment.revision -= 1;
    let owned = OwnedCastFacts::from_owner_reads(
        binding,
        build,
        8,
        equipment,
        AccessProjections::default(),
    )
    .expect("matching owner reads");
    assert_eq!(
        owned.wheel_stage("unknown", 0),
        Err(OwnedFactsError::UnavailableWheel)
    );
    assert!(owned.magnitude().is_none());
    assert!(owned.access.premium.is_none());
    assert!(owned.access.learned.is_none());
}
#[test]
fn wheel_presence_distinguishes_current_malformed_stage_from_missing_authority() {
    let binding = binding();
    let equipment = EquipmentSnapshot {
        character: binding.character,
        content_digest: binding.content_digest,
        character_revision: binding.character_revision,
        revision: binding.equipment_revision,
        combat_mode: None,
        items: vec![],
    };
    let mut owned = OwnedCastFacts::from_owner_reads(
        binding.clone(),
        DurableBuildState::new("elder_druid", (0, 0), [(10, 0); 7]).expect("current build"),
        1000,
        equipment,
        AccessProjections {
            wheel: Some(CurrentProjection {
                binding: binding.clone(),
                authority_revision: 7,
                valid_until_micros: 100,
                value: BTreeMap::from([("Twin Burst".into(), 4)]),
            }),
            ..AccessProjections::default()
        },
    )
    .expect("current owner snapshot");
    assert!(owned.has_current_wheel(99));
    assert_eq!(
        owned.wheel_stage("Twin Burst", 99),
        Err(OwnedFactsError::UnavailableWheel)
    );
    assert!(!owned.has_current_wheel(100));
    owned.binding.lease_generation += 1;
    assert!(!owned.has_current_wheel(99));
    owned.access.wheel = None;
    assert!(!owned.has_current_wheel(99));
}
#[test]
fn accepted_source_chain_due_keeps_numeric_owners_after_premium_expires() {
    use crate::content::native_gameplay::{
        NativeGameplayInput, NativeGameplayMapProfile, PinnedGameplayBytes,
        compile_native_gameplay, decode,
    };
    use sha2::{Digest, Sha256};
    let pinned = |bytes: &[u8]| PinnedGameplayBytes {
        bytes: bytes.to_vec(),
        sha256: Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
    };
    let input = NativeGameplayInput {
        cosmetics: NativeGameplayInput::unavailable_cosmetics(),
        native_map_profile: NativeGameplayMapProfile::AcceptedEntryR1,
        catalog: pinned(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../tools/content-schema/spell-authoring/samples/executable-spell-catalog.json"
        ))),
        source_selection: pinned(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../tools/content-schema/spell-authoring/samples/executable-spell-source-selection.json"
        ))),
        creature_profiles: pinned(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../tools/content-schema/native-gameplay/creature_profiles.json"
        ))),
        presentation_profiles: pinned(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../tools/content-schema/native-gameplay/presentation_profiles.json"
        ))),
        item_profiles: None,
        spell_appearances: None,
        build_training: None,
        familiar_config: None,
        familiar_defenses: None,
        wheel_profile: None,
        source_world: None,
        progression: None,
        item_keys: None,
        loot_tables: None,
    };
    let (runtime, _, _) = crate::gameplay_transport::actor_spell::tests::runtime_with_player(87);
    let world = runtime.binding().world_id();
    let baseline = crate::content::qualify_native_entry_room(world).expect("actual accepted map");
    let wrapped =
        compile_native_gameplay(baseline.compiled(), &input).expect("actual compiled owner source");
    let content = decode(&wrapped.server_artifact)
        .expect("qualified content")
        .state;
    let catalog: serde_json::Value = serde_json::from_slice(&input.catalog.bytes).expect("catalog");
    let row = catalog["bundles"]
        .as_array()
        .expect("bundles")
        .iter()
        .find(|row| row["bundle"]["spell"]["identity"]["key"] == "candidate:spell/lightning")
        .expect("source chain");
    let spell = oteryn_gameplay_content::content::spell_data::authoring::spell_from_bundle(
        &row["bundle"],
        &row["dependencies"],
    )
    .expect("closed genuine source");
    let state = PlayerSpellState::new(
        crate::spell::cast::CharacterCastFacts {
            vocation: crate::spell::Vocation::Sorcerer,
            level: 100,
            magic_level: 50,
            max_health: 500,
            max_mana: 5000,
            max_soul: 100,
        },
        0,
        0,
    )
    .expect("actual player");
    let mut b = binding();
    b.player_revision = state.revision();
    b.content_digest = content.source_digest();
    let premium = CurrentProjection {
        binding: b.clone(),
        authority_revision: 1,
        valid_until_micros: 10,
        value: true,
    };
    let build = DurableBuildState::new("sorcerer", (50, 0), [(10, 0); 7]).expect("actual build");
    let equipment = EquipmentSnapshot {
        character: b.character,
        content_digest: b.content_digest,
        character_revision: b.character_revision,
        revision: b.equipment_revision,
        combat_mode: None,
        items: vec![],
    };
    let owned = OwnedCastFacts::from_owner_reads(
        b,
        build,
        100,
        equipment,
        AccessProjections {
            premium: Some(premium),
            ..AccessProjections::default()
        },
    )
    .expect("matched owners");
    let accepted = owned
        .caster(&state, &spell, &content, HarmonyMultiplier::ONE, 9)
        .expect("initial admitted source cast");
    assert_eq!(accepted.magic_level, 50);
    assert!(matches!(
        owned.caster(&state, &spell, &content, HarmonyMultiplier::ONE, 10),
        Err(OwnedFactsError::UnavailablePremium)
    ));
    let due = owned
        .numerical_caster(&state, &spell, &content, HarmonyMultiplier::ONE, 10)
        .expect("already accepted numerical phase");
    assert_eq!(due.magic_level, accepted.magic_level);
    let effects =
        crate::spell::cast::resolve_ordinary_due_effects(&spell, &due, 1, &mut |minimum, _| {
            minimum
        })
        .expect("real source due formula");
    assert!(
        matches!(&effects[0],crate::spell::ResolvedEffect::Damage{magnitude,..} if *magnitude > 0)
    );
    let mut changed = state.clone();
    changed
        .advance_batch_revision()
        .expect("independent owner revision");
    assert!(matches!(
        owned.numerical_caster(&changed, &spell, &content, HarmonyMultiplier::ONE, 10),
        Err(OwnedFactsError::StaleBinding)
    ));
}
#[test]
fn attack_dependencies_are_found_inside_native_parameter_asts() {
    assert!(attack_input(
        &serde_json::json!({"native_behavior":{"parameters":{"minimum":{"multiply":[2,{"input":"attack_value"}]}}}})
    ));
    assert!(!attack_input(&serde_json::json!({"input":"magic_level"})));
}
