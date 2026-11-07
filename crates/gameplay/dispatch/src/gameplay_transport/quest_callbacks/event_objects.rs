//! Declarative projections of shared World objects; native USE stays fenced by its callback.
use super::*;
use crate::durability::world_event::changes::timed::{EventKind, EventState, Phase};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    schema: u8,
    bindings: Vec<serde_json::Value>,
    #[serde(default)]
    projections: Vec<Projection>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Projection {
    event: EventKind,
    member: u16,
    position: [u16; 3],
    appearance: u16,
    condition: String,
    #[serde(default)]
    water_bucket: bool,
}
fn policies() -> Option<&'static [Projection]> {
    static DATA: OnceLock<Option<Vec<Projection>>> = OnceLock::new();
    DATA.get_or_init(|| {
        let policy: Policy = serde_json::from_str(include_str!(
            "../../../../../../content/quests/world-event-object-policy.json"
        ))
        .ok()?;
        if policy.schema != 1
            || policy.bindings.len() > 64
            || policy.projections.len() > 64
            || policy.projections.iter().any(|p| {
                p.position[2] > 15
                    || p.appearance == 0
                    || !matches!(p.condition.as_str(), "active" | "lit")
            })
        {
            return None;
        }
        Some(policy.projections)
    })
    .as_deref()
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn project_shared_event_objects(&self, map: &BundleMap) {
        let Some(policies) = policies() else {
            return;
        };
        let mut kinds = Vec::new();
        for p in policies {
            if !kinds.contains(&p.event) {
                kinds.push(p.event);
            }
        }
        for kind in kinds {
            let state = match self
                .root
                .timed_world_event(self.world_id, self.reset_epoch, kind)
                .await
            {
                Ok(state) => state,
                Err(_) => continue,
            };
            let objects: Vec<_> = policies
                .iter()
                .filter(|p| p.event == kind)
                .map(|p| {
                    let active = state.as_ref().is_some_and(|s| s.phase == Phase::Active);
                    let object = state
                        .as_ref()
                        .and_then(|s: &EventState| s.lifecycle.objects.get(&p.member));
                    crate::map::facts::EventObjectProjection {
                        position: TilePos {
                            x: p.position[0],
                            y: p.position[1],
                            floor: -(p.position[2] as i8),
                        },
                        appearance: p.appearance,
                        visible: active
                            && (p.condition == "active" || object.is_some_and(|o| o.lit())),
                        sub_type: u16::from(
                            p.water_bucket && object.is_some_and(|o| o.units() > 0),
                        ),
                    }
                })
                .collect();
            map.facts
                .project_event_objects(map.overlay.base(), kind.key(), &objects);
        }
    }
}
