use super::*;
pub fn row_text(entity: &WorldSpatialEntity) -> String {
    let health = match entity.detail {
        oteryn_client_runtime::attack::EntityDetail::Actor { health_percent, .. } => health_percent,
        _ => 0,
    };
    let identity: String = entity
        .entity
        .identity
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    format!(
        "{:?} {}:{} — {}% ({}, {}, {})",
        entity.kind,
        identity,
        entity.entity.generation,
        health,
        entity.position.x,
        entity.position.y,
        entity.position.floor
    )
}
