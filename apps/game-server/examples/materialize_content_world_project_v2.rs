use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use oteryn_game_server::content::{
    CW2_B1_FULL_ITEM_FAMILY_COUNT, CW2_B1_FULL_ITEM_REVISION, CandidateValue,
    CanonicalProjectDocuments, ImportBatch, ProjectDraft, ProjectEvidenceLimits,
    ProjectReferenceRecord, ProjectV2AuthoringProfile, ProjectV2Declaration,
    ProjectV2DefinitionRef, ProjectV2Draft, ProjectV2EditorEntry, ProjectV2EvidenceClass,
    ProjectV2Family, ProjectV2Identity, ProjectV2ItemAuthoring, ProjectV2ItemForgeProfile,
    ProjectV2ItemLifecycle, ProjectV2ItemSourceLifecycle, ProjectV2ItemTaxonomy, ProjectV2Source,
    ProjectV2SourceIdentityBinding, ProjectV2SourceIdentityDisposition, ProjectV2State,
    R7_P04_GOLD_COIN_EVIDENCE_PACKET, ReferenceCells, ReferenceItemField, ReferenceItemImbuement,
    ReferenceItemPresentation, ReferenceItemSemantics, ReferenceItemStack,
    ReferenceItemTradeRestrictions, ReferenceItemWeapon, ReferenceRationalPercent,
    ReferenceSignedPoints, ReferenceWeaponType, ReimportDecision, ReimportFieldState,
    item_identity::{ItemKeyAliasTable, apply_tibia_id_key_rule},
    protected_r7_p04_gold_coin_item_family_import,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

const B1_EVIDENCE: &[u8] = include_bytes!(
    "../../../docs/agents/evidence/OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json"
);
const DOCUMENT_COUNT: usize = 11;
const ITEM_KEY_ALIASES: &[u8] = include_bytes!("../../../content/items/aliases.json");
/// A12: 38,157 protected Item records less the 4,590 D149 records.
const ITEM_TIBIA_KEYS: usize = 33_567;
const ITEM_D149_REMOVED: usize = 4_590;
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
const CREATURE_STAGED: &[u8] = include_bytes!(
    "../../../docs/agents/evidence/OTV2-20260927-creature-admission-wave-a-staged.json"
);
const CREATURE_STAGED_SHA256: &str =
    "509be1dc6376f0e9c75e0743535ab3c4e426971aa9d11dabd909866d30a64670";
const CREATURE_STAGE_TOOL_SHA256: &str =
    "0fc3154a417b522fd8f3886822bf44fed804c7217f80483acf0341f601efd8fd";
const CANARY_REVISION: &str = "47dfd51f45280a59a1d3e50ba7edd573d7234446";
/// D44: creatures Tibia has at the target and Canary lacks, authored from TibiaWiki (`wiki_authored.py`).
const CREATURE_WIKI_SAMPLE_SHA256: &str =
    "88d21c748283df4cf059dbf1bd04cbcf7b9d00ca6cd14dc1e913e947f0242c8c";
const CREATURE_WIKI_REVISION: &str = "tibiawiki-wiki-authored-creature-88d21c748283df4c";
const CREATURE_WIKI_COUNT: usize = 1;
const CREATURE_WIKI_BINDINGS: [(&str, &str); CREATURE_WIKI_COUNT] =
    [("108320", "oteryn:creature.dark_merudri")];
/// Game version 15.30: creatures CrystalServer has at its pinned 15.30 commit and Canary lacks (`crystal_batch.py`).
const CREATURE_CRYSTAL_REVISION: &str = "00ce02a57ca5a12e48f32a3476e37471167e4c3f";
/// A v2 source is unique per key and revision and names one import batch; the NPC summer supplement
/// already holds `crystalserver` at the commit itself. The creature batch therefore gets its own
/// source revision, which keeps the commit, as the TibiaWiki batches of one source do.
const CREATURE_CRYSTAL_SOURCE_REVISION: &str =
    "crystalserver-creature-1530:00ce02a57ca5a12e48f32a3476e37471167e4c3f";
const CREATURE_CRYSTAL_COUNT: usize = 13;
const CANARY_BUNDLE_INDEX_SHA256: &str =
    "d7983f63eb6d8cb2ef47537bace41835a3bfe6346ac14da4e55ca746f049b93d";
const ITEM_ALLOCATION_SHA256: &str =
    "ee9219ccf9d8b2350911abca321507ff924ccd4cb83196efd08b91fbdf098966";
const NPC_STAGED: &[u8] =
    include_bytes!("../../../docs/agents/evidence/OTV2-20260927-npc-admission-wave-a-staged.json");
const NPC_STAGED_SHA256: &str = "74cb17b45e073702ef36b09bb35f86bb259d264b437d20219a68a83d1f82d333";
const NPC_STAGE_TOOL_SHA256: &str =
    "4b1569375cb675f31fb64a00d94e97c73719b9224eff9569dd38d0ee01a362ff";
const NPC_CANDIDATES_SHA256: &str =
    "c895b8976078abf6f63402aa93565657d94660aee7420c1be6799fddff02f54f";
const NPC_WIKI_SNAPSHOT_SHA256: &str =
    "52f87d29eddd1a4e99d154e832813a711ba4d07d34487b76776d80d8884ade42";
const NPC_ITEM_MAP_SHA256: &str =
    "83ba3c26d10af8834191bf5491280882b6453bca0911b86d180c07a15cec679a";
const NPC_WIKI_REVISION: &str = "tibiawiki-npc-52f87d29eddd1a4e";
/// D12: offer prices TibiaWiki Fandom and TibiaWiki BR agree on also come from the committed BR facts.
const NPC_BR_FACTS_SHA256: &str =
    "0773232ddd356be273474be7b3aea645ed5dbbf93e5832a94d565ad9e579657a";
const NPC_BR_REVISION: &str = "tibiawiki-br-npc-0773232ddd356be2";
/// D13: offer prices two of Fandom, TibiaWiki BR and Tibiopedia agree on also come from the committed Tibiopedia facts.
const NPC_TIBIOPEDIA_FACTS_SHA256: &str =
    "43bfc91ec7721df150d3803f7123df1909a167c6606e54b112075a433fda8651";
const NPC_TIBIOPEDIA_REVISION: &str = "tibiopedia-npc-43bfc91ec7721df1";
const CRYSTAL_REVISION: &str = "ff7ede593c69d4c658b382c97443e8155926924a";
/// D14: NPC files Crystal added after `CRYSTAL_REVISION` come from one more pinned commit (`summer-update`).
const CRYSTAL_SUPPLEMENT_REVISION: &str = "00ce02a57ca5a12e48f32a3476e37471167e4c3f";
const CRYSTAL_SUPPLEMENT_BUNDLES_SHA256: &str =
    "154083cc71de6e41d73c13854307c4c2cbc134df5afdd1e1c899cfc12f5b3fb5";
const NPC_COUNT: usize = 1110;
const NPC_RECORDS: usize = 2220;
const NPC_DECLARATIONS: usize = 2184;
const NPC_DIALOGUE_STAGED: &[u8] =
    include_bytes!("../../../docs/agents/evidence/OTV2-20260928-npc-dialogue-wave-a-staged.json");
const NPC_DIALOGUE_STAGED_SHA256: &str =
    "182063117ccc43558e2abdc39af3c3830117b3ae2256231c26c517e8e784ee7b";
const NPC_DIALOGUES: usize = 694;
const NPC_DIALOGUE_NODES: usize = 6306;
const NPC_BINDINGS: usize = 2376;
const CREATURE_COUNT: usize = 1477;
const CREATURE_RECORDS: usize = 20652;
const CREATURE_PROFILES: usize = 19706;
/// Encounter admission E1-E5: encounters admitted with the creatures they cover.
const ENCOUNTER_COUNT: usize = 61;

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
        max_reference_records: CW2_B1_FULL_ITEM_FAMILY_COUNT + CREATURE_RECORDS + NPC_RECORDS,
        max_import_records: 11,
        max_reimport_states: ENCOUNTER_COUNT,
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

/// Exactly the 8 `presentation.name` disagreements between the G4 165-item wiki census
/// (`populate_items`) and the #1048 Item semantic-promotion lowering v1 packet that
/// ASCII-case-insensitive equality does not resolve. Owner decision: the lowering
/// value is the single promotion source and wins verbatim for exactly these pinned
/// `(native_key, wiki census title, lowering items.xml value)` triples, nowhere else.
/// `i00037538` and `i00037526` are tracked as a separate data-quality finding (their
/// names look like crosswalk/identity drift, not a formatting difference, even though
/// their `weapon.*` facts agree exactly) — see the task record.
const ITEM_NAME_LOWERING_OVERRIDES: &[(&str, &str, &str)] = &[
    (
        "oteryn:item.registry.i00037538",
        "Staff",
        "pair of monk fists",
    ),
    ("oteryn:item.registry.i00006361", "The Avenger", "avenger"),
    ("oteryn:item.registry.i00007913", "The Epiphany", "epiphany"),
    (
        "oteryn:item.registry.i00007205",
        "The Justice Seeker",
        "justice seeker",
    ),
    ("oteryn:item.registry.i00007835", "The Devileye", "devileye"),
    (
        "oteryn:item.registry.i00021963",
        "Ferumbras' Staff (Club)",
        "Ferumbras' staff",
    ),
    (
        "oteryn:item.registry.i00032484",
        "Souleater (Axe)",
        "souleater",
    ),
    (
        "oteryn:item.registry.i00037526",
        "Crypt Strike",
        "falcon sai",
    ),
];

/// `promote`'s `presentation.name` counterpart. The #1048 Item semantic-promotion
/// lowering pass now runs ahead of this wiki census and already sets `presentation.name`
/// to the lowercase `items.xml` text for any item it covers, so a wiki-sourced Title
/// Case candidate that agrees with the existing value save for ASCII case is treated as
/// the same fact rather than a contradiction; the existing lowercase value is kept
/// unchanged. A residual disagreement is a no-op (the existing lowering value is kept)
/// only when it exactly matches a pinned `ITEM_NAME_LOWERING_OVERRIDES` entry — no
/// generic "The " or parenthetical stripping. Every other disagreement still errors
/// exactly as `promote` does.
fn promote_name(
    slot: &mut ReferenceItemField<String>,
    native_key: &str,
    value: String,
    overrides_hit: &mut BTreeSet<&'static str>,
) -> Result<bool, Box<dyn std::error::Error>> {
    match slot {
        ReferenceItemField::Unknown => {
            *slot = ReferenceItemField::Known(value);
            Ok(true)
        }
        ReferenceItemField::Known(existing) if existing.eq_ignore_ascii_case(&value) => Ok(false),
        ReferenceItemField::Known(existing) => {
            match ITEM_NAME_LOWERING_OVERRIDES
                .iter()
                .find(|(key, wiki, lowering)| {
                    *key == native_key && *wiki == value && *lowering == existing.as_str()
                }) {
                Some((key, _, _)) => {
                    overrides_hit.insert(key);
                    Ok(false)
                }
                None => Err("Item candidate contradicts an existing field state".into()),
            }
        }
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
    native_key: &str,
    overrides_hit: &mut BTreeSet<&'static str>,
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
            promote_name(
                &mut presentation(semantics)?.name,
                native_key,
                value.to_owned(),
                overrides_hit,
            )
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
    let mut overrides_hit = BTreeSet::new();
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
                if apply_field(
                    semantics,
                    field,
                    title,
                    &binding.target.key,
                    &mut overrides_hit,
                )? {
                    return Err("post-cut Item field cannot be promoted".into());
                }
                post_cut_count += 1;
            } else if apply_field(
                semantics,
                field,
                title,
                &binding.target.key,
                &mut overrides_hit,
            )? {
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
    if overrides_hit.len() != ITEM_NAME_LOWERING_OVERRIDES.len() {
        let unused = ITEM_NAME_LOWERING_OVERRIDES
            .iter()
            .filter(|(key, _, _)| !overrides_hit.contains(key))
            .map(|(key, _, _)| *key)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(
            format!("Item name lowering override table entries never hit: {unused}").into(),
        );
    }
    if promoted_count != 12 || equal_count != 546 || post_cut_count != 3 {
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

struct CreaturePopulation {
    import: ImportBatch,
    source: ProjectV2Source,
    wiki_import: ImportBatch,
    wiki_source: ProjectV2Source,
    crystal_import: ImportBatch,
    crystal_source: ProjectV2Source,
    records: Vec<ProjectReferenceRecord>,
    declarations: Vec<ProjectV2Declaration>,
    profiles: Vec<ProjectV2AuthoringProfile>,
    bindings: Vec<ProjectV2SourceIdentityBinding>,
}

fn populate_creatures() -> Result<CreaturePopulation, Box<dyn std::error::Error>> {
    if hex_sha256(CREATURE_STAGED) != CREATURE_STAGED_SHA256 {
        return Err("staged creature admission input digest drifted".into());
    }
    let packet: Value = serde_json::from_slice(CREATURE_STAGED)?;
    let counts = &packet["counts"];
    if packet["schema"] != "OTERYN_CREATURE_ADMISSION_STAGED/v1"
        || packet["wave"] != "A"
        || packet["source"]["repository"] != "opentibiabr/canary"
        || packet["source"]["revision"] != CANARY_REVISION
        || packet["source"]["census_index_sha256"] != CANARY_BUNDLE_INDEX_SHA256
        || packet["source"]["item_allocation_sha256"] != ITEM_ALLOCATION_SHA256
        || counts["creatures"] != CREATURE_COUNT
        || counts["records"] != CREATURE_RECORDS
        || counts["profiles"] != CREATURE_PROFILES
        || counts["encounters"] != ENCOUNTER_COUNT
        || packet["source"]["wiki_authored"]["revision"] != CREATURE_WIKI_REVISION
        || packet["source"]["wiki_authored"]["sample_sha256"] != CREATURE_WIKI_SAMPLE_SHA256
        || packet["source"]["crystal"]["repository"] != "zimbadev/crystalserver"
        || packet["source"]["crystal"]["revision"] != CREATURE_CRYSTAL_REVISION
        || packet["source"]["crystal"]["creatures"]
            .as_array()
            .is_none_or(|creatures| creatures.len() != CREATURE_CRYSTAL_COUNT)
    {
        return Err("staged creature admission source identity drifted".into());
    }
    let records: Vec<ProjectReferenceRecord> = serde_json::from_value(packet["records"].clone())?;
    let profiles: Vec<ProjectV2AuthoringProfile> =
        serde_json::from_value(packet["authoring_profiles"].clone())?;
    let mut bindings: Vec<ProjectV2SourceIdentityBinding> =
        serde_json::from_value(packet["source_identity_bindings"].clone())?;
    let declarations: Vec<ProjectV2Declaration> =
        serde_json::from_value(packet["declarations"].clone())?;
    if declarations.len() != ENCOUNTER_COUNT
        || declarations
            .iter()
            .any(|declaration| !matches!(declaration, ProjectV2Declaration::Encounter { .. }))
    {
        return Err("staged encounter declarations drifted".into());
    }
    let creatures = records
        .iter()
        .filter(|record| matches!(record, ProjectReferenceRecord::Creature { .. }))
        .count();
    if records.len() != CREATURE_RECORDS
        || profiles.len() != CREATURE_PROFILES
        || creatures != CREATURE_COUNT
        || bindings.len() != CREATURE_COUNT + ENCOUNTER_COUNT
        || bindings
            .iter()
            .filter(|binding| binding.source_key == "oteryn:source.tibiawiki")
            .count()
            != CREATURE_WIKI_COUNT
        || bindings
            .iter()
            .filter(|binding| binding.source_key == "oteryn:source.crystalserver")
            .count()
            != CREATURE_CRYSTAL_COUNT
        || bindings.iter().any(|binding| {
            !matches!(
                (
                    binding.target.family,
                    binding.source_key.as_str(),
                    binding.source_revision.as_str(),
                    binding.identity_namespace.as_str()
                ),
                (
                    ProjectV2Family::Creature,
                    "oteryn:source.canary",
                    CANARY_REVISION,
                    "canary/monster-file"
                ) | (
                    ProjectV2Family::Encounter,
                    "oteryn:source.canary",
                    CANARY_REVISION,
                    "canary/encounter"
                ) | (
                    ProjectV2Family::Creature,
                    "oteryn:source.tibiawiki",
                    CREATURE_WIKI_REVISION,
                    "mediawiki/page_id"
                ) | (
                    ProjectV2Family::Creature,
                    "oteryn:source.crystalserver",
                    CREATURE_CRYSTAL_REVISION,
                    "crystalserver/monster-file"
                )
            )
        })
    {
        return Err("staged creature admission counts drifted".into());
    }
    let wiki_bindings: Vec<(&str, &str)> = bindings
        .iter()
        .filter(|binding| binding.source_key == "oteryn:source.tibiawiki")
        .map(|binding| (binding.external_id.as_str(), binding.target.key.as_str()))
        .collect();
    if wiki_bindings != CREATURE_WIKI_BINDINGS
        || bindings
            .iter()
            .any(|binding| binding.disposition != ProjectV2SourceIdentityDisposition::Exact)
    {
        return Err("wiki-authored creature page binding drifted".into());
    }
    // The staged Crystal bindings name the commit; they resolve to the creature batch's own source.
    for binding in &mut bindings {
        if binding.source_key == "oteryn:source.crystalserver" {
            binding.source_revision = CREATURE_CRYSTAL_SOURCE_REVISION.to_owned();
        }
    }
    // E5: each admitted encounter keeps the digest of the manifest it was mapped from as its reimport baseline.
    let manifests = packet["encounter_manifests"]
        .as_object()
        .ok_or("staged encounter manifests are missing")?;
    let mut reimport_states = Vec::with_capacity(ENCOUNTER_COUNT);
    for binding in bindings
        .iter()
        .filter(|binding| binding.target.family == ProjectV2Family::Encounter)
    {
        let digest = manifests
            .get(&binding.external_id)
            .and_then(Value::as_str)
            .filter(|digest| {
                digest.len() == 64
                    && digest
                        .bytes()
                        .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
            })
            .ok_or("staged encounter manifest digest is missing")?;
        let value = Some(CandidateValue::Text(digest.to_owned()));
        reimport_states.push(ReimportFieldState {
            stable_identity: binding.target.key.clone(),
            field_path: "encounter_manifest_sha256".to_owned(),
            baseline: value.clone(),
            upstream: value.clone(),
            local: value,
            decision: ReimportDecision::Unchanged,
        });
    }
    if manifests.len() != ENCOUNTER_COUNT || reimport_states.len() != ENCOUNTER_COUNT {
        return Err("staged encounter manifests drifted".into());
    }
    reimport_states.sort_by(|left, right| left.stable_identity.cmp(&right.stable_identity));
    let import = ImportBatch {
        batch_id: "g4-creature-canary-wave-a-r1".to_owned(),
        source_repository: "opentibiabr/canary".to_owned(),
        source_revision: CANARY_REVISION.to_owned(),
        source_artifact_sha256: CANARY_BUNDLE_INDEX_SHA256.to_owned(),
        access_disposition: "PENDING".to_owned(),
        source_generation_profile: "OTERYN_MONSTER_AUTHORING_BUNDLE/v1".to_owned(),
        importer: "OTERYN_CANARY_MONSTER_POPULATION_CENSUS/v1".to_owned(),
        mapper: "OTERYN_CREATURE_ADMISSION_STAGE/v1".to_owned(),
        mapper_revision: "creature-admission-r1".to_owned(),
        mapper_sha256: CREATURE_STAGE_TOOL_SHA256.to_owned(),
        candidates: Vec::new(),
        reimport_states,
    };
    let source = ProjectV2Source {
        key: "oteryn:source.canary".to_owned(),
        import_batch_id: import.batch_id.clone(),
        revision: import.source_revision.clone(),
        sha256: import.source_artifact_sha256.clone(),
        evidence: ProjectV2EvidenceClass::OtsHypothesisOnly,
    };
    let wiki_import = ImportBatch {
        batch_id: "g4-wiki-authored-creature-d44-r1".to_owned(),
        source_repository: "tibia.fandom.com".to_owned(),
        source_revision: CREATURE_WIKI_REVISION.to_owned(),
        source_artifact_sha256: CREATURE_WIKI_SAMPLE_SHA256.to_owned(),
        access_disposition: "PENDING".to_owned(),
        source_generation_profile: "OTERYN_WIKI_AUTHORED_MONSTER/v1".to_owned(),
        importer: "OTERYN_CANARY_MONSTER_POPULATION_CENSUS/v1".to_owned(),
        mapper: "OTERYN_CREATURE_ADMISSION_STAGE/v1".to_owned(),
        mapper_revision: "creature-admission-r1".to_owned(),
        mapper_sha256: CREATURE_STAGE_TOOL_SHA256.to_owned(),
        candidates: Vec::new(),
        reimport_states: Vec::new(),
    };
    let wiki_source = ProjectV2Source {
        key: "oteryn:source.tibiawiki".to_owned(),
        import_batch_id: wiki_import.batch_id.clone(),
        revision: wiki_import.source_revision.clone(),
        sha256: wiki_import.source_artifact_sha256.clone(),
        evidence: ProjectV2EvidenceClass::Derived,
    };
    let crystal_import = ImportBatch {
        batch_id: "g4-creature-crystal-1530-r1".to_owned(),
        source_repository: "zimbadev/crystalserver".to_owned(),
        source_revision: CREATURE_CRYSTAL_SOURCE_REVISION.to_owned(),
        source_artifact_sha256: CANARY_BUNDLE_INDEX_SHA256.to_owned(),
        access_disposition: "PENDING".to_owned(),
        source_generation_profile: "OTERYN_MONSTER_AUTHORING_BUNDLE/v1".to_owned(),
        importer: "OTERYN_CANARY_MONSTER_POPULATION_CENSUS/v1".to_owned(),
        mapper: "OTERYN_CREATURE_ADMISSION_STAGE/v1".to_owned(),
        mapper_revision: "creature-admission-r1".to_owned(),
        mapper_sha256: CREATURE_STAGE_TOOL_SHA256.to_owned(),
        candidates: Vec::new(),
        reimport_states: Vec::new(),
    };
    let crystal_source = ProjectV2Source {
        key: "oteryn:source.crystalserver".to_owned(),
        import_batch_id: crystal_import.batch_id.clone(),
        revision: crystal_import.source_revision.clone(),
        sha256: crystal_import.source_artifact_sha256.clone(),
        evidence: ProjectV2EvidenceClass::OtsHypothesisOnly,
    };
    Ok(CreaturePopulation {
        import,
        source,
        wiki_import,
        wiki_source,
        crystal_import,
        crystal_source,
        records,
        declarations,
        profiles,
        bindings,
    })
}

struct NpcPopulation {
    import: ImportBatch,
    source: ProjectV2Source,
    br_import: ImportBatch,
    br_source: ProjectV2Source,
    tibiopedia_import: ImportBatch,
    tibiopedia_source: ProjectV2Source,
    supplement_import: ImportBatch,
    supplement_source: ProjectV2Source,
    records: Vec<ProjectReferenceRecord>,
    declarations: Vec<ProjectV2Declaration>,
    profiles: Vec<ProjectV2AuthoringProfile>,
    bindings: Vec<ProjectV2SourceIdentityBinding>,
}

/// Every admitted Dialogue must be exactly the declaration in the pinned dialogue evidence.
fn verify_npc_dialogues(
    declarations: &[ProjectV2Declaration],
) -> Result<(), Box<dyn std::error::Error>> {
    if hex_sha256(NPC_DIALOGUE_STAGED) != NPC_DIALOGUE_STAGED_SHA256 {
        return Err("staged NPC dialogue input digest drifted".into());
    }
    let packet: Value = serde_json::from_slice(NPC_DIALOGUE_STAGED)?;
    if packet["schema"] != "OTERYN_NPC_DIALOGUE_STAGED/v1"
        || packet["source"]["canary_revision"] != CANARY_REVISION
        || packet["source"]["crystal_revision"] != CRYSTAL_REVISION
        || packet["source"]["candidates_sha256"] != NPC_CANDIDATES_SHA256
    {
        return Err("staged NPC dialogue source identity drifted".into());
    }
    let mut staged = BTreeMap::new();
    let mut staged_by_npc = BTreeMap::new();
    for entry in packet["dialogues"]
        .as_array()
        .ok_or("staged NPC dialogues missing")?
    {
        let declaration: ProjectV2Declaration =
            serde_json::from_value(entry["declaration"].clone())?;
        let ProjectV2Declaration::Dialogue { identity, .. } = &declaration else {
            return Err("staged NPC dialogue is not a Dialogue".into());
        };
        let npc = entry["npc"]
            .as_str()
            .ok_or("staged NPC dialogue names no NPC")?;
        staged_by_npc.insert(npc.to_owned(), identity.key.clone());
        staged.insert(identity.key.clone(), declaration);
    }
    for declaration in declarations {
        match declaration {
            ProjectV2Declaration::Dialogue { identity, .. }
                if staged.get(&identity.key) != Some(declaration) =>
            {
                return Err(
                    "admitted NPC dialogue differs from the staged dialogue evidence".into(),
                );
            }
            ProjectV2Declaration::Npc {
                identity, dialogue, ..
            } if dialogue.as_ref().map(|reference| &reference.key)
                != staged_by_npc.get(&identity.key) =>
            {
                return Err(
                    "NPC dialogue reference differs from the staged dialogue evidence".into(),
                );
            }
            _ => {}
        }
    }
    Ok(())
}

fn populate_npcs() -> Result<NpcPopulation, Box<dyn std::error::Error>> {
    if hex_sha256(NPC_STAGED) != NPC_STAGED_SHA256 {
        return Err("staged NPC admission input digest drifted".into());
    }
    let packet: Value = serde_json::from_slice(NPC_STAGED)?;
    let source = &packet["source"];
    let counts = &packet["counts"];
    if packet["schema"] != "OTERYN_NPC_ADMISSION_STAGED/v1"
        || packet["wave"] != "A"
        || source["canary_revision"] != CANARY_REVISION
        || source["crystal_revision"] != CRYSTAL_REVISION
        || source["crystal_supplement_revision"] != CRYSTAL_SUPPLEMENT_REVISION
        || source["crystal_supplement_bundles_sha256"] != CRYSTAL_SUPPLEMENT_BUNDLES_SHA256
        || source["candidates_sha256"] != NPC_CANDIDATES_SHA256
        || source["item_map_sha256"] != NPC_ITEM_MAP_SHA256
        || source["wiki_snapshot_sha256"] != NPC_WIKI_SNAPSHOT_SHA256
        || source["wiki_revision"] != NPC_WIKI_REVISION
        || source["br_facts_sha256"] != NPC_BR_FACTS_SHA256
        || source["br_revision"] != NPC_BR_REVISION
        || source["tibiopedia_facts_sha256"] != NPC_TIBIOPEDIA_FACTS_SHA256
        || source["tibiopedia_revision"] != NPC_TIBIOPEDIA_REVISION
        || counts["npcs"] != NPC_COUNT
        || counts["records"] != NPC_RECORDS
        || counts["declarations"] != NPC_DECLARATIONS
        || counts["bindings"] != NPC_BINDINGS
        || source["dialogue_staged_sha256"] != NPC_DIALOGUE_STAGED_SHA256
        || counts["dialogues"] != NPC_DIALOGUES
        || counts["dialogue_nodes"] != NPC_DIALOGUE_NODES
    {
        return Err("staged NPC admission source identity drifted".into());
    }
    let records: Vec<ProjectReferenceRecord> = serde_json::from_value(packet["records"].clone())?;
    let declarations: Vec<ProjectV2Declaration> =
        serde_json::from_value(packet["declarations"].clone())?;
    verify_npc_dialogues(&declarations)?;
    let profiles: Vec<ProjectV2AuthoringProfile> =
        serde_json::from_value(packet["authoring_profiles"].clone())?;
    let bindings: Vec<ProjectV2SourceIdentityBinding> =
        serde_json::from_value(packet["source_identity_bindings"].clone())?;
    let npcs = declarations
        .iter()
        .filter(|declaration| matches!(declaration, ProjectV2Declaration::Npc { .. }))
        .count();
    let dialogues = declarations
        .iter()
        .filter(|declaration| matches!(declaration, ProjectV2Declaration::Dialogue { .. }))
        .count();
    if records.len() != NPC_RECORDS
        || profiles.len() != NPC_RECORDS
        || declarations.len() != NPC_DECLARATIONS
        || npcs != NPC_COUNT
        || dialogues != NPC_DIALOGUES
        || bindings.len() != NPC_BINDINGS
        || bindings.iter().any(|binding| {
            binding.target.family != ProjectV2Family::Npc
                || !matches!(
                    (
                        binding.source_key.as_str(),
                        binding.source_revision.as_str(),
                        binding.identity_namespace.as_str()
                    ),
                    ("oteryn:source.canary", CANARY_REVISION, "canary/npc-file")
                        | (
                            "oteryn:source.crystalserver",
                            CRYSTAL_REVISION | CRYSTAL_SUPPLEMENT_REVISION,
                            "crystalserver/npc-file"
                        )
                        | (
                            "oteryn:source.tibiawiki",
                            NPC_WIKI_REVISION,
                            "mediawiki/page_id"
                        )
                )
        })
    {
        return Err("staged NPC admission counts drifted".into());
    }
    let import = ImportBatch {
        batch_id: "g4-npc-wave-a-tibiawiki-r9".to_owned(),
        source_repository: "tibia.fandom.com".to_owned(),
        source_revision: NPC_WIKI_REVISION.to_owned(),
        source_artifact_sha256: NPC_WIKI_SNAPSHOT_SHA256.to_owned(),
        access_disposition: "PENDING".to_owned(),
        source_generation_profile: "OTERYN_NPC_FANDOM_SNAPSHOT/v1".to_owned(),
        importer: "OTERYN_NPC_PROMOTION_CANDIDATES/v1".to_owned(),
        mapper: "OTERYN_NPC_ADMISSION_STAGE/v1".to_owned(),
        mapper_revision: "npc-admission-r9".to_owned(),
        mapper_sha256: NPC_STAGE_TOOL_SHA256.to_owned(),
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
    let br_import = ImportBatch {
        batch_id: "g4-npc-prices-tibiawiki-br-r1".to_owned(),
        source_repository: "tibiawiki.com.br".to_owned(),
        source_revision: NPC_BR_REVISION.to_owned(),
        source_artifact_sha256: NPC_BR_FACTS_SHA256.to_owned(),
        access_disposition: "PENDING".to_owned(),
        source_generation_profile: "OTERYN_NPC_TIBIAWIKI_BR_FACTS/v1".to_owned(),
        importer: "OTERYN_NPC_PROMOTION_CANDIDATES/v1".to_owned(),
        mapper: "OTERYN_NPC_ADMISSION_STAGE/v1".to_owned(),
        mapper_revision: "npc-admission-r9".to_owned(),
        mapper_sha256: NPC_STAGE_TOOL_SHA256.to_owned(),
        candidates: Vec::new(),
        reimport_states: Vec::new(),
    };
    let br_source = ProjectV2Source {
        key: "oteryn:source.tibiawiki".to_owned(),
        import_batch_id: br_import.batch_id.clone(),
        revision: br_import.source_revision.clone(),
        sha256: br_import.source_artifact_sha256.clone(),
        evidence: ProjectV2EvidenceClass::Derived,
    };
    let tibiopedia_import = ImportBatch {
        batch_id: "g4-npc-prices-tibiopedia-r1".to_owned(),
        source_repository: "tibiopedia.pl".to_owned(),
        source_revision: NPC_TIBIOPEDIA_REVISION.to_owned(),
        source_artifact_sha256: NPC_TIBIOPEDIA_FACTS_SHA256.to_owned(),
        access_disposition: "PENDING".to_owned(),
        source_generation_profile: "OTERYN_NPC_TIBIOPEDIA_FACTS/v1".to_owned(),
        importer: "OTERYN_NPC_PROMOTION_CANDIDATES/v1".to_owned(),
        mapper: "OTERYN_NPC_ADMISSION_STAGE/v1".to_owned(),
        mapper_revision: "npc-admission-r9".to_owned(),
        mapper_sha256: NPC_STAGE_TOOL_SHA256.to_owned(),
        candidates: Vec::new(),
        reimport_states: Vec::new(),
    };
    let tibiopedia_source = ProjectV2Source {
        key: "oteryn:source.tibiawiki".to_owned(),
        import_batch_id: tibiopedia_import.batch_id.clone(),
        revision: tibiopedia_import.source_revision.clone(),
        sha256: tibiopedia_import.source_artifact_sha256.clone(),
        evidence: ProjectV2EvidenceClass::Derived,
    };
    let supplement_import = ImportBatch {
        batch_id: "g4-npc-crystal-summer-supplement-r1".to_owned(),
        source_repository: "zimbadev/crystalserver".to_owned(),
        source_revision: CRYSTAL_SUPPLEMENT_REVISION.to_owned(),
        source_artifact_sha256: CRYSTAL_SUPPLEMENT_BUNDLES_SHA256.to_owned(),
        access_disposition: "PENDING".to_owned(),
        source_generation_profile: "OTERYN_NPC_AUTHORING_CANDIDATE/v1".to_owned(),
        importer: "OTERYN_NPC_PROMOTION_CANDIDATES/v1".to_owned(),
        mapper: "OTERYN_NPC_ADMISSION_STAGE/v1".to_owned(),
        mapper_revision: "npc-admission-r9".to_owned(),
        mapper_sha256: NPC_STAGE_TOOL_SHA256.to_owned(),
        candidates: Vec::new(),
        reimport_states: Vec::new(),
    };
    let supplement_source = ProjectV2Source {
        key: "oteryn:source.crystalserver".to_owned(),
        import_batch_id: supplement_import.batch_id.clone(),
        revision: supplement_import.source_revision.clone(),
        sha256: supplement_import.source_artifact_sha256.clone(),
        evidence: ProjectV2EvidenceClass::OtsHypothesisOnly,
    };
    Ok(NpcPopulation {
        import,
        source,
        br_import,
        br_source,
        tibiopedia_import,
        tibiopedia_source,
        supplement_import,
        supplement_source,
        records,
        declarations,
        profiles,
        bindings,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = output_root()?;
    let promoted = protected_r7_p04_gold_coin_item_family_import(
        B1_EVIDENCE,
        R7_P04_GOLD_COIN_EVIDENCE_PACKET,
    )?;
    if promoted.family.records.len() != CW2_B1_FULL_ITEM_FAMILY_COUNT {
        return Err("protected promoted Item family count drifted".into());
    }
    let mut provenance = promoted.family.batch;
    // The historical key of every Item record and its CW2-B1 source id: the §4.1 rule input.
    let item_source_ids = provenance
        .candidates
        .iter()
        .map(|candidate| {
            let key = candidate
                .candidate_target
                .strip_suffix(&format!("@{CW2_B1_FULL_ITEM_REVISION}"))
                .ok_or("protected Item candidate target drifted")?;
            let source_id = candidate
                .source_numeric_id
                .ok_or("protected Item candidate source id missing")?;
            Ok((key.to_owned(), source_id))
        })
        .collect::<Result<BTreeMap<_, _>, &str>>()?;
    if item_source_ids.len() != CW2_B1_FULL_ITEM_FAMILY_COUNT {
        return Err("protected Item source id closure drifted".into());
    }
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
    let CreaturePopulation {
        import: creature_import,
        source: creature_source,
        wiki_import: creature_wiki_import,
        wiki_source: creature_wiki_source,
        crystal_import: creature_crystal_import,
        crystal_source: creature_crystal_source,
        records: creature_records,
        declarations: encounter_declarations,
        profiles: mut authoring_profiles,
        bindings: creature_bindings,
    } = populate_creatures()?;
    records.extend(creature_records);
    declarations.extend(encounter_declarations);
    bindings.extend(creature_bindings);
    let NpcPopulation {
        import: npc_import,
        source: npc_source,
        br_import: npc_br_import,
        br_source: npc_br_source,
        tibiopedia_import: npc_tibiopedia_import,
        tibiopedia_source: npc_tibiopedia_source,
        supplement_import: npc_supplement_import,
        supplement_source: npc_supplement_source,
        records: npc_records,
        declarations: npc_declarations,
        profiles: npc_profiles,
        bindings: npc_bindings,
    } = populate_npcs()?;
    records.extend(npc_records);
    declarations.extend(npc_declarations);
    authoring_profiles.extend(npc_profiles);
    bindings.extend(npc_bindings);
    let mut draft = ProjectV2Draft {
        core: ProjectDraft {
            project_revision: "g4-npc-wave-a-r9".to_owned(),
            package_key: "oteryn:content.world-project".to_owned(),
            semantic_schema_version: "reference-schema-v1".to_owned(),
            licensing_metadata: "PENDING".to_owned(),
            world_id: "0123456789ab70cd8ef0123456789abc".to_owned(),
            coordinate_frame: "global-target-2026-09-27".to_owned(),
            records,
            imports: vec![
                provenance,
                wiki_import,
                wave1_import,
                mount_import,
                creature_import,
                creature_wiki_import,
                creature_crystal_import,
                npc_import,
                npc_br_import,
                npc_tibiopedia_import,
                npc_supplement_import,
            ],
            metadata: Vec::new(),
        },
        state: ProjectV2State {
            sources: vec![
                source,
                wiki_source,
                wave1_source,
                mount_source,
                creature_source,
                creature_wiki_source,
                creature_crystal_source,
                npc_source,
                npc_br_source,
                npc_tibiopedia_source,
                npc_supplement_source,
            ],
            declarations,
            source_identity_bindings: bindings,
            editor,
            item_authoring,
            authoring_profiles,
            ..ProjectV2State::default()
        },
    };
    // A12 §4.1: every Item key becomes its Tibia key and the D149 records leave content.
    let aliases = ItemKeyAliasTable::parse(ITEM_KEY_ALIASES)?;
    let switch = apply_tibia_id_key_rule(&mut draft, &aliases, &item_source_ids)?;
    if switch.item_records != ITEM_TIBIA_KEYS
        || switch.removed_without_successor != ITEM_D149_REMOVED
    {
        return Err(format!("Item key switch drifted: {switch:?}").into());
    }
    let documents = CanonicalProjectDocuments::from_v2_draft(draft, limits())?;
    if documents.documents().len() != DOCUMENT_COUNT {
        return Err("canonical WorldProject/v2 document count drifted".into());
    }
    let tree_sha256 = write_documents(&root, &documents)?;
    println!(
        "documents={DOCUMENT_COUNT} items={ITEM_TIBIA_KEYS} d149_removed={ITEM_D149_REMOVED} promoted_items={} promoted_fields={} item_bindings=165 item_fields=12 wave1_items={ITEM_WAVE1_ITEMS} wave1_promoted={wave1_promoted} mounts=252 mount_fields=0 outfits=133 outfit_fields=0 outfit_blocked_post_cut=1 creatures={CREATURE_COUNT} creature_records={CREATURE_RECORDS} creature_profiles={CREATURE_PROFILES} encounters={ENCOUNTER_COUNT} npcs={NPC_COUNT} npc_declarations={NPC_DECLARATIONS} tree_sha256={tree_sha256}",
        promoted.promoted_items, promoted.promoted_fields
    );
    Ok(())
}
