//! Town filtering uses the shipped server catalogue, never a client-supplied property set.
use super::{
    ComposedFreshAdmission,
    connection::{FreshAdmissionAuthority, UseCommand},
};
use crate::foundation::ExactActorRef;
use oteryn_protocol_oteryn::house::{self as wire, HouseDisposition as D};
use serde::Deserialize;
use std::sync::OnceLock;
#[derive(Deserialize)]
struct Catalogue {
    houses: Vec<Record>,
}
#[derive(Deserialize)]
struct Record {
    identity: Identity,
    kind: CatalogueKind,
    town: Town,
    provenance: Provenance,
}
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum CatalogueKind {
    PrivateHouse,
    Shop,
    Guildhall,
}
impl CatalogueKind {
    fn database(self) -> &'static str {
        match self {
            Self::PrivateHouse => "private_house",
            Self::Shop => "shop",
            Self::Guildhall => "guildhall",
        }
    }
}
#[derive(Deserialize)]
struct Identity {
    key: String,
}
#[derive(Deserialize)]
struct Town {
    key: String,
}
#[derive(Deserialize)]
struct Provenance {
    engine_house: EngineHouse,
}
#[derive(Deserialize)]
struct EngineHouse {
    house_id: u32,
}
pub(super) fn source_house_id(key: &str) -> Option<u32> {
    catalogue()?
        .iter()
        .find(|house| house.identity.key == key)
        .map(|house| house.provenance.engine_house.house_id)
        .filter(|id| *id != 0)
}
pub(super) fn source_house_kind(key: &str) -> Option<CatalogueKind> {
    catalogue()?
        .iter()
        .find(|house| house.identity.key == key)
        .map(|house| house.kind)
}
fn catalogue() -> Option<&'static Vec<Record>> {
    static CATALOGUE: OnceLock<Option<Vec<Record>>> = OnceLock::new();
    CATALOGUE
        .get_or_init(|| {
            let mut all = Vec::new();
            for body in [
                include_str!("../../../../../content/houses/houses-00000-00499.json"),
                include_str!("../../../../../content/houses/houses-00500-00994.json"),
            ] {
                let parsed: Catalogue = serde_json::from_str(body).ok()?;
                if parsed.houses.len() > 500 {
                    return None;
                }
                all.extend(parsed.houses);
            }
            (all.len() <= 1024).then_some(all)
        })
        .as_ref()
}
pub(super) async fn dispatch(
    authority: &impl FreshAdmissionAuthority,
    actor: ExactActorRef,
    command: UseCommand,
    body: &[u8],
) -> Option<Vec<u8>> {
    let Ok(query) = wire::decode_house_query(body) else {
        return Some(wire::encode_house_result(D::Rejected, 0));
    };
    authority.house_query(actor, command, query).await
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn query_houses(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        query: wire::HouseQuery,
    ) -> Option<Vec<u8>> {
        let refuse = || Some(wire::encode_house_result(D::Rejected, 0));
        let Some(fence) = command.item_fence else {
            return refuse();
        };
        if *self.world_id.as_bytes() != query.world
            || command.game_session_id != fence.game_session_id
            || fence.runtime_scope.world_id() != self.world_id
        {
            return refuse();
        }
        let content_digest = {
            let runtime = self.runtime.lock().await;
            let Ok(lease) = runtime.bound_attacker_lease(actor, command.game_session_id) else {
                return refuse();
            };
            if runtime.owner_fence().is_err()
                || lease.character_id().as_bytes() != fence.character_id.as_bytes()
                || lease.generation() != fence.character_lease_generation
                || !runtime
                    .player_control_facts(actor, command.game_session_id)
                    .is_ok_and(|facts| facts.control_loss.is_none())
            {
                return refuse();
            }
            runtime.content_pin().server_artifact_digest()
        };
        let houses = catalogue()?
            .iter()
            .filter(|h| h.town.key == query.town)
            .map(|h| crate::durability::house_management::QualifiedHouse {
                key: h.identity.key.clone(),
                kind: h.kind.database(),
            })
            .collect();
        match self
            .root
            .read_house_management(
                self.character,
                self.holder,
                fence,
                houses,
                query.cursor,
                content_digest,
            )
            .await
        {
            Ok(mut rows) => {
                let more = rows.len() > wire::PAGE_SIZE;
                rows.truncate(wire::PAGE_SIZE);
                let next = more.then(|| rows.last().map(|r| r.house.clone())).flatten();
                let rows: Vec<_> = rows
                    .into_iter()
                    .map(|r| {
                        Some(wire::HouseSummary {
                            house: r.house,
                            kind: match r.kind.as_str() {
                                "private_house" => wire::HouseKind::PrivateHouse,
                                "shop" => wire::HouseKind::Shop,
                                "guildhall" => wire::HouseKind::Guildhall,
                                _ => return None,
                            },
                            state: r.state,
                            rent: r.rent,
                            revision: r.revision,
                            acl_revision: r.acl_revision,
                            owned: r.owned,
                            move_out_ms: r.move_out_ms,
                            price: r.price,
                            ends_ms: r.ends_ms,
                            own_maximum: r.own_maximum,
                        })
                    })
                    .collect::<Option<Vec<_>>>()?;
                wire::encode_house_page(&rows, next.as_deref()).ok()
            }
            Err(error) => super::house_intent::reply(Err(error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_catalogue_retains_all_three_global_building_classes() {
        assert!(catalogue().is_some(), "shipped catalogue parses");
        let empty = Vec::new();
        let catalogue = catalogue().unwrap_or(&empty);
        assert_eq!(
            catalogue
                .iter()
                .filter(|h| h.kind == CatalogueKind::Guildhall)
                .count(),
            66
        );
        assert_eq!(
            catalogue
                .iter()
                .filter(|h| h.kind == CatalogueKind::Shop)
                .count(),
            51
        );
        assert_eq!(
            source_house_kind("oteryn:content.house.zz_kept_key"),
            Some(CatalogueKind::Guildhall)
        );
    }
}
