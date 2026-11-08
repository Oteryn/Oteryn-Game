use super::*;
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{UseDisposition,WorldObjectTarget};
pub(super) async fn quantity(h: &Harness, item: u32) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1").bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}
pub(super) async fn use_at(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    p: Position,
    expected: UseDisposition,
) -> TestResult {
    let ordinal = u8::from((p.x, p.y, p.floor) != (32408, 32107, -10));
    use_at_ordinal(owner, admitted, p, ordinal, expected).await
}
pub(super) async fn use_at_ordinal(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    p: Position,
    ordinal: u8,
    expected: UseDisposition,
) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let map = owner.bundle_map().ok_or("map")?;
    let key = oteryn_world_bundle::bundle::placement_key(p.floor, p.x, p.y, ordinal)
        .ok_or("source placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    };
    let payload =
        oteryn_protocol_oteryn::world_object::encode_use_intent(&target).map_err(debug)?;
    let id = admitted.continuity.next_command_id;
    let result =
        super::super::quest_world_callbacks::wire(owner, admitted, id, 2, payload.clone()).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&result).map_err(debug)?,
        expected
    );
    assert_eq!(
        super::super::quest_world_callbacks::wire(owner, admitted, id, 2, payload).await?,
        result,
        "world command replay"
    );
    Ok(())
}
