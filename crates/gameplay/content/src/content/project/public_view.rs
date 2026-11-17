//! Read-only public projection of the exact graph admitted by the server's CONTENT loader.
//! No donor documents, migration shards or display names are parsed as native authority.
use super::*;
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

/// Candidate records are useful to Game tools, but are not capability-promotion evidence.
/// Contract export §15 requires matching Platform payload validation before promotion.
#[derive(Debug, Serialize)]
pub struct PublicContentView {
    pub entities: Vec<Value>,
    pub relations: Vec<Value>,
    /// Local coverage evidence, never included in the Platform artifact (may name hidden keys).
    #[serde(skip)]
    pub coverage: Vec<PublicContentCoverage>,
    #[serde(skip)]
    pub field_inventory: BTreeSet<String>,
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
    /// Physical v1 export stays fail-closed until the capability payload contracts and
    /// matching consumer validators required by §15 exist. Never mislabel candidate facts.
    pub fn native_input(&self, source_revision: &str, generated_at: &str) -> Value {
        json!({
            "authority_epoch": "preproduction-unallocated",
            "source_revision": source_revision,
            "generated_at": generated_at,
            "ruleset_id": "oteryn:ruleset.public-catalog-candidate",
            "content_profile_id": "oteryn:profile.public-content-view-v1",
            "required_capabilities": [],
            "capability_manifest": CATALOG_CAPABILITIES.map(|id| json!({"capability_id":id,"support":"unsupported"})),
            "completeness_manifest": CATALOG_CAPABILITIES.map(|id| json!({"capability_id":id,"state":"unknown"})),
            "entities": [], "relations": [], "tombstones": []
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
            ProjectReferenceRecord::Item { .. } => Some("item"),
            ProjectReferenceRecord::Creature { .. } => Some("creature"),
            ProjectReferenceRecord::Ability { .. } => match profiles.get(identity.key.as_str()) {
                Some(ProjectV2AuthoringProfileData::Ability(profile))
                    if profile.incantation.is_some() =>
                {
                    Some("spell")
                }
                _ => None,
            },
            ProjectReferenceRecord::Loot { .. } => {
                loot.insert(identity.key.as_str(), raw.clone());
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
                    &["materializable", "stack_class", "semantics"]
                } else {
                    &[]
                },
            );
            if let Some(profile) = profiles.get(identity.key.as_str()) {
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
                "candidate public payload; export promotion requires contract §15"
            } else {
                "not a proven public capability payload; retained in server graph"
            },
        });
    }
    let mut public_npcs = Vec::new();
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
        let kind = if hidden {
            None
        } else {
            match declaration {
                ProjectV2Declaration::Npc { services, .. } => {
                    public_npcs.push((identity.key.as_str(), services));
                    Some("npc")
                }
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
                "candidate public payload; export promotion requires contract §15"
            } else {
                "outside the closed v1 public capability vocabulary"
            },
        });
    }
    let items: BTreeSet<_> = view
        .entities
        .iter()
        .filter(|e| e["type"] == "item")
        .filter_map(|e| e["content_key"].as_str().map(str::to_owned))
        .collect();
    for (npc, services) in public_npcs {
        for service in services {
            let Some(ProjectV2Declaration::Service { offers, .. }) = state
                .declarations
                .iter()
                .find(|d| d.identity().key == service.key)
            else {
                continue;
            };
            for (ordinal, offer) in offers.iter().enumerate() {
                if !items.contains(&offer.item.key) || offer.parity_pending {
                    continue;
                }
                view.relations.push(json!({
                    "type":"npc_shop", "capability_id":"npc_shop",
                    "relation_key":format!("{npc}.shop.{}.row.{ordinal}", service.key.replace(':', ".")),
                    "source":npc,"target":offer.item.key,
                    "data":{"direction":project_json(&offer.direction)?,"unit_price":offer.unit_price,
                        "count":offer.count,"sub_type":offer.sub_type,
                        "currency":offer.currency.as_ref().map(|currency| &currency.key)}
                }));
            }
        }
    }
    for record in &project.reference.records {
        let ProjectReferenceRecord::Creature {
            identity,
            loot: Some(table),
            ..
        } = record
        else {
            continue;
        };
        let Some(raw) = loot.get(table.key.as_str()) else {
            continue;
        };
        if let Some(entries) = raw["entries"].as_array() {
            for (ordinal, entry) in entries.iter().enumerate() {
                let Some(item) = entry["item"]["key"].as_str() else {
                    continue;
                };
                if !items.contains(item)
                    || entry["children"].as_array().is_some_and(|v| !v.is_empty())
                {
                    continue;
                }
                view.relations.push(json!({"type":"creature_loot","capability_id":"creature_loot",
                    "relation_key":format!("{}.loot.{ordinal}", identity.key),
                    "source":identity.key,"target":item,
                    "data":{"algorithm":raw["algorithm"],"min_count":entry["min_count"],
                        "max_count":entry["max_count"],"probability_ppm":entry["probability_ppm"]}}));
            }
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
                // Source display-name provenance is candidate metadata, with export
                // capabilities still explicitly unsupported below.
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
                assert!(
                    hidden_source["entities"]
                        .as_array()
                        .is_some_and(Vec::is_empty)
                );
                assert!(
                    hidden_source["capability_manifest"]
                        .as_array()
                        .ok_or("manifest")?
                        .iter()
                        .all(|row| row["support"] == "unsupported")
                );
                Ok(())
            })?
            .join()
            .map_err(|_| "public view test thread failed")?
    }
}
