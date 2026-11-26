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
fn crystal_equipment_weapon_skill_cast_reads_intrinsic_attack() {
    use crate::content::native_gameplay::{NativeGameplayInput, compile_native_gameplay, decode};
    use crate::durability::character_equipment::{CombatMode, EquippedItem};
    use crate::durability::item_mint::TypedDefinitionRef;
    use crate::spell::{Vocation, cast::CharacterCastFacts};

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let mut input = NativeGameplayInput::from_manifest(&root.join("content/spells.manifest.json"))
        .expect("committed native inputs");
    input.source_world = None;
    let room = crate::content::qualify_native_entry_room(
        crate::content::native_gameplay_test_world_id().expect("world identity"),
    )
    .expect("qualified room");
    let content = decode(
        &compile_native_gameplay(room.compiled(), &input)
            .expect("compile native")
            .server_artifact,
    )
    .expect("native artifact")
    .state;
    let state = PlayerSpellState::new(
        CharacterCastFacts {
            vocation: Vocation::Knight,
            level: 330,
            magic_level: 0,
            max_health: 5000,
            max_mana: 1000,
            max_soul: 100,
        },
        0,
        0,
    )
    .expect("player state");
    let mut binding = binding();
    binding.player_revision = state.revision();
    binding.content_digest = content.source_digest();
    let weapon = content
        .item_policy_for_source_id(47368)
        .expect("Amber Slayer");
    let definition = &weapon.record().production_definition;
    let equipment = EquipmentSnapshot {
        character: binding.character,
        content_digest: binding.content_digest,
        character_revision: binding.character_revision,
        revision: binding.equipment_revision,
        combat_mode: Some(CombatMode::Attack),
        items: vec![EquippedItem {
            slot: 5,
            item_instance_id: [2; 16],
            tier: 0,
            quantity: 1,
            state_revision: 1,
            definition: TypedDefinitionRef {
                family: "Item".into(),
                production_key: definition.production_key.clone(),
                revision_ref: definition.revision_ref.clone(),
            },
        }],
    };
    let expected = oteryn_gameplay_combat_rules::resolution::swing_weapon(&content, |slot| {
        (slot == 5).then_some((
            definition.production_key.as_str(),
            definition.revision_ref.as_str(),
        ))
    })
    .expect("live melee weapon")
    .attack(0);
    assert!(expected > 0);
    let owned = OwnedCastFacts::from_owner_reads(
        binding,
        DurableBuildState::new("knight", (0, 0), [(100, 0); 7]).expect("knight build"),
        330,
        equipment,
        AccessProjections::default(),
    )
    .expect("fenced owner reads");
    for name in ["fierce_berserk", "berserk"] {
        let key = format!("candidate:spell/{name}");
        let spell = content
            .spell_book()
            .spells
            .iter()
            .find(|spell| spell.key == key)
            .expect("published weapon-skill cast");
        let caster = owned
            .numerical_caster(&state, spell, &content, HarmonyMultiplier::ONE, 1)
            .expect("owned weapon-skill caster");
        assert_eq!(caster.attack_value, expected, "{name}");
    }
}

#[test]
fn crystal_equipment_ethereal_cast_uses_qualified_projectile_element() {
    use crate::content::native_gameplay::{NativeGameplayInput, compile_native_gameplay, decode};
    use crate::content::{ProjectV2RangedElement, ProjectV2RangedWeapon};
    use crate::durability::character_equipment::{CombatMode, EquippedItem};
    use crate::durability::item_mint::TypedDefinitionRef;
    use crate::spell::{Vocation, cast::CharacterCastFacts};

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let mut input = NativeGameplayInput::from_manifest(&root.join("content/spells.manifest.json"))
        .expect("committed native inputs");
    input.source_world = None;
    let room = crate::content::qualify_native_entry_room(
        crate::content::native_gameplay_test_world_id().expect("world identity"),
    )
    .expect("qualified room");
    let content = decode(
        &compile_native_gameplay(room.compiled(), &input)
            .expect("compile native")
            .server_artifact,
    )
    .expect("native artifact")
    .state;
    let state = PlayerSpellState::new(
        CharacterCastFacts {
            vocation: Vocation::Paladin,
            level: 500,
            magic_level: 0,
            max_health: 5000,
            max_mana: 1000,
            max_soul: 100,
        },
        0,
        0,
    )
    .expect("player state");
    let mut binding = binding();
    binding.player_revision = state.revision();
    binding.content_digest = content.source_digest();
    let launcher = content
        .item_policy_for_source_id(52332)
        .expect("Crypt Spine");
    let arrow = content
        .item_policy_for_source_id(763)
        .expect("Flaming Arrow");
    let equip = |slot, key: &str, revision: &str| EquippedItem {
        slot,
        item_instance_id: [slot; 16],
        tier: 0,
        quantity: 1,
        state_revision: 1,
        definition: TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: revision.into(),
        },
    };
    let bow_def = &launcher.record().production_definition;
    let arrow_def = &arrow.record().production_definition;
    let equipment = EquipmentSnapshot {
        character: binding.character,
        content_digest: binding.content_digest,
        character_revision: binding.character_revision,
        revision: binding.equipment_revision,
        combat_mode: Some(CombatMode::Attack),
        items: vec![
            equip(5, &bow_def.production_key, &bow_def.revision_ref),
            equip(10, &arrow_def.production_key, &arrow_def.revision_ref),
        ],
    };
    let Some(ProjectV2RangedWeapon::Distance { attack: bow, .. }) =
        launcher.record().authoring.ranged
    else {
        unreachable!();
    };
    let Some(ProjectV2RangedWeapon::Ammunition {
        attack,
        elemental_attack,
        element: ProjectV2RangedElement::Fire,
        ..
    }) = arrow.record().authoring.ranged
    else {
        unreachable!();
    };
    let total = u32::from(bow) + u32::from(attack) + u32::from(elemental_attack);
    let owned = |equipment| {
        OwnedCastFacts::from_owner_reads(
            binding.clone(),
            DurableBuildState::new("paladin", (0, 0), [(100, 0); 7]).expect("paladin build"),
            500,
            equipment,
            AccessProjections::default(),
        )
        .expect("fenced owner reads")
    };
    let direct = owned(equipment.clone());
    let quiver = content.item_policy_for_source_id(35562).expect("Quiver");
    let quiver_def = &quiver.record().production_definition;
    let quiver_cast = |slot| {
        let mut equipment = equipment.clone();
        let mut projectile = equipment.items.remove(1);
        projectile.slot = slot;
        equipment.items.push(equip(
            slot,
            &quiver_def.production_key,
            &quiver_def.revision_ref,
        ));
        // Equipped containers and their children have distinct item identities.
        projectile.item_instance_id = [42; 16];
        owned(equipment).with_quiver_ammunition(vec![projectile])
    };
    let loadouts = [direct, quiver_cast(10), quiver_cast(6)];
    let mut empty_quiver = equipment.clone();
    empty_quiver.items[1] = equip(10, &quiver_def.production_key, &quiver_def.revision_ref);
    let bolt = content.item_policy_for_source_id(3446).expect("bolt");
    let bolt_def = &bolt.record().production_definition;
    let mut incompatible = equip(10, &bolt_def.production_key, &bolt_def.revision_ref);
    incompatible.item_instance_id = [45; 16];
    let no_projectile = [
        ("empty", owned(empty_quiver.clone())),
        (
            "incompatible",
            owned(empty_quiver).with_quiver_ammunition(vec![incompatible]),
        ),
    ];
    let standard = content
        .item_policy_for_source_id(3447)
        .expect("standard arrow");
    let standard_def = &standard.record().production_definition;
    let Some(ProjectV2RangedWeapon::Ammunition {
        attack: standard_attack,
        ..
    }) = standard.record().authoring.ranged
    else {
        unreachable!();
    };
    let mut low_level_equipment = equipment.clone();
    let mut flaming = low_level_equipment.items.remove(1);
    flaming.item_instance_id = [43; 16];
    let mut standard = equip(10, &standard_def.production_key, &standard_def.revision_ref);
    standard.item_instance_id = [44; 16];
    let low_level = OwnedCastFacts::from_owner_reads(
        binding.clone(),
        DurableBuildState::new("paladin", (0, 0), [(100, 0); 7]).expect("paladin build"),
        15,
        low_level_equipment,
        AccessProjections::default(),
    )
    .expect("fenced low-level owner reads")
    .with_quiver_ammunition(vec![flaming, standard]);
    let lesser = content
        .spell_book()
        .spells
        .iter()
        .find(|spell| spell.key == "candidate:spell/lesser_ethereal_spear")
        .expect("Lesser Ethereal Spear");
    let damage = low_level
        .intrinsic_weapon_for_spell(lesser, &content)
        .expect("eligible standard arrow");
    assert_eq!(
        damage.total_attack(),
        u32::from(bow) + u32::from(standard_attack)
    );
    assert_eq!(damage.element, ProjectV2RangedElement::Physical);
    assert_eq!(damage.elemental_attack, 0);

    for name in [
        "lesser_ethereal_spear",
        "ethereal_spear",
        "strong_ethereal_spear",
        "ethereal_barrage",
    ] {
        let key = format!("candidate:spell/{name}");
        let spell = content
            .spell_book()
            .spells
            .iter()
            .find(|spell| spell.key == key)
            .expect("published weapon-skill cast");
        for owned in &loadouts {
            let weapon = owned
                .intrinsic_weapon_for_spell(spell, &content)
                .expect("weapon-skill callback must observe Flaming Arrow");
            assert_eq!(weapon.total_attack(), total, "{name}");
            assert_eq!(weapon.element, ProjectV2RangedElement::Fire, "{name}");
            assert_eq!(
                weapon.elemental_attack,
                u32::from(elemental_attack),
                "{name}"
            );
            let caster = owned
                .numerical_caster(&state, spell, &content, HarmonyMultiplier::ONE, 1)
                .expect("owned weapon-skill caster");
            assert_eq!(caster.attack_value, total, "{name}");
            assert!(!caster.melee_weapon, "{name}");
        }
        for (quiver, owned) in &no_projectile {
            assert!(owned.intrinsic_weapon_for_spell(spell, &content).is_none());
            let caster = owned
                .numerical_caster(&state, spell, &content, HarmonyMultiplier::ONE, 1)
                .expect("skill callback can run without a projectile");
            assert_eq!(
                (
                    caster.attack_skill,
                    caster.attack_value,
                    caster.attack_factor
                ),
                (0, 7, 0.0),
                "{name} with {quiver} quiver"
            );
            assert!(!caster.melee_weapon, "{name} with {quiver} quiver");
        }
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
