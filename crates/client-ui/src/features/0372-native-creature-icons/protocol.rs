//! Native icon data remains attached to its exact entity identity and generation.
pub fn project(
    entity: &oteryn_client_runtime::WorldSpatialEntity,
) -> &oteryn_client_runtime::WorldSpatialEntity {
    entity
}
/// Source icon identity/counts use native glyph labels until an icon atlas is qualified.
pub fn labels(entity: &oteryn_client_runtime::WorldSpatialEntity) -> Vec<String> {
    use oteryn_client_runtime::attack::EntityDetail;
    match entity.detail {
        EntityDetail::ActorWithIcons { icons, .. } => icons
            .iter()
            .map(|icon| {
                let name = match (icon.category, icon.id) {
                    (0, 2) => "Cross",
                    (0, 17) => "Up",
                    (0, 22) => "Hazard",
                    (1, 6) => "ReducedHealth",
                    (1, 7) => "ReducedHealth!",
                    _ => "Unknown",
                };
                format!("[{name} {}]", icon.count)
            })
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
#[test]
fn preserves_entity_identity() {
    let entity = oteryn_client_runtime::WorldSpatialEntity {
        entity: oteryn_client_runtime::combat::EntityRef {
            identity: [9; 16],
            generation: 2,
        },
        kind: oteryn_client_runtime::attack::EntityKind::Creature,
        position: oteryn_client_runtime::attack::ActorPosition {
            x: 1,
            y: 2,
            floor: 0,
        },
        detail: oteryn_client_runtime::attack::EntityDetail::Actor {
            direction: oteryn_client_runtime::attack::StepDirection::North,
            appearance_ref: 1,
            health_percent: 100,
        },
        display_name: None,
    };
    assert_eq!(project(&entity), &entity);
}
