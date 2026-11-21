//! Native Terrified Elephant death schedules the complete source corpse chain.
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use super::fixture::{Harness, debug};

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

pub(super) const NAME: &str = "qengine-rewards-terrified-elephant-corpse-decay";
const CREATURE: &str = "oteryn:creature.terrified_elephant";

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("active native policies")?
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    super::qengine_kill_stories_fixtures::death_actor(owner, admitted, CREATURE, 60000).await?;
    super::m25_corpse_chain::assert_native_corpse_chain(&h.pool, CREATURE).await
}
