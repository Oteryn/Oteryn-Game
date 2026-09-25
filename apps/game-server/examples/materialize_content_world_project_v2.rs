use std::collections::BTreeSet;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use oteryn_game_server::content::{
    CW2_B1_FULL_ITEM_FAMILY_COUNT, CanonicalProjectDocuments, ImportBatch, ProjectDraft,
    ProjectEvidenceLimits, ProjectReferenceRecord, ProjectV2Draft, ProjectV2EditorEntry,
    ProjectV2EvidenceClass, ProjectV2Source, ProjectV2SourceIdentityBinding, ProjectV2State,
    ReferenceCells, ReferenceItemField, ReferenceItemPresentation, ReferenceItemSemantics,
    ReferenceItemWeapon, ReferenceRationalPercent, ReferenceSignedPoints,
    protected_cw2_b1_promoted_item_family_import,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

const B1_EVIDENCE: &[u8] = include_bytes!(
    "../../../docs/agents/evidence/OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json"
);
const DOCUMENT_COUNT: usize = 11;
const FULL_FAMILY_MAX_DECODED_FIELDS: usize = 2_120_000;
const FULL_FAMILY_MAX_STRING_BYTES: usize = 43_000_000;
const ITEM_SELECTED: &[u8] =
    include_bytes!("../../../docs/agents/evidence/OTV2-20260925-g4-item-exact-165-selected.json");
const ITEM_SELECTED_SHA256: &str =
    "c624a978bfc83d2dc57865126c6cda63dc21eb4134ee6eb7dc91a6b198ec945c";
const WIKI_REVISION: &str =
    "tibiawiki-item-census:389875abd364aa9bcb0b09a591989c82ece5098d63b3c23376274048f6ac2f5a";
const WIKI_CENSUS_SHA256: &str = "583a0b0080f3e08633c8d6cde11d9fd073b47088d84774bfdf851382569dd675";

fn limits() -> ProjectEvidenceLimits {
    ProjectEvidenceLimits {
        max_documents: DOCUMENT_COUNT,
        max_document_bytes: 96_000_000,
        max_total_bytes: 160_000_000,
        max_json_depth: 24,
        max_decoded_fields: FULL_FAMILY_MAX_DECODED_FIELDS,
        max_string_bytes: FULL_FAMILY_MAX_STRING_BYTES,
        max_locator_bytes: 160,
        max_locator_segments: 8,
        max_reference_records: CW2_B1_FULL_ITEM_FAMILY_COUNT,
        max_import_records: 2,
        max_reimport_states: 1,
    }
}

fn output_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut arguments = env::args_os().skip(1);
    let flag = arguments.next();
    let Some(value) = arguments.next() else {
        return Err("usage: materialize_content_world_project_v2 --output-root <path>".into());
    };
    if flag.as_deref() != Some(std::ffi::OsStr::new("--output-root")) || arguments.next().is_some()
    {
        return Err("usage: materialize_content_world_project_v2 --output-root <path>".into());
    }
    Ok(PathBuf::from(value))
}

fn require_fresh_root(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if fs::symlink_metadata(root).is_ok() {
        return Err(format!("output root already exists: {}", root.display()).into());
    }
    let parent = root
        .parent()
        .ok_or("output root must have an existing parent")?;
    let metadata = fs::symlink_metadata(parent)?;
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Err("output parent must be a real directory".into());
    }
    Ok(())
}

fn write_documents(
    root: &Path,
    documents: &CanonicalProjectDocuments,
) -> Result<String, Box<dyn std::error::Error>> {
    require_fresh_root(root)?;
    fs::create_dir(root)?;
    let mut tree = Sha256::new();
    for (locator, bytes) in documents.documents() {
        tree.update((locator.len() as u64).to_be_bytes());
        tree.update(locator.as_bytes());
        tree.update((bytes.len() as u64).to_be_bytes());
        tree.update(bytes);

        let destination = root.join(locator);
        let parent = destination
            .parent()
            .ok_or("document locator has no parent")?;
        fs::create_dir_all(parent)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = tree.finalize();
    let mut value = String::with_capacity(64);
    for byte in digest {
        value.push(char::from(HEX[usize::from(byte >> 4)]));
        value.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    Ok(value)
}

fn promote<T: PartialEq>(
    slot: &mut ReferenceItemField<T>,
    value: T,
) -> Result<bool, Box<dyn std::error::Error>> {
    match slot {
        ReferenceItemField::Unknown => {
            *slot = ReferenceItemField::Known(value);
            Ok(true)
        }
        ReferenceItemField::Known(existing) if *existing == value => Ok(false),
        _ => Err("Item candidate contradicts an existing field state".into()),
    }
}

fn presentation(
    semantics: &mut ReferenceItemSemantics,
) -> Result<&mut ReferenceItemPresentation, Box<dyn std::error::Error>> {
    if matches!(semantics.presentation, ReferenceItemField::Unknown) {
        semantics.presentation = ReferenceItemField::Known(ReferenceItemPresentation {
            name: ReferenceItemField::Unknown,
            description: ReferenceItemField::Unknown,
        });
    }
    match &mut semantics.presentation {
        ReferenceItemField::Known(value) => Ok(value),
        _ => Err("Item presentation group conflicts with source field".into()),
    }
}

fn weapon(
    semantics: &mut ReferenceItemSemantics,
) -> Result<&mut ReferenceItemWeapon, Box<dyn std::error::Error>> {
    if matches!(semantics.weapon, ReferenceItemField::Unknown) {
        semantics.weapon = ReferenceItemField::Known(ReferenceItemWeapon {
            weapon_type: ReferenceItemField::Unknown,
            attack: ReferenceItemField::Unknown,
            defense: ReferenceItemField::Unknown,
            extra_defense: ReferenceItemField::Unknown,
            range: ReferenceItemField::Unknown,
            hit_chance: ReferenceItemField::Unknown,
            max_hit_chance: ReferenceItemField::Unknown,
            ammunition: ReferenceItemField::Unknown,
            elemental: ReferenceItemField::Unknown,
        });
    }
    match &mut semantics.weapon {
        ReferenceItemField::Known(value) => Ok(value),
        _ => Err("Item weapon group conflicts with source field".into()),
    }
}

fn candidate_value<'a>(
    field: &'a Value,
    kind: &str,
) -> Result<&'a Value, Box<dyn std::error::Error>> {
    let value = &field["typed_value"];
    if value["kind"] != kind {
        return Err("Item candidate value has unexpected kind".into());
    }
    Ok(value)
}

fn apply_field(
    semantics: &mut ReferenceItemSemantics,
    field: &Value,
    title: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let path = field["field_path"]
        .as_str()
        .ok_or("Item field path missing")?;
    match path {
        "presentation.name" => {
            let value = candidate_value(field, "TEXT")?["value"]
                .as_str()
                .ok_or("Item name missing")?;
            if field["source_value"] != value || value != title {
                return Err("Item name disagrees with source page".into());
            }
            promote(&mut presentation(semantics)?.name, value.to_owned())
        }
        "weapon.attack" | "weapon.defense" | "weapon.extra_defense" => {
            let value = candidate_value(field, "SIGNED_POINTS")?["value"]
                .as_i64()
                .ok_or("Item points missing")?;
            if field["source_value"] != value {
                return Err("Item points disagree with source".into());
            }
            let value = ReferenceSignedPoints(i32::try_from(value)?);
            let weapon = weapon(semantics)?;
            match path {
                "weapon.attack" => promote(&mut weapon.attack, value),
                "weapon.defense" => promote(&mut weapon.defense, value),
                _ => promote(&mut weapon.extra_defense, value),
            }
        }
        "weapon.range_cells" => {
            let value = candidate_value(field, "CELLS")?["value"]
                .as_u64()
                .ok_or("Item range missing")?;
            if field["source_value"] != value {
                return Err("Item range disagrees with source".into());
            }
            promote(
                &mut weapon(semantics)?.range,
                ReferenceCells(u16::try_from(value)?),
            )
        }
        "weapon.hit_chance" => {
            let value = candidate_value(field, "RATIONAL_PERCENT")?;
            if value["source_unit"] != "PERCENT_POINTS" {
                return Err("Item hit chance source unit mismatch".into());
            }
            let numerator = value["numerator"]
                .as_i64()
                .ok_or("Item percent numerator missing")?;
            let denominator = value["denominator"]
                .as_u64()
                .ok_or("Item percent denominator missing")?;
            let source = field["source_value"]
                .as_i64()
                .ok_or("Item source percent missing")?;
            if i128::from(numerator) * 100 != i128::from(source) * i128::from(denominator) {
                return Err("Item percent disagrees with source".into());
            }
            promote(
                &mut weapon(semantics)?.hit_chance,
                ReferenceRationalPercent::new(numerator, denominator)?,
            )
        }
        _ => Err("Item candidate path is outside this bounded batch".into()),
    }
}

fn populate_items(
    records: &mut [ProjectReferenceRecord],
) -> Result<
    (
        ImportBatch,
        ProjectV2Source,
        Vec<ProjectV2SourceIdentityBinding>,
        Vec<ProjectV2EditorEntry>,
    ),
    Box<dyn std::error::Error>,
> {
    let selected_sha256 = Sha256::digest(ITEM_SELECTED)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if selected_sha256 != ITEM_SELECTED_SHA256 {
        return Err("selected Item input digest drifted".into());
    }
    let packet: Value = serde_json::from_slice(ITEM_SELECTED)?;
    if packet["schema"] != "oteryn-item-g4-selected-input/v1"
        || packet["source_population_sha256"]
            != "853aa2d1cc3cde0e3b77ae7d4f16eaaf21f1f63eaf45ef19e87e7ad12ba1cf16"
        || packet["source"]["source_revision"] != WIKI_REVISION
        || packet["source"]["census_sha256"] != WIKI_CENSUS_SHA256
    {
        return Err("selected Item source identity drifted".into());
    }
    let selected = packet["selected"]
        .as_array()
        .ok_or("selected Item rows missing")?;
    if selected.len() != 165 {
        return Err("selected Item count drifted".into());
    }
    let mut bindings = Vec::with_capacity(165);
    let mut editor = Vec::with_capacity(165);
    let mut target_keys = BTreeSet::new();
    let mut source_ids = BTreeSet::new();
    let mut promoted_count = 0;
    let mut equal_count = 0;
    let mut post_cut_count = 0;
    for row in selected {
        let binding: ProjectV2SourceIdentityBinding =
            serde_json::from_value(row["binding"].clone())?;
        if binding.source_key != "oteryn:source.tibiawiki"
            || binding.source_revision != WIKI_REVISION
            || binding.identity_namespace != "mediawiki/page_id"
            || format!("{:?}", binding.disposition) != "Exact"
            || format!("{:?}", binding.target.family) != "Item"
            || binding.target.revision != "definition-r1"
            || !target_keys.insert(binding.target.key.clone())
            || !source_ids.insert(binding.external_id.clone())
        {
            return Err("selected Item binding is invalid or duplicated".into());
        }
        let page = &row["page"];
        let title = page["title"].as_str().ok_or("Item page title missing")?;
        let digest = page["source_digest"]
            .as_str()
            .ok_or("Item page digest missing")?;
        let timestamp = page["revision_timestamp"]
            .as_str()
            .ok_or("Item page revision time missing")?;
        if page["page_key"] != format!("mediawiki/tibiawiki.com.br/page_id/{}", binding.external_id)
            || page["revision_id"].as_u64().unwrap_or(0) == 0
            || digest.len() != 64
            || title.is_empty()
        {
            return Err("selected Item page identity is invalid".into());
        }
        let record = records.iter_mut().find(|record| matches!(
            record,
            ProjectReferenceRecord::Item { identity, .. } if identity.key == binding.target.key && identity.revision == binding.target.revision
        )).ok_or("selected Item target is absent")?;
        let ProjectReferenceRecord::Item { semantics, .. } = record else {
            return Err("selected target is not an Item".into());
        };
        let fields = row["field_candidates"]
            .as_array()
            .ok_or("selected Item fields missing")?;
        let mut paths = BTreeSet::new();
        for field in fields {
            let path = field["field_path"]
                .as_str()
                .ok_or("Item field path absent")?;
            if !paths.insert(path)
                || field["source_external_id"] != binding.external_id
                || field["source_revision"] != WIKI_REVISION
                || field["source_page_digest"] != digest
                || field["source_page_revision_id"] != page["revision_id"]
                || field["target"] != row["binding"]["target"]
                || field["promotion_state"] != "FIELD_VERIFIED_CANDIDATE_NOT_APPLIED"
            {
                return Err("Item field/source binding mismatch".into());
            }
            if timestamp > "2026-07-28T23:59:59Z" {
                // The sole later revision has fields already present in the protected cut.
                if apply_field(semantics, field, title)? {
                    return Err("post-cut Item field cannot be promoted".into());
                }
                post_cut_count += 1;
            } else if apply_field(semantics, field, title)? {
                promoted_count += 1;
            } else {
                equal_count += 1;
            }
        }
        editor.push(ProjectV2EditorEntry {
            target: binding.target.clone(),
            display_name: title.to_owned(),
            description: String::new(),
            categories: Vec::new(),
            notes: Vec::new(),
            aliases: Vec::new(),
            tags: vec!["oteryn:editor.item".to_owned()],
        });
        bindings.push(binding);
    }
    if promoted_count != 526 || equal_count != 32 || post_cut_count != 3 {
        return Err(format!("Item field partition drifted: promoted={promoted_count} equal={equal_count} post_cut={post_cut_count}").into());
    }
    let import = ImportBatch {
        batch_id: "g4-item-exact-165-tibiawiki-r1".to_owned(),
        source_repository: "tibiawiki.com.br".to_owned(),
        source_revision: WIKI_REVISION.to_owned(),
        source_artifact_sha256: WIKI_CENSUS_SHA256.to_owned(),
        access_disposition: "PENDING".to_owned(),
        source_generation_profile: "OTERYN_ITEM_CURRENT_SOURCE_TIBIAWIKI_MANIFEST/v1".to_owned(),
        importer: "OTERYN_G4_ITEM_BULK_POPULATION/v1".to_owned(),
        mapper: "OTERYN_G4_ITEM_BULK_POPULATION/v1".to_owned(),
        mapper_revision: "71247200039d6894e09bf9310fd87694ca85b9c7".to_owned(),
        mapper_sha256: "d5ac8e924cf384fbd74027f4477f4f086f7f9df2514946a5708bb567c9d8d92c"
            .to_owned(),
        candidates: Vec::new(),
        reimport_states: Vec::new(),
    };
    let source = ProjectV2Source {
        key: "oteryn:source.tibiawiki".to_owned(),
        import_batch_id: import.batch_id.clone(),
        revision: import.source_revision.clone(),
        sha256: import.source_artifact_sha256.clone(),
        evidence: ProjectV2EvidenceClass::Derived,
    };
    Ok((import, source, bindings, editor))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = output_root()?;
    let promoted = protected_cw2_b1_promoted_item_family_import(B1_EVIDENCE)?;
    if promoted.family.records.len() != CW2_B1_FULL_ITEM_FAMILY_COUNT {
        return Err("protected promoted Item family count drifted".into());
    }
    let mut provenance = promoted.family.batch;
    provenance.candidates.clear();
    provenance.reimport_states.clear();
    let source = ProjectV2Source {
        key: "oteryn:source.crystalserver".to_owned(),
        import_batch_id: provenance.batch_id.clone(),
        revision: provenance.source_revision.clone(),
        sha256: provenance.source_artifact_sha256.clone(),
        evidence: ProjectV2EvidenceClass::OtsHypothesisOnly,
    };
    let mut records = promoted.family.records;
    let (wiki_import, wiki_source, bindings, editor) = populate_items(&mut records)?;
    let documents = CanonicalProjectDocuments::from_v2_draft(
        ProjectV2Draft {
            core: ProjectDraft {
                project_revision: "g4-item-exact-165-r1".to_owned(),
                package_key: "oteryn:content.world-project".to_owned(),
                semantic_schema_version: "reference-schema-v1".to_owned(),
                licensing_metadata: "PENDING".to_owned(),
                world_id: "0123456789ab70cd8ef0123456789abc".to_owned(),
                coordinate_frame: "global-target-2026-07-28".to_owned(),
                records,
                imports: vec![provenance, wiki_import],
                metadata: Vec::new(),
            },
            state: ProjectV2State {
                sources: vec![source, wiki_source],
                source_identity_bindings: bindings,
                editor,
                ..ProjectV2State::default()
            },
        },
        limits(),
    )?;
    if documents.documents().len() != DOCUMENT_COUNT {
        return Err("canonical WorldProject/v2 document count drifted".into());
    }
    let tree_sha256 = write_documents(&root, &documents)?;
    println!(
        "documents={DOCUMENT_COUNT} items={CW2_B1_FULL_ITEM_FAMILY_COUNT} promoted_items={} promoted_fields={} wiki_bindings=165 wiki_fields=526 tree_sha256={tree_sha256}",
        promoted.promoted_items, promoted.promoted_fields
    );
    Ok(())
}
