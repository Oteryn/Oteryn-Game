//! Server-selected Premium services require the NPC's own admitted Service reference.
use crate::content::ProjectV2DefinitionRef;
use crate::content::{ProjectV2Declaration, ProjectV2Draft, ProjectV2Family, ProjectV2Identity};

/// Materialize original Oteryn Service declarations on the existing source-bound NPCs.
pub fn install(draft: &mut ProjectV2Draft) -> Result<(), &'static str> {
    let catalogue = BlessingCatalogue::load()?;
    let mut bindings = vec![
        (
            "oteryn:npc.king_tibianus".to_owned(),
            "oteryn:service.promotion.king_tibianus".to_owned(),
        ),
        (
            "oteryn:npc.queen_eloise".to_owned(),
            "oteryn:service.promotion.queen_eloise".to_owned(),
        ),
        (
            "oteryn:npc.emperor_kruzak".to_owned(),
            "oteryn:service.promotion.emperor_kruzak".to_owned(),
        ),
        (
            "oteryn:npc.eremo".to_owned(),
            "oteryn:service.promotion.eremo".to_owned(),
        ),
    ];
    for blessing in catalogue
        .blessings
        .into_iter()
        .filter(|blessing| blessing.premium)
    {
        for npc in blessing.npcs {
            bindings.push((npc, blessing.service.clone()));
        }
    }
    for (npc_key, service_key) in bindings {
        if draft.state.declarations.iter().any(|record|matches!(record,ProjectV2Declaration::Service {identity,..} if identity.key==service_key)) {return Err("Premium Service already declared");}
        let Some(ProjectV2Declaration::Npc {services,..})=draft.state.declarations.iter_mut().find(|record|matches!(record,ProjectV2Declaration::Npc {identity,..} if identity.key==npc_key)) else {return Err("Premium NPC is absent");};
        services.push(ProjectV2DefinitionRef {
            family: ProjectV2Family::Service,
            key: service_key.clone(),
            revision: "definition-r1".into(),
        });
        services.sort();
        draft
            .state
            .declarations
            .push(ProjectV2Declaration::Service {
                identity: ProjectV2Identity {
                    key: service_key,
                    revision: "definition-r1".into(),
                },
                bank: false,
                offers: vec![],
                recipes: vec![],
                routes: vec![],
                fields: vec![],
            });
    }
    draft.core.project_revision = "npc-premium-services-20261022-r1".into();
    Ok(())
}
use crate::content::blessing_catalogue::{Blessing, BlessingCatalogue};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PremiumNpcService {
    Promotion { service: String },
    Blessing(Blessing),
}

pub fn resolve(npc: &str, services: &[ProjectV2DefinitionRef]) -> Vec<PremiumNpcService> {
    let mut offered = Vec::new();
    let Some(stem) = npc.strip_prefix("oteryn:npc.") else {
        return offered;
    };
    let promotion = format!("oteryn:service.promotion.{stem}");
    if services.iter().any(|reference| reference.key == promotion) {
        offered.push(PremiumNpcService::Promotion { service: promotion });
    }
    if let Ok(catalogue) = BlessingCatalogue::load() {
        for blessing in catalogue.blessings {
            // This card activates only the three Premium blessing services.
            if blessing.premium
                && blessing.npcs.iter().any(|key| key == npc)
                && services
                    .iter()
                    .any(|reference| reference.key == blessing.service)
            {
                offered.push(PremiumNpcService::Blessing(blessing));
            }
        }
    }
    offered
}
