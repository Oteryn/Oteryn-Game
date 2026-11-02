//! Hunt journey over the real node; reference: `scenarios/s1_hunt.rs`.
use super::*;
pub(super) async fn hunt(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    let moved = client
        .step(oteryn_protocol_oteryn::world_spatial::StepDirection::West)
        .await?;
    if moved.disposition != oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved {
        return Err("hunt setup movement was refused".into());
    }
    let monster = client.world_entities().and_then(|entities| {
        entities
            .iter()
            .find(|entity| entity.kind == oteryn_dev_client::EntityKind::Creature)
    });
    if monster.is_none() {
        evidence(
            "mechanic=hunt move=passed gap=native_entry_room_has_no_fixture_monster attack_kill_loot_experience_stamina=pending",
        );
        return Ok(());
    }
    Err(
        "hunt fixture monster is visible but the node_boot phase has no deterministic kill loop"
            .into(),
    )
}
