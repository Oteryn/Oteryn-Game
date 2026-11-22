//! Read-only public projection of the exact graph admitted by the server's CONTENT loader.
//! No donor documents, migration shards or display names are parsed as native authority.
use super::*;
use crate::content::ReferenceItemField;
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
    /// Emit the Game-owned payloads registered for this profile in export contract §15.
    pub fn native_input(&self, source_revision: &str, generated_at: &str) -> Value {
        let entities: Vec<_> = self
            .entities
            .iter()
            .filter(|row| {
                matches!(
                    row["capability_id"].as_str(),
                    Some("item" | "creature" | "npc")
                )
            })
            .collect();
        let supported: BTreeSet<_> = entities
            .iter()
            .copied()
            .chain(&self.relations)
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
            "entities": entities, "relations": self.relations, "tombstones": []
        })
    }
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
        .map(|profile| (profile.target.key.as_str(), &profile.data))
        .collect();
    let mut loot = BTreeMap::new();
    for record in &project.reference.records {
        let raw = project_json(record)?;
        let identity = record.identity();
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
            ProjectReferenceRecord::Ability { .. } => match profiles.get(identity.key.as_str()) {
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
                loot.insert(identity.key.as_str(), (algorithm, entries));
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
                projected["data"]["name"] = json!(name);
            }
            if let Some(profile) = profiles.get(identity.key.as_str()) {
                if let ProjectV2AuthoringProfileData::Creature(profile) = profile
                    && let Some(details) = &profile.details
                {
                    projected["data"]["name"] = json!(details.display_name);
                }
                let profile = project_json(profile)?;
                let selected = match kind {
                    "creature" => &[
                        "health",
                        "experience",
                        "speed",
                        "armor",
                        "resistances",
                        "immunities",
                    ][..],
                    "spell" => &[
                        "incantation",
                        "vocations",
                        "required_level",
                        "premium",
                        "mana_cost",
                        "cooldown_ms",
                        "group_cooldown_ms",
                        "range",
                    ][..],
                    _ => &[],
                };
                for field in selected {
                    if let Some(value) = profile["profile"].get(field) {
                        projected["data"][*field] = value.clone();
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
                npcs.push((identity.key.as_str(), services, hidden));
                if hidden {
                    view.incomplete.insert("npc");
                }
            }
            ProjectV2Declaration::Service { offers, .. } => {
                shop_services.insert(identity.key.as_str(), offers);
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
    let items: BTreeSet<_> = view
        .entities
        .iter()
        .filter(|e| e["type"] == "item")
        .filter_map(|e| e["content_key"].as_str())
        .collect();
    for (npc, services, hidden) in npcs {
        for service in services {
            let Some(offers) = shop_services.get(service.key.as_str()) else {
                view.incomplete.insert("npc_shop");
                continue;
            };
            for (ordinal, offer) in offers.iter().enumerate() {
                if hidden
                    || offer.parity_pending
                    || !items.contains(offer.item.key.as_str())
                    || offer
                        .currency
                        .as_ref()
                        .is_some_and(|currency| !items.contains(currency.key.as_str()))
                {
                    view.incomplete.insert("npc_shop");
                    continue;
                }
                view.relations.push(json!({
                    "type":"npc_shop", "capability_id":"npc_shop",
                    "relation_key":format!("{npc}.shop.{}.row.{ordinal}", service.key.replace(':', ".")),
                    "source":npc,"target":offer.item.key,
                    "data":{"payload_revision":1,"direction":project_json(&offer.direction)?,"unit_price":offer.unit_price,
                        "count":offer.count,"sub_type":offer.sub_type,
                        "currency":offer.currency.as_ref().map(|currency| &currency.key)}
                }));
            }
        }
    }
    for record in &project.reference.records {
        let ProjectReferenceRecord::Creature {
            identity,
            client_projection,
            loot: Some(table),
            ..
        } = record
        else {
            continue;
        };
        let Some((algorithm, entries)) = loot.get(table.key.as_str()) else {
            view.incomplete.insert("creature_loot");
            continue;
        };
        if entries.is_empty() {
            continue;
        }
        let conditional = matches!(profiles.get(table.key.as_str()), Some(ProjectV2AuthoringProfileData::Loot(profile))
            if profile.entries.iter().any(|entry| entry.skip_later_same_item_after_success));
        if !matches!(client_projection, ProjectionDocument::ClientSafe)
            || !matches!(algorithm, LootAlgorithmDocument::IndependentBernoulliPpm)
            || conditional
            || entries.iter().any(|entry| {
                !entry.children.is_empty()
                    || entry.unique
                    || entry.subtype.is_some()
                    || entry.probability_ppm.is_none()
            })
        {
            view.incomplete.insert("creature_loot");
            continue;
        }
        for (ordinal, entry) in entries.iter().enumerate() {
            if !items.contains(entry.item.key.as_str()) {
                view.incomplete.insert("creature_loot");
                continue;
            }
            view.relations.push(
                json!({"type":"creature_loot","capability_id":"creature_loot",
                "relation_key":format!("{}.loot.{ordinal}", identity.key),
                "source":identity.key,"target":entry.item.key,
                "data":{"payload_revision":1,"algorithm":algorithm,"min_count":entry.min_count,
                    "max_count":entry.max_count,"probability_ppm":entry.probability_ppm}}),
            );
        }
    }
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

    #[test]
    fn public_view_coverage_visibility_and_determinism()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                let root =
                    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../content");
                // Exercise the portable server parser as well as Linux filesystem capture.
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
                    .map(|locator| {
                        Ok((
                            locator.to_owned(),
                            std::fs::read(root.join("world").join(locator))?,
                        ))
                    })
                    .collect::<Result<Vec<_>, std::io::Error>>()?;
                let mut project = ProjectSnapshot::new(documents, limits)?.parse(limits)?;
                let first = public_content_view(&project)?;
                let second = public_content_view(&project)?;
                assert_eq!(serde_json::to_vec(&first)?, serde_json::to_vec(&second)?);
                // Source display-name provenance stays in the field inventory.
                assert!(first.field_inventory.contains(
                    "declaration.Npc.fields[].field_path=\"oteryn:source.npc.display_name\""));
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
                let hidden_source = hidden.native_input(
                    "d34807f5bd234ee8318e57ba454ba39be0df113d",
                    "2026-11-07T00:00:00Z",
                );
                let states = |source: &Value| -> Result<BTreeMap<String, String>, serde_json::Error> {
                    let rows: Vec<BTreeMap<String, String>> =
                        serde_json::from_value(source["completeness_manifest"].clone())?;
                    Ok(rows.into_iter().filter_map(|mut row|
                        Some((row.remove("capability_id")?, row.remove("state")?))).collect())
                };
                let first_source = first.native_input(
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
