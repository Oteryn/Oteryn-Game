//! Wires the pinned #1018 v1 lowering candidate
//! (`docs/agents/evidence/OTV2-20260928-item-promotion-lowering-v1.json`,
//! `protected_cw2_b1_item_semantic_promotion_lowering_v1_import`) through the same
//! production project draft/canonicalize/link path
//! `content_world_cw2_b1_import.rs` already exercises for the protected full Item
//! family, proving the promoted fields actually compile into the existing Reference
//! artifact (not merely a JSON manifest).
#![allow(clippy::expect_used, clippy::panic)]

use oteryn_game_server::content::*;
use std::collections::BTreeMap;

const B1_EVIDENCE: &[u8] = include_bytes!(
    "../../../docs/agents/evidence/OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json"
);

fn lowering_limits() -> ProjectEvidenceLimits {
    ProjectEvidenceLimits {
        max_documents: 8,
        max_document_bytes: 128_000_000,
        max_total_bytes: 220_000_000,
        max_json_depth: 24,
        max_decoded_fields: 3_000_000,
        max_string_bytes: 128_000_000,
        max_locator_bytes: 160,
        max_locator_segments: 8,
        max_reference_records: CW2_B1_FULL_ITEM_FAMILY_COUNT,
        max_import_records: CW2_B1_FULL_ITEM_FAMILY_COUNT,
        max_reimport_states: CW2_B1_FULL_ITEM_FAMILY_COUNT,
    }
}

/// The identity/materializable/stack_class shape to compare the lowering-v1 import
/// against. This must be the *renamed* full family (source item `3031` already moved
/// to `oteryn:item.currency.gold_coin`), since the #1018 lowering candidate's own
/// population census resolves that same current identity, not the plain opaque
/// allocation. Its `.semantics` (which also carries the unrelated existing 69-field
/// promotion) is intentionally never read here — only identity/materializable/
/// stack_class are compared.
fn base_family_import() -> ProtectedCw2B1FullItemFamilyImport {
    protected_r7_p04_gold_coin_item_family_import(B1_EVIDENCE, R7_P04_GOLD_COIN_EVIDENCE_PACKET)
        .expect("protected B1 gold-coin-renamed full Item family")
        .family
}

fn lowering_v1_import() -> ProtectedCw2B1PromotedItemFamilyImport {
    protected_cw2_b1_item_semantic_promotion_lowering_v1_import(B1_EVIDENCE)
        .expect("protected B1 item semantic promotion lowering v1 import")
}

fn lowering_draft(imported: ProtectedCw2B1PromotedItemFamilyImport) -> ProjectDraft {
    ProjectDraft {
        project_revision: "project-r1".to_owned(),
        package_key: "oteryn:content.world-project".to_owned(),
        semantic_schema_version: "reference-schema-v1".to_owned(),
        licensing_metadata: "PENDING".to_owned(),
        world_id: "0123456789ab70cd8ef0123456789abc".to_owned(),
        coordinate_frame: "global-target-2026-09-27".to_owned(),
        records: imported.family.records,
        imports: vec![imported.family.batch],
        metadata: Vec::new(),
    }
}

fn candidate_binding(candidate: &ImportCandidate) -> &NativeItemBindingDocument {
    candidate
        .normalized_fields
        .iter()
        .find_map(|field| match &field.value {
            CandidateValue::NativeItemBinding(binding) => Some(binding),
            _ => None,
        })
        .expect("typed native item binding")
}

fn native_key_for_source_item(batch: &ImportBatch, source_item_id: u64) -> String {
    batch
        .candidates
        .iter()
        .find(|candidate| candidate.source_numeric_id == Some(source_item_id))
        .map(|candidate| candidate_binding(candidate).identity.key.clone())
        .unwrap_or_else(|| panic!("no candidate binds source item {source_item_id}"))
}

/// Count the exact atoms `apply_item_semantic_promotion` can ever set, and confirm no
/// other semantics group carries a value (this lowering packet's 9 field paths are a
/// strict subset of `ReferenceItemSemantics`).
fn count_promoted_atoms(semantics: &ReferenceItemSemantics) -> usize {
    use ReferenceItemField::{Known, Unknown};

    let mut count = 0_usize;
    if let Known(presentation) = &semantics.presentation {
        if matches!(&presentation.name, Known(_)) {
            count += 1;
        }
        assert!(matches!(&presentation.description, Unknown));
    }
    if let Known(weapon) = &semantics.weapon {
        if matches!(&weapon.attack, Known(_)) {
            count += 1;
        }
        if matches!(&weapon.defense, Known(_)) {
            count += 1;
        }
        if matches!(&weapon.extra_defense, Known(_)) {
            count += 1;
        }
        if matches!(&weapon.range, Known(_)) {
            count += 1;
        }
        if matches!(&weapon.hit_chance, Known(_)) {
            count += 1;
        }
        assert!(matches!(&weapon.weapon_type, Unknown));
        assert!(matches!(&weapon.max_hit_chance, Unknown));
        assert!(matches!(&weapon.ammunition, Unknown));
        assert!(matches!(&weapon.elemental, Unknown));
    }
    if let Known(protection) = &semantics.protection {
        if matches!(&protection.armor, Known(_)) {
            count += 1;
        }
        assert!(matches!(&protection.resistances, Unknown));
    }
    if let Known(charges) = &semantics.charges
        && matches!(&charges.count, Known(_))
    {
        count += 1;
    }
    if let Known(container) = &semantics.container
        && matches!(&container.capacity, Known(_))
    {
        count += 1;
    }
    assert!(matches!(&semantics.classification, Unknown));
    assert!(matches!(&semantics.physical, Unknown));
    assert!(matches!(&semantics.stack, Unknown));
    assert!(matches!(&semantics.equipment, Unknown));
    assert!(matches!(&semantics.skill_modifiers, Unknown));
    assert!(matches!(&semantics.temporal, Unknown));
    assert!(matches!(&semantics.imbuement, Unknown));
    assert!(matches!(&semantics.use_transform, Unknown));
    assert!(matches!(&semantics.trade_restrictions, Unknown));
    assert!(matches!(&semantics.fluid, Unknown));
    assert!(matches!(&semantics.readable_writeable, Unknown));
    count
}

#[test]
fn lowering_v1_packet_bytes_are_pinned() {
    assert_eq!(
        ITEM_SEMANTIC_PROMOTION_LOWERING_V1_PACKET.len(),
        ITEM_SEMANTIC_PROMOTION_LOWERING_V1_PACKET_BYTES
    );
    assert_eq!(
        world_project_sha256(ITEM_SEMANTIC_PROMOTION_LOWERING_V1_PACKET),
        ITEM_SEMANTIC_PROMOTION_LOWERING_V1_PACKET_SHA256
    );
}

#[test]
fn lowering_v1_promotes_exactly_14643_atoms_across_12021_items_without_identity_or_materialization_drift()
 {
    let base = base_family_import();
    let promoted = lowering_v1_import();

    assert_eq!(
        promoted.promoted_fields,
        ITEM_SEMANTIC_PROMOTION_LOWERING_V1_FIELD_COUNT
    );
    assert_eq!(
        promoted.promoted_items,
        ITEM_SEMANTIC_PROMOTION_LOWERING_V1_ITEM_COUNT
    );
    assert_eq!(promoted.family.records.len(), CW2_B1_FULL_ITEM_FAMILY_COUNT);
    assert_eq!(
        promoted.family.allocation_digest_sha256, base.allocation_digest_sha256,
        "lowering v1 promotion must not change the protected identity allocation"
    );

    let base_shape = base
        .records
        .iter()
        .map(|record| {
            let ProjectReferenceRecord::Item {
                identity,
                materializable,
                stack_class,
                ..
            } = record
            else {
                panic!("full family contains only Items");
            };
            (identity.key.clone(), (*materializable, *stack_class))
        })
        .collect::<BTreeMap<_, _>>();

    let mut atom_count = 0_usize;
    let mut promoted_items = 0_usize;
    for record in &promoted.family.records {
        let ProjectReferenceRecord::Item {
            identity,
            materializable,
            stack_class,
            semantics,
            ..
        } = record
        else {
            panic!("promoted full family contains only Items");
        };
        assert_eq!(
            base_shape.get(&identity.key),
            Some(&(*materializable, *stack_class)),
            "lowering v1 promotion changed materializable/stack shape for {}",
            identity.key
        );
        let atoms = count_promoted_atoms(semantics);
        atom_count += atoms;
        promoted_items += usize::from(atoms > 0);
    }

    assert_eq!(atom_count, ITEM_SEMANTIC_PROMOTION_LOWERING_V1_FIELD_COUNT);
    assert_eq!(
        promoted_items,
        ITEM_SEMANTIC_PROMOTION_LOWERING_V1_ITEM_COUNT
    );

    // Self-check triple from `lower_promotion_packet.py::self_check`, resolved by
    // exact source item id rather than a hardcoded opaque registry key.
    let by_key = promoted
        .family
        .records
        .iter()
        .map(|record| {
            let ProjectReferenceRecord::Item {
                identity,
                semantics,
                ..
            } = record
            else {
                panic!("promoted full family contains only Items");
            };
            (identity.key.as_str(), semantics)
        })
        .collect::<BTreeMap<_, _>>();

    let magic_sword_key = native_key_for_source_item(&promoted.family.batch, 3288);
    let magic_sword = by_key
        .get(magic_sword_key.as_str())
        .expect("magic sword record");
    let ReferenceItemField::Known(presentation) = &magic_sword.presentation else {
        panic!("magic sword presentation unset");
    };
    assert_eq!(
        presentation.name,
        ReferenceItemField::Known("magic sword".to_owned())
    );
    let ReferenceItemField::Known(weapon) = &magic_sword.weapon else {
        panic!("magic sword weapon unset");
    };
    assert_eq!(
        weapon.attack,
        ReferenceItemField::Known(ReferenceSignedPoints(48))
    );
    assert_eq!(
        weapon.defense,
        ReferenceItemField::Known(ReferenceSignedPoints(35))
    );

    let container_key = native_key_for_source_item(&promoted.family.batch, 116);
    let container = by_key
        .get(container_key.as_str())
        .expect("container record");
    let ReferenceItemField::Known(container_semantics) = &container.container else {
        panic!("container capacity unset");
    };
    assert_eq!(container_semantics.capacity, ReferenceItemField::Known(15));

    let charges_key = native_key_for_source_item(&promoted.family.batch, 814);
    let charges = by_key.get(charges_key.as_str()).expect("charges record");
    let ReferenceItemField::Known(charges_semantics) = &charges.charges else {
        panic!("charges count unset");
    };
    assert_eq!(charges_semantics.count, ReferenceItemField::Known(200));
}

#[test]
fn lowering_v1_compiles_into_the_reference_artifact_v4_path_and_survives_round_trip() {
    let promoted = lowering_v1_import();
    let magic_sword_key = native_key_for_source_item(&promoted.family.batch, 3288);

    let documents =
        CanonicalProjectDocuments::from_draft(lowering_draft(promoted), lowering_limits())
            .expect("canonical lowering-v1 project");
    let snapshot = documents
        .into_snapshot(lowering_limits())
        .expect("lowering-v1 snapshot");
    let project = snapshot
        .parse(lowering_limits())
        .expect("parsed lowering-v1 project");
    let linked = project.link().expect("linked lowering-v1 Items");
    assert_eq!(linked.definitions.len(), CW2_B1_FULL_ITEM_FAMILY_COUNT);

    let promoted_atom_total: usize = linked
        .definitions
        .iter()
        .map(|definition| {
            let ReferenceDefinitionKind::Item(item) = &definition.kind else {
                panic!("Reference artifact requires typed Item definitions");
            };
            count_promoted_atoms(&item.semantics)
        })
        .sum();
    assert_eq!(
        promoted_atom_total,
        ITEM_SEMANTIC_PROMOTION_LOWERING_V1_FIELD_COUNT
    );

    let magic_sword = linked
        .definitions
        .iter()
        .find(|definition| definition.definition.key().as_str() == magic_sword_key)
        .expect("magic sword linked definition");
    let ReferenceDefinitionKind::Item(item) = &magic_sword.kind else {
        panic!("magic sword linked as non-Item");
    };
    assert!(
        !item.semantics.is_all_unknown(),
        "magic sword must carry typed semantics through link()"
    );

    // This population carries typed, non-Unknown Item semantics, so the production
    // Reference compiler auto-selects the typed v4 artifact profile
    // (`ReferenceArtifactProfile::for_source`) rather than the identity-only v3 one.
    let first = compile_reference_playable(&linked).expect("lowering-v1 typed-v4 artifact");
    let second = compile_reference_playable(&linked).expect("deterministic lowering-v1 artifact");
    assert_eq!(first.server_artifact, second.server_artifact);
    assert_eq!(first.client_artifact, second.client_artifact);
}
