use super::fixture::debug;
use super::soul_war_fixtures::current;
use super::{ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::AdmittedSession;
use oteryn_gameplay_encounters::bosses::endgame_entry::EntryRefusal;
use oteryn_protocol_oteryn::boss_encounter::encode_boss_key;

#[allow(
    dead_code,
    reason = "only the shards containing the W11 chain cases use this fixture"
)]
pub(super) async fn prerequisite(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    key: &str,
) -> TestResult<Result<(), EntryRefusal>> {
    let fence = current(owner, admitted.game_session_id).await?;
    admitted.item_fence = Some(fence.item_fence());
    admitted.continuity.connection_generation = fence.connection_generation.get();
    owner
        .endgame_boss_entry(admitted, &encode_boss_key(key).map_err(debug)?)
        .await
        .map_err(debug)
}
