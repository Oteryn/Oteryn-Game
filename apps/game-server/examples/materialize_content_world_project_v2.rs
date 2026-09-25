use std::collections::BTreeSet;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use oteryn_game_server::content::{
    CW2_B1_FULL_ITEM_FAMILY_COUNT, CanonicalProjectDocuments, ImportBatch, ProjectDraft,
    ProjectEvidenceLimits, ProjectReferenceRecord, ProjectV2Declaration, ProjectV2DefinitionRef,
    ProjectV2Draft, ProjectV2EditorEntry, ProjectV2EvidenceClass, ProjectV2Family,
    ProjectV2Identity, ProjectV2ItemAuthoring, ProjectV2ItemForgeProfile, ProjectV2ItemLifecycle,
    ProjectV2ItemSourceLifecycle, ProjectV2ItemTaxonomy, ProjectV2Source,
    ProjectV2SourceIdentityBinding, ProjectV2SourceIdentityDisposition, ProjectV2State,
    ReferenceCells, ReferenceItemField, ReferenceItemImbuement, ReferenceItemPresentation,
    ReferenceItemSemantics, ReferenceItemStack, ReferenceItemTradeRestrictions,
    ReferenceItemWeapon, ReferenceRationalPercent, ReferenceSignedPoints, ReferenceWeaponType,
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
const ITEM_WAVE1_STAGED: &[u8] =
    include_bytes!("../../../docs/agents/evidence/OTV2-20260925-item-enrichment-wave1-staged.json");
const ITEM_WAVE1_STAGED_SHA256: &str =
    "00f2acd441e9146bdd7f67821bef446c171ea67cd06f666028324b61e544cb95";
const ITEM_WAVE1_SNAPSHOT_SHA256: &str =
    "5d8b84eee85e226e99d516beb7b40b8dc201c923e9b63b5ef18313085c3cbdf5";
const ITEM_WAVE1_STAGE_TOOL_SHA256: &str =
    "55636673ba3acce7e5276243d38701ec30de9ddfd6ec644da1934b5772bc3d4b";
const ITEM_WAVE1_ITEMS: usize = 164;
const ITEM_WAVE1_DEFINITION_FACTS: usize = 290;
const MOUNT_SELECTED: &[u8] =
    include_bytes!("../../../docs/agents/evidence/OTV2-20260925-g4-mount-252-selected.json");
const MOUNT_SELECTED_SHA256: &str =
    "31201df8f737a1e22ed719152deb98a9825e6b3f196d7ee8e3109dd1759067b0";
const MOUNT_SOURCE_REVISION: &str = "tibiawiki-nonitem-g1-snapshot:0b7caf98940305a91c5384dfb828c0afcf016f087f71572ec71d7c936c6387df";
const MOUNT_SOURCE_SHA256: &str =
    "f47dbe5832e7b1accd652852303d638a4f19367bab3951f260cc39a0d93b7713";
const MOUNT_CROSSWALK_SHA256: &str =
    "38d827ba66bb7a04a3f5a94bbb873ca4485d4b7c873de957812a9c1be20a67c5";
const OUTFIT_SELECTED: &[u8] =
    include_bytes!("../../../docs/agents/evidence/OTV2-20260925-g4-outfit-133-selected.json");
const OUTFIT_SELECTED_SHA256: &str =
    "4cb19c97ca4047fe79ca4033af7e99344cdaaca9898ad7c76bea5f77742b0a31";
const OUTFIT_CROSSWALK_SHA256: &str =
    "1d3c8944bf68c63814942578ac07fe232eff900d6d261476d46bb742e64c5396";
const OUTFIT_SOURCE_REVISION: &str = MOUNT_SOURCE_REVISION;
const OUTFIT_SOURCE_SHA256: &str = MOUNT_SOURCE_SHA256;

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
        max_import_records: 4,
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

struct ItemPopulation {
    import: ImportBatch,
    source: ProjectV2Source,
    bindings: Vec<ProjectV2SourceIdentityBinding>,
    editor: Vec<ProjectV2EditorEntry>,
}

fn populate_items(
    records: &mut [ProjectReferenceRecord],
) -> Result<ItemPopulation, Box<dyn std::error::Error>> {
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
    Ok(ItemPopulation {
        import,
        source,
        bindings,
        editor,
    })
}

fn hex_sha256(payload: &[u8]) -> String {
    Sha256::digest(payload)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn wave1_group<T>(
    slot: &mut ReferenceItemField<T>,
    empty: impl FnOnce() -> T,
) -> Result<&mut T, Box<dyn std::error::Error>> {
    if matches!(slot, ReferenceItemField::Unknown) {
        *slot = ReferenceItemField::Known(empty());
    }
    match slot {
        ReferenceItemField::Known(value) => Ok(value),
        _ => Err("Item Wave 1 group conflicts with its source field".into()),
    }
}

fn apply_wave1_fact(
    semantics: &mut ReferenceItemSemantics,
    fact: &Value,
) -> Result<bool, Box<dyn std::error::Error>> {
    let value = &fact["value"];
    match fact["field_path"]
        .as_str()
        .ok_or("Wave 1 fact path missing")?
    {
        "weapon.weapon_type" => {
            let weapon_type = match value.as_str().ok_or("Wave 1 weapon type missing")? {
                "AXE" => ReferenceWeaponType::Axe,
                "CLUB" => ReferenceWeaponType::Club,
                "DISTANCE" => ReferenceWeaponType::Distance,
                "FIST" => ReferenceWeaponType::Fist,
                "SWORD" => ReferenceWeaponType::Sword,
                _ => return Err("Wave 1 weapon type is outside the source mapping".into()),
            };
            promote(&mut weapon(semantics)?.weapon_type, weapon_type)
        }
        "imbuement.slot_count" => {
            let slots = u8::try_from(value.as_u64().ok_or("Wave 1 imbuement slots missing")?)?;
            let group = wave1_group(&mut semantics.imbuement, || ReferenceItemImbuement {
                slot_count: ReferenceItemField::Unknown,
                allowed_family_tiers: ReferenceItemField::Unknown,
                excluded_families: ReferenceItemField::Unknown,
            })?;
            promote(&mut group.slot_count, slots)
        }
        "stack.stackable" => {
            if value.as_bool() != Some(false) {
                return Err("Wave 1 promotes only stackable=false".into());
            }
            let group = wave1_group(&mut semantics.stack, || ReferenceItemStack {
                stackable: ReferenceItemField::Unknown,
                stack_max: ReferenceItemField::Unknown,
            })?;
            promote(&mut group.stackable, false)
        }
        "trade_restrictions.marketable" => {
            let marketable = value.as_bool().ok_or("Wave 1 marketable missing")?;
            let group = wave1_group(&mut semantics.trade_restrictions, || {
                ReferenceItemTradeRestrictions {
                    tradeable: ReferenceItemField::Unknown,
                    marketable: ReferenceItemField::Unknown,
                    vocations: ReferenceItemField::Unknown,
                    account_binding_policy: ReferenceItemField::Unknown,
                    character_binding_policy: ReferenceItemField::Unknown,
                }
            })?;
            promote(&mut group.marketable, marketable)
        }
        _ => Err("Wave 1 fact path is outside this bounded batch".into()),
    }
}

fn wave1_text(value: &Value) -> Result<Option<String>, Box<dyn std::error::Error>> {
    match value {
        Value::Null => Ok(None),
        Value::String(text) => Ok(Some(text.clone())),
        _ => Err("Wave 1 authoring text is not a string".into()),
    }
}

fn wave1_authoring(
    item: ProjectV2DefinitionRef,
    authoring: &Value,
) -> Result<ProjectV2ItemAuthoring, Box<dyn std::error::Error>> {
    let taxonomy = match &authoring["taxonomy"] {
        Value::Null => None,
        value => Some(ProjectV2ItemTaxonomy {
            primary: wave1_text(&value["primary"])?.ok_or("Wave 1 taxonomy primary missing")?,
            secondary: wave1_text(&value["secondary"])?,
            tertiary: wave1_text(&value["tertiary"])?,
        }),
    };
    let forge = match &authoring["forge"] {
        Value::Null => None,
        value => Some(ProjectV2ItemForgeProfile {
            classification: u8::try_from(
                value["classification"]
                    .as_u64()
                    .ok_or("Wave 1 Forge class missing")?,
            )?,
            max_tier: u8::try_from(
                value["max_tier"]
                    .as_u64()
                    .ok_or("Wave 1 max tier missing")?,
            )?,
        }),
    };
    let lifecycle = match &authoring["lifecycle"] {
        Value::Null => None,
        value => Some(ProjectV2ItemLifecycle {
            enchantable: Some(
                value["enchantable"]
                    .as_bool()
                    .ok_or("Wave 1 enchantable missing")?,
            ),
            ..ProjectV2ItemLifecycle::default()
        }),
    };
    let source_lifecycle = match &authoring["source_lifecycle"] {
        Value::Null => None,
        value => Some(ProjectV2ItemSourceLifecycle {
            implemented: wave1_text(&value["implemented"])?,
            removed: wave1_text(&value["removed"])?,
        }),
    };
    Ok(ProjectV2ItemAuthoring {
        item,
        presentation: None,
        document: None,
        taxonomy,
        forge,
        proficiency: None,
        augments: Vec::new(),
        on_use_interactions: Vec::new(),
        use_ability: None,
        required_magic_level: None,
        consumable: None,
        use_observation: None,
        lifecycle,
        source_lifecycle,
    })
}

struct ItemWave1Population {
    import: ImportBatch,
    source: ProjectV2Source,
    authoring: Vec<ProjectV2ItemAuthoring>,
    promoted: usize,
}

fn populate_item_wave1(
    records: &mut [ProjectReferenceRecord],
    bindings: &[ProjectV2SourceIdentityBinding],
) -> Result<ItemWave1Population, Box<dyn std::error::Error>> {
    if hex_sha256(ITEM_WAVE1_STAGED) != ITEM_WAVE1_STAGED_SHA256 {
        return Err("staged Item Wave 1 input digest drifted".into());
    }
    let packet: Value = serde_json::from_slice(ITEM_WAVE1_STAGED)?;
    if packet["schema"] != "OTERYN_G4_ITEM_WAVE1_STAGED/v1"
        || packet["batch_id"] != "g4-item-wave1-tibiawiki-r1"
        || packet["source"]["source_revision"] != WIKI_REVISION
        || packet["source"]["snapshot_sha256"] != ITEM_WAVE1_SNAPSHOT_SHA256
        || packet["counts"]["conflicts"] != 0
    {
        return Err("staged Item Wave 1 source identity drifted".into());
    }
    let items = packet["items"].as_array().ok_or("Wave 1 items missing")?;
    if items.len() != ITEM_WAVE1_ITEMS {
        return Err("staged Item Wave 1 count drifted".into());
    }
    let mut authoring = Vec::with_capacity(items.len());
    let mut promoted = 0;
    let mut facts_seen = 0;
    for item in items {
        let target: ProjectV2DefinitionRef = serde_json::from_value(item["target"].clone())?;
        let external_id = item["external_id"]
            .as_str()
            .ok_or("Wave 1 source id missing")?;
        if !bindings.iter().any(|binding| {
            binding.target == target
                && binding.external_id == external_id
                && binding.source_revision == WIKI_REVISION
        }) {
            return Err("Wave 1 target has no protected EXACT source binding".into());
        }
        let record = records.iter_mut().find(|record| matches!(
            record,
            ProjectReferenceRecord::Item { identity, .. } if identity.key == target.key && identity.revision == target.revision
        )).ok_or("Wave 1 target is absent")?;
        let ProjectReferenceRecord::Item { semantics, .. } = record else {
            return Err("Wave 1 target is not an Item".into());
        };
        for fact in item["facts"].as_array().ok_or("Wave 1 facts missing")? {
            facts_seen += 1;
            promoted += usize::from(apply_wave1_fact(semantics, fact)?);
        }
        authoring.push(wave1_authoring(target, &item["authoring"])?);
    }
    if facts_seen != ITEM_WAVE1_DEFINITION_FACTS {
        return Err("staged Item Wave 1 fact count drifted".into());
    }
    authoring.sort_by(|left, right| left.item.cmp(&right.item));
    let import = ImportBatch {
        batch_id: "g4-item-wave1-tibiawiki-r1".to_owned(),
        source_repository: "tibiawiki.com.br".to_owned(),
        source_revision: format!("tibiawiki-item-wave1-snapshot:{ITEM_WAVE1_SNAPSHOT_SHA256}"),
        source_artifact_sha256: ITEM_WAVE1_SNAPSHOT_SHA256.to_owned(),
        access_disposition: "PENDING".to_owned(),
        source_generation_profile: "OTERYN_G4_ITEM_WAVE1_SOURCE_SNAPSHOT/v1".to_owned(),
        importer: "OTERYN_G4_ITEM_WAVE1_CAPTURE/v1".to_owned(),
        mapper: "OTERYN_G4_ITEM_WAVE1_STAGE/v1".to_owned(),
        mapper_revision: "item-wave1-r1".to_owned(),
        mapper_sha256: ITEM_WAVE1_STAGE_TOOL_SHA256.to_owned(),
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
    Ok(ItemWave1Population {
        import,
        source,
        authoring,
        promoted,
    })
}

struct MountPopulation {
    import: ImportBatch,
    source: ProjectV2Source,
    declarations: Vec<ProjectV2Declaration>,
    bindings: Vec<ProjectV2SourceIdentityBinding>,
    editor: Vec<ProjectV2EditorEntry>,
}

fn mount_key(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    if name.is_empty() || !name.is_ascii() {
        return Err("Mount name cannot produce an ASCII canonical key".into());
    }
    let mut slug = String::new();
    for byte in name.bytes() {
        if byte.is_ascii_alphanumeric() {
            slug.push(char::from(byte.to_ascii_lowercase()));
        } else if !slug.is_empty() && !slug.ends_with('_') {
            slug.push('_');
        }
    }
    let slug = slug.trim_end_matches('_');
    if slug.is_empty() {
        return Err("Mount name has no canonical key characters".into());
    }
    Ok(format!("oteryn:content.mount.{slug}"))
}

fn populate_mounts(
    records: &[ProjectReferenceRecord],
    item_bindings: &[ProjectV2SourceIdentityBinding],
) -> Result<MountPopulation, Box<dyn std::error::Error>> {
    let selected_sha256 = Sha256::digest(MOUNT_SELECTED)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if selected_sha256 != MOUNT_SELECTED_SHA256 {
        return Err("selected Mount input digest drifted".into());
    }
    let packet: Value = serde_json::from_slice(MOUNT_SELECTED)?;
    let source = &packet["source"];
    if packet["schema"] != "OTERYN_G4_MOUNT_252_SELECTED/v1"
        || source["source_key"] != "oteryn:source.tibiawiki"
        || source["source_revision"] != MOUNT_SOURCE_REVISION
        || source["source_capture_sha256"] != MOUNT_SOURCE_SHA256
        || source["crosswalk_sha256"] != MOUNT_CROSSWALK_SHA256
        || source["source_capture_artifact_id"] != 10831362943_u64
        || source["crosswalk_artifact_id"] != 10848111721_u64
        || source["exact_raw_source_tuple_matches"] != 252_u64
        || packet["population_policy"]["typed_speed_bonus"] != "UNKNOWN"
        || packet["population_policy"]["typed_premium"] != "UNKNOWN"
    {
        return Err("selected Mount source or field policy drifted".into());
    }
    let selected = packet["selected"].as_array().ok_or("Mount rows missing")?;
    if selected.len() != 252 {
        return Err("Mount selection count drifted".into());
    }
    let mut declarations = Vec::with_capacity(252);
    let mut bindings = Vec::with_capacity(252);
    let mut editor = Vec::with_capacity(252);
    let mut keys = BTreeSet::new();
    let mut page_ids = BTreeSet::new();
    let item_keys = records
        .iter()
        .filter_map(|record| match record {
            ProjectReferenceRecord::Item { identity, .. } => Some(identity.key.as_str()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    for row in selected {
        let page_id = row["external_id"].as_str().ok_or("Mount page ID missing")?;
        let parsed_id: u64 = page_id.parse()?;
        let title = row["display_name"].as_str().ok_or("Mount title missing")?;
        let key = mount_key(title)?;
        let revision = row["current_revision_id"].as_u64().unwrap_or(0);
        let timestamp = row["current_revision_timestamp"]
            .as_str()
            .ok_or("Mount page timestamp missing")?;
        let digest = row["current_raw_utf8_sha256"]
            .as_str()
            .ok_or("Mount page digest missing")?;
        if parsed_id == 0
            || page_id != parsed_id.to_string()
            || revision == 0
            || timestamp.len() != 20
            || !timestamp.ends_with('Z')
            || timestamp > "2026-07-28T23:59:59Z"
            || !timestamp
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'-' | b':' | b'T' | b'Z'))
            || digest.len() != 64
            || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
            || row["canonical_key_proposal"] != key
            || row["canonical_revision"] != "definition-r1"
            || row["source_key"] != "oteryn:source.tibiawiki"
            || row["identity_namespace"] != "mediawiki/page_id"
            || row["source_state"] != "SOURCE_REVISION_STABLE"
            || row["concept_resolution"] != "UNIQUE_MULTI_SIGNAL_SOURCE_CONCEPT_CANDIDATE"
            || row["structured_fields"]["name"] != title
            || row["structured_fields"]
                .as_object()
                .map(|fields| fields.len())
                != Some(3)
            || !keys.insert(key.clone())
            || !page_ids.insert(page_id.to_owned())
            || item_keys.contains(key.as_str())
            || item_bindings.iter().any(|binding| {
                binding.identity_namespace == "mediawiki/page_id" && binding.external_id == page_id
            })
        {
            return Err("Mount identity, provenance or collision check failed".into());
        }
        let target = ProjectV2DefinitionRef {
            family: ProjectV2Family::Mount,
            key: key.clone(),
            revision: "definition-r1".to_owned(),
        };
        declarations.push(ProjectV2Declaration::Mount {
            identity: ProjectV2Identity {
                key,
                revision: target.revision.clone(),
            },
            presentation: None,
            speed_bonus: None,
            premium: None,
            taming_item: None,
            acquisition_interactions: Vec::new(),
            fields: Vec::new(),
        });
        bindings.push(ProjectV2SourceIdentityBinding {
            source_key: "oteryn:source.tibiawiki".to_owned(),
            source_revision: MOUNT_SOURCE_REVISION.to_owned(),
            identity_namespace: "mediawiki/page_id".to_owned(),
            external_id: page_id.to_owned(),
            target: target.clone(),
            disposition: ProjectV2SourceIdentityDisposition::Exact,
        });
        editor.push(ProjectV2EditorEntry {
            target,
            display_name: title.to_owned(),
            description: String::new(),
            categories: Vec::new(),
            notes: Vec::new(),
            aliases: Vec::new(),
            tags: vec!["oteryn:editor.mount".to_owned()],
        });
    }
    let import = ImportBatch {
        batch_id: "g4-mount-252-tibiawiki-r1".to_owned(),
        source_repository: "tibiawiki.com.br".to_owned(),
        source_revision: MOUNT_SOURCE_REVISION.to_owned(),
        source_artifact_sha256: MOUNT_SOURCE_SHA256.to_owned(),
        access_disposition: "PENDING".to_owned(),
        source_generation_profile: "OTERYN_G4_NON_ITEM_SOURCE_CAPTURE/v1".to_owned(),
        importer: "OTERYN_G4_NONITEM_BULK_CROSSWALK/v1".to_owned(),
        mapper: "OTERYN_G4_NONITEM_BULK_CROSSWALK/v1".to_owned(),
        mapper_revision: "a3cc319aa4582c90cd9d03efd81e2ab0cabf1bb0".to_owned(),
        mapper_sha256: "d927b2bebf3616f8c6e716ad32018698edd323d745078363448eb3f933c7f2d2"
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
    Ok(MountPopulation {
        import,
        source,
        declarations,
        bindings,
        editor,
    })
}

struct OutfitPopulation {
    declarations: Vec<ProjectV2Declaration>,
    bindings: Vec<ProjectV2SourceIdentityBinding>,
    editor: Vec<ProjectV2EditorEntry>,
}

fn outfit_key(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    if name.is_empty() || !name.is_ascii() {
        return Err("Outfit name cannot produce an ASCII canonical key".into());
    }
    let mut slug = String::new();
    for byte in name.bytes() {
        if byte.is_ascii_alphanumeric() {
            slug.push(char::from(byte.to_ascii_lowercase()));
        } else if !slug.is_empty() && !slug.ends_with('_') {
            slug.push('_');
        }
    }
    let slug = slug.trim_end_matches('_');
    if slug.is_empty() {
        return Err("Outfit name has no canonical key characters".into());
    }
    Ok(format!("oteryn:content.outfit.{slug}"))
}

fn populate_outfits(
    existing_bindings: &[ProjectV2SourceIdentityBinding],
) -> Result<OutfitPopulation, Box<dyn std::error::Error>> {
    let selected_sha256 = Sha256::digest(OUTFIT_SELECTED)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if selected_sha256 != OUTFIT_SELECTED_SHA256 {
        return Err("selected Outfit input digest drifted".into());
    }
    let packet: Value = serde_json::from_slice(OUTFIT_SELECTED)?;
    let source = &packet["source"];
    if packet["schema"] != "OTERYN_G4_OUTFIT_133_SELECTED/v1"
        || source["source_key"] != "oteryn:source.tibiawiki"
        || source["source_revision"] != OUTFIT_SOURCE_REVISION
        || source["identity_namespace"] != "mediawiki/page_id"
        || source["source_capture_sha256"] != OUTFIT_SOURCE_SHA256
        || source["crosswalk_sha256"] != OUTFIT_CROSSWALK_SHA256
        || source["source_capture_artifact_id"] != 10831362943_u64
        || source["crosswalk_artifact_id"] != 10832769419_u64
        || source["exact_page_id_join_rows"] != 134_u64
        || source["target_cut"] != "2026-07-28T23:59:59Z"
        || packet["population_policy"]["typed_premium"] != "UNKNOWN"
        || packet["population_policy"]["presentation_assets"] != "UNKNOWN"
        || packet["population_policy"]["acquisition_interactions"] != "UNKNOWN"
        || packet["population_policy"]["runtime_activation"] != false
    {
        return Err("selected Outfit source or policy drifted".into());
    }
    let selected = packet["selected"].as_array().ok_or("Outfit rows missing")?;
    let blocked = packet["blocked"]
        .as_array()
        .ok_or("Outfit blocked rows missing")?;
    if selected.len() != 133 || blocked.len() != 1 {
        return Err("Outfit selection partition drifted".into());
    }
    let blocked_row = blocked[0]
        .as_array()
        .ok_or("Outfit blocked row malformed")?;
    if blocked_row.len() != 6
        || blocked_row[0] != "68724"
        || blocked_row[1] != 441933_u64
        || blocked_row[2] != "2026-08-04T13:38:39Z"
        || blocked_row[3] != "4c2172a553e341247ff923099a3e16dde22fb66535fbda9b48f3adba7318ed22"
        || blocked_row[4] != "Captain's Outfits"
        || blocked_row[5] != "POST_TARGET_CUT_REVISION"
    {
        return Err("Outfit post-cut blocker drifted".into());
    }

    let mut declarations = Vec::with_capacity(133);
    let mut bindings = Vec::with_capacity(133);
    let mut editor = Vec::with_capacity(133);
    let mut keys = BTreeSet::new();
    let mut page_ids = BTreeSet::new();
    for row in selected {
        let row = row.as_array().ok_or("Outfit selected row malformed")?;
        if row.len() != 6 {
            return Err("Outfit selected row width drifted".into());
        }
        let page_id = row[0].as_str().ok_or("Outfit page ID missing")?;
        let parsed_id: u64 = page_id.parse()?;
        let revision = row[1].as_u64().unwrap_or(0);
        let timestamp = row[2].as_str().ok_or("Outfit page timestamp missing")?;
        let digest = row[3].as_str().ok_or("Outfit page digest missing")?;
        let title = row[4].as_str().ok_or("Outfit title missing")?;
        let key = row[5].as_str().ok_or("Outfit canonical key missing")?;
        if parsed_id == 0
            || page_id != parsed_id.to_string()
            || revision == 0
            || timestamp.len() != 20
            || !timestamp.ends_with('Z')
            || timestamp > "2026-07-28T23:59:59Z"
            || !timestamp
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'-' | b':' | b'T' | b'Z'))
            || digest.len() != 64
            || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
            || outfit_key(title)? != key
            || !keys.insert(key.to_owned())
            || !page_ids.insert(page_id.to_owned())
            || existing_bindings.iter().any(|binding| {
                binding.identity_namespace == "mediawiki/page_id" && binding.external_id == page_id
            })
        {
            return Err("Outfit identity, provenance or collision check failed".into());
        }
        let target = ProjectV2DefinitionRef {
            family: ProjectV2Family::Outfit,
            key: key.to_owned(),
            revision: "definition-r1".to_owned(),
        };
        declarations.push(ProjectV2Declaration::Outfit {
            identity: ProjectV2Identity {
                key: key.to_owned(),
                revision: target.revision.clone(),
            },
            presentations: Vec::new(),
            premium: None,
            acquisition_interactions: Vec::new(),
            fields: Vec::new(),
        });
        bindings.push(ProjectV2SourceIdentityBinding {
            source_key: "oteryn:source.tibiawiki".to_owned(),
            source_revision: OUTFIT_SOURCE_REVISION.to_owned(),
            identity_namespace: "mediawiki/page_id".to_owned(),
            external_id: page_id.to_owned(),
            target: target.clone(),
            disposition: ProjectV2SourceIdentityDisposition::Exact,
        });
        editor.push(ProjectV2EditorEntry {
            target,
            display_name: title.to_owned(),
            description: String::new(),
            categories: Vec::new(),
            notes: Vec::new(),
            aliases: Vec::new(),
            tags: vec!["oteryn:editor.outfit".to_owned()],
        });
    }
    Ok(OutfitPopulation {
        declarations,
        bindings,
        editor,
    })
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
    let ItemPopulation {
        import: wiki_import,
        source: wiki_source,
        mut bindings,
        mut editor,
    } = populate_items(&mut records)?;
    let ItemWave1Population {
        import: wave1_import,
        source: wave1_source,
        authoring: item_authoring,
        promoted: wave1_promoted,
    } = populate_item_wave1(&mut records, &bindings)?;
    let MountPopulation {
        import: mount_import,
        source: mount_source,
        mut declarations,
        bindings: mount_bindings,
        editor: mount_editor,
    } = populate_mounts(&records, &bindings)?;
    bindings.extend(mount_bindings);
    editor.extend(mount_editor);
    let OutfitPopulation {
        declarations: outfit_declarations,
        bindings: outfit_bindings,
        editor: outfit_editor,
    } = populate_outfits(&bindings)?;
    declarations.extend(outfit_declarations);
    bindings.extend(outfit_bindings);
    editor.extend(outfit_editor);
    let documents = CanonicalProjectDocuments::from_v2_draft(
        ProjectV2Draft {
            core: ProjectDraft {
                project_revision: "g4-item-wave1-r1".to_owned(),
                package_key: "oteryn:content.world-project".to_owned(),
                semantic_schema_version: "reference-schema-v1".to_owned(),
                licensing_metadata: "PENDING".to_owned(),
                world_id: "0123456789ab70cd8ef0123456789abc".to_owned(),
                coordinate_frame: "global-target-2026-07-28".to_owned(),
                records,
                imports: vec![provenance, wiki_import, wave1_import, mount_import],
                metadata: Vec::new(),
            },
            state: ProjectV2State {
                sources: vec![source, wiki_source, wave1_source, mount_source],
                declarations,
                source_identity_bindings: bindings,
                editor,
                item_authoring,
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
        "documents={DOCUMENT_COUNT} items={CW2_B1_FULL_ITEM_FAMILY_COUNT} promoted_items={} promoted_fields={} item_bindings=165 item_fields=526 wave1_items={ITEM_WAVE1_ITEMS} wave1_promoted={wave1_promoted} mounts=252 mount_fields=0 outfits=133 outfit_fields=0 outfit_blocked_post_cut=1 tree_sha256={tree_sha256}",
        promoted.promoted_items, promoted.promoted_fields
    );
    Ok(())
}
