//! Read-only public projection of the exact graph admitted by the server's CONTENT loader.
//! No donor documents, migration shards or display names are parsed as native authority.
use super::*;
use crate::content::{
    REFERENCE_ITEM_MAX_NAME_BYTES, REFERENCE_LOOT_PROBABILITY_PPM_SCALE, ReferenceItemField,
};
use serde_json::{Value, json};

pub const CATALOG_CAPABILITIES: [&str; 8] = [
    "achievement",
    "creature",
    "creature_loot",
    "item",
    "npc",
    "npc_shop",
    "quest",
    "spell",
];

const WIRE_MAX_STRING_BYTES: usize = 2_048;
const WIRE_MAX_ARRAY_ENTRIES: usize = 200_000;
const WIRE_MAX_RELATIONS: usize = 1_000_000;
const WIRE_MAX_FILE_BYTES: usize = 256 * 1024 * 1024;
// Each capability gets a work limit independent of the shared wire row/byte
// limits. Shared definitions can fan out even if no candidate makes a row.
const MAX_RELATION_CANDIDATES_PER_CAPABILITY: usize = 2 * WIRE_MAX_RELATIONS;
// Covers the fixed v1 preproduction envelope, manifests, digests and final newline.
const WIRE_ENVELOPE_BYTES: usize = 2_048;

// These are the v1 carrier bounds enforced by the producer. The native graph
// can admit wider values and stays intact when the public view omits them.
fn wire_key(key: &str) -> bool {
    fn segment(value: &str) -> bool {
        value.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
            && value.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"_.-".contains(&byte)
            })
    }
    key.len() <= WIRE_MAX_STRING_BYTES
        && key
            .split_once(':')
            .is_some_and(|(namespace, local)| segment(namespace) && segment(local))
}

fn wire_revision(revision: &str) -> bool {
    let lower = revision.to_ascii_lowercase();
    !revision.is_empty()
        && revision.len() <= 512
        && revision.bytes().all(|byte| byte.is_ascii_graphic())
        && !["fixture", "synthetic", "evidence", "test-only"]
            .iter()
            .any(|marker| lower.contains(marker))
        && !lower
            .split([':', '.', '/', '_', '-'])
            .any(|part| part == "test")
}

fn wire_source_text(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= WIRE_MAX_STRING_BYTES
        && text.trim() == text
        && !text.chars().any(char::is_control)
}

fn project_creature_fields(profile: &ProjectV2CreatureAuthoring, data: &mut Value) -> bool {
    let mut complete = true;
    if let Some(details) = &profile.details {
        if wire_source_text(&details.display_name) {
            data["name"] = json!(details.display_name);
        } else {
            complete = false;
        }
    }
    for (field, value) in [
        ("health", profile.health),
        ("experience", profile.experience),
        ("speed", profile.speed.map(u64::from)),
        ("armor", profile.armor.map(u64::from)),
    ] {
        if let Some(value) = value {
            if let Ok(value) = i64::try_from(value) {
                data[field] = json!(value);
            } else {
                complete = false;
            }
        }
    }
    // Preserve a list only when every member can be represented exactly.
    if !profile.resistances.is_empty() {
        if profile.resistances.len() <= WIRE_MAX_ARRAY_ENTRIES
            && profile.resistances.iter().all(|entry| {
                wire_source_text(&entry.damage_type)
                    && i64::try_from(entry.percent.denominator).is_ok()
            })
        {
            data["resistances"] = json!(profile.resistances);
        } else {
            complete = false;
        }
    }
    if !profile.immunities.is_empty() {
        if profile.immunities.len() <= WIRE_MAX_ARRAY_ENTRIES
            && profile.immunities.iter().all(|text| wire_source_text(text))
        {
            data["immunities"] = json!(profile.immunities);
        } else {
            complete = false;
        }
    }
    complete
}

/// Public projection and private coverage evidence from one admitted WorldProject.
#[derive(Debug, Serialize)]
pub struct PublicContentView {
    pub entities: Vec<Value>,
    pub relations: Vec<Value>,
    /// Local coverage evidence, never included in the Platform artifact (may name hidden keys).
    #[serde(skip)]
    pub coverage: Vec<PublicContentCoverage>,
    #[serde(skip)]
    pub field_inventory: BTreeSet<String>,
    #[serde(skip)]
    incomplete: BTreeSet<&'static str>,
}

#[derive(Debug)]
pub struct PublicContentCoverage {
    pub key: String,
    pub category: String,
    pub disposition: &'static str,
    pub reason: &'static str,
}

impl PublicContentView {
    /// Compact review fixture: any populated new field or category changes its fingerprint.
    /// Values are excluded except extensible candidate field names, which need review too.
    pub fn field_fingerprints(&self) -> BTreeMap<String, String> {
        use sha2::{Digest, Sha256};
        let mut groups = BTreeMap::<String, Sha256>::new();
        for path in &self.field_inventory {
            let group = if path.starts_with("item_authoring.") {
                "item_authoring".to_owned()
            } else {
                path.split('.').take(2).collect::<Vec<_>>().join(".")
            };
            let digest = groups.entry(group).or_default();
            digest.update(path.as_bytes());
            digest.update(b"\n");
        }
        groups
            .into_iter()
            .map(|(group, digest)| {
                (
                    group,
                    digest
                        .finalize()
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect(),
                )
            })
            .collect()
    }
    /// Emit the locked v1 artifact, with capabilities gated on Platform validation.
    pub fn native_input(&self, source_revision: &str, generated_at: &str) -> Value {
        self.catalog_input(source_revision, generated_at, false)
    }

    /// Emit the opt-in profile for offline compatibility under export contract §15.
    pub fn preproduction_input(&self, source_revision: &str, generated_at: &str) -> Value {
        self.catalog_input(source_revision, generated_at, true)
    }

    fn catalog_input(
        &self,
        source_revision: &str,
        generated_at: &str,
        preproduction: bool,
    ) -> Value {
        let entities: Vec<_> = self
            .entities
            .iter()
            .filter(|row| preproduction && is_catalogue_entity(row))
            .collect();
        let relations = if preproduction {
            self.relations.as_slice()
        } else {
            &[]
        };
        let supported: BTreeSet<_> = entities
            .iter()
            .copied()
            .chain(relations)
            .filter_map(|row| row["capability_id"].as_str())
            .collect();
        json!({
            "authority_epoch": "preproduction-unallocated",
            "source_revision": source_revision,
            "generated_at": generated_at,
            "ruleset_id": "oteryn:ruleset.public-catalog-candidate",
            "content_profile_id": "oteryn:profile.public-content-view-v1",
            "required_capabilities": [],
            "capability_manifest": CATALOG_CAPABILITIES.map(|id| json!({
                "capability_id":id,"support":if supported.contains(id) { "supported" } else { "unsupported" }
            })),
            "completeness_manifest": CATALOG_CAPABILITIES.map(|id| json!({
                "capability_id":id,"state":if !supported.contains(id) { "unknown" }
                    else if self.incomplete.contains(id) { "partial" } else { "complete" }
            })),
            "entities": entities, "relations": relations, "tombstones": []
        })
    }
}

fn is_catalogue_entity(row: &Value) -> bool {
    matches!(
        row["capability_id"].as_str(),
        Some("item" | "creature" | "npc")
    )
}

fn wire_row_bytes(row: &Value) -> Result<usize, ProjectError> {
    // Charge one byte per row for its array separator.
    serde_json::to_vec(row)
        .map(|bytes| bytes.len() + 1)
        .map_err(|error| ProjectError::InvalidJson(error.to_string()))
}

fn push_relation(
    relations: &mut Vec<Value>,
    remaining_bytes: &mut usize,
    row: Value,
) -> Result<bool, ProjectError> {
    let bytes = wire_row_bytes(&row)?;
    if relations.len() == WIRE_MAX_RELATIONS || bytes > *remaining_bytes {
        return Ok(false);
    }
    *remaining_bytes -= bytes;
    relations.push(row);
    Ok(true)
}

fn inventory(value: &Value, path: &str, output: &mut BTreeSet<String>) {
    match value {
        Value::Object(fields) => {
            for (key, value) in fields {
                let next = format!("{path}.{key}");
                output.insert(next.clone());
                if key == "field_path" {
                    output.insert(format!("{next}={value}"));
                }
                inventory(value, &next, output);
            }
        }
        Value::Array(values) => {
            for value in values {
                inventory(value, &format!("{path}[]"), output);
            }
        }
        _ => {}
    }
}

fn project_json<T: Serialize>(value: &T) -> Result<Value, ProjectError> {
    serde_json::to_value(value).map_err(|error| ProjectError::InvalidJson(error.to_string()))
}

fn entity(kind: &str, key: &str, revision: &str, source: &Value, fields: &[&str]) -> Value {
    let mut data = serde_json::Map::new();
    data.insert("payload_revision".into(), json!(1));
    data.insert("definition_revision".into(), json!(revision));
    for field in fields {
        if let Some(value) = source.get(field) {
            data.insert((*field).into(), value.clone());
        }
    }
    json!({"type":kind,"content_key":key,"capability_id":kind,"data":data})
}

// The wire identity is shared across capabilities. Omit every claimant of an
// ambiguous key so downstream relations cannot bind to an arbitrary definition.
fn omit_ambiguous_rows(
    rows: &mut Vec<Value>,
    key_field: &str,
    incomplete: &mut BTreeSet<&'static str>,
) -> BTreeSet<String> {
    let mut observed = BTreeSet::new();
    let mut ambiguous = BTreeSet::new();
    for row in rows.iter() {
        if let Some(key) = row[key_field].as_str()
            && !observed.insert(key)
        {
            ambiguous.insert(key.to_owned());
        }
    }
    rows.retain(|row| {
        if row[key_field]
            .as_str()
            .is_some_and(|key| ambiguous.contains(key))
        {
            if let Some(cap) = CATALOG_CAPABILITIES
                .into_iter()
                .find(|cap| row["capability_id"] == *cap)
            {
                incomplete.insert(cap);
            }
            return false;
        }
        true
    });
    ambiguous
}

/// Consume an already validated immutable WorldProject. The server and tools share
/// capture_world_project / ProjectSnapshot::parse; there is deliberately no JSON input here.
pub fn public_content_view(project: &WorldProject) -> Result<PublicContentView, ProjectError> {
    let mut view = PublicContentView {
        entities: Vec::new(),
        relations: Vec::new(),
        coverage: Vec::new(),
        field_inventory: BTreeSet::new(),
        incomplete: BTreeSet::new(),
    };
    let state = project
        .v2
        .as_ref()
        .ok_or(ProjectError::InvalidProject("public view requires v2"))?;
    let profiles: BTreeMap<_, _> = state
        .authoring_profiles
        .iter()
        .map(|profile| {
            let target = &profile.target;
            (
                (target.family, target.key.as_str(), target.revision.as_str()),
                &profile.data,
            )
        })
        .collect();
    let mut loot = BTreeMap::new();
    for record in &project.reference.records {
        let raw = project_json(record)?;
        let identity = record.identity();
        let profile_key = (
            ProjectV2Family::from_reference(&identity.family)?,
            identity.key.as_str(),
            identity.revision.as_str(),
        );
        inventory(
            &raw,
            &format!("reference.{}", identity.family),
            &mut view.field_inventory,
        );
        let kind = match record {
            ProjectReferenceRecord::Item {
                client_projection: ProjectionDocument::ClientSafe,
                ..
            } => Some("item"),
            ProjectReferenceRecord::Creature {
                client_projection: ProjectionDocument::ClientSafe,
                ..
            } => Some("creature"),
            ProjectReferenceRecord::Item { .. } => {
                view.incomplete.insert("item");
                None
            }
            ProjectReferenceRecord::Creature { .. } => {
                view.incomplete.insert("creature");
                None
            }
            ProjectReferenceRecord::Ability { .. } => match profiles.get(&profile_key) {
                Some(ProjectV2AuthoringProfileData::Ability(profile))
                    if profile.incantation.is_some() =>
                {
                    Some("spell")
                }
                _ => None,
            },
            ProjectReferenceRecord::Loot {
                algorithm, entries, ..
            } => {
                let conditional = matches!(
                    profiles.get(&profile_key),
                    Some(ProjectV2AuthoringProfileData::Loot(profile))
                        if profile.entries.iter().any(|entry| entry.skip_later_same_item_after_success)
                );
                let wire_eligible =
                    matches!(algorithm, LootAlgorithmDocument::IndependentBernoulliPpm)
                        && !conditional
                        && entries.iter().all(|entry| {
                            entry.children.is_empty()
                                && !entry.unique
                                && entry.subtype.is_none()
                                && entry.probability_ppm.is_some()
                        });
                loot.insert(
                    (identity.key.as_str(), identity.revision.as_str()),
                    (algorithm, entries, wire_eligible),
                );
                None
            }
            _ => None,
        };
        if let Some(kind) = kind {
            let mut projected = entity(
                kind,
                &identity.key,
                &identity.revision,
                &raw,
                if kind == "item" {
                    &["materializable", "stack_class"]
                } else {
                    &[]
                },
            );
            if let ProjectReferenceRecord::Item { semantics, .. } = record
                && let ReferenceItemField::Known(presentation) = &semantics.presentation
                && let ReferenceItemField::Known(name) = &presentation.name
            {
                if name.is_empty() || name.len() > REFERENCE_ITEM_MAX_NAME_BYTES {
                    view.incomplete.insert("item");
                } else {
                    projected["data"]["name"] = json!(name);
                }
            }
            if let Some(profile) = profiles.get(&profile_key) {
                if let ProjectV2AuthoringProfileData::Creature(profile) = profile {
                    if !project_creature_fields(profile, &mut projected["data"]) {
                        view.incomplete.insert("creature");
                    }
                } else if kind == "spell" {
                    let profile = project_json(profile)?;
                    for field in [
                        "incantation",
                        "vocations",
                        "required_level",
                        "premium",
                        "mana_cost",
                        "cooldown_ms",
                        "group_cooldown_ms",
                        "range",
                    ] {
                        if let Some(value) = profile["profile"].get(field) {
                            projected["data"][field] = value.clone();
                        }
                    }
                }
            }
            view.entities.push(projected);
        }
        view.coverage.push(PublicContentCoverage {
            key: identity.key.clone(),
            category: identity.family.clone(),
            disposition: if kind.is_some() {
                "projected"
            } else {
                "unsupported"
            },
            reason: if kind.is_some() {
                "admitted public projection"
            } else {
                "not a proven public capability payload; retained in server graph"
            },
        });
    }
    let mut npcs = Vec::new();
    let mut shop_services = BTreeMap::new();
    for declaration in &state.declarations {
        let raw = project_json(declaration)?;
        let identity = declaration.identity();
        inventory(
            &raw,
            &format!("declaration.{:?}", declaration.family()),
            &mut view.field_inventory,
        );
        let hidden = !declaration.public_visibility().is_public()
            || matches!(
                declaration,
                ProjectV2Declaration::Achievement {
                    secret: Some(true),
                    ..
                }
            );
        match declaration {
            ProjectV2Declaration::Npc { services, .. } => {
                npcs.push((identity, services));
                if hidden {
                    view.incomplete.insert("npc");
                }
            }
            ProjectV2Declaration::Service { offers, .. } => {
                shop_services.insert((identity.key.as_str(), identity.revision.as_str()), offers);
            }
            _ => {}
        }
        let kind = if hidden {
            None
        } else {
            match declaration {
                ProjectV2Declaration::Npc { .. } => Some("npc"),
                ProjectV2Declaration::Quest { .. } => Some("quest"),
                ProjectV2Declaration::Achievement { .. } => Some("achievement"),
                _ => None,
            }
        };
        if let Some(kind) = kind {
            view.entities.push(entity(
                kind,
                &identity.key,
                &identity.revision,
                &raw,
                if kind == "achievement" {
                    &["degree", "points", "premium"]
                } else {
                    &[]
                },
            ));
        }
        view.coverage.push(PublicContentCoverage {
            key: identity.key.clone(),
            category: format!("{:?}", declaration.family()),
            disposition: if hidden {
                "hidden"
            } else if kind.is_some() {
                "projected"
            } else {
                "unsupported"
            },
            reason: if hidden {
                "G-2 visibility or secret achievement: never publish identity or references"
            } else if kind.is_some() {
                "admitted public projection"
            } else if matches!(declaration, ProjectV2Declaration::Service { routes, .. } if !routes.is_empty()) {
                // Premium, minimum level and required quest tracks (track, minimum, maximum)
                // are server-side travel gates. Only shop offers have a public projection below.
                "travel routes and eligibility gates are unsupported in the public view"
            } else {
                "outside the closed v1 public capability vocabulary"
            },
        });
    }
    let ambiguous = omit_ambiguous_rows(&mut view.entities, "content_key", &mut view.incomplete);
    let mut outside_wire = BTreeSet::new();
    view.entities.retain(|row| {
        let Some((capability, key, revision)) = row["content_key"]
            .as_str()
            .zip(row["data"]["definition_revision"].as_str())
            .and_then(|(key, revision)| {
                ["item", "creature", "npc"]
                    .into_iter()
                    .find(|cap| row["capability_id"] == *cap)
                    .map(|cap| (cap, key, revision))
            })
        else {
            return true;
        };
        if wire_key(key) && wire_revision(revision) {
            return true;
        }
        view.incomplete.insert(capability);
        outside_wire.insert(key.to_owned());
        false
    });
    for row in &mut view.coverage {
        if row.disposition != "projected" {
            continue;
        }
        if ambiguous.contains(row.key.as_str()) {
            row.disposition = "unsupported";
            row.reason = "ambiguous public identity";
        } else if outside_wire.contains(row.key.as_str()) {
            row.disposition = "unsupported";
            row.reason = "public identity outside v1 wire bounds";
        }
    }
    let endpoints: BTreeMap<_, _> = view
        .entities
        .iter()
        .filter_map(|row| {
            Some((
                row["content_key"].as_str()?,
                (
                    row["type"].as_str()?,
                    row["data"]["definition_revision"].as_str()?,
                ),
            ))
        })
        .collect();
    let is_exported = |kind, key, revision| endpoints.get(key) == Some(&(kind, revision));

    let mut relation_budget = WIRE_MAX_FILE_BYTES - WIRE_ENVELOPE_BYTES;
    for row in view.entities.iter().filter(|row| is_catalogue_entity(row)) {
        relation_budget = relation_budget.saturating_sub(wire_row_bytes(row)?);
    }
    let mut remaining_bytes = relation_budget;

    // Drop an overflowing capability as a unit. Keeping a prefix could keep
    // one claimant of an ambiguous relation key whose other claimant was cut off.
    let mut shop_candidates = MAX_RELATION_CANDIDATES_PER_CAPABILITY;
    'shops: for (identity, services) in npcs {
        let npc = identity.key.as_str();
        let npc_exported = is_exported("npc", npc, &identity.revision);
        let mut seen = BTreeSet::new();
        for service in services {
            if !seen.insert(service) {
                continue;
            }
            let Some(offers) =
                shop_services.get(&(service.key.as_str(), service.revision.as_str()))
            else {
                view.incomplete.insert("npc_shop");
                continue;
            };
            if !npc_exported && !offers.is_empty() {
                view.incomplete.insert("npc_shop");
                continue;
            }
            let Some(remaining_candidates) = shop_candidates.checked_sub(offers.len()) else {
                view.relations.clear();
                remaining_bytes = relation_budget;
                view.incomplete.insert("npc_shop");
                break 'shops;
            };
            shop_candidates = remaining_candidates;
            for (ordinal, offer) in offers.iter().enumerate() {
                if offer.parity_pending
                    || i64::try_from(offer.unit_price).is_err()
                    || !is_exported(
                        "item",
                        offer.item.key.as_str(),
                        offer.item.revision.as_str(),
                    )
                    || offer.currency.as_ref().is_some_and(|currency| {
                        !is_exported("item", currency.key.as_str(), currency.revision.as_str())
                    })
                {
                    view.incomplete.insert("npc_shop");
                    continue;
                }
                let relation_key =
                    format!("{npc}.shop.{}.row.{ordinal}", service.key.replace(':', "."));
                if !wire_key(&relation_key) {
                    view.incomplete.insert("npc_shop");
                    continue;
                }
                let row = json!({
                    "type":"npc_shop", "capability_id":"npc_shop",
                    "relation_key":relation_key,
                    "source":npc,"target":offer.item.key,
                    "data":{"payload_revision":1,"direction":project_json(&offer.direction)?,"unit_price":offer.unit_price,
                        "count":offer.count,"sub_type":offer.sub_type,
                        "currency":offer.currency.as_ref().map(|currency| &currency.key)}
                });
                if !push_relation(&mut view.relations, &mut remaining_bytes, row)? {
                    view.relations.clear();
                    remaining_bytes = relation_budget;
                    view.incomplete.insert("npc_shop");
                    break 'shops;
                }
            }
        }
    }
    let shop_rows = view.relations.len();
    let mut loot_candidates = MAX_RELATION_CANDIDATES_PER_CAPABILITY;
    'loot: for record in &project.reference.records {
        let ProjectReferenceRecord::Creature {
            identity,
            loot: Some(table),
            ..
        } = record
        else {
            continue;
        };
        let Some((algorithm, entries, wire_eligible)) =
            loot.get(&(table.key.as_str(), table.revision.as_str()))
        else {
            view.incomplete.insert("creature_loot");
            continue;
        };
        if entries.is_empty() {
            continue;
        }
        if !is_exported(
            "creature",
            identity.key.as_str(),
            identity.revision.as_str(),
        ) || !wire_eligible
        {
            view.incomplete.insert("creature_loot");
            continue;
        }
        let Some(remaining_candidates) = loot_candidates.checked_sub(entries.len()) else {
            view.relations.truncate(shop_rows);
            view.incomplete.insert("creature_loot");
            break 'loot;
        };
        loot_candidates = remaining_candidates;
        for (ordinal, entry) in entries.iter().enumerate() {
            let relation_key = format!("{}.loot.{ordinal}", identity.key);
            if !wire_key(&relation_key)
                || entry.min_count == 0
                || entry.min_count > entry.max_count
                || entry
                    .probability_ppm
                    .is_some_and(|probability| probability > REFERENCE_LOOT_PROBABILITY_PPM_SCALE)
                || !is_exported(
                    "item",
                    entry.item.key.as_str(),
                    entry.item.revision.as_str(),
                )
            {
                view.incomplete.insert("creature_loot");
                continue;
            }
            let row = json!({"type":"creature_loot","capability_id":"creature_loot",
                "relation_key":relation_key,
                "source":identity.key,"target":entry.item.key,
                "data":{"payload_revision":1,"algorithm":algorithm,"min_count":entry.min_count,
                    "max_count":entry.max_count,"probability_ppm":entry.probability_ppm}});
            if !push_relation(&mut view.relations, &mut remaining_bytes, row)? {
                view.relations.truncate(shop_rows);
                view.incomplete.insert("creature_loot");
                break 'loot;
            }
        }
    }
    omit_ambiguous_rows(&mut view.relations, "relation_key", &mut view.incomplete);
    for profile in &state.authoring_profiles {
        inventory(
            &project_json(&profile.data)?,
            &format!("profile.{:?}", profile.target.family),
            &mut view.field_inventory,
        );
    }
    for item in &state.item_authoring {
        inventory(
            &project_json(item)?,
            "item_authoring",
            &mut view.field_inventory,
        );
    }
    view.entities
        .sort_by(|a, b| a["content_key"].as_str().cmp(&b["content_key"].as_str()));
    view.relations
        .sort_by(|a, b| a["relation_key"].as_str().cmp(&b["relation_key"].as_str()));
    Ok(view)
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

    fn project_for_projection_test()
    -> Result<WorldProject, Box<dyn std::error::Error + Send + Sync>> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../content/world");
        let limits = crate::content::npc_catalogue_preproduction_limits().project;
        let locators = [
            "project.json",
            "manifest.json",
            "content.lock.json",
            "assets/catalog.json",
            "definitions/declarations.json",
            "definitions/reference.json",
            "editor/author.json",
            "presentations/bindings.json",
            "provenance/imports.json",
            "provenance/sources.json",
            "worlds/world.json",
        ];
        let documents = locators
            .into_iter()
            .map(|locator| Ok((locator.to_owned(), std::fs::read(root.join(locator))?)))
            .collect::<Result<Vec<_>, std::io::Error>>()?;
        Ok(ProjectSnapshot::new(documents, limits)?.parse(limits)?)
    }

    fn project_mutation(
        edit: impl FnOnce(&mut ProjectV2Draft) -> TestResult + Send + 'static,
        check: impl FnOnce(PublicContentView) -> TestResult + Send + 'static,
    ) -> TestResult {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(move || {
                let mut draft = project_for_projection_test()?.migrate_to_v2();
                edit(&mut draft)?;
                let mut limits = crate::content::npc_catalogue_preproduction_limits().project;
                limits.max_reference_records += 2;
                let project = CanonicalProjectDocuments::from_v2_draft(draft, limits)?
                    .into_snapshot(limits)?
                    .parse(limits)?;
                check(public_content_view(&project)?)
            })?
            .join()
            .map_err(|_| "projection test thread panicked")?
    }

    const CLEDWYN: &str = "oteryn:npc.cledwyn";
    const ABYSSADOR: &str = "oteryn:creature.abyssador";
    const ABYSSADOR_LOOT: &str = "oteryn:loot.creature.abyssador";

    fn cledwyn_services(draft: &mut ProjectV2Draft) -> Option<&mut Vec<ProjectV2DefinitionRef>> {
        draft
            .state
            .declarations
            .iter_mut()
            .find_map(|row| match row {
                ProjectV2Declaration::Npc {
                    identity, services, ..
                } if identity.key == CLEDWYN => Some(services),
                _ => None,
            })
    }

    #[test]
    fn admitted_shared_service_stays_within_catalogue_limits() -> TestResult {
        shared_service_projection(false)
    }

    #[test]
    fn rejected_shared_offers_spend_projection_budget() -> TestResult {
        shared_service_projection(true)
    }

    fn shared_service_projection(reject_rows: bool) -> TestResult {
        project_mutation(
            move |draft| {
                let npc = draft
                    .state
                    .declarations
                    .iter()
                    .find(|row| row.identity().key == CLEDWYN)
                    .ok_or("Cledwyn NPC missing")?
                    .clone();
                let service = cledwyn_services(draft)
                    .and_then(|refs| refs.first())
                    .ok_or("Cledwyn service link missing")?
                    .clone();
                let offers = draft
                    .state
                    .declarations
                    .iter_mut()
                    .find_map(|row| match row {
                        ProjectV2Declaration::Service {
                            identity, offers, ..
                        } if identity.key == service.key => Some(offers),
                        _ => None,
                    })
                    .ok_or("Cledwyn service missing")?;
                let template = offers.first().ok_or("Cledwyn offer missing")?.clone();
                let offer_count = if reject_rows { 2_100 } else { 1_000 };
                *offers = (1..=offer_count)
                    .map(|count| ProjectV2ServiceOffer {
                        count: Some(count),
                        parity_pending: reject_rows,
                        ..template.clone()
                    })
                    .collect();
                for index in 0..1_001 {
                    let mut npc = npc.clone();
                    let ProjectV2Declaration::Npc {
                        identity, services, ..
                    } = &mut npc
                    else {
                        return Err("expected NPC declaration".into());
                    };
                    identity.key = format!("{CLEDWYN}.catalogue-bound-{index:04}");
                    services.clear();
                    services.push(service.clone());
                    draft.state.declarations.push(npc);
                }
                Ok(())
            },
            |view| assert_bounded_catalogue(view, "npc_shop", "creature_loot"),
        )
    }

    #[test]
    fn admitted_shared_loot_stays_within_catalogue_limits() -> TestResult {
        shared_loot_projection(false)
    }

    #[test]
    fn rejected_shared_loot_entries_spend_projection_budget() -> TestResult {
        shared_loot_projection(true)
    }

    fn shared_loot_projection(reject_rows: bool) -> TestResult {
        project_mutation(
            move |draft| {
                let table = draft
                    .core
                    .records
                    .iter()
                    .find(|row| row.identity().key == ABYSSADOR_LOOT)
                    .ok_or("Abyssador loot table missing")?;
                let ProjectReferenceRecord::Loot {
                    identity,
                    algorithm,
                    entries,
                } = table
                else {
                    return Err("expected Loot record".into());
                };
                let mut identity = identity.clone();
                identity.key.push_str(".catalogue-bound");
                let shared = DefinitionReferenceDocument {
                    family: identity.family.clone(),
                    key: identity.key.clone(),
                    revision: identity.revision.clone(),
                };
                let mut entry = entries
                    .first()
                    .ok_or("Abyssador loot entry missing")?
                    .clone();
                let mut item = draft
                    .core
                    .records
                    .iter()
                    .find(|row| row.identity().key == entry.item.key)
                    .ok_or("loot item missing")?
                    .clone();
                let ProjectReferenceRecord::Item {
                    identity: item_id,
                    client_projection,
                    ..
                } = &mut item
                else {
                    return Err("expected Item record".into());
                };
                if reject_rows {
                    *client_projection = ProjectionDocument::ServerOnly;
                }
                item_id.key.push_str(&"x".repeat(450));
                entry.item.key.clone_from(&item_id.key);
                let table = ProjectReferenceRecord::Loot {
                    identity,
                    algorithm: *algorithm,
                    entries: vec![entry; if reject_rows { 3_500 } else { 1_000 }],
                };
                for loot in draft
                    .core
                    .records
                    .iter_mut()
                    .filter_map(|row| match row {
                        ProjectReferenceRecord::Creature {
                            client_projection: ProjectionDocument::ClientSafe,
                            loot,
                            ..
                        } => Some(loot),
                        _ => None,
                    })
                    .take(600)
                {
                    *loot = Some(shared.clone());
                }
                draft.core.records.extend([item, table]);
                Ok(())
            },
            |view| assert_bounded_catalogue(view, "creature_loot", "npc_shop"),
        )
    }

    fn assert_bounded_catalogue(
        view: PublicContentView,
        oversized: &str,
        unaffected: &str,
    ) -> TestResult {
        let rows = view.relations.len();
        assert!(
            rows <= 1_000_000,
            "projection emitted {rows} rows above the v1 limit"
        );
        let input = view.preproduction_input(
            "0123456789abcdef0123456789abcdef01234567",
            "2026-01-01T00:00:00Z",
        );
        let bytes = serde_json::to_vec(&input)?.len();
        assert!(
            bytes < 256 * 1024 * 1024,
            "projection emitted {bytes} bytes above the v1 file limit"
        );
        let manifest = input["capability_manifest"]
            .as_array()
            .ok_or("manifest missing")?;
        assert!(
            manifest
                .iter()
                .any(|row| row["capability_id"] == oversized && row["support"] == "unsupported")
        );
        assert!(view.relations.iter().any(|row| row["type"] == unaffected));
        Ok(())
    }

    #[test]
    fn native_values_outside_wire_bounds_preserve_representable_rows() -> TestResult {
        const PRICE: u64 = 1 << 63;
        const ITEM: &str = "oteryn:item.tibia.i1000";
        project_mutation(
            |draft| {
                let original = cledwyn_services(draft)
                    .and_then(|refs| refs.first().cloned())
                    .ok_or("Cledwyn service missing")?;
                let service = draft
                    .state
                    .declarations
                    .iter_mut()
                    .find(|row| row.identity().key == original.key)
                    .ok_or("Cledwyn service definition missing")?;
                let ProjectV2Declaration::Service { offers, .. } = service else {
                    return Err("expected a Service".into());
                };
                offers
                    .first_mut()
                    .ok_or("Cledwyn offer missing")?
                    .unit_price = PRICE;
                let mut slash_service = service.clone();
                let ProjectV2Declaration::Service { identity, .. } = &mut slash_service else {
                    return Err("expected a Service".into());
                };
                identity.key = identity.key.replace('.', "/");
                let mut reference = original;
                reference.key.clone_from(&identity.key);
                draft.state.declarations.push(slash_service);
                cledwyn_services(draft)
                    .ok_or("Cledwyn NPC missing")?
                    .push(reference);
                let mut slash_npc = draft
                    .state
                    .declarations
                    .iter()
                    .find(|row| row.identity().key == CLEDWYN)
                    .ok_or("Cledwyn NPC missing")?
                    .clone();
                let ProjectV2Declaration::Npc { identity, .. } = &mut slash_npc else {
                    return Err("expected an NPC".into());
                };
                identity.key = identity.key.replace('.', "/");
                draft.state.declarations.push(slash_npc);

                let profile = draft
                    .state
                    .authoring_profiles
                    .iter_mut()
                    .find(|row| row.target.key == ABYSSADOR)
                    .ok_or("Abyssador profile missing")?;
                let ProjectV2AuthoringProfileData::Creature(creature) = &mut profile.data else {
                    return Err("expected a Creature profile".into());
                };
                creature.health = Some(PRICE);
                creature.experience = Some(PRICE);
                creature.resistances = vec![ProjectV2Resistance {
                    damage_type: "x".repeat(2049),
                    percent: ProjectV2ExactRatio {
                        numerator: 1,
                        denominator: PRICE,
                    },
                }];
                creature.immunities = vec!["x".repeat(2049)];

                let item = draft
                    .core
                    .records
                    .iter_mut()
                    .find(|row| row.identity().key == ITEM)
                    .ok_or("Item missing")?;
                let ProjectReferenceRecord::Item { semantics, .. } = item else {
                    return Err("expected an Item".into());
                };
                let ReferenceItemField::Known(presentation) = &mut semantics.presentation else {
                    return Err("item presentation missing".into());
                };
                presentation.name = ReferenceItemField::Known(String::new());
                Ok(())
            },
            |view| {
                let shop: Vec<_> = view
                    .relations
                    .iter()
                    .filter(|row| row["type"] == "npc_shop")
                    .filter_map(|row| row["relation_key"].as_str())
                    .collect();
                assert!(shop.iter().all(|key| !key.contains('/')));
                assert!(
                    !shop
                        .iter()
                        .any(|key| { key.starts_with(CLEDWYN) && key.ends_with(".row.0") })
                );
                assert!(
                    shop.iter()
                        .any(|key| { key.starts_with(CLEDWYN) && key.ends_with(".row.1") })
                );
                assert!(
                    !view
                        .entities
                        .iter()
                        .any(|row| row["content_key"] == CLEDWYN.replace('.', "/"))
                );
                let data = |key| {
                    view.entities
                        .iter()
                        .find(|row| row["content_key"] == key)
                        .map(|row| &row["data"])
                };
                let creature = data(ABYSSADOR).ok_or("Abyssador missing")?;
                for field in ["health", "experience", "resistances", "immunities"] {
                    assert!(creature.get(field).is_none(), "{field} exceeds wire bounds");
                }
                assert!(data(ITEM).ok_or("Item missing")?.get("name").is_none());
                for capability in ["item", "creature", "npc", "npc_shop"] {
                    assert!(view.incomplete.contains(capability));
                }
                Ok(())
            },
        )
    }

    #[test]
    fn admitted_payload_bounds_preserve_representable_catalogue_rows() -> TestResult {
        const ITEM: &str = "oteryn:item.tibia.i1000";
        project_mutation(
            |draft| {
                let item = draft
                    .core
                    .records
                    .iter_mut()
                    .find_map(|row| match row {
                        ProjectReferenceRecord::Item {
                            identity,
                            semantics,
                            ..
                        } if identity.key == ITEM => Some(semantics),
                        _ => None,
                    })
                    .ok_or("Item missing")?;
                let ReferenceItemField::Known(presentation) = &mut item.presentation else {
                    return Err("Item presentation missing".into());
                };
                presentation.name = ReferenceItemField::Known(format!("{}x", "é".repeat(23)));

                let profile = draft
                    .state
                    .authoring_profiles
                    .iter_mut()
                    .find(|row| row.target.key == ABYSSADOR)
                    .ok_or("Abyssador profile missing")?;
                let ProjectV2AuthoringProfileData::Creature(creature) = &mut profile.data else {
                    return Err("expected a Creature profile".into());
                };
                creature.immunities = (0..=200_000).map(|index| format!("i{index:06}")).collect();

                let entries = draft
                    .core
                    .records
                    .iter_mut()
                    .find_map(|row| match row {
                        ProjectReferenceRecord::Loot {
                            identity, entries, ..
                        } if identity.key == ABYSSADOR_LOOT => Some(entries),
                        _ => None,
                    })
                    .ok_or("Abyssador loot definition missing")?;
                let [zero, inverted, probability, ..] = entries.as_mut_slice() else {
                    return Err("expected at least three loot rows".into());
                };
                zero.min_count = 0;
                inverted.min_count = inverted.max_count + 1;
                probability.probability_ppm = Some(1_000_001);
                Ok(())
            },
            |view| {
                let input = view.preproduction_input(
                    "0123456789abcdef0123456789abcdef01234567",
                    "2026-01-01T00:00:00Z",
                );
                let entities = input["entities"].as_array().ok_or("entities missing")?;
                let relations = input["relations"].as_array().ok_or("relations missing")?;
                let mut leaked = Vec::new();
                for (key, field) in [(ITEM, "name"), (ABYSSADOR, "immunities")] {
                    let entity = entities
                        .iter()
                        .find(|row| row["content_key"] == key)
                        .ok_or("expected projected entity")?;
                    if entity["data"].get(field).is_some() {
                        leaked.push(field.to_owned());
                    }
                }
                for ordinal in 0..3 {
                    let key = format!("{ABYSSADOR}.loot.{ordinal}");
                    if relations.iter().any(|row| row["relation_key"] == key) {
                        leaked.push(key);
                    }
                }
                assert!(
                    leaked.is_empty(),
                    "native values outside wire bounds: {leaked:?}"
                );
                assert!(
                    relations
                        .iter()
                        .any(|row| row["relation_key"] == format!("{ABYSSADOR}.loot.3"))
                );
                for capability in ["item", "creature", "creature_loot"] {
                    assert!(view.incomplete.contains(capability));
                }
                Ok(())
            },
        )
    }

    #[test]
    fn repeated_service_links_preserve_one_copy_of_each_offer() -> TestResult {
        project_mutation(
            |draft| {
                let services = cledwyn_services(draft).ok_or("Cledwyn NPC missing")?;
                services.push(services.first().ok_or("Cledwyn service missing")?.clone());
                Ok(())
            },
            |view| {
                let mut keys = BTreeSet::new();
                for row in view.relations.iter().filter(|row| row["source"] == CLEDWYN) {
                    assert!(
                        keys.insert(row["relation_key"].as_str().ok_or("relation key missing")?)
                    );
                }
                assert!(
                    keys.contains("oteryn:npc.cledwyn.shop.oteryn.service.trade.cledwyn.row.0")
                );
                Ok(())
            },
        )
    }

    #[test]
    fn colliding_service_keys_omit_ambiguous_offers() -> TestResult {
        project_mutation(
            |draft| {
                let original = cledwyn_services(draft)
                    .and_then(|refs| refs.first().cloned())
                    .ok_or("Cledwyn service missing")?;
                let service = draft
                    .state
                    .declarations
                    .iter()
                    .find(|row| row.identity().key == original.key)
                    .ok_or("Cledwyn service definition missing")?
                    .clone();
                for key in ["a:b.c", "a.b:c"] {
                    let mut declaration = service.clone();
                    let ProjectV2Declaration::Service { identity, .. } = &mut declaration else {
                        return Err("expected a Service".into());
                    };
                    identity.key = key.to_owned();
                    draft.state.declarations.push(declaration);
                    let mut reference = original.clone();
                    reference.key = key.to_owned();
                    cledwyn_services(draft)
                        .ok_or("Cledwyn NPC missing")?
                        .push(reference);
                }
                Ok(())
            },
            |view| {
                assert!(!view.relations.iter().any(|row| {
                    row["relation_key"]
                        .as_str()
                        .is_some_and(|key| key.starts_with("oteryn:npc.cledwyn.shop.a.b.c.row."))
                }));
                assert!(view.relations.iter().any(|row| row["relation_key"]
                    == "oteryn:npc.cledwyn.shop.oteryn.service.trade.cledwyn.row.0"));
                assert!(view.incomplete.contains("npc_shop"));
                Ok(())
            },
        )
    }

    #[test]
    fn colliding_entity_keys_omit_ambiguous_endpoints() -> TestResult {
        project_mutation(
            |draft| {
                let mut npc = draft
                    .state
                    .declarations
                    .iter()
                    .find(|row| row.identity().key == CLEDWYN)
                    .ok_or("Cledwyn NPC missing")?
                    .clone();
                let ProjectV2Declaration::Npc { identity, .. } = &mut npc else {
                    return Err("expected NPC".into());
                };
                identity.key = ABYSSADOR.to_owned();
                draft.state.declarations.push(npc);
                Ok(())
            },
            |view| {
                assert!(
                    !view
                        .entities
                        .iter()
                        .any(|row| row["content_key"] == ABYSSADOR)
                );
                assert!(!view.relations.iter().any(|row| row["source"] == ABYSSADOR));
                for capability in ["creature", "npc", "creature_loot", "npc_shop"] {
                    assert!(view.incomplete.contains(capability));
                }
                Ok(())
            },
        )
    }

    #[test]
    fn stale_loot_table_revision_emits_no_rows() -> TestResult {
        project_mutation(
            |draft| {
                let table = draft
                    .core
                    .records
                    .iter_mut()
                    .find_map(|row| match row {
                        ProjectReferenceRecord::Creature { identity, loot, .. }
                            if identity.key == ABYSSADOR =>
                        {
                            loot.as_mut()
                        }
                        _ => None,
                    })
                    .ok_or("Abyssador loot ref missing")?;
                table.revision = "definition-r0".to_owned();
                Ok(())
            },
            |view| {
                assert!(!view.relations.iter().any(|row| row["source"] == ABYSSADOR));
                assert!(view.incomplete.contains("creature_loot"));
                Ok(())
            },
        )
    }

    #[test]
    fn stale_loot_item_revision_omits_only_the_affected_row() -> TestResult {
        project_mutation(
            |draft| {
                let entries = draft
                    .core
                    .records
                    .iter_mut()
                    .find_map(|row| match row {
                        ProjectReferenceRecord::Loot {
                            identity, entries, ..
                        } if identity.key == ABYSSADOR_LOOT => Some(entries),
                        _ => None,
                    })
                    .ok_or("Abyssador loot definition missing")?;
                entries
                    .first_mut()
                    .ok_or("loot entry missing")?
                    .item
                    .revision = "definition-r0".to_owned();
                Ok(())
            },
            |view| {
                assert!(
                    !view
                        .relations
                        .iter()
                        .any(|row| row["relation_key"] == "oteryn:creature.abyssador.loot.0")
                );
                assert!(
                    view.relations
                        .iter()
                        .any(|row| row["relation_key"] == "oteryn:creature.abyssador.loot.1")
                );
                assert!(view.incomplete.contains("creature_loot"));
                Ok(())
            },
        )
    }

    #[test]
    fn same_key_behavior_keeps_conditional_loot_suppressed() -> TestResult {
        project_mutation(
            |draft| {
                const ANOMALY_LOOT: &str = "oteryn:loot.creature.anomaly";
                let mut profile = draft
                    .state
                    .authoring_profiles
                    .iter()
                    .find(|row| row.target.key == "oteryn:behavior.creature.anomaly")
                    .ok_or("Anomaly Behavior profile missing")?
                    .clone();
                profile.target.key = ANOMALY_LOOT.to_owned();
                draft.core.records.push(ProjectReferenceRecord::Generic {
                    identity: DefinitionIdentityDocument {
                        family: "Behavior".to_owned(),
                        key: ANOMALY_LOOT.to_owned(),
                        revision: profile.target.revision.clone(),
                    },
                    client_projection: ProjectionDocument::ServerOnly,
                });
                draft.state.authoring_profiles.push(profile);
                Ok(())
            },
            |view| {
                assert!(
                    !view
                        .relations
                        .iter()
                        .any(|row| row["source"] == "oteryn:creature.anomaly")
                );
                assert!(view.relations.iter().any(|row| row["source"] == ABYSSADOR));
                Ok(())
            },
        )
    }

    #[test]
    fn public_view_coverage_visibility_and_determinism()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                let mut project = project_for_projection_test()?;
                let first = public_content_view(&project)?;
                let second = public_content_view(&project)?;
                assert_eq!(serde_json::to_vec(&first)?, serde_json::to_vec(&second)?);
                // Source display-name provenance stays in the field inventory.
                assert!(first.field_inventory.contains(
                    "declaration.Npc.fields[].field_path=\"oteryn:source.npc.display_name\""));
                assert!(first.field_inventory.contains(
                    "declaration.Npc.fields[].field_path=\"oteryn:source.npc.crystal_source\""));
                let expected: BTreeMap<String, String> = serde_json::from_str(include_str!(
                    "../../../../../../tools/game-platform-catalog/fixtures/public-view-fields.json"
                ))?;
                assert_eq!(
                    first.field_fingerprints(),
                    expected,
                    "every new field needs an explicit public/unsupported disposition"
                );
                let state = project.v2.as_mut().ok_or("v2 absent")?;
                assert_eq!(
                    first.coverage.len(),
                    project.reference.records.len() + state.declarations.len()
                );
                assert!(first.coverage.iter().all(|row| !row.reason.is_empty()));
                let mut hidden_key = None;
                for declaration in &mut state.declarations {
                    if let ProjectV2Declaration::Npc {
                        identity,
                        visibility,
                        services,
                        ..
                    } = declaration
                        && !services.is_empty()
                    {
                        *visibility = PublicVisibility::Hidden;
                        hidden_key = Some(identity.key.clone());
                        break;
                    }
                }
                let hidden_key = hidden_key.ok_or("shop NPC absent")?;
                for kind in ["Quest", "Area"] {
                    state.declarations.push(serde_json::from_value(json!({
                        "kind":kind, "identity":{"key":format!("oteryn:test.hidden_{}", kind.to_lowercase()),"revision":"definition-r1"},
                        "fields":[], "visibility":"hidden"
                    }))?);
                }
                let candidate_quest = "oteryn:test.public_quest";
                state.declarations.push(serde_json::from_value(json!({
                    "kind":"Quest", "identity":{"key":candidate_quest,"revision":"definition-r1"},
                    "fields":[]
                }))?);
                let hidden = public_content_view(&project)?;
                assert!(
                    first
                        .entities
                        .iter()
                        .any(|row| row["content_key"] == hidden_key)
                );
                assert!(
                    first
                        .relations
                        .iter()
                        .any(|row| row["source"] == hidden_key)
                );
                assert!(!String::from_utf8(serde_json::to_vec(&hidden)?)?.contains(&hidden_key));
                for key in ["oteryn:test.hidden_quest", "oteryn:test.hidden_area"] {
                    assert!(!String::from_utf8(serde_json::to_vec(&hidden)?)?.contains(key));
                    assert!(hidden.coverage.iter().any(|row| row.key == key && row.disposition == "hidden"));
                }
                let hidden_source = hidden.preproduction_input(
                    "d34807f5bd234ee8318e57ba454ba39be0df113d",
                    "2026-11-07T00:00:00Z",
                );
                let states = |source: &Value| -> Result<BTreeMap<String, String>, serde_json::Error> {
                    let rows: Vec<BTreeMap<String, String>> =
                        serde_json::from_value(source["completeness_manifest"].clone())?;
                    Ok(rows.into_iter().filter_map(|mut row|
                        Some((row.remove("capability_id")?, row.remove("state")?))).collect())
                };
                let first_source = first.preproduction_input(
                    "d34807f5bd234ee8318e57ba454ba39be0df113d",
                    "2026-11-07T00:00:00Z",
                );
                let first_states = states(&first_source)?;
                let hidden_states = states(&hidden_source)?;
                for capability in ["npc", "npc_shop"] {
                    assert_eq!(first_states[capability], "complete");
                    assert_eq!(hidden_states[capability], "partial");
                }
                assert_eq!(hidden_states["quest"], "unknown");
                assert!(hidden.entities.iter().any(|row| row["content_key"] == candidate_quest));
                let exported = serde_json::to_string(&hidden_source)?;
                for key in [&hidden_key, candidate_quest, "oteryn:test.hidden_quest", "oteryn:test.hidden_area"] {
                    assert!(!exported.contains(key));
                }
                Ok(())
            })?
            .join()
            .map_err(|_| "public view test thread failed")?
    }
}
