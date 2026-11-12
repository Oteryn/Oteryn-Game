//! Source-bound S10 additions, applied by the existing NPC materializer before bundle pinning.
use crate::content::{
    ProjectV2AuthoringProfileData, ProjectV2CandidateField, ProjectV2Declaration,
    ProjectV2DefinitionRef, ProjectV2Draft, ProjectV2Family, ProjectV2Identity, ProjectV2Wander,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Supplement {
    schema: String,
    voices: Vec<Fields>,
    walkers: Vec<Walker>,
    bankers: Vec<Banker>,
    free_blessings: Vec<FreeBlessing>,
    monk_promotions: Vec<String>,
}

#[derive(Deserialize)]
struct Fields {
    key: String,
    fields: Vec<ProjectV2CandidateField>,
}

#[derive(Deserialize)]
struct Walker {
    key: String,
    interval_ms: u64,
    radius_tiles: u16,
    fields: Vec<ProjectV2CandidateField>,
}

#[derive(Deserialize)]
struct Banker {
    key: String,
}

#[derive(Deserialize)]
struct FreeBlessing {
    service: String,
    npcs: Vec<String>,
}

pub(super) fn nonpremium_binding(service: &str, npc: &str) -> bool {
    static BINDINGS: std::sync::OnceLock<Option<Vec<FreeBlessing>>> = std::sync::OnceLock::new();
    BINDINGS
        .get_or_init(|| {
            let data: Supplement = serde_json::from_str(include_str!(
                "../../../../../../content/npcs/s10-nonquest.json"
            ))
            .ok()?;
            Some(data.free_blessings)
        })
        .as_ref()
        .is_some_and(|rows| {
            rows.iter()
                .any(|row| row.service == service && row.npcs.iter().any(|key| key == npc))
        })
}

fn attach_service<'a>(
    draft: &mut ProjectV2Draft,
    key: &str,
    npcs: impl IntoIterator<Item = &'a str>,
) -> Result<(), &'static str> {
    if draft.state.declarations.iter().any(|record| {
        matches!(record,
        ProjectV2Declaration::Service { identity, .. } if identity.key == key)
    }) {
        return Err("S10 blessing Service already authored");
    }
    for npc in npcs {
        let Some(ProjectV2Declaration::Npc { services, .. }) = draft.state.declarations.iter_mut()
            .find(|record| matches!(record, ProjectV2Declaration::Npc { identity, .. } if identity.key == npc))
        else { return Err("S10 blessing NPC absent"); };
        services.push(ProjectV2DefinitionRef {
            family: ProjectV2Family::Service,
            key: key.into(),
            revision: "definition-r1".into(),
        });
    }
    draft
        .state
        .declarations
        .push(ProjectV2Declaration::Service {
            identity: ProjectV2Identity {
                key: key.into(),
                revision: "definition-r1".into(),
            },
            bank: false,
            fields: vec![],
            offers: vec![],
            recipes: vec![],
            routes: vec![],
        });
    Ok(())
}

fn add_fields(draft: &mut ProjectV2Draft, row: Fields) -> Result<(), &'static str> {
    let Some(ProjectV2Declaration::Npc { fields, .. }) = draft.state.declarations.iter_mut()
        .find(|record| matches!(record, ProjectV2Declaration::Npc { identity, .. } if identity.key == row.key))
    else { return Err("S10 NPC absent"); };
    for field in row.fields {
        if fields
            .iter()
            .any(|existing| existing.field_path == field.field_path)
        {
            return Err("S10 field already authored");
        }
        fields.push(field);
    }
    fields.sort_by(|left, right| left.field_path.cmp(&right.field_path));
    Ok(())
}

pub(super) fn install(draft: &mut ProjectV2Draft) -> Result<(), &'static str> {
    let supplement: Supplement = serde_json::from_str(include_str!(
        "../../../../../../content/npcs/s10-nonquest.json"
    ))
    .map_err(|_| "invalid S10 supplement")?;
    if supplement.schema != "OTERYN_S10_NONQUEST/v1" {
        return Err("invalid S10 schema");
    }
    for row in supplement.voices {
        add_fields(draft, row)?;
    }
    for row in supplement.walkers {
        let behavior = draft
            .state
            .declarations
            .iter()
            .find_map(|record| match record {
                ProjectV2Declaration::Npc {
                    identity, behavior, ..
                } if identity.key == row.key => behavior.clone(),
                _ => None,
            })
            .ok_or("S10 Behavior absent")?;
        let profile = draft
            .state
            .authoring_profiles
            .iter_mut()
            .find(|profile| profile.target == behavior)
            .ok_or("S10 Behavior profile absent")?;
        let ProjectV2AuthoringProfileData::Behavior(profile) = &mut profile.data else {
            return Err("S10 profile is not Behavior");
        };
        profile.movement.can_walk = true;
        profile.movement.wander = Some(ProjectV2Wander {
            interval_ms: row.interval_ms,
            radius_tiles: row.radius_tiles,
        });
        add_fields(
            draft,
            Fields {
                key: row.key,
                fields: row.fields,
            },
        )?;
    }
    // Bind the shared height catalogue into the same source tree as the NPC profiles.
    for record in &mut draft.state.declarations {
        if let ProjectV2Declaration::Npc { fields, .. } = record
            && fields
                .iter()
                .any(|field| field.field_path == "oteryn:source.npc.base_speed")
        {
            fields.push(ProjectV2CandidateField {
                field_path: super::walk_terrain::FIELD.into(),
                value: crate::content::ProjectV2CandidateValue::Text(
                    super::walk_terrain::digest().into(),
                ),
            });
        }
    }
    let service = "oteryn:service.bank.standard";
    if draft.state.declarations.iter().any(|record| {
        matches!(record,
        ProjectV2Declaration::Service { identity, .. } if identity.key == service)
    }) {
        return Err("S10 Bank service already authored");
    }
    for banker in supplement.bankers {
        let Some(ProjectV2Declaration::Npc { services, .. }) = draft.state.declarations.iter_mut()
            .find(|record| matches!(record, ProjectV2Declaration::Npc { identity, .. } if identity.key == banker.key))
        else { return Err("S10 banker absent"); };
        services.push(ProjectV2DefinitionRef {
            family: ProjectV2Family::Service,
            key: service.into(),
            revision: "definition-r1".into(),
        });
    }
    draft
        .state
        .declarations
        .push(ProjectV2Declaration::Service {
            identity: ProjectV2Identity {
                key: service.into(),
                revision: "definition-r1".into(),
            },
            bank: true,
            fields: vec![],
            offers: vec![],
            recipes: vec![],
            routes: vec![],
        });
    for binding in supplement.free_blessings {
        attach_service(
            draft,
            &binding.service,
            binding.npcs.iter().map(String::as_str),
        )?;
    }
    for npc in supplement.monk_promotions {
        let stem = npc.strip_prefix("oteryn:npc.").ok_or("invalid monk NPC")?;
        attach_service(
            draft,
            &format!("oteryn:service.promotion.{stem}"),
            [npc.as_str()],
        )?;
    }
    attach_service(
        draft,
        super::premium_services::PHOENIX_PREPARATION,
        ["oteryn:npc.kawill"],
    )?;
    Ok(())
}
