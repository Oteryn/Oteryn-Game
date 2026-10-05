//! Explicit Outfit definitions for temporary spell conditions. The source importer verifies
//! pinned git blobs and the exact Outfit_t default closure; this closed document is retained
//! inside the approved active artifact. Numeric appearance membership alone cannot create it.
#![allow(
    dead_code,
    reason = "spell import candidate; awaits its production owner caller"
)]
use super::ContentError;
use super::project::{ProjectV2DefinitionRef, ProjectV2Family};
use crate::domain::appearance::AppearanceSelection;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
const PIN: &str = "99902524e052f37574194466c2949c576e4ab269";
const DEFAULT_SHA: &str = "dd403b4c2291b66bd5df49d1a59b45667698d8a916ecfb8fa87801688e1ca622";
const CRYSTAL_PIN: &str = "ff7ede593c69d4c658b382c97443e8155926924a";
const STAG_PATH: &str = "data-global/monster/winter_update_2025/stag.lua";
const STAG_BLOB: &str = "0756e605384b018f75c498bad89ddc72b961c0ca";
const STAG_SHA: &str = "e3ec6434cabac687b502bb69d0a5abd3f40a11fc24ffa92c04d3cef45e777aeb";
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SpellAppearanceDefaultSource {
    pub(crate) revision: String,
    pub(crate) path: String,
    pub(crate) git_blob: String,
    pub(crate) sha256: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SpellAppearanceSource {
    pub(crate) revision: String,
    pub(crate) path: String,
    pub(crate) git_blob: String,
    pub(crate) sha256: String,
    pub(crate) explicit_fields: BTreeMap<String, u32>,
    pub(crate) defaulted_fields: Vec<String>,
    // Canary uses the enclosing document's existing default closure. The
    // separately imported Crystal Stag must carry its actual Crystal header.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) default_source: Option<SpellAppearanceDefaultSource>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SpellAppearanceRecord {
    pub(crate) outfit_key: String,
    pub(crate) outfit_revision: String,
    pub(crate) look_type: u32,
    pub(crate) colours: [u16; 4],
    pub(crate) addons: u8,
    pub(crate) mount_key: Option<String>,
    pub(crate) creature: Option<ProjectV2DefinitionRef>,
    pub(crate) source: SpellAppearanceSource,
    pub(crate) qualification_sha256: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SpellItemAppearanceRecord {
    pub(crate) item: ProjectV2DefinitionRef,
    pub(crate) source_revision: String,
    pub(crate) appearances_sha256: String,
    pub(crate) canonical_definition_sha256: String,
    pub(crate) appearance_object_sha256: String,
    pub(crate) qualification_sha256: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SpellAppearancesDocument {
    pub(crate) schema: String,
    pub(crate) default_source_revision: String,
    pub(crate) default_source_path: String,
    pub(crate) default_source_sha256: String,
    pub(crate) records: Vec<SpellAppearanceRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) item_appearances: Vec<SpellItemAppearanceRecord>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompiledSpellAppearances {
    digest: [u8; 32],
    records: Vec<SpellAppearanceRecord>,
    item_appearances: Vec<SpellItemAppearanceRecord>,
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct QualifiedSpellAppearance<'a> {
    digest: [u8; 32],
    record: &'a SpellAppearanceRecord,
}
impl QualifiedSpellAppearance<'_> {
    pub(crate) const fn source_digest(self) -> [u8; 32] {
        self.digest
    }
    pub(crate) const fn look_type(self) -> u32 {
        self.record.look_type
    }
    pub(crate) fn selection(self) -> AppearanceSelection<String> {
        AppearanceSelection {
            outfit_key: self.record.outfit_key.clone(),
            colours: self.record.colours,
            addons: self.record.addons,
            mount_key: self.record.mount_key.clone(),
        }
    }
}
impl CompiledSpellAppearances {
    /// Data-only outer artifact binding. The staging owner separately verifies
    /// the independently issued enclosing artifact before calling this.
    pub(crate) fn bind_qualified_outer_artifact(
        &mut self,
        digest: [u8; 32],
    ) -> Result<(), ContentError> {
        if digest == [0; 32] {
            return Err(ContentError::InvalidArtifact(
                "zero outer appearance artifact pin",
            ));
        }
        self.digest = digest;
        Ok(())
    }
    pub(crate) const fn source_digest(&self) -> [u8; 32] {
        self.digest
    }
    pub(crate) fn for_creature(
        &self,
        key: &str,
        revision: &str,
    ) -> Option<QualifiedSpellAppearance<'_>> {
        let record = self.records.iter().find(|r| {
            r.creature
                .as_ref()
                .is_some_and(|c| c.key == key && c.revision == revision)
        })?;
        Some(QualifiedSpellAppearance {
            digest: self.digest,
            record,
        })
    }
    pub(crate) fn for_look_type(&self, look_type: u32) -> Option<QualifiedSpellAppearance<'_>> {
        let record = self
            .records
            .iter()
            .find(|r| r.creature.is_none() && r.look_type == look_type)?;
        Some(QualifiedSpellAppearance {
            digest: self.digest,
            record,
        })
    }
    /// Resolve complete current temporary selection from the already qualified artifact.
    /// A bare numeric look never supplies a Creature/Outfit identity or revision.
    pub(crate) fn for_selection(
        &self,
        look_type: u32,
        selection: &AppearanceSelection<String>,
    ) -> Option<QualifiedSpellAppearance<'_>> {
        let record = self.records.iter().find(|record| {
            record.look_type == look_type
                && record.outfit_key == selection.outfit_key
                && record.colours == selection.colours
                && record.addons == selection.addons
                && record.mount_key == selection.mount_key
        })?;
        Some(QualifiedSpellAppearance {
            digest: self.digest,
            record,
        })
    }
    /// Appearance membership never implies Item materialization or ownership.
    pub(crate) fn for_item(&self, key: &str, revision: &str) -> Option<&ProjectV2DefinitionRef> {
        self.item_appearances
            .iter()
            .find(|r| r.item.key == key && r.item.revision == revision)
            .map(|r| &r.item)
    }
    pub(crate) fn creature_links(&self) -> impl Iterator<Item = (&ProjectV2DefinitionRef, u32)> {
        self.records
            .iter()
            .filter_map(|r| r.creature.as_ref().map(|c| (c, r.look_type)))
    }
}
fn invalid() -> ContentError {
    ContentError::InvalidArtifact("native spell appearance qualification")
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn hash(value: &str, width: usize) -> bool {
    value.len() == width
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
// Explicit project acceptance of the40 source records independently witnessed at cached Canary47.
// This is NOT a revision-wide permission and does not relax the enclosing999025 default closure.
fn qualified_cached_canary_record(record: &SpellAppearanceRecord) -> bool {
    let Some(defaults) = &record.source.default_source else {
        return false;
    };
    record.source.revision == "47dfd51f45280a59a1d3e50ba7edd573d7234446"
        && defaults.revision == "47dfd51f45280a59a1d3e50ba7edd573d7234446"
        && defaults.path == "src/creatures/creatures_definitions.hpp"
        && defaults.git_blob == "c8f64483241e0bb7bcca4e1768fda4c0ca386c56"
        && defaults.sha256 == "54f0a5bcdc10e33d8850300c5c5022eb7fe8b4549394407482c57245bdab3e33"
        && [
            "1d4cabafc023f1eeaec7b38a5634ba2423722ee435b7f82779c04931dbf8e818",
            "7889146bfd1ab37c511306ad654ab78813c036f82aae679bdd51e41c5e50e1b6",
            "83483403e6770c9bc4e570bcacd54d349dd8c0c8ca59585ebaa48ebf6486fed8",
            "17e6ec493bd1257dff3a65f508968ab8c2434e321a1df353f367207a6c1f7219",
            "09f144bdfd246d3109ccf25144988dc736372d904128869f25ac20b0c776c7aa",
            "5985cc61443ba9de6ef582fb11221bc66f728431cfcf7e4699e042aa73874a34",
            "db0db3c41076de4d61d3b2d0346fc5584d2456e284df90d7b22a22b02e10429e",
            "532466b13ba70b292ab3b75096e6e987bf1f3fccc46501263335df06cbf906a0",
            "35c4551de4522661b8f9395c6d9fa051c51c49f7cc713384638d0c672fae3e04",
            "f024edcbc00361fe47c284e95202d6ab6beed70e8b0699bdc81d23c5826362bc",
            "6e4b8c2d0b1ac99f6a814f362d1c0728ebd79ffdac27101f6491695648cd2f60",
            "51c418edfb31be69c2e5678789b46e7d04822131d8dd1e0664295b5ab3d8c98a",
            "69e17d3cb54a5bc79caae5fda8a78d65ca2773e37244d8ab49b5747e1cf0e31b",
            "f163ad3f10842162b5c3adca322e299902b6f0aa82d87df1df13f1d6123461d1",
            "ec3f3957b17b33f4ed471a75df859c45cc96a13e1554ff9922c7096a89ecb52c",
            "abcd4aa7f1e99837472770c6c9d7e550ff16c8f63bee1498c044ee0a0be76160",
            "1f36d37f5d5a74d6161aaf4c911afe6d1a7e77b44cb1d18506c0a0bc6bfbce83",
            "1005ef1ff657f2fd8e99618d45c552ae1e345957bf147b268811b4c1d923419c",
            "3b330d83d33d24db6d2e10b0d9f91ddb3d9f133ab54e801a43360cf009be2475",
            "0f37611587a5570f9f2814b85b3397ee111b7f3b61a20e8d88882c90de77611f",
            "c4e23b8c69d1e43bf83164e9b4722a1561343a8472307cd7aeec2f9866dbba76",
            "b1e9a793089d61f6e2061a74e4bfb32c29333c9313ec34bd4fa244e395919846",
            "ae880c919fd3206346a8756735af5549abd2d94ca84a089ea083c2a843c22509",
            "32e01c22773245797ef4ec56bbf518fbc3abc3a0aaf7b05434924c530e92f158",
            "819d707a5fec90253e083f721e3472ea48349b0dc5aed066af3ec19391763e74",
            "db02a65d162bc72f21156d80ccac3f3ab7c1ab9589d6b3c716c1336760ed2099",
            "86c7a0979a6d2306bdbf7d73068cf9c5fdd425c9c153a08fd4a566b5ef1b26cc",
            "bc8ddeef62d85215a3cfcaca3384cdb5d8d671217d41ace736ea45f36dd225e1",
            "12321da511b1dbffedef7b6f52317688f625f09d2051f584687b7010b224847f",
            "d5412170a0a2448f189dc92b1e8ca4fdcd2648b6c9abb5700d5ba45427919075",
            "7889bf31363db32adf11f6540b9bb27e1e1082fe467e3c7beb4610cc15b02025",
            "7f113cc08cd81906d4d539ab5bd964033e076289d72d70b498aa78b9d080af85",
            "9648b0223901f313f47cb45623fa739aa6bce4592444208e1945e441d25f4d3a",
            "86431ac1a7c5cb96d641f30ec96db3cba5bb22b353cf2c00fcbb4eb239b8a9fe",
            "f20e6bd6cb521a0b3482e6e5af95d2a1044f0fd2b4700136be6d0ab8420b4d22",
            "4fcd256995aef33f8f333e2dd22280568baaa914186020331ff861059e98b40d",
            "dd228a5ec59bb99bc6a16074aac7dff2811ec4311b7c62bba3ba5e061cabfff4",
            "62643758586fa8f95e327a7b95610e8698f8c7cceb4d0dde1abfdce18a987869",
            "e4179349d281a258d8543435994ac0b44a66a69c6da18f4387aec5af7d24ff41",
            "b3f057edb00e5e7fff12e2b6a669f78119e3f7692fa3d470cd5c94fa815fcb9c",
        ]
        .contains(&record.qualification_sha256.as_str())
}

// Explicit acceptance of the26 missing illusionable Outfit records from canonical source bindings.
// Exact per-record hashes; never a repository-wide revision permission.
fn qualified_project_illusionable_record(record: &SpellAppearanceRecord) -> bool {
    let Some(defaults) = &record.source.default_source else {
        return false;
    };
    if defaults.path != "src/creatures/creatures_definitions.hpp"
        || defaults.revision != record.source.revision
    {
        return false;
    }
    let header = matches!(
        (
            record.source.revision.as_str(),
            defaults.git_blob.as_str(),
            defaults.sha256.as_str(),
        ),
        (
            "00ce02a57ca5a12e48f32a3476e37471167e4c3f",
            "fc4ae0cf45a56ee7c363b0f452acfb14cbd3c63a",
            "8fbc70bf2416b6e76f17cce3dc1b075f8a5451f0f85b522b0a4ae34cf2cccf94",
        ) | (
            "47dfd51f45280a59a1d3e50ba7edd573d7234446",
            "c8f64483241e0bb7bcca4e1768fda4c0ca386c56",
            "54f0a5bcdc10e33d8850300c5c5022eb7fe8b4549394407482c57245bdab3e33",
        )
    );
    header
        && [
            "2a3c3ef09d34a1a7858588d986d6f151f02c7dcca587552b7e396c684baeef63",
            "91346789cf5b6fef97b6d359cf5f4e88a21ca27279364138a5698c03a43e7b31",
            "ea091ab2a118c93532bff0623338a98e8d8ef97e5ca10457b8cd373779c480c1",
            "89429cb9752ee0aaf7deaadc678c95af9e8e7ff89e05cbeaaaeb4f06818e53ae",
            "6040eb8062205871aa06e5a19265004c08807bc0b8c0ef96139ff2860c5c4f8f",
            "d32cb908e0c618ff50459770ffa5bc06b4e40e427a87242a61e23f3d6db82985",
            "5b6447754148c7fcc7ac078ebbebe2f17f37768229b9ceba10a57ccfdce32ccf",
            "0180246e7bcd7f6bb96355c31bd1351afab7f5fb829879725a3337abc6326bc5",
            "89fbf8310f9123bc210102b41e7321e68948ecd1f89516bf18abc5cfa3143a68",
            "cbd4251248544cb8de20ced19bd8c6d339b79c294deb6939cfa4955a86aed5ab",
            "e29afeb88c553ebf32f82aa1abfca1fca341eb3195d3cf6856be701cabf63691",
            "5e614ab3367ec0b31236c0a5e7c8915ad0620f89a2a4351c9f958848c93fb7f5",
            "d0b635f924025a2b11d4274c1144c9b717dfe69c38047221325d74584af98f41",
            "1d2b0951897e250854d0868fac7c2d555bfb93216d301aafaf8c37713752d766",
            "4fe8c56239af931e8aaac3282dba82f7c189b72f48ff8e860f3c38e59c068d1d",
            "ca6c3efdb0ba0417c94e23ec49fa413c9927ebeb47359f03fccb7e5ed43d1359",
            "d194b7d620a2a6147e90d4cf834b60f76ad51f4c0e4ea7c981688e86cb5b3960",
            "c537b23d07e6e95196a0f6f0f25cc9622160c03c8bd2d84d39417ca869b1f746",
            "f695b0d60348ad5aa26bf3cf161e9b656c126837a77d6a94e83389554f6129f3",
            "33d2d90f98b3e1a9ededb53a9d2e309fdc997a0ac92f7bfbd80dc41aa3dfabeb",
            "f9ff272c11dc2cba4cd9c156915fa6ce5b824fb8cbc85d696b43b440b51dce13",
            "ae7883b6354fc98284e3f694e6bef34a0636fd0045b6d29a26048c9c8ed6d660",
            "bbda60f883946b8ba44193c6dd382011ec982afc09b1f89fc3b2e2b27cd02c53",
            "5e09b9361eb89cd1de45269bc92265037e16de51f861109fde130482d7e956fa",
            "3bc38e4ed050bd6a371f7e491f8c90199b36535ab33359f6cb9928c05e7e336c",
            "133c4593d0c2559a695889fb12cad42602d21922d61405e8b75b43e57f73898c",
        ]
        .contains(&record.qualification_sha256.as_str())
}

fn qualified_crystal_stag(record: &SpellAppearanceRecord) -> bool {
    let Some(creature) = &record.creature else {
        return false;
    };
    let Some(defaults) = &record.source.default_source else {
        return false;
    };
    record.source.revision == CRYSTAL_PIN
        && creature.family == ProjectV2Family::Creature
        && creature.key == "oteryn:creature.stag"
        && creature.revision == "definition-r1"
        && record.outfit_key == "oteryn:outfit.source.crystal.look1913"
        && record.look_type == 1913
        && record.source.path == STAG_PATH
        && record.source.git_blob == STAG_BLOB
        && record.source.sha256 == STAG_SHA
        && defaults.revision == CRYSTAL_PIN
        && defaults.path == "src/creatures/creatures_definitions.hpp"
        && defaults.git_blob == "fc4ae0cf45a56ee7c363b0f452acfb14cbd3c63a"
        && defaults.sha256 == "8fbc70bf2416b6e76f17cce3dc1b075f8a5451f0f85b522b0a4ae34cf2cccf94"
        && record.source.defaulted_fields == ["lookTypeEx"]
        && record.source.explicit_fields.len() == 7
        && record.source.explicit_fields.iter().all(|(key, value)| {
            if key == "lookType" {
                *value == 1913
            } else {
                *value == 0
                    && matches!(
                        key.as_str(),
                        "lookHead"
                            | "lookBody"
                            | "lookLegs"
                            | "lookFeet"
                            | "lookAddons"
                            | "lookMount"
                    )
            }
        })
}
pub(crate) fn compile(
    bytes: &[u8],
    outer_content_digest: [u8; 32],
) -> Result<CompiledSpellAppearances, ContentError> {
    if bytes.len() > 8 * 1024 * 1024 || outer_content_digest == [0; 32] {
        return Err(invalid());
    }
    let document: SpellAppearancesDocument =
        serde_json::from_slice(bytes).map_err(|_| invalid())?;
    if document.schema != "OTERYN_NATIVE_SPELL_APPEARANCES/v1"
        || document.default_source_revision != PIN
        || document.default_source_path != "src/creatures/creatures_definitions.hpp"
        || document.default_source_sha256 != DEFAULT_SHA
        || document.records.is_empty()
        || document.records.len() > 4096
    {
        return Err(invalid());
    }
    let fields = [
        "lookAddons",
        "lookBody",
        "lookFeet",
        "lookHead",
        "lookLegs",
        "lookMount",
        "lookType",
        "lookTypeEx",
    ];
    let mut owners = BTreeSet::new();
    let mut fixed = BTreeSet::new();
    for record in &document.records {
        let crystal_stag = qualified_crystal_stag(record);
        let cached_canary = qualified_cached_canary_record(record);
        let project_illusionable = qualified_project_illusionable_record(record);
        let canary = record.source.revision == PIN && record.source.default_source.is_none();
        if (!canary && !crystal_stag && !cached_canary && !project_illusionable)
            || ((canary || cached_canary)
                && record.outfit_key
                    != format!("oteryn:outfit.source.canary.look{}", record.look_type))
            || record.outfit_revision != "source-outfit-r1"
            || record.look_type == 0
            || record.look_type > u32::from(u16::MAX)
            || record.mount_key.is_some()
            || !hash(&record.source.git_blob, 40)
            || !hash(&record.source.sha256, 64)
            || record.source.path.starts_with('/')
            || record
                .source
                .path
                .split('/')
                .any(|p| p.is_empty() || p == "..")
            || !record.source.path.ends_with(".lua")
            || record
                .source
                .defaulted_fields
                .windows(2)
                .any(|w| w[0] >= w[1])
        {
            return Err(invalid());
        }
        if let Some(creature) = &record.creature {
            if creature.family != ProjectV2Family::Creature
                || !owners.insert((creature.key.clone(), creature.revision.clone()))
                || (!crystal_stag
                    && !project_illusionable
                    && !record
                        .source
                        .path
                        .starts_with("data-otservbr-global/monster/"))
            {
                return Err(invalid());
            }
        } else if !fixed.insert(record.look_type)
            || !record
                .source
                .path
                .starts_with("data/scripts/spells/support/avatar_of_")
        {
            return Err(invalid());
        }
        if record
            .source
            .explicit_fields
            .keys()
            .any(|key| !fields.contains(&key.as_str()))
            || record.source.defaulted_fields.iter().any(|key| {
                !fields.contains(&key.as_str()) || record.source.explicit_fields.contains_key(key)
            })
            || fields.iter().any(|key| {
                !record.source.explicit_fields.contains_key(*key)
                    && !record.source.defaulted_fields.iter().any(|s| s == key)
            })
        {
            return Err(invalid());
        }
        let value = |key: &str| record.source.explicit_fields.get(key).copied().unwrap_or(0);
        if value("lookType") != record.look_type
            || value("lookTypeEx") != 0
            || value("lookMount") != 0
            || value("lookAddons") != u32::from(record.addons)
            || [
                value("lookHead"),
                value("lookBody"),
                value("lookLegs"),
                value("lookFeet"),
            ] != record.colours.map(u32::from)
            || record.colours.iter().any(|v| *v > 255)
        {
            return Err(invalid());
        }
        let mut qualified = serde_json::to_value(record).map_err(|_| invalid())?;
        qualified
            .as_object_mut()
            .ok_or_else(invalid)?
            .remove("qualification_sha256");
        let canonical = serde_json::to_vec(&qualified).map_err(|_| invalid())?;
        if record.qualification_sha256 != hex(&super::digest::sha256(&canonical)) {
            return Err(invalid());
        }
    }
    let mut seen_items = BTreeSet::new();
    if document.item_appearances.len() > 6 {
        return Err(invalid());
    }
    for item in &document.item_appearances {
        if item.item.family != ProjectV2Family::Item
            || !seen_items.insert(item.item.clone())
            || item.source_revision != "47dfd51f45280a59a1d3e50ba7edd573d7234446"
            || item.appearances_sha256
                != "aa44a154f30c7ed59acc25f246286396e4043851ef0b54ef3cf3951e46d1ce50"
            || ![
                "6c4c26d610256cb61a5fce2f3042be1593c1b6417f1b7409ff662e4c137e842e",
                "9cf7f0137cc286f230db23762fbe6c00d3943b8e097f4a848adb16f7a0f9c47a",
                "02fd7503257b05572d3039118e15f2d29748296ea1b4275297ba8c73a68cd2e8",
                "79982193638bc2d037e7edf1d8a6aaa611ba1c8d4aae99ec44bcd1f01d318c36",
                "c0899ab57dab45595afeb53e828d9f8df221f2dcfce6c8b0fd8812b93eeabe4f",
                "2c4b59c75e374f9f1aaaa3ded61353859dc2fa25277e08c01ca6cd1e36c38723",
            ]
            .contains(&item.qualification_sha256.as_str())
        {
            return Err(invalid());
        }
        let mut value = serde_json::to_value(item).map_err(|_| invalid())?;
        value
            .as_object_mut()
            .ok_or_else(invalid)?
            .remove("qualification_sha256");
        let canonical = serde_json::to_vec(&value).map_err(|_| invalid())?;
        if hex(&super::digest::sha256(&canonical)) != item.qualification_sha256 {
            return Err(invalid());
        }
    }
    Ok(CompiledSpellAppearances {
        digest: outer_content_digest,
        records: document.records,
        item_appearances: document.item_appearances,
    })
}
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    fn fixture() -> Vec<u8> {
        include_bytes!("../../../../tools/content-schema/native-gameplay/spell_appearances.json")
            .to_vec()
    }
    fn requalify(record: &mut SpellAppearanceRecord) {
        let mut value = serde_json::to_value(&*record).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .remove("qualification_sha256");
        record.qualification_sha256 = hex(&super::super::digest::sha256(
            &serde_json::to_vec(&value).unwrap(),
        ));
    }
    fn crystal_fixture() -> SpellAppearancesDocument {
        let mut document: SpellAppearancesDocument = serde_json::from_slice(&fixture()).unwrap();
        let mut stag: SpellAppearanceRecord = serde_json::from_value(serde_json::json!({
            "outfit_key":"oteryn:outfit.source.crystal.look1913", "outfit_revision":"source-outfit-r1",
            "look_type":1913, "colours":[0,0,0,0], "addons":0, "mount_key":null,
            "creature":{"family":"Creature","key":"oteryn:creature.stag","revision":"definition-r1"},
            "source":{"revision":"ff7ede593c69d4c658b382c97443e8155926924a",
                "path":"data-global/monster/winter_update_2025/stag.lua",
                "git_blob":"0756e605384b018f75c498bad89ddc72b961c0ca",
                "sha256":"e3ec6434cabac687b502bb69d0a5abd3f40a11fc24ffa92c04d3cef45e777aeb",
                "explicit_fields":{"lookType":1913,"lookHead":0,"lookBody":0,"lookLegs":0,"lookFeet":0,"lookAddons":0,"lookMount":0},
                "defaulted_fields":["lookTypeEx"],
                "default_source":{"revision":"ff7ede593c69d4c658b382c97443e8155926924a",
                    "path":"src/creatures/creatures_definitions.hpp",
                    "git_blob":"fc4ae0cf45a56ee7c363b0f452acfb14cbd3c63a",
                    "sha256":"8fbc70bf2416b6e76f17cce3dc1b075f8a5451f0f85b522b0a4ae34cf2cccf94"}},
            "qualification_sha256":""
        })).unwrap();
        requalify(&mut stag);
        document.records.push(stag);
        document
    }
    #[test]
    fn crystal_stag_uses_its_exact_primary_and_default_source() {
        let document = crystal_fixture();
        let compiled = compile(&serde_json::to_vec(&document).unwrap(), [7; 32]).unwrap();
        let stag = compiled
            .for_creature("oteryn:creature.stag", "definition-r1")
            .unwrap();
        assert_eq!(stag.look_type(), 1913);
        assert_eq!(
            stag.selection().outfit_key,
            "oteryn:outfit.source.crystal.look1913"
        );
        assert_eq!(stag.source_digest(), [7; 32]);
        assert!(
            compiled
                .for_creature("canary:creature/stag", "definition-r1")
                .is_none()
        );
    }
    #[test]
    fn crystal_primary_default_and_identity_substitutions_refuse_after_rehash() {
        let mutations: &[fn(&mut SpellAppearanceRecord)] = &[
            |r| r.source.revision = PIN.into(),
            |r| r.creature.as_mut().unwrap().key = "oteryn:creature.rat".into(),
            |r| r.creature.as_mut().unwrap().revision = "definition-r2".into(),
            |r| r.source.path = "data-global/monster/winter_update_2025/other.lua".into(),
            |r| r.source.git_blob.replace_range(0..1, "1"),
            |r| r.source.sha256.replace_range(0..1, "1"),
            |r| r.source.default_source = None,
            |r| r.source.default_source.as_mut().unwrap().revision = PIN.into(),
            |r| r.source.default_source.as_mut().unwrap().path = "other.hpp".into(),
            |r| {
                r.source
                    .default_source
                    .as_mut()
                    .unwrap()
                    .git_blob
                    .replace_range(0..1, "1")
            },
            |r| {
                r.source
                    .default_source
                    .as_mut()
                    .unwrap()
                    .sha256
                    .replace_range(0..1, "1")
            },
            |r| {
                r.source.defaulted_fields.clear();
                r.source.explicit_fields.insert("lookTypeEx".into(), 0);
            },
        ];
        for (index, change) in mutations.iter().enumerate() {
            let mut document = crystal_fixture();
            let stag = document.records.last_mut().unwrap();
            change(stag);
            requalify(stag);
            assert!(
                compile(&serde_json::to_vec(&document).unwrap(), [7; 32]).is_err(),
                "mutation {index}"
            );
        }
    }
    #[test]
    fn actual_source_selections_bind_active_generation() {
        let compiled = compile(&fixture(), [7; 32]).unwrap();
        let rat = compiled
            .for_creature("canary:creature/rat", "canary-47dfd51f")
            .unwrap();
        assert_eq!(rat.look_type(), 21);
        assert_eq!(rat.source_digest(), [7; 32]);
        assert_eq!(rat.selection().colours, [0; 4]);
        assert!(
            compiled
                .for_creature("canary:creature/rat", "tampered")
                .is_none()
        );
        assert!(compiled.for_look_type(1593).is_some());
    }
    #[test]
    fn changed_selection_missing_source_and_unknown_generation_refuse() {
        assert!(compile(&fixture(), [0; 32]).is_err());
        let mut document: serde_json::Value = serde_json::from_slice(&fixture()).unwrap();
        document["records"][0]["colours"][0] = serde_json::json!(33);
        assert!(compile(&serde_json::to_vec(&document).unwrap(), [7; 32]).is_err());
        document["default_source_sha256"] = serde_json::json!("unqualified");
        assert!(compile(&serde_json::to_vec(&document).unwrap(), [7; 32]).is_err());
    }
}

#[cfg(test)]
mod exact_selection_regression_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    fn compiled() -> CompiledSpellAppearances {
        compile(
            include_bytes!(
                "../../../../tools/content-schema/native-gameplay/spell_appearances.json"
            ),
            [9; 32],
        )
        .unwrap()
    }
    #[test]
    fn creature_linked_record_reaches_complete_selection_reader_without_revision_alias() {
        let registry = compiled();
        let record = registry
            .records
            .iter()
            .find(|r| r.creature.is_some())
            .unwrap();
        let creature = record.creature.as_ref().unwrap();
        let member = registry
            .for_creature(&creature.key, &creature.revision)
            .unwrap();
        assert!(registry.for_look_type(member.look_type()).is_none());
        assert_eq!(
            registry
                .for_selection(member.look_type(), &member.selection())
                .unwrap()
                .selection(),
            member.selection()
        );
        assert_eq!(
            registry
                .for_selection(member.look_type(), &member.selection())
                .unwrap()
                .source_digest(),
            [9; 32]
        );
        assert!(
            registry
                .for_creature(&creature.key, "definition-not-current")
                .is_none()
        );
        assert!(
            registry
                .for_creature("oteryn:creature.rat", &creature.revision)
                .is_none()
        );
    }
    #[test]
    fn wrong_selection_and_numeric_membership_refuse_while_native_fixed_spell_still_resolves() {
        let registry = compiled();
        let record = registry
            .records
            .iter()
            .find(|r| r.creature.is_none())
            .unwrap();
        let member = registry.for_look_type(record.look_type).unwrap();
        let original = member.selection();
        assert_eq!(
            registry
                .for_selection(record.look_type, &original)
                .unwrap()
                .selection(),
            original
        );
        let mut wrong = original.clone();
        wrong.colours[0] = wrong.colours[0].wrapping_add(1);
        assert!(registry.for_selection(record.look_type, &wrong).is_none());
        wrong = original.clone();
        wrong.outfit_key = "forged:outfit".into();
        assert!(registry.for_selection(record.look_type, &wrong).is_none());
        wrong = original.clone();
        wrong.addons = wrong.addons.wrapping_add(1);
        assert!(registry.for_selection(record.look_type, &wrong).is_none());
        wrong = original.clone();
        wrong.mount_key = Some("forged:mount".into());
        assert!(registry.for_selection(record.look_type, &wrong).is_none());
        assert!(registry.for_selection(u32::MAX, &original).is_none());
    }
}

#[cfg(test)]
mod cached_canary_admission_regression_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    fn actual_document() -> SpellAppearancesDocument {
        serde_json::from_slice(include_bytes!(
            "../../../../content/presentations/bindings/spell-appearances.json"
        ))
        .unwrap()
    }
    fn requalify(record: &mut SpellAppearanceRecord) {
        let mut value = serde_json::to_value(&*record).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .remove("qualification_sha256");
        record.qualification_sha256 = hex(&super::super::digest::sha256(
            &serde_json::to_vec(&value).unwrap(),
        ));
    }
    #[test]
    fn actual_project_admits_exact40_cached_records_preserving_enclosing_source_pin() {
        let document = actual_document();
        assert_eq!(document.records.len(), 334);
        assert_eq!(document.default_source_revision, PIN);
        let cached: Vec<_> = document
            .records
            .iter()
            .filter(|r| qualified_cached_canary_record(r))
            .collect();
        assert_eq!(cached.len(), 40);
        assert!(cached.iter().all(|r| qualified_cached_canary_record(r)));
        let registry = compile(&serde_json::to_vec(&document).unwrap(), [9; 32]).unwrap();
        for record in cached {
            let owner = record.creature.as_ref().unwrap();
            let member = registry.for_creature(&owner.key, &owner.revision).unwrap();
            assert_eq!(member.look_type(), record.look_type);
            assert_eq!(
                registry
                    .for_selection(member.look_type(), &member.selection())
                    .unwrap()
                    .selection(),
                member.selection()
            );
            assert!(
                registry
                    .for_creature(&owner.key, "definition-forged")
                    .is_none()
            );
        }
    }
    #[test]
    fn valid_self_checksum_does_not_admit_tampered_look_target_header_or_revision() {
        let original = actual_document();
        let index = original
            .records
            .iter()
            .position(|r| r.source.revision == "47dfd51f45280a59a1d3e50ba7edd573d7234446")
            .unwrap();
        for mutation in 0..4 {
            let mut document = original.clone();
            let record = &mut document.records[index];
            match mutation {
                0 => {
                    record.look_type += 1;
                    record.outfit_key =
                        format!("oteryn:outfit.source.canary.look{}", record.look_type);
                    record
                        .source
                        .explicit_fields
                        .insert("lookType".into(), record.look_type);
                }
                1 => record.creature.as_mut().unwrap().key.push_str("_forged"),
                2 => record.source.default_source.as_mut().unwrap().sha256 = "0".repeat(64),
                _ => record.source.revision = PIN.into(),
            }
            requalify(record);
            assert!(!qualified_cached_canary_record(record));
            assert!(compile(&serde_json::to_vec(&document).unwrap(), [9; 32]).is_err());
        }
    }
}

#[cfg(test)]
mod project_illusionable_admission_regression_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    fn document() -> SpellAppearancesDocument {
        serde_json::from_slice(include_bytes!(
            "../../../../content/presentations/bindings/spell-appearances.json"
        ))
        .unwrap()
    }
    fn requalify(record: &mut SpellAppearanceRecord) {
        let mut value = serde_json::to_value(&*record).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .remove("qualification_sha256");
        record.qualification_sha256 = hex(&super::super::digest::sha256(
            &serde_json::to_vec(&value).unwrap(),
        ));
    }
    #[test]
    fn exact26_canonical_missing_illusionable_outfits_compile_without_object_grant_or_old40_scope_change()
     {
        let document = document();
        assert_eq!(document.records.len(), 334);
        assert_eq!(document.default_source_revision, PIN);
        let added: Vec<_> = document
            .records
            .iter()
            .filter(|r| qualified_project_illusionable_record(r))
            .collect();
        assert_eq!(added.len(), 26);
        assert_eq!(
            document
                .records
                .iter()
                .filter(|r| qualified_cached_canary_record(r))
                .count(),
            40
        );
        let registry = compile(&serde_json::to_vec(&document).unwrap(), [9; 32]).unwrap();
        for record in added {
            let owner = record.creature.as_ref().unwrap();
            let actual = registry.for_creature(&owner.key, &owner.revision).unwrap();
            assert_eq!(actual.look_type(), record.look_type);
            assert_eq!(actual.selection().colours, record.colours);
            assert!(
                registry
                    .for_creature(&owner.key, "definition-forged")
                    .is_none()
            );
        }
        assert!(
            registry
                .for_creature("oteryn:creature.enraged_bookworm", "definition-r1")
                .is_none()
        );
    }
    #[test]
    fn recomputed_self_checksums_do_not_expand_additional26_source_scope() {
        let original = document();
        for revision in [
            "00ce02a57ca5a12e48f32a3476e37471167e4c3f",
            "47dfd51f45280a59a1d3e50ba7edd573d7234446",
        ] {
            let index = original
                .records
                .iter()
                .position(|r| {
                    r.source.revision == revision && qualified_project_illusionable_record(r)
                })
                .unwrap();
            for mutation in 0..4 {
                let mut document = original.clone();
                let record = &mut document.records[index];
                match mutation {
                    0 => {
                        record.look_type += 1;
                        record
                            .source
                            .explicit_fields
                            .insert("lookType".into(), record.look_type);
                    }
                    1 => record.creature.as_mut().unwrap().key.push_str("_forged"),
                    2 => record.source.default_source.as_mut().unwrap().sha256 = "0".repeat(64),
                    _ => record.source.revision = PIN.into(),
                }
                requalify(record);
                assert!(!qualified_project_illusionable_record(record));
                assert!(compile(&serde_json::to_vec(&document).unwrap(), [9; 32]).is_err());
            }
        }
    }
}

#[cfg(test)]
mod bounded_item_appearance_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    #[test]
    fn exact_six_item_appearances_do_not_grant_arbitrary_item_or_revision() {
        let bytes =
            include_bytes!("../../../../content/presentations/bindings/spell-appearances.json");
        let registry = compile(bytes, [9; 32]).unwrap();
        assert_eq!(registry.item_appearances.len(), 6);
        for r in &registry.item_appearances {
            assert_eq!(
                registry.for_item(&r.item.key, &r.item.revision),
                Some(&r.item)
            );
            assert!(registry.for_item(&r.item.key, "definition-r2").is_none());
        }
        assert!(
            registry
                .for_item("oteryn:item.tibia.i1", "definition-r1")
                .is_none()
        );
        let doc: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        for field in [
            "source_revision",
            "canonical_definition_sha256",
            "appearance_object_sha256",
            "appearances_sha256",
        ] {
            let mut wrong = doc.clone();
            wrong["item_appearances"][0][field] = serde_json::Value::String("f".repeat(64));
            let mut record = wrong["item_appearances"][0].clone();
            record
                .as_object_mut()
                .unwrap()
                .remove("qualification_sha256");
            wrong["item_appearances"][0]["qualification_sha256"] = serde_json::Value::String(hex(
                &super::super::digest::sha256(&serde_json::to_vec(&record).unwrap()),
            ));
            assert!(compile(&serde_json::to_vec(&wrong).unwrap(), [9; 32]).is_err());
        }
        let mut wrong = doc.clone();
        wrong["item_appearances"][0]["item"]["revision"] = serde_json::json!("definition-r2");
        assert!(compile(&serde_json::to_vec(&wrong).unwrap(), [9; 32]).is_err());
    }
}
